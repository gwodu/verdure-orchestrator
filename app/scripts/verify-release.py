#!/usr/bin/env python3
"""Assert that a Verdure macOS release is what we promised a user it is.

    python3 app/scripts/verify-release.py --dir  <bundle dir>     # in CI, pre-upload
    python3 app/scripts/verify-release.py --url  <release URL>    # after publishing

Why this exists, and why it is shaped this way.

Three defects reached artifacts in one session (2026-08-28) while every check we
had reported success. They had two root causes, and this script is built around
both:

  A. CHECKS THAT PASS BY FINDING NOTHING. A licence search returned zero because
     its base path was wrong; a crate-root scan missed `whisper.cpp/LICENSE` one
     level down. Both "succeeded". A search-based assertion with no positive
     control cannot distinguish "absent" from "my instrument is broken".
     → Every tool used here is first proved against a case known to fail.

  B. ASSERTIONS THAT DRIFT FROM THE ARTIFACT. Verification ran `spctl` on the
     .app while the .dmg — the thing a user actually double-clicks — shipped
     unnotarized. Everything asserted was true, about the wrong object.
     → This checks the .dmg AND the .app inside it, and `--url` mode re-downloads
       the published bytes so the whole chain is covered: build, sign, staple,
       upload, host.

The rule both cases point at: assert what the user receives, and prove the
instrument works before believing a pass.
"""

from __future__ import annotations

import argparse
import json
import plistlib
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

FAILURES: list[str] = []
CHECKS = 0


def check(name: str, ok: bool, detail: str = "") -> bool:
    global CHECKS
    CHECKS += 1
    print(f"  {'PASS' if ok else 'FAIL'}  {name}" + (f" — {detail}" if detail else ""))
    if not ok:
        FAILURES.append(name)
    return ok


def run(*args: str) -> tuple[int, str]:
    p = subprocess.run(args, capture_output=True, text=True)
    return p.returncode, (p.stdout + p.stderr)


# --- positive controls: prove the instruments work before trusting them -------


def prove_gatekeeper_can_reject(tmp: Path) -> None:
    """`spctl` must reject something we know is bad.

    Without this, a blanket "accepted" could equally mean Gatekeeper is
    disabled on the runner (`spctl --status` off) — in which case every
    signature check silently passes and tells us nothing.
    """
    fake = tmp / "control.app" / "Contents" / "MacOS"
    fake.mkdir(parents=True, exist_ok=True)
    (fake / "control").write_bytes(b"#!/bin/sh\necho hi\n")
    (fake / "control").chmod(0o755)
    (tmp / "control.app" / "Contents" / "Info.plist").write_bytes(
        plistlib.dumps({"CFBundleExecutable": "control", "CFBundleIdentifier": "ai.verdure.control"})
    )
    code, _ = run("spctl", "--assess", "--type", "execute", str(tmp / "control.app"))
    check(
        "control: Gatekeeper rejects an unsigned app",
        code != 0,
        "if this fails, spctl is not enforcing and every verdict below is meaningless",
    )


def prove_strings_can_find(binary: Path) -> bool:
    """`strings` must find something we know is in the binary.

    The licence-search bug twice: a search that finds nothing reads exactly like
    a clean result. Tauri compresses embedded frontend assets, so a UI string is
    NOT a valid control — we use a symbol the linker always leaves behind.
    """
    code, out = run("strings", "-a", str(binary))
    found = "ggml" in out or "llama" in out
    check(
        "control: strings can read this binary",
        found,
        "a known symbol was located, so absence below means absent",
    )
    return found


# --- the actual contract -----------------------------------------------------


def verify_dmg(dmg: Path, tmp: Path) -> None:
    print(f"\n[dmg] {dmg.name}")
    # Simulate a browser download. Quarantine is what makes Gatekeeper assess at
    # all; without it a broken DMG can look fine locally.
    run("xattr", "-w", "com.apple.quarantine", "0081;00000000;Safari;", str(dmg))
    code, out = run("xcrun", "stapler", "validate", str(dmg))
    check("dmg is stapled (verifies offline)", code == 0, out.strip().splitlines()[-1] if out else "")
    code, out = run(
        "spctl", "--assess", "--type", "open",
        "--context", "context:primary-signature", "-vv", str(dmg),
    )
    check(
        "dmg passes Gatekeeper on double-click",
        code == 0 and "accepted" in out,
        next((l.strip() for l in out.splitlines() if "source=" in l), out.strip()[:80]),
    )

    mnt = tmp / "mnt"
    mnt.mkdir(exist_ok=True)
    code, out = run("hdiutil", "attach", "-nobrowse", "-quiet", str(dmg), "-mountpoint", str(mnt))
    if not check("dmg mounts", code == 0, out.strip()[:80]):
        return
    try:
        apps = list(mnt.glob("*.app"))
        if check("dmg contains an .app", bool(apps)):
            verify_app(apps[0])
    finally:
        run("hdiutil", "detach", str(mnt), "-quiet")


def verify_app(app: Path) -> None:
    print(f"\n[app] {app.name}")
    binary = next((app / "Contents" / "MacOS").iterdir(), None)
    if not check("app has an executable", binary is not None):
        return
    assert binary is not None

    code, out = run("codesign", "--verify", "--deep", "--strict", str(app))
    check("code signature intact", code == 0, out.strip()[:80])
    code, out = run("xcrun", "stapler", "validate", str(app))
    check("app is stapled", code == 0)
    code, out = run("spctl", "--assess", "--type", "execute", "-vv", str(app))
    check(
        "app passes Gatekeeper on launch",
        code == 0 and "accepted" in out,
        next((l.strip() for l in out.splitlines() if "source=" in l), ""),
    )

    _, arch = run("file", str(binary))
    check(
        "universal binary (Apple Silicon + Intel)",
        "x86_64" in arch and "arm64" in arch,
        "an Intel-only build runs under Rosetta with NEON and Metal disabled",
    )

    notices = app / "Contents" / "Resources" / "THIRD-PARTY-NOTICES.md"
    check(
        "third-party notices bundled",
        notices.is_file() and notices.stat().st_size > 100_000,
        f"{notices.stat().st_size // 1024} KB" if notices.is_file() else "absent",
    )
    if notices.is_file():
        text = notices.read_text(errors="replace")
        check(
            "the engines we wrap are credited",
            "The ggml authors" in text,
            "llama.cpp / whisper.cpp copyright present",
        )

    if prove_strings_can_find(binary):
        _, out = run("strings", "-a", str(binary))
        check(
            "updater public key compiled in",
            "minisign public key" in out or "dW50cnVzdGVk" in out,
            "without it the app cannot verify any update",
        )
        # Distribution safety: nothing secret should ever be inside a shipped binary.
        import re

        leaks = [
            p for p in ("minisign secret", "BEGIN PRIVATE KEY", "BEGIN RSA PRIVATE")
            if p in out
        ] + (["api-key-like token"] if re.search(r"sk-[A-Za-z0-9]{20,}", out) else [])
        check("no secrets embedded in the binary", not leaks, ", ".join(leaks) or "clean")


def verify_latest_json(path: Path, sig: Path | None) -> None:
    print(f"\n[updater] {path.name}")
    try:
        data = json.loads(path.read_text())
    except Exception as e:  # noqa: BLE001
        check("latest.json parses", False, str(e))
        return
    check("latest.json parses", True)
    plat = data.get("platforms", {}).get("darwin-universal", {})
    check("declares darwin-universal", bool(plat), "covers both architectures")
    check("has a signature", bool(plat.get("signature")))
    check("has a download url", bool(plat.get("url")), plat.get("url", ""))
    if sig and sig.is_file():
        check(
            "signature matches the .sig file",
            plat.get("signature", "").strip() == sig.read_text().strip(),
            "a mismatch makes every client silently reject the update",
        )


def fetch(url: str, dest: Path) -> bool:
    try:
        with urllib.request.urlopen(url, timeout=180) as r, dest.open("wb") as f:
            shutil.copyfileobj(r, f)
        return True
    except Exception as e:  # noqa: BLE001
        check(f"download {Path(url).name}", False, str(e))
        return False


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--dir", type=Path, help="a local bundle dir (CI, before upload)")
    g.add_argument("--url", help="base URL of a published release (after publishing)")
    args = ap.parse_args()

    if sys.platform != "darwin":
        print("This verifies macOS artifacts and needs macOS tooling.")
        return 2

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        print("[controls]")
        prove_gatekeeper_can_reject(tmp)

        if args.url:
            base = args.url.rstrip("/")
            print(f"\nverifying the PUBLISHED bytes at {base}")
            dmg = tmp / "Verdure.dmg"
            lj = tmp / "latest.json"
            sig = tmp / "Verdure.app.tar.gz.sig"
            ok = fetch(f"{base}/Verdure_0.1.0_universal.dmg", dmg)
            fetch(f"{base}/latest.json", lj)
            fetch(f"{base}/Verdure.app.tar.gz.sig", sig)
            if ok:
                verify_dmg(dmg, tmp)
            if lj.is_file() and lj.stat().st_size:
                verify_latest_json(lj, sig if sig.is_file() and sig.stat().st_size else None)
        else:
            root = args.dir
            dmgs = list(root.rglob("*.dmg"))
            if check("a .dmg was produced", bool(dmgs)):
                verify_dmg(dmgs[0], tmp)
            ljs = list(root.rglob("latest.json"))
            sigs = list(root.rglob("*.app.tar.gz.sig"))
            if ljs:
                verify_latest_json(ljs[0], sigs[0] if sigs else None)

    print(f"\n{CHECKS - len(FAILURES)}/{CHECKS} checks passed")
    if FAILURES:
        print("FAILED:")
        for f in FAILURES:
            print(f"  - {f}")
        return 1
    print("This release is what we say it is.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
