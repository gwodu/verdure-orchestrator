#!/usr/bin/env python3
"""Generate THIRD-PARTY-NOTICES.md for everything compiled into Verdure.

Why this exists ([OQ-79]): MIT and Apache-2.0 let us use, modify and sell
software built on this code for free, on one condition — that the copyright
notice travels with the binaries we distribute. That obligation starts the
moment a .dmg reaches anyone but the founder. MPL-2.0 components are stronger
still: weak copyleft, so each needs a pointer to its source as well, which this
script generates for every MPL component it finds.

There is also a non-legal reason, which is the better one. knowledge/18 commits
Verdure to never building a model or engine — we wrap llama.cpp and whisper.cpp,
and they are the product's engine. Naming their authors is simply true.

Reads the real dependency graph via `cargo metadata` and the licence texts that
`cargo` has already vendored under ~/.cargo/registry/src, so it needs no network
and cannot drift from what actually compiled. Run it from anywhere:

    python3 app/scripts/generate-notices.py

Deliberately NOT wired into the build: a generated legal file that changes
silently is worse than one a human regenerated and read. Re-run it when
dependencies change; `cargo test` has a guard that fails if it goes stale.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
MANIFEST = REPO / "app" / "src-tauri" / "Cargo.toml"
OUT = REPO / "app" / "src-tauri" / "THIRD-PARTY-NOTICES.md"

# Licence files sit at a crate's root under these names.
LICENSE_NAMES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE")


def crate_license_texts(src: Path) -> list[tuple[str, str]]:
    """Every licence file in a crate, as (relative path, text).

    Searches TWO levels, not just the crate root. The `-sys` crates vendor whole
    upstream projects — `whisper-rs-sys` ships `whisper.cpp/LICENSE`, which a
    root-only scan silently misses. That miss is invisible: the file still
    generates, just without the attribution that matters most.
    """
    out = []
    if not src.is_dir():
        return out
    candidates = [f for f in src.iterdir()]
    for d in [f for f in src.iterdir() if f.is_dir() and not f.name.startswith(".")]:
        try:
            candidates.extend(d.iterdir())
        except OSError:
            continue
    for f in sorted(candidates):
        if not f.is_file():
            continue
        stem = f.name.upper()
        if any(stem.startswith(n) for n in LICENSE_NAMES):
            try:
                text = f.read_text(encoding="utf-8", errors="replace").strip()
            except OSError:
                continue
            if text:
                rel = f.relative_to(src)
                out.append((str(rel), text))
    return out


def main() -> int:
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(MANIFEST)],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )

    # Group crates by the exact licence text they ship, so the file lists each
    # distinct text once with everything it covers. Without this, ~360 near-identical
    # MIT texts would bury the handful that actually differ.
    by_text: dict[str, dict] = {}
    missing: list[str] = []
    packages = []

    for pkg in meta["packages"]:
        name, version = pkg["name"], pkg["version"]
        if name == "verdure-porter":  # ourselves
            continue
        packages.append((name, version, pkg.get("license") or "see notice"))
        src = Path(pkg["manifest_path"]).parent
        texts = crate_license_texts(src)
        if not texts:
            missing.append(f"{name} {version} ({pkg.get('license') or 'unknown'})")
            continue
        for fname, text in texts:
            key = hashlib.sha256(text.encode()).hexdigest()
            entry = by_text.setdefault(key, {"text": text, "file": fname, "crates": set()})
            entry["crates"].add(f"{name} {version}")

    # Licence texts the crates do not publish (see app/src-tauri/licenses/README.md).
    # llama.cpp is the chat engine and ships no LICENSE inside llama-cpp-sys-2, so
    # without this the most important attribution in the file would be absent.
    for extra in sorted((REPO / "app" / "src-tauri" / "licenses").glob("*-LICENSE")):
        text = extra.read_text(encoding="utf-8").strip()
        key = hashlib.sha256(text.encode()).hexdigest()
        entry = by_text.setdefault(key, {"text": text, "file": extra.name, "crates": set()})
        entry["crates"].add(f"{extra.name.replace('-LICENSE', '')} (vendored)")

    packages.sort()
    lines: list[str] = []
    lines.append("# Third-party notices\n")
    lines.append(
        "Verdure is built on open-source software. The components below are compiled\n"
        "into the application; their licences and copyright notices are reproduced in\n"
        "full, as those licences require.\n"
    )
    lines.append(
        "Two deserve naming directly, because they are the engine rather than a\n"
        "dependency: **llama.cpp** and **whisper.cpp** by Georgi Gerganov and\n"
        "contributors. Verdure runs local models by wrapping them; it does not\n"
        "implement inference itself.\n"
    )
    # MPL-2.0 is weak copyleft: each component needs a pointer to its source.
    # Derived from the resolved licences, so a new MPL dependency is covered
    # automatically (a hand-written line here once named only symphonia).
    mpl = [(n, v) for n, v, lic in packages if "MPL" in lic]
    if mpl:
        lines.append(
            "**MPL-2.0 components.** Verdure uses these unmodified; the source of each is\n"
            "published on crates.io:\n\n"
            + "".join(f"- `{n}` {v}: <https://crates.io/crates/{n}/{v}>\n" for n, v in mpl)
        )
    lines.append(
        f"\n_Generated by `app/scripts/generate-notices.py` from the resolved dependency\n"
        f"graph — {len(packages)} components. Re-run it when dependencies change._\n"
    )

    lines.append("\n## Components\n")
    lines.append("| Component | Version | Licence |")
    lines.append("|---|---|---|")
    for name, version, lic in packages:
        lines.append(f"| {name} | {version} | {lic} |")

    lines.append("\n## Licence texts\n")
    for entry in sorted(by_text.values(), key=lambda e: (-len(e["crates"]), sorted(e["crates"])[0])):
        crates = sorted(entry["crates"])
        shown = ", ".join(crates[:12]) + (f", and {len(crates) - 12} more" if len(crates) > 12 else "")
        lines.append(f"\n### {entry['file']} — applies to {len(crates)} component(s)\n")
        lines.append(f"_{shown}_\n")
        lines.append("```")
        lines.append(entry["text"])
        lines.append("```")

    if missing:
        lines.append("\n## Components without a bundled licence file\n")
        lines.append(
            "These ship no licence text in their published crate; the licence named in\n"
            "their manifest governs, and the canonical text is the standard one for that\n"
            "identifier.\n"
        )
        for m in sorted(missing):
            lines.append(f"- {m}")

    OUT.write_text("\n".join(lines) + "\n")
    print(f"wrote {OUT.relative_to(REPO)}")
    print(f"  components: {len(packages)}")
    print(f"  distinct licence texts: {len(by_text)}")
    print(f"  without bundled text: {len(missing)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
