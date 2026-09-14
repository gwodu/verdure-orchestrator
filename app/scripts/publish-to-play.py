#!/usr/bin/env python3
"""Upload a signed .aab to Google Play and assign it to a track.

WHY THIS EXISTS, AND WHY IT IS NOT A THIRD-PARTY ACTION
-------------------------------------------------------
iOS uploads to TestFlight automatically on a `release-N` tag, and the pipeline's
stated property is "one tag -> three platforms" (KNOWLEDGE.md, 2026-08-12).
Android was the one platform that broke it: the .aab was published as a GitHub
artifact for a human to download and upload by hand.

The usual fix is a marketplace action. We do not use one, because this step
handles a credential that can publish to the founder's Play account on their
behalf. A third-party action would receive that key on every run, and a single
compromised release of it would be enough. Talking to the API directly is ~100
lines, has no supply-chain surface beyond Google's own client library, and keeps
the credential inside code we own (knowledge/06 - trust is the product).

FOUNDER-SIDE SETUP (once) - see knowledge/11-platform-deployment.md:
  1. Google Cloud console -> create/choose a project -> enable the
     "Google Play Android Developer API".
  2. Create a service account; create a JSON key for it.
  3. Play Console -> Users and permissions -> Invite the service account's
     email -> grant "Release to testing tracks" (and "Release to production"
     only if you want that automated too) for THIS app.
  4. Add the whole JSON file as the GitHub secret PLAY_SERVICE_ACCOUNT_JSON.

THE ONE THING THIS CANNOT DO: the FIRST bundle of a brand-new app must be
uploaded through the Play Console by hand - Google's API refuses it. Every
upload after that is automated by this script.

Usage:
  python3 app/scripts/publish-to-play.py \
      --aab verdure-release.aab --package ai.verdure.app --track alpha

`alpha` is Play's CLOSED track. Internal testing does NOT count toward the
12-testers/14-days production requirement ([OQ-66]); closed testing does.
"""

import argparse
import json
import os
import sys

API = "https://androidpublisher.googleapis.com"
SCOPE = "https://www.googleapis.com/auth/androidpublisher"
# Play's own track names. "alpha" is the closed track; "internal" deliberately
# omitted from the default because it does not count toward production access.
TRACKS = ("internal", "alpha", "beta", "production")


def parse_args(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--aab", default="", help="path to the signed .aab (not needed with --verify)")
    p.add_argument("--package", required=True, help="applicationId, e.g. ai.verdure.app")
    p.add_argument("--track", default="alpha", choices=TRACKS)
    p.add_argument(
        "--status",
        default="completed",
        choices=("draft", "inProgress", "halted", "completed"),
        help="release status on the track",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="validate inputs and exit without touching Play",
    )
    p.add_argument(
        "--verify",
        action="store_true",
        help=(
            "check the credential works, then change nothing: authenticate, open "
            "an edit, delete it. Proves the service account can reach THIS app "
            "before a real release depends on it."
        ),
    )
    return p.parse_args(argv)


def validate(args):
    """Fail on the predictable mistakes BEFORE opening an edit on Play.

    An edit left dangling because we aborted halfway is harmless but confusing,
    and a 0-byte .aab produces an opaque server-side error rather than a clear
    local one.
    """
    if args.verify:
        # A credential check needs no bundle; requiring one would mean building
        # the app just to answer "did the secret work?".
        return 0
    if not os.path.isfile(args.aab):
        raise SystemExit(f"no such .aab: {args.aab}")
    size = os.path.getsize(args.aab)
    if size < 1_000_000:
        raise SystemExit(
            f"{args.aab} is only {size} bytes - that is not a real app bundle. "
            "Refusing to upload; check the build step produced the .aab."
        )
    if not args.package.count(".") >= 1 or args.package.endswith("."):
        raise SystemExit(f"implausible package name: {args.package!r}")
    return size


def main(argv=None):
    args = parse_args(argv)
    size = validate(args)
    if args.verify:
        print(f"verifying credential against {args.package} - nothing will change")
    else:
        if not args.aab:
            raise SystemExit("--aab is required unless --verify is given")
        print(f"bundle: {args.aab} ({size / 1_048_576:.1f} MiB)")
        print(f"package: {args.package}  track: {args.track}  status: {args.status}")

    if args.dry_run:
        print("dry run - not contacting Play")
        return 0

    # Imported here so --dry-run works without the dependency installed, which
    # is what makes this script unit-testable on a machine with no credentials.
    from google.oauth2 import service_account
    from google.auth.transport.requests import AuthorizedSession

    raw = os.environ.get("PLAY_SERVICE_ACCOUNT_JSON")
    if not raw:
        raise SystemExit("PLAY_SERVICE_ACCOUNT_JSON is empty - see this file's header.")
    creds = service_account.Credentials.from_service_account_info(
        json.loads(raw), scopes=[SCOPE]
    )
    http = AuthorizedSession(creds)
    base = f"{API}/androidpublisher/v3/applications/{args.package}"

    def check(r, what):
        if not r.ok:
            raise SystemExit(f"{what} failed: HTTP {r.status_code}\n{r.text}")
        return r.json() if r.content else {}

    edit = check(http.post(f"{base}/edits"), "opening an edit")
    eid = edit["id"]
    print(f"edit: {eid}")

    if args.verify:
        # An edit is a transaction: opening one proves the service account
        # authenticated AND has permission on this specific package, which is the
        # part that actually goes wrong (the key is fine, but nobody invited it in
        # Play Console). Deleting it leaves no trace - nothing is committed.
        check(http.delete(f"{base}/edits/{eid}"), "discarding the edit")
        print(
            f"OK: the service account can open an edit on {args.package}. "
            "Credential and permissions are good; nothing was changed."
        )
        return 0

    with open(args.aab, "rb") as fh:
        up = check(
            http.post(
                f"{API}/upload/androidpublisher/v3/applications/{args.package}/edits/{eid}/bundles",
                params={"uploadType": "media"},
                headers={"Content-Type": "application/octet-stream"},
                data=fh,
            ),
            "uploading the bundle",
        )
    version = up["versionCode"]
    print(f"uploaded versionCode {version}")

    check(
        http.put(
            f"{base}/edits/{eid}/tracks/{args.track}",
            json={
                "track": args.track,
                "releases": [{"versionCodes": [str(version)], "status": args.status}],
            },
        ),
        f"assigning to {args.track}",
    )
    check(http.post(f"{base}/edits/{eid}:commit"), "committing the edit")
    print(f"published versionCode {version} to {args.track}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
