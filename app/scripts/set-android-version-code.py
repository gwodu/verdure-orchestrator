#!/usr/bin/env python3
"""Set the Android `versionCode` in tauri.conf.json before an Android build.

WHY THIS IS NOT PATCHED INTO THE GENERATED PROJECT
--------------------------------------------------
Google Play rejects a bundle whose versionCode has already been used, so every
upload needs a higher number than the last. An earlier attempt wrote the value
into `gen/android/app/tauri.properties` and failed in CI: that file is generated,
its path is not what the template text suggests, and it cannot be verified on a
machine without the Android SDK.

`bundle.android.versionCode` is Tauri's own supported field for exactly this, it
lives in a file we can read and test locally, and it survives regeneration of
gen/. Prefer the documented knob over patching someone else's output.

WHY NOT `autoIncrementVersionCode`: it increments by reading the PREVIOUS value
out of `tauri.properties`, which does not survive a CI runner. Every build would
compute the same number.

Left to itself Tauri derives the code from the app version
(`major*1000000 + minor*1000 + patch`, so 0.1.0 -> 1000). We override it with the
CI run number, which only ever increases and points back at the exact run that
produced a given bundle.

Usage:
  VERDURE_ANDROID_VERSION_CODE=42 python3 app/scripts/set-android-version-code.py
Unset, it does nothing - a local build keeps Tauri's semver-derived default.
"""

import json
import os
import pathlib
import sys

CONFIG = pathlib.Path("app/src-tauri/tauri.conf.json")
# Google Play's hard ceiling.
MAX_VERSION_CODE = 2_100_000_000


def main() -> int:
    raw = os.environ.get("VERDURE_ANDROID_VERSION_CODE")
    if not raw:
        print("VERDURE_ANDROID_VERSION_CODE unset - leaving Tauri's default in place")
        return 0

    if not raw.isdigit() or not 0 < int(raw) < MAX_VERSION_CODE:
        raise SystemExit(
            f"VERDURE_ANDROID_VERSION_CODE must be a positive integer below "
            f"{MAX_VERSION_CODE}, got {raw!r}"
        )
    code = int(raw)

    assert CONFIG.is_file(), f"expected the Tauri config at {CONFIG}"
    config = json.loads(CONFIG.read_text())
    bundle = config.setdefault("bundle", {})
    android = bundle.setdefault("android", {})
    android["versionCode"] = code
    # Keep the file formatted as Tauri writes it, so the diff is one line.
    CONFIG.write_text(json.dumps(config, indent=2) + "\n")

    # Read it back: a versionCode that silently failed to apply surfaces much
    # later, in Play, as "version code already used".
    check = json.loads(CONFIG.read_text())["bundle"]["android"]["versionCode"]
    assert check == code, f"versionCode did not apply: {check} != {code}"
    print(f"set bundle.android.versionCode = {code}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
