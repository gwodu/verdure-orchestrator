#!/usr/bin/env python3
"""Copy Verdure's app icons into the freshly generated mobile projects.

WHY THIS EXISTS (the bug it fixes, found 2026-08-14):
`tauri ios init` / `tauri android init` scaffold `gen/` from Tauri's own
template, and that template ships Tauri's logo as the app icon. Because we
generate `gen/` fresh on every CI build and commit nothing under it, the icons in
`src-tauri/icons/` were never consulted — every build since June shipped Tauri's
yellow/cyan swirl on the home screen, on BOTH platforms. (`tauri icon` updates a
gen/ project that already exists; in CI the project is created afterwards, from
the template.)

So: after init, copy our icons in. Run this from the repo root.

It fails LOUDLY rather than skipping. A silently-wrong icon is what cost us six
builds; a red CI job costs one push.
"""

import json
import pathlib
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ICONS = ROOT / "app" / "src-tauri" / "icons"
GEN = ROOT / "app" / "src-tauri" / "gen"

problems: list[str] = []
copied = 0


def ios() -> int:
    """iOS: the AppIcon asset catalog inside the generated Xcode project."""
    global problems
    sets = list(GEN.glob("apple/**/AppIcon.appiconset"))
    if not sets:
        return 0  # no iOS project generated in this workflow — not an error
    n = 0
    for appiconset in sets:
        src_dir = ICONS / "ios"
        if not src_dir.is_dir():
            problems.append(f"missing source icons: {src_dir}")
            return 0
        for png in src_dir.glob("*.png"):
            shutil.copy2(png, appiconset / png.name)
            n += 1
        # The catalog is compiled by actool against Contents.json; if a name it
        # references is absent, Xcode silently falls back. Check every one.
        contents = appiconset / "Contents.json"
        if contents.is_file():
            spec = json.loads(contents.read_text())
            missing = [
                img["filename"]
                for img in spec.get("images", [])
                if img.get("filename") and not (appiconset / img["filename"]).is_file()
            ]
            if missing:
                problems.append(
                    f"{appiconset}: Contents.json references {len(missing)} file(s) we "
                    f"did not supply, e.g. {missing[:3]} — regenerate with "
                    f"`npx tauri icon` so the names line up"
                )
        else:
            problems.append(f"{appiconset}: no Contents.json — template layout changed?")
    return n


def android() -> int:
    """Android: mipmap-<density>/ic_launcher*.png in the generated Gradle project."""
    global problems
    res_dirs = list(GEN.glob("android/**/src/main/res"))
    if not res_dirs:
        return 0  # no Android project generated in this workflow — not an error
    n = 0
    for res in res_dirs:
        src_root = ICONS / "android"
        if not src_root.is_dir():
            problems.append(f"missing source icons: {src_root}")
            return 0
        for src_mipmap in sorted(src_root.glob("mipmap-*")):
            dest = res / src_mipmap.name
            if not dest.is_dir():
                # A density the template doesn't use: create it, Android picks
                # the closest match anyway.
                dest.mkdir(parents=True, exist_ok=True)
            for png in src_mipmap.glob("*.png"):
                shutil.copy2(png, dest / png.name)
                n += 1
    return n


copied = ios() + android()

if copied == 0:
    print(
        "install-icons: no generated mobile project found under "
        f"{GEN} — run this AFTER `tauri ios init` / `tauri android init`.",
        file=sys.stderr,
    )
    sys.exit(1)

if problems:
    for p in problems:
        print(f"::error::install-icons: {p}", file=sys.stderr)
    sys.exit(1)

print(f"install-icons: copied {copied} icon file(s) into the generated project(s)")
