"""Tests for verify-release.py's pure helpers. Run:
python3 -B -m unittest discover -s app/scripts

The macOS checks themselves (spctl, stapler, codesign) need a real signed bundle,
so they run in build-app.yml; what is tested here is the logic that decides
WHICH bytes get checked — the part that let a check pass by checking nothing.
"""

import importlib.util
import pathlib
import tempfile
import unittest

_spec = importlib.util.spec_from_file_location(
    "verify_release", pathlib.Path(__file__).with_name("verify-release.py")
)
vr = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(vr)


class DmgNameTests(unittest.TestCase):
    def test_name_follows_the_manifest_version(self):
        self.assertEqual(vr.dmg_name({"version": "0.2.0"}), "Verdure_0.2.0_universal.dmg")
        self.assertEqual(vr.dmg_name({"version": "v1.4.1"}), "Verdure_1.4.1_universal.dmg")

    def test_no_version_is_an_error_not_a_guess(self):
        with self.assertRaises(ValueError):
            vr.dmg_name({})


class FindLatestJsonTests(unittest.TestCase):
    def test_explicit_path_outside_the_bundle_is_used(self):
        # CI writes latest.json at the repo root, not in the bundle dir.
        with tempfile.TemporaryDirectory() as d:
            bundle = pathlib.Path(d, "bundle"); bundle.mkdir()
            manifest = pathlib.Path(d, "latest.json"); manifest.write_text("{}")
            self.assertEqual(vr.find_latest_json(bundle, manifest), manifest)

    def test_missing_explicit_path_is_none_so_the_check_fails(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertIsNone(vr.find_latest_json(pathlib.Path(d), pathlib.Path(d, "nope.json")))

    def test_search_finds_one_inside_the_bundle(self):
        with tempfile.TemporaryDirectory() as d:
            nested = pathlib.Path(d, "macos"); nested.mkdir()
            (nested / "latest.json").write_text("{}")
            self.assertEqual(vr.find_latest_json(pathlib.Path(d), None), nested / "latest.json")

    def test_nothing_found_is_none_not_a_silent_skip(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertIsNone(vr.find_latest_json(pathlib.Path(d), None))


if __name__ == "__main__":
    unittest.main()
