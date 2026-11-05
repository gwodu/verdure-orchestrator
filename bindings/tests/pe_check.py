#!/usr/bin/env python3
"""Inspect the Windows library a vendor receives — the file, not the intention
(skills/testing.md, "Verifying artifacts"). Standard library only, so it needs
no Visual Studio prompt and its logic is tested on any machine.

What it asserts about `verdure.dll`:

  * it is a 64-bit x86 DLL;
  * it exports the SDK's API (the `uniffi_verdure_*` symbols bindings call);
  * it needs no Microsoft C++ runtime to be installed: no VCRUNTIME140,
    MSVCP140 or VCOMP140 among its imports (a PC without the redistributable
    fails to load such a library with an error that names none of this);
  * both engines are inside it, and were compiled for ordinary PCs: the engine
    builds' CMake caches say "not for the build machine's processor", AVX2 on,
    AVX-512 off — the level `verdure-hw::x86_engine_shortfall` refuses below.

Every "not found" assertion has a control that proves the instrument can find
things: a system DLL must parse and show imports and exports; an engine-less
build of the same library must lack the engine strings the real one has.

    python bindings/tests/pe_check.py --dll verdure.dll --build-dir <target>/release/build \
        --engineless <debug verdure.dll> --control C:/Windows/System32/kernel32.dll
"""
from __future__ import annotations

import argparse
import re
import struct
import sys
from pathlib import Path

MACHINES = {0x8664: "x64", 0x14C: "x86", 0xAA64: "arm64"}

# Runtime libraries that are NOT part of Windows: a PC only has them if some
# installer put them there. (ucrtbase / api-ms-win-crt-* ship with Windows 10
# and later, so they are reported but allowed.)
REDISTRIBUTABLE = re.compile(r"^(vcruntime\d+(_\d+)?|msvcp\d+(_\w+)?|vcomp\d+|concrt\d+|vccorlib\d+|libomp\w*|libgomp\S*|libstdc\+\+\S*|libgcc\S*|libwinpthread\S*)\.dll$")

ENGINE_STRINGS = {"whisper.cpp": b"whisper_init_with_params_no_state", "llama.cpp": b"llama_model_loader"}


class NotPE(Exception):
    pass


def parse_pe(data: bytes) -> dict:
    """Machine, kind, imported DLL names and exported symbol names of a PE file."""
    if data[:2] != b"MZ" or len(data) < 0x40:
        raise NotPE("no MZ header")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe : pe + 4] != b"PE\0\0":
        raise NotPE("no PE signature")
    machine, n_sections, _, _, _, opt_size, characteristics = struct.unpack_from("<HHIIIHH", data, pe + 4)
    opt = pe + 24
    magic = struct.unpack_from("<H", data, opt)[0]
    if magic not in (0x10B, 0x20B):
        raise NotPE(f"unknown optional header magic {magic:#x}")
    plus = magic == 0x20B
    subsystem = struct.unpack_from("<H", data, opt + 68)[0]
    dirs_at = opt + (112 if plus else 96)
    n_dirs = struct.unpack_from("<I", data, dirs_at - 4)[0]
    dirs = [struct.unpack_from("<II", data, dirs_at + 8 * i) for i in range(min(n_dirs, 16))]
    sections = []
    at = opt + opt_size
    for i in range(n_sections):
        vsize, vaddr, rsize, raddr = struct.unpack_from("<IIII", data, at + 40 * i + 8)
        sections.append((vaddr, max(vsize, rsize), raddr))

    def off(rva: int) -> int | None:
        for vaddr, size, raddr in sections:
            if vaddr <= rva < vaddr + size:
                return rva - vaddr + raddr
        return None

    def cstr(rva: int) -> str:
        o = off(rva)
        if o is None:
            return ""
        return data[o : data.index(b"\0", o)].decode("latin-1")

    def directory(i: int) -> int | None:
        return off(dirs[i][0]) if i < len(dirs) and dirs[i][0] else None

    imports: list[str] = []
    o = directory(1)  # import table: 20-byte descriptors, name RVA at +12
    while o is not None:
        name_rva = struct.unpack_from("<I", data, o + 12)[0]
        if not any(data[o : o + 20]):
            break
        imports.append(cstr(name_rva).lower())
        o += 20
    delayed: list[str] = []
    o = directory(13)  # delay-load table: 32-byte descriptors, name RVA at +4
    while o is not None:
        if not any(data[o : o + 32]):
            break
        delayed.append(cstr(struct.unpack_from("<I", data, o + 4)[0]).lower())
        o += 32
    exports: list[str] = []
    o = directory(0)
    if o is not None:
        n_names, names_rva = struct.unpack_from("<I", data, o + 24)[0], struct.unpack_from("<I", data, o + 32)[0]
        names = off(names_rva)
        if names is not None:
            exports = [cstr(struct.unpack_from("<I", data, names + 4 * i)[0]) for i in range(n_names)]
    return {
        "machine": MACHINES.get(machine, hex(machine)),
        "is_dll": bool(characteristics & 0x2000),
        "pe32plus": plus,
        "subsystem": {2: "gui", 3: "console"}.get(subsystem, str(subsystem)),
        "imports": imports,
        "delay_imports": delayed,
        "exports": exports,
    }


def redistributable_imports(info: dict) -> list[str]:
    """Imported libraries a clean Windows PC does not have."""
    return sorted({d for d in info["imports"] + info["delay_imports"] if REDISTRIBUTABLE.match(d)})


def library_problems(info: dict, min_api_exports: int = 20) -> list[str]:
    """Everything wrong with the SDK library as a vendor would meet it."""
    problems = []
    if info["machine"] != "x64" or not info["pe32plus"]:
        problems.append(f"not a 64-bit x86 file (machine {info['machine']})")
    if not info["is_dll"]:
        problems.append("not a DLL")
    api = [e for e in info["exports"] if e.startswith("uniffi_verdure_")]
    if len(api) < min_api_exports:
        problems.append(f"only {len(api)} uniffi_verdure_* exports (expected at least {min_api_exports}): the bindings would find nothing to call")
    bad = redistributable_imports(info)
    if bad:
        problems.append(f"needs a runtime Windows does not ship: {', '.join(bad)} — a PC without the redistributable cannot load the library")
    return problems


def cmake_cache_values(text: str) -> dict[str, str]:
    """`NAME:TYPE=VALUE` lines of a CMakeCache.txt as {NAME: VALUE}."""
    out = {}
    for line in text.splitlines():
        m = re.match(r"^([A-Za-z0-9_]+)(?::[A-Z]+)?=(.*)$", line.strip())
        if m:
            out[m.group(1)] = m.group(2)
    return out


def _on(v: str | None) -> bool:
    return (v or "").upper() in ("ON", "1", "TRUE", "YES")


def engine_build_problems(cache: dict[str, str], name: str) -> list[str]:
    """Was this engine compiled for ordinary PCs rather than the build machine?"""
    problems = []
    if "GGML_NATIVE" not in cache:
        problems.append(f"{name}: GGML_NATIVE is not in the CMake cache — this is not a ggml build, or the cache was not read")
    elif _on(cache["GGML_NATIVE"]):
        problems.append(f"{name}: built for the build machine's own processor (GGML_NATIVE=ON); it can crash on a customer's PC")
    if not _on(cache.get("GGML_AVX2")):
        problems.append(f"{name}: AVX2 is off (GGML_AVX2={cache.get('GGML_AVX2')}); the SDK's processor check and this build disagree")
    for k in ("GGML_AVX512", "GGML_AVX512_VBMI", "GGML_AVX512_VNNI", "GGML_AVX512_BF16", "GGML_AMX_TILE", "GGML_AMX_INT8", "GGML_AMX_BF16"):
        if _on(cache.get(k)):
            problems.append(f"{name}: {k} is on; most PCs lack it and the library would crash there")
    if _on(cache.get("GGML_OPENMP")):
        problems.append(f"{name}: OpenMP is on; the library would need vcomp140.dll")
    if _on(cache.get("BUILD_SHARED_LIBS")):
        problems.append(f"{name}: built as shared libraries; the engines would be separate DLLs the zip does not carry")
    return problems


def find_engine_caches(build_dir: Path) -> dict[str, Path]:
    """The CMake cache of each engine's build under cargo's `build/` folder."""
    found = {}
    for key, pattern in (("llama.cpp", "llama-cpp-sys-2-*/out/build/CMakeCache.txt"), ("whisper.cpp", "whisper-rs-sys-*/out/build/CMakeCache.txt")):
        hits = sorted(build_dir.glob(pattern), key=lambda p: p.stat().st_mtime)
        if hits:
            found[key] = hits[-1]
    return found


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dll", required=True)
    ap.add_argument("--exe", help="the developer CLI, checked for the same runtime independence")
    ap.add_argument("--build-dir", required=True, help="cargo's <target>/release/build folder")
    ap.add_argument("--engineless", required=True, help="the same library built WITHOUT engines (negative control)")
    ap.add_argument("--control", required=True, help="a system DLL that must parse (positive control), e.g. kernel32.dll")
    a = ap.parse_args()
    problems: list[str] = []

    # Positive control: the parser reads a DLL Windows itself ships.
    ctl = parse_pe(Path(a.control).read_bytes())
    if not (ctl["is_dll"] and ctl["exports"] and ctl["imports"]):
        print(f"::error::control {a.control} parsed to nothing useful ({len(ctl['exports'])} exports, {len(ctl['imports'])} imports): the instrument is broken")
        return 2
    print(f"control ok: {a.control}: {ctl['machine']}, {len(ctl['exports'])} exports, imports {ctl['imports'][:4]}…")

    dll_bytes = Path(a.dll).read_bytes()
    info = parse_pe(dll_bytes)
    print(f"{a.dll}: {len(dll_bytes) / 1e6:.1f} MB, {info['machine']}, {len(info['exports'])} exports, imports: {info['imports']} delay: {info['delay_imports']}")
    if "kernel32.dll" not in info["imports"]:
        problems.append("the library shows no import of kernel32.dll — every Windows library has one, so the import table was not read")
    problems += library_problems(info)

    less = Path(a.engineless).read_bytes()
    for engine, needle in ENGINE_STRINGS.items():
        if needle in less:
            problems.append(f"the engine-less control contains {engine}'s strings — the control proves nothing")
        if needle not in dll_bytes:
            problems.append(f"{engine} is not inside the library")

    caches = find_engine_caches(Path(a.build_dir))
    for engine in ENGINE_STRINGS:
        if engine not in caches:
            problems.append(f"no CMake cache found for {engine} under {a.build_dir}")
            continue
        cache = cmake_cache_values(caches[engine].read_text(encoding="utf-8", errors="replace"))
        shown = {k: cache.get(k) for k in ("GGML_NATIVE", "GGML_AVX", "GGML_AVX2", "GGML_AVX512", "GGML_FMA", "GGML_F16C", "GGML_BMI2", "GGML_OPENMP", "BUILD_SHARED_LIBS", "CMAKE_BUILD_TYPE", "CMAKE_MSVC_RUNTIME_LIBRARY")}
        print(f"{engine}: {caches[engine]}\n   {shown}")
        problems += engine_build_problems(cache, engine)

    if a.exe:
        exe = parse_pe(Path(a.exe).read_bytes())
        print(f"{a.exe}: {exe['machine']}, {exe['subsystem']}, imports: {exe['imports']}")
        if exe["machine"] != "x64" or exe["is_dll"]:
            problems.append("the CLI is not a 64-bit program")
        bad = redistributable_imports(exe)
        if bad:
            problems.append(f"the CLI needs a runtime Windows does not ship: {', '.join(bad)}")

    for p in problems:
        print(f"::error::{p}")
    print("library verified" if not problems else f"{len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
