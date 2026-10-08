"""Tests for ios-signing.py. Run: python3 -m unittest app/scripts/test_ios_signing.py

Everything up to the network is exercised for real — including an ES256 JWT
signed by `openssl` with a throwaway key and verified back. App Store Connect
itself is faked; the first TestFlight run is the test of the live API.
"""

import base64
import importlib.util
import json
import pathlib
import subprocess
import tempfile
import unittest

_spec = importlib.util.spec_from_file_location(
    "ios_signing", pathlib.Path(__file__).with_name("ios-signing.py")
)
sig = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(sig)


def raw_to_der(raw: bytes) -> bytes:
    def integer(b: bytes) -> bytes:
        b = b.lstrip(b"\x00") or b"\x00"
        if b[0] & 0x80:
            b = b"\x00" + b
        return bytes([0x02, len(b)]) + b
    body = integer(raw[:32]) + integer(raw[32:])
    return bytes([0x30, len(body)]) + body


class FakeAsc:
    """Records calls and answers from a routing table of (method, path-prefix)."""

    def __init__(self, routes):
        self.routes = routes
        self.calls = []

    def __call__(self, method, url, body):
        path = url[len(sig.API):]
        self.calls.append((method, path, json.loads(body) if body else None))
        for (m, prefix), answer in self.routes.items():
            if m == method and path.startswith(prefix):
                return answer(path, body) if callable(answer) else answer
        return 404, {"errors": [{"title": "not routed", "detail": path}]}


class JwtTests(unittest.TestCase):
    def test_der_to_raw_pads_and_strips(self):
        r = b"\x00" + b"\x80" + b"\x11" * 31  # sign-padded 33 bytes
        s = b"\x22" * 30  # short: needs left-padding
        der = bytes([0x30, 2 + 33 + 2 + 30, 0x02, 33]) + r + bytes([0x02, 30]) + s
        raw = sig.der_ecdsa_to_raw(der)
        self.assertEqual(len(raw), 64)
        self.assertEqual(raw[:32], r[1:])
        self.assertEqual(raw[32:], b"\x00\x00" + s)

    def test_rejects_garbage(self):
        with self.assertRaises(ValueError):
            sig.der_ecdsa_to_raw(b"\x01\x02\x03\x04\x05\x06\x07\x08")

    def test_jwt_signature_verifies_with_openssl(self):
        with tempfile.TemporaryDirectory() as d:
            key = pathlib.Path(d, "k.p8")
            pub = pathlib.Path(d, "k.pub")
            subprocess.run(
                ["openssl", "ecparam", "-name", "prime256v1", "-genkey", "-noout", "-out", str(key)],
                check=True, capture_output=True,
            )
            subprocess.run(["openssl", "ec", "-in", str(key), "-pubout", "-out", str(pub)],
                           check=True, capture_output=True)
            token = sig.make_jwt("KID123", "issuer-uuid", str(key), now=1_000)
            head, claims, s = token.split(".")
            pad = lambda x: x + "=" * (-len(x) % 4)
            self.assertEqual(json.loads(base64.urlsafe_b64decode(pad(head)))["kid"], "KID123")
            c = json.loads(base64.urlsafe_b64decode(pad(claims)))
            self.assertEqual((c["iss"], c["aud"], c["exp"] - c["iat"]), ("issuer-uuid", "appstoreconnect-v1", 1200))
            raw = base64.urlsafe_b64decode(pad(s))
            self.assertEqual(len(raw), 64)
            sigfile = pathlib.Path(d, "sig.der")
            sigfile.write_bytes(raw_to_der(raw))
            v = subprocess.run(
                ["openssl", "dgst", "-sha256", "-verify", str(pub), "-signature", str(sigfile)],
                input=f"{head}.{claims}".encode(), capture_output=True,
            )
            self.assertEqual(v.returncode, 0, v.stdout + v.stderr)


class AscFlowTests(unittest.TestCase):
    def test_bundle_id_is_matched_exactly_not_by_prefix(self):
        fake = FakeAsc({("GET", "/v1/bundleIds"): (200, {"data": [
            {"id": "A", "attributes": {"identifier": "ai.verdure.porter.898PY37K5S.widget"}},
            {"id": "B", "attributes": {"identifier": "ai.verdure.porter.898PY37K5S"}},
        ]})})
        self.assertEqual(sig.find_bundle_id(sig.Asc("t", fake), "ai.verdure.porter.898PY37K5S"), "B")

    def test_capabilities_only_adds_missing_ones(self):
        fake = FakeAsc({
            ("GET", "/v1/bundleIds/B/bundleIdCapabilities"): (200, {"data": [
                {"attributes": {"capabilityType": "INCREASED_MEMORY_LIMIT"}}]}),
            ("POST", "/v1/bundleIdCapabilities"): (201, {}),
        })
        added = sig.ensure_capabilities(sig.Asc("t", fake), "B",
                                        ["INCREASED_MEMORY_LIMIT", "EXTENDED_VIRTUAL_ADDRESSING"])
        self.assertEqual(added, ["EXTENDED_VIRTUAL_ADDRESSING"])
        posts = [c for c in fake.calls if c[0] == "POST"]
        self.assertEqual(len(posts), 1)
        self.assertEqual(posts[0][2]["data"]["relationships"]["bundleId"]["data"]["id"], "B")

    def test_capability_forbidden_says_what_to_click(self):
        fake = FakeAsc({
            ("GET", "/v1/bundleIds/B/bundleIdCapabilities"): (200, {"data": []}),
            ("POST", "/v1/bundleIdCapabilities"): (403, {"errors": [{"title": "Forbidden", "detail": "no"}]}),
        })
        with self.assertRaises(SystemExit) as e:
            sig.ensure_capabilities(sig.Asc("t", fake), "B", ["INCREASED_MEMORY_LIMIT"])
        self.assertIn("Capabilities", str(e.exception))

    def test_certificate_matched_on_exact_bytes(self):
        ours, other = b"\x30\x82our-cert", b"\x30\x82other-cert"
        fake = FakeAsc({("GET", "/v1/certificates"): (200, {"data": [
            {"id": "X", "attributes": {"certificateContent": base64.b64encode(other).decode()}},
            {"id": "Y", "attributes": {"certificateContent": base64.b64encode(ours).decode()}},
        ]})})
        self.assertEqual(sig.find_certificate_id(sig.Asc("t", fake), ours), "Y")

    def test_unknown_certificate_fails_with_a_fix(self):
        fake = FakeAsc({("GET", "/v1/certificates"): (200, {"data": []})})
        with self.assertRaises(SystemExit) as e:
            sig.find_certificate_id(sig.Asc("t", fake), b"nope")
        self.assertIn("IOS_DIST_CERT_P12", str(e.exception))

    def test_profile_is_replaced_not_accumulated(self):
        fake = FakeAsc({
            ("GET", "/v1/profiles"): (200, {"data": [
                {"id": "OLD", "attributes": {"name": sig.PROFILE_NAME}},
                {"id": "KEEP", "attributes": {"name": "Someone else's profile"}},
            ]}),
            ("DELETE", "/v1/profiles/"): (204, {}),
            ("POST", "/v1/profiles"): (201, {"data": {"attributes": {
                "name": sig.PROFILE_NAME, "uuid": "U-1", "profileContent": base64.b64encode(b"prof").decode()}}}),
        })
        attrs = sig.replace_profile(sig.Asc("t", fake), "B", "Y")
        deletes = [c[1] for c in fake.calls if c[0] == "DELETE"]
        self.assertEqual(deletes, ["/v1/profiles/OLD"])
        create = [c[2] for c in fake.calls if c[0] == "POST"][0]["data"]
        self.assertEqual(create["attributes"]["profileType"], "IOS_APP_STORE")
        self.assertEqual(create["relationships"]["certificates"]["data"], [{"type": "certificates", "id": "Y"}])
        with tempfile.TemporaryDirectory() as d:
            files = sig.install_profile(attrs, dirs=(f"{d}/a", f"{d}/b"))
            self.assertEqual([f.name for f in files], ["U-1.mobileprovision"] * 2)
            self.assertTrue(all(f.read_bytes() == b"prof" for f in files))

    def test_http_errors_carry_apple_detail(self):
        fake = FakeAsc({})
        with self.assertRaises(sig.AscError) as e:
            sig.Asc("t", fake).call("GET", "/v1/nowhere")
        self.assertIn("404", str(e.exception))

    def test_pem_to_der(self):
        pem = "-----BEGIN CERTIFICATE-----\n" + base64.b64encode(b"abc").decode() + "\n-----END CERTIFICATE-----\n"
        self.assertEqual(sig.pem_to_der(pem), b"abc")


if __name__ == "__main__":
    unittest.main()
