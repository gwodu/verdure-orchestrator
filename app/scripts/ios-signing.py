#!/usr/bin/env python3
"""Fetch a fresh App Store provisioning profile for Verdure's ONE distribution cert.

WHY THIS EXISTS
---------------
The TestFlight build used Xcode's automatic signing. That needs a certificate's
private key in the local keychain, and a GitHub runner's keychain is empty every
run, so Xcode minted a NEW certificate per build (a development one to archive,
a distribution one to export) and threw the key away with the runner. They piled
up on the account until Apple refused: "Your account has reached the maximum
number of certificates" (release-14, 2026-08-22).

Now the build reuses one Apple Distribution certificate, stored once as a GitHub
secret, and signs manually. This script does the part automatic signing used to
do: ask App Store Connect for an App Store profile that names THAT certificate.
It also makes sure the App ID carries the capabilities the app's entitlements
request, because a profile missing one fails the archive.

It creates no certificates, ever. It never sees the certificate's private key —
only the public certificate, to find its id on the account.

Like publish-to-play.py, this talks to Apple's API directly instead of through a
third-party action, so the App Store Connect key stays inside code we own. No
pip installs: the JWT is signed with the system `openssl`.

Usage (all secrets come from the environment, never argv):
  ASC_API_KEY_P8_PATH=... ASC_API_KEY_ID=... ASC_API_ISSUER_ID=... \\
  python3 app/scripts/ios-signing.py \\
      --bundle-id ai.verdure.porter.898PY37K5S \\
      --cert-pem /path/to/distribution-cert.pem \\
      --capability INCREASED_MEMORY_LIMIT --capability EXTENDED_VIRTUAL_ADDRESSING

Prints the profile's NAME on stdout (for PROVISIONING_PROFILE_SPECIFIER) and
installs it where Xcode looks.
"""

import argparse
import base64
import json
import os
import pathlib
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

API = "https://api.appstoreconnect.apple.com"
# One fixed name: each run deletes the previous profile by this name and makes a
# new one, so the account holds exactly one CI profile, always current with the
# App ID's capabilities. Profiles (unlike certificates) are cheap to replace.
PROFILE_NAME = "Verdure CI App Store"
# Where Xcode looks for installed profiles (the second is Xcode 16+).
PROFILE_DIRS = (
    "~/Library/MobileDevice/Provisioning Profiles",
    "~/Library/Developer/Xcode/UserData/Provisioning Profiles",
)


# --- JWT (ES256) without third-party packages -------------------------------

def _b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


def der_ecdsa_to_raw(der: bytes, size: int = 32) -> bytes:
    """openssl emits ECDSA signatures as ASN.1 DER (SEQUENCE { INTEGER r, INTEGER s });
    JWS ES256 wants the raw 64-byte r||s. Each INTEGER may carry a leading 0x00
    (sign padding) or be shorter than 32 bytes, so normalise both to `size`."""
    if len(der) < 8 or der[0] != 0x30:
        raise ValueError("not a DER ECDSA signature")
    i = 2
    if der[1] & 0x80:  # long-form length (never for P-256, but be strict)
        i = 2 + (der[1] & 0x7F)
    out = b""
    for _ in range(2):
        if der[i] != 0x02:
            raise ValueError("expected an INTEGER in the signature")
        n = der[i + 1]
        val = der[i + 2 : i + 2 + n].lstrip(b"\x00")
        if len(val) > size:
            raise ValueError("signature integer too long")
        out += val.rjust(size, b"\x00")
        i += 2 + n
    return out


def make_jwt(key_id: str, issuer: str, p8_path: str, now: int | None = None) -> str:
    now = int(time.time()) if now is None else now
    header = {"alg": "ES256", "kid": key_id, "typ": "JWT"}
    # 20 minutes is Apple's maximum lifetime for an ASC token.
    claims = {"iss": issuer, "iat": now, "exp": now + 1200, "aud": "appstoreconnect-v1"}
    signing_input = (
        _b64url(json.dumps(header, separators=(",", ":")).encode())
        + "."
        + _b64url(json.dumps(claims, separators=(",", ":")).encode())
    )
    der = subprocess.run(
        ["openssl", "dgst", "-sha256", "-sign", p8_path],
        input=signing_input.encode(),
        capture_output=True,
        check=True,
    ).stdout
    return signing_input + "." + _b64url(der_ecdsa_to_raw(der))


# --- App Store Connect ------------------------------------------------------

class Asc:
    """Minimal App Store Connect client. `send` is swappable so tests can fake it."""

    def __init__(self, token: str, send=None):
        self.token = token
        self.send = send or self._urllib_send

    def _urllib_send(self, method, url, body):
        req = urllib.request.Request(url, method=method, data=body)
        req.add_header("Authorization", f"Bearer {self.token}")
        if body is not None:
            req.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                raw = r.read()
                return r.status, (json.loads(raw) if raw else {})
        except urllib.error.HTTPError as e:
            raw = e.read()
            try:
                return e.code, json.loads(raw)
            except ValueError:
                return e.code, {"raw": raw.decode(errors="replace")}

    def call(self, method, path, payload=None, ok=(200, 201, 204)):
        body = json.dumps(payload).encode() if payload is not None else None
        status, data = self.send(method, API + path, body)
        if status not in ok:
            raise AscError(method, path, status, data)
        return status, data


class AscError(RuntimeError):
    def __init__(self, method, path, status, data):
        self.status = status
        detail = "; ".join(
            f"{e.get('title', '')}: {e.get('detail', '')}" for e in data.get("errors", [])
        ) or json.dumps(data)[:500]
        super().__init__(f"{method} {path} -> HTTP {status}: {detail}")


def find_bundle_id(asc: Asc, identifier: str) -> str:
    q = urllib.parse.urlencode({"filter[identifier]": identifier, "limit": 200})
    _, data = asc.call("GET", f"/v1/bundleIds?{q}")
    # The filter is not an exact match (it also returns longer identifiers that
    # share the prefix), so pick the exact one ourselves.
    for item in data.get("data", []):
        if item["attributes"]["identifier"] == identifier:
            return item["id"]
    raise SystemExit(f"No App ID '{identifier}' on this account.")


def ensure_capabilities(asc: Asc, bundle_id: str, wanted: list[str]) -> list[str]:
    """Enable each capability on the App ID if it isn't already. Returns the ones added."""
    _, data = asc.call("GET", f"/v1/bundleIds/{bundle_id}/bundleIdCapabilities?limit=200")
    have = {c["attributes"]["capabilityType"] for c in data.get("data", [])}
    added = []
    for cap in wanted:
        if cap in have:
            continue
        try:
            asc.call(
                "POST",
                "/v1/bundleIdCapabilities",
                {
                    "data": {
                        "type": "bundleIdCapabilities",
                        "attributes": {"capabilityType": cap},
                        "relationships": {"bundleId": {"data": {"type": "bundleIds", "id": bundle_id}}},
                    }
                },
            )
        except AscError as e:
            if e.status == 403:
                raise SystemExit(
                    f"The API key may not change App ID capabilities ({e}). Enable "
                    f"'{cap}' once by hand: developer.apple.com -> Identifiers -> this "
                    f"App ID -> Capabilities, then re-run."
                )
            raise
        added.append(cap)
    return added


def find_certificate_id(asc: Asc, cert_der: bytes) -> str:
    """The account's id for OUR certificate, matched on the exact DER bytes — not on
    name or serial — so a lookalike or a re-issued cert can never be picked."""
    q = urllib.parse.urlencode({"filter[certificateType]": "DISTRIBUTION,IOS_DISTRIBUTION", "limit": 200})
    _, data = asc.call("GET", f"/v1/certificates?{q}")
    for item in data.get("data", []):
        content = item["attributes"].get("certificateContent") or ""
        if content and base64.b64decode(content) == cert_der:
            return item["id"]
    raise SystemExit(
        "The certificate in IOS_DIST_CERT_P12 is not an active distribution "
        "certificate on this account (revoked or expired?). Export a current "
        "'Apple Distribution' certificate as .p12 and update the secret."
    )


def replace_profile(asc: Asc, bundle_id: str, cert_id: str, name: str = PROFILE_NAME) -> dict:
    q = urllib.parse.urlencode({"filter[name]": name, "limit": 200})
    _, data = asc.call("GET", f"/v1/profiles?{q}")
    for item in data.get("data", []):
        if item["attributes"]["name"] == name:
            asc.call("DELETE", f"/v1/profiles/{item['id']}")
    _, created = asc.call(
        "POST",
        "/v1/profiles",
        {
            "data": {
                "type": "profiles",
                "attributes": {"name": name, "profileType": "IOS_APP_STORE"},
                "relationships": {
                    "bundleId": {"data": {"type": "bundleIds", "id": bundle_id}},
                    "certificates": {"data": [{"type": "certificates", "id": cert_id}]},
                },
            }
        },
    )
    return created["data"]["attributes"]


def install_profile(attrs: dict, dirs=PROFILE_DIRS) -> list[pathlib.Path]:
    content = base64.b64decode(attrs["profileContent"])
    written = []
    for d in dirs:
        p = pathlib.Path(os.path.expanduser(d))
        p.mkdir(parents=True, exist_ok=True)
        f = p / f"{attrs['uuid']}.mobileprovision"
        f.write_bytes(content)
        written.append(f)
    return written


def pem_to_der(pem: str) -> bytes:
    body = "".join(
        line for line in pem.splitlines() if line and not line.startswith("-----")
    )
    if not body:
        raise SystemExit("--cert-pem holds no certificate")
    return base64.b64decode(body)


def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--bundle-id", required=True)
    p.add_argument("--cert-pem", required=True, help="the distribution certificate (public part, PEM)")
    p.add_argument("--capability", action="append", default=[], help="ASC capabilityType to ensure on the App ID")
    a = p.parse_args(argv)

    env = {k: os.environ.get(k, "") for k in ("ASC_API_KEY_P8_PATH", "ASC_API_KEY_ID", "ASC_API_ISSUER_ID")}
    missing = [k for k, v in env.items() if not v]
    if missing:
        raise SystemExit(f"missing environment: {', '.join(missing)}")

    cert_der = pem_to_der(pathlib.Path(a.cert_pem).read_text())
    asc = Asc(make_jwt(env["ASC_API_KEY_ID"], env["ASC_API_ISSUER_ID"], env["ASC_API_KEY_P8_PATH"]))
    bundle = find_bundle_id(asc, a.bundle_id)
    added = ensure_capabilities(asc, bundle, a.capability)
    if added:
        print(f"enabled on the App ID: {', '.join(added)}", file=sys.stderr)
    cert = find_certificate_id(asc, cert_der)
    attrs = replace_profile(asc, bundle, cert)
    for f in install_profile(attrs):
        print(f"installed {f}", file=sys.stderr)
    print(attrs["name"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
