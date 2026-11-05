"""Tests for pe_check.py: the rules are pure and run anywhere; the parser is
exercised on a real Windows DLL when one is at hand (the Rust standard library
for the Windows target, present wherever `rustup target add
x86_64-pc-windows-msvc` has run, and any DLL on a Windows machine)."""
import glob
import os
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pe_check  # noqa: E402


def a_real_dll():
    for pattern in (
        os.path.expanduser("~/.rustup/toolchains/*/lib/rustlib/x86_64-pc-windows-msvc/lib/std-*.dll"),
        r"C:\Windows\System32\kernel32.dll",
    ):
        hits = glob.glob(pattern)
        if hits:
            return hits[0]
    return None


def info(imports=(), exports=(), machine="x64", is_dll=True, delay=()):
    return {"machine": machine, "is_dll": is_dll, "pe32plus": machine != "x86", "subsystem": "console", "imports": list(imports), "delay_imports": list(delay), "exports": list(exports)}


API = [f"uniffi_verdure_fn_method_verdure_{i}" for i in range(30)]


class Rules(unittest.TestCase):
    def test_a_clean_library_has_no_problems(self):
        self.assertEqual(pe_check.library_problems(info(["kernel32.dll", "bcryptprimitives.dll", "api-ms-win-core-synch-l1-2-0.dll"], API)), [])

    def test_the_visual_cpp_runtime_is_named_as_the_problem(self):
        for dll in ("vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll", "msvcp140_atomic_wait.dll", "vcomp140.dll"):
            problems = pe_check.library_problems(info(["kernel32.dll", dll], API))
            self.assertEqual(len(problems), 1, dll)
            self.assertIn(dll, problems[0])
        # Delay-loaded counts too: it fails later, at first use, with the same error.
        self.assertTrue(pe_check.library_problems(info(["kernel32.dll"], API, delay=["vcomp140.dll"])))

    def test_what_windows_itself_ships_is_allowed(self):
        self.assertEqual(pe_check.redistributable_imports(info(["ucrtbase.dll", "api-ms-win-crt-runtime-l1-1-0.dll", "ws2_32.dll", "ntdll.dll"])), [])

    def test_a_library_without_the_api_or_of_the_wrong_kind_is_refused(self):
        self.assertTrue(any("exports" in p for p in pe_check.library_problems(info(["kernel32.dll"], API[:3]))))
        self.assertTrue(any("64-bit" in p for p in pe_check.library_problems(info(["kernel32.dll"], API, machine="arm64"))))
        self.assertTrue(any("not a DLL" in p for p in pe_check.library_problems(info(["kernel32.dll"], API, is_dll=False))))

    def test_cmake_cache_lines_are_read_with_or_without_a_type(self):
        cache = pe_check.cmake_cache_values("# comment\n//doc\nGGML_NATIVE:BOOL=OFF\nGGML_AVX2:BOOL=ON\nGGML_NATIVE_DEFAULT=ON\nCMAKE_BUILD_TYPE:STRING=Release\n\n")
        self.assertEqual(cache["GGML_NATIVE"], "OFF")
        self.assertEqual(cache["GGML_AVX2"], "ON")
        self.assertEqual(cache["CMAKE_BUILD_TYPE"], "Release")

    def test_an_engine_built_for_the_build_machine_is_refused(self):
        good = {"GGML_NATIVE": "OFF", "GGML_AVX2": "ON", "GGML_AVX512": "OFF", "GGML_OPENMP": "OFF", "BUILD_SHARED_LIBS": "OFF"}
        self.assertEqual(pe_check.engine_build_problems(good, "whisper.cpp"), [])
        self.assertTrue(any("build machine" in p for p in pe_check.engine_build_problems({**good, "GGML_NATIVE": "ON"}, "x")))
        self.assertTrue(any("AVX512" in p for p in pe_check.engine_build_problems({**good, "GGML_AVX512": "ON"}, "x")))
        self.assertTrue(any("AVX2 is off" in p for p in pe_check.engine_build_problems({**good, "GGML_AVX2": "OFF"}, "x")))
        self.assertTrue(any("vcomp140" in p for p in pe_check.engine_build_problems({**good, "GGML_OPENMP": "ON"}, "x")))
        self.assertTrue(any("shared" in p for p in pe_check.engine_build_problems({**good, "BUILD_SHARED_LIBS": "ON"}, "x")))
        # Microsoft's compiler with the optimiser lost (what the cmake crate does
        # to a Release build), and with it given back either way round.
        msvc = {**good, "CMAKE_GENERATOR": "Visual Studio 18 2026", "CMAKE_C_FLAGS": " -nologo -MT -Brepro", "CMAKE_C_FLAGS_RELEASE": " -nologo -MT -Brepro",
                "CMAKE_CXX_FLAGS": "/utf-8 -nologo -MT -Brepro", "CMAKE_CXX_FLAGS_RELEASE": "/utf-8 -nologo -MT -Brepro"}
        slow = pe_check.engine_build_problems(msvc, "whisper.cpp")
        self.assertEqual(len(slow), 2, slow)
        self.assertTrue(all("without optimisation" in p for p in slow))
        self.assertEqual(pe_check.engine_build_problems({**msvc, "CMAKE_C_FLAGS_RELEASE": "/MT /O2 /Ob2 /DNDEBUG", "CMAKE_CXX_FLAGS_RELEASE": "/MT /O2 /Ob2 /DNDEBUG"}, "x"), [])
        self.assertEqual(pe_check.engine_build_problems({**msvc, "CMAKE_C_FLAGS": "/O2 /DNDEBUG /Ob2 -nologo -MT", "CMAKE_CXX_FLAGS": "/O2 /DNDEBUG /Ob2 -nologo -MT"}, "x"), [])
        # "/Ob2" alone is inlining, not optimisation; "/Od" is off.
        self.assertTrue(pe_check.engine_build_problems({**msvc, "CMAKE_C_FLAGS_RELEASE": "/Ob2 /Od", "CMAKE_CXX_FLAGS_RELEASE": "/O2"}, "x"))
        # Other compilers' caches (no Visual Studio generator) are not judged by this rule.
        self.assertEqual(pe_check.engine_build_problems({**good, "CMAKE_C_FLAGS_RELEASE": "-O3 -DNDEBUG", "CMAKE_GENERATOR": "Unix Makefiles"}, "x"), [])
        # A cache that is not a ggml build at all must not pass by saying nothing.
        self.assertTrue(any("not in the CMake cache" in p for p in pe_check.engine_build_problems({}, "x")))


class Parser(unittest.TestCase):
    def test_a_file_that_is_not_a_windows_binary_is_refused(self):
        with self.assertRaises(pe_check.NotPE):
            pe_check.parse_pe(b"\x7fELF" + b"\0" * 100)
        with self.assertRaises(pe_check.NotPE):
            pe_check.parse_pe(b"MZ" + b"\0" * 200)

    @unittest.skipUnless(a_real_dll(), "no Windows DLL on this machine")
    def test_a_real_dll_is_read(self):
        got = pe_check.parse_pe(Path(a_real_dll()).read_bytes())
        self.assertEqual(got["machine"], "x64")
        self.assertTrue(got["is_dll"] and got["pe32plus"])
        self.assertGreater(len(got["exports"]), 50)
        self.assertTrue(all(e and e.isprintable() for e in got["exports"][:200]))
        self.assertTrue(got["imports"], "a DLL imports from somewhere")
        self.assertTrue(all(d.endswith(".dll") for d in got["imports"]), got["imports"])


if __name__ == "__main__":
    unittest.main()
