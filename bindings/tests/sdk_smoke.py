#!/usr/bin/env python3
"""The SDK through its Python binding, the way a vendor's developer calls it,
as a test with named checks. Runs on any desktop the library is built for; it
exists because the Windows library is built and tested on a machine nobody on
the team can sit at (knowledge/26, "Windows").

Two modes:

  online     start → device → manifest → recommend → download → the shared
             model store → load → transcribe a known recording → chat →
             extract → search → delete. Real models, real network.
  preplaced  the vendor shipped the model files inside their own installer:
             downloads forbidden by policy (and, when the caller has blocked
             the network, proven blocked), files copied into place, then
             load → transcribe → chat → delete.

    python3 -I bindings/tests/sdk_smoke.py --binding DIR --assets DIR \
        --data-dir DIR --audio jfk.wav [--mode online|preplaced] ...

Every check runs even when an earlier, unrelated one failed, so one run on an
expensive machine says as much as it can. Exit status 0 only when no check
failed. Standard library only.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import struct
import sys
import time
import traceback
import wave
from pathlib import Path

# A folder name with a space, accents and CJK: Windows usernames have them,
# and both engines take the model path through C++ file APIs.
AWKWARD = "Vérdure tèst 数据"

JFK_PHRASES = ["ask not what your country can do for you", "ask what you can do for your country"]

RESULTS: list[dict] = []


def check(name: str, fn, needs: tuple[str, ...] = ()) -> bool:
    """Run one named check. `needs` are earlier checks it cannot run without."""
    missing = [n for n in needs if not any(r["name"] == n and r["status"] == "pass" for r in RESULTS)]
    if missing:
        RESULTS.append({"name": name, "status": "skip", "detail": f"needs {', '.join(missing)}"})
        print(f"SKIP  {name}  (needs {', '.join(missing)})", flush=True)
        return False
    t0 = time.time()
    try:
        detail = fn()
        RESULTS.append({"name": name, "status": "pass", "detail": detail, "seconds": round(time.time() - t0, 2)})
        print(f"PASS  {name}  {'' if detail is None else detail}", flush=True)
        return True
    except Exception as e:  # noqa: BLE001 — a failed check is data, not a crash
        RESULTS.append({"name": name, "status": "fail", "detail": f"{type(e).__name__}: {e}", "seconds": round(time.time() - t0, 2)})
        print(f"FAIL  {name}  {type(e).__name__}: {e}", flush=True)
        traceback.print_exc()
        return False


def note(name: str, detail) -> None:
    """Something observed and worth reading, with no right answer to assert."""
    RESULTS.append({"name": name, "status": "note", "detail": detail})
    print(f"NOTE  {name}  {detail}", flush=True)


def read_wav_f32(path: Path) -> tuple[list[float], int]:
    with wave.open(str(path), "rb") as w:
        assert w.getsampwidth() == 2 and w.getnchannels() == 1, f"{path}: expected 16-bit mono"
        rate = w.getframerate()
        raw = w.readframes(w.getnframes())
    n = len(raw) // 2
    return [s / 32768.0 for s in struct.unpack(f"<{n}h", raw)], rate


def plain(text: str) -> str:
    """Lower-case words only, so punctuation and casing never fail a transcript."""
    return " ".join("".join(c if c.isalnum() else " " for c in text.lower()).split())


def expected_os() -> str:
    return {"win32": "windows", "darwin": "macos"}.get(sys.platform, "linux")


class Listener:
    def __init__(self):
        self.statuses: list[str] = []
        self.progress: list[tuple[int, int]] = []

    def on_progress(self, done, total):
        self.progress.append((done, total))

    def on_status(self, message):
        self.statuses.append(message)
        print(f"        {message}", flush=True)

    def is_cancelled(self):
        return False


class Sink:
    def __init__(self):
        self.pieces: list[str] = []

    def on_token(self, piece):
        self.pieces.append(piece)
        return True


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--binding", required=True, help="folder holding verdure.py and the library beside it")
    ap.add_argument("--assets", required=True, help="folder holding example.json, example.json.sig, sandbox.token, issuer.pub")
    ap.add_argument("--data-dir", required=True)
    ap.add_argument("--audio", required=True, help="the JFK clip from whisper.cpp's samples (16 kHz mono)")
    ap.add_argument("--mode", choices=["online", "preplaced"], default="online")
    ap.add_argument("--bundle", help="use this bundle instead of recommend()")
    ap.add_argument("--also-bundle", help="a second bundle that shares a model with the first, for the one-copy check; 'auto' picks the smallest")
    ap.add_argument("--export-bundles", help="online: copy the downloaded bundle folders here before deleting")
    ap.add_argument("--seed-from", help="preplaced: a folder of bundle folders to copy into place, as a vendor's installer would")
    ap.add_argument("--awkward-path", action="store_true", help=f"put the data under a folder named {AWKWARD!r}")
    ap.add_argument("--expect-no-network", action="store_true", help="the caller blocked this program's network; prove it, then run")
    ap.add_argument("--require-engines", action="store_true", help="fail if the library was built without whisper or llama")
    ap.add_argument("--max-rtf", type=float, help="fail if the recording takes more than this many times its own length to transcribe")
    ap.add_argument("--max-chat-seconds", type=float, help="fail if the one-word answer takes longer than this")
    ap.add_argument("--report", help="write the results here as JSON")
    a = ap.parse_args()

    # A Windows console or CI log is not UTF-8 by default; this script prints
    # paths and model replies that are.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")

    assets = Path(a.assets).resolve()
    data_dir = Path(a.data_dir).resolve()
    if a.awkward_path:
        data_dir = data_dir / AWKWARD
    ctx: dict = {}

    def c_import():
        sys.path.insert(0, str(Path(a.binding).resolve()))
        import verdure  # noqa: PLC0415

        ctx["v"] = verdure
        return f"binding loaded from {a.binding}"

    check("the binding imports and finds the library", c_import)
    if "v" not in ctx:
        return finish(a)
    v = ctx["v"]

    # Text files are read as UTF-8, explicitly: the manifest carries characters
    # outside ASCII, and Python on Windows otherwise decodes with the system
    # code page, which changes the bytes the signature covers.
    token = (assets / "sandbox.token").read_text(encoding="utf-8").strip()
    pub = (assets / "issuer.pub").read_text(encoding="utf-8").strip()
    manifest_text = (assets / "example.json").read_text(encoding="utf-8")
    signature = (assets / "example.json.sig").read_text(encoding="utf-8")
    manifest = json.loads(manifest_text)
    online = a.mode == "online"

    def policy(downloads: bool):
        return v.PolicyConfig(network_downloads=downloads, allowed_hosts=[], local_counters=False, encrypted_storage=True, retention_days=None)

    def c_no_network():
        import urllib.request  # noqa: PLC0415

        try:
            urllib.request.urlopen("https://huggingface.co", timeout=15)  # noqa: S310
        except Exception as e:  # noqa: BLE001
            return f"a request from this program fails as it should: {type(e).__name__}"
        raise AssertionError("this program can still reach the internet; the block is not in force, so the run below would prove nothing")

    if a.expect_no_network:
        check("control: this program has no network", c_no_network)

    def c_start():
        ctx["sdk"] = v.Verdure.start(token, policy(online), str(data_dir))
        return f"data folder {data_dir}"

    check("start with the sandbox licence", c_start)

    def c_bad_licence():
        try:
            v.Verdure.start(token[:-6] + "AAAAAA", policy(online), str(data_dir))
        except v.VerdureError.License as e:
            return f"refused: {str(e)[:80]}"
        raise AssertionError("a tampered licence token was accepted")

    check("control: a tampered licence is refused", c_bad_licence)

    def c_caps():
        caps = ctx["sdk"].capabilities()
        ctx["caps"] = caps
        assert caps.os == expected_os(), f"os reads {caps.os!r} on {sys.platform}"
        assert caps.total_ram_mb >= 1024, caps.total_ram_mb
        assert 0 < caps.available_ram_mb <= caps.total_ram_mb, (caps.available_ram_mb, caps.total_ram_mb)
        assert caps.free_disk_mb > 0, "free disk reads 0 MB: the data folder's volume was not found, and every bundle would be refused"
        assert caps.cores >= 1 and caps.tier
        engines = list(caps.backends)
        if a.require_engines:
            assert "whisper" in engines and "llama" in engines, f"engines compiled in: {engines}"
        return f"tier {caps.tier}, {caps.arch}, {caps.cores} cores, {caps.available_ram_mb}/{caps.total_ram_mb} MB RAM free, {caps.free_disk_mb} MB disk free, engines {engines}, notes {list(caps.notes)}"

    check("the device is read sanely", c_caps, needs=("start with the sandbox licence",))

    def c_relative():
        # What a script does: a folder name, not a full path. Run from the
        # parent so the relative name lands beside the real data folder.
        here = os.getcwd()
        data_dir.parent.mkdir(parents=True, exist_ok=True)
        os.chdir(data_dir.parent)
        try:
            caps = v.Verdure.start(token, policy(online), "verdure-relative").capabilities()
        finally:
            os.chdir(here)
            shutil.rmtree(data_dir.parent / "verdure-relative", ignore_errors=True)
        assert caps.free_disk_mb > 0, "a relative data folder reads 0 MB free"
        return f"{caps.free_disk_mb} MB free"

    check("a relative data folder still finds its disk", c_relative, needs=("start with the sandbox licence",))

    def c_manifest():
        ctx["sdk"].load_manifest(manifest_text, signature, pub)
        return f"{len(manifest['bundles'])} bundles, signature verified"

    check("the signed manifest verifies", c_manifest, needs=("start with the sandbox licence",))

    def c_tampered():
        other = v.Verdure.start(token, policy(online), str(data_dir))
        try:
            other.load_manifest(manifest_text.replace("scribe", "scribE", 1), signature, pub)
        except v.VerdureError.Integrity as e:
            return f"refused: {str(e)[:80]}"
        raise AssertionError("a manifest changed after signing was accepted")

    check("control: a changed manifest is refused", c_tampered, needs=("the signed manifest verifies",))

    def c_recommend():
        sdk = ctx["sdk"]
        if a.bundle:
            ctx["bundle"] = a.bundle
            fit = sdk.fit(a.bundle)
            assert fit.fits, f"{a.bundle} does not fit this machine: {list(fit.problems)}"
            return f"pinned {a.bundle}: fits, needs {fit.bundle.required_mb} MB"
        b = sdk.recommend(v.Task.SCRIBE)
        assert any(x["id"] == b.id for x in manifest["bundles"]), b.id
        ctx["bundle"] = b.id
        return f"{b.id} (needs {b.required_mb} MB); passed over: {list(b.skipped)}"

    check("recommend picks a bundle this machine can run", c_recommend, needs=("the signed manifest verifies",))

    def models_of(bundle_id: str) -> list[dict]:
        return next(b["models"] for b in manifest["bundles"] if b["id"] == bundle_id)

    def bundle_folder(bundle_id: str) -> Path:
        return data_dir / "bundles" / bundle_id

    if online:

        def c_download():
            sdk, bid = ctx["sdk"], ctx["bundle"]
            ls = Listener()
            t0 = time.time()
            sdk.download(bid, ls)
            secs = time.time() - t0
            assert sdk.is_downloaded(bid), "download returned but is_downloaded says no"
            total = sum(m["size_bytes"] for m in models_of(bid))
            assert ls.progress, "no progress was reported"
            assert ls.progress[-1] == (total, total), f"progress ended at {ls.progress[-1]}, expected {total}"
            dones = [d for d, _ in ls.progress]
            assert dones == sorted(dones), "progress went backwards"
            assert any("downloaded and verified" in s for s in ls.statuses), ls.statuses
            for m in models_of(bid):
                f = bundle_folder(bid) / m["file"]
                assert f.stat().st_size == m["size_bytes"], f"{f.name}: {f.stat().st_size} bytes, manifest says {m['size_bytes']}"
            ctx["download_seconds"] = secs
            return f"{total / 1e6:.0f} MB in {secs:.0f} s; {len(ls.statuses)} status lines"

        check("download fetches and verifies every file", c_download, needs=("recommend picks a bundle this machine can run",))

        def c_again():
            ls = Listener()
            ctx["sdk"].download(ctx["bundle"], ls)
            assert all("already on this device" in s or s.startswith("Checking") for s in ls.statuses), ls.statuses
            return "nothing fetched the second time"

        check("a second download of the same bundle fetches nothing", c_again, needs=("download fetches and verifies every file",))

        def c_shared():
            sdk, first, second = ctx["sdk"], ctx["bundle"], a.also_bundle
            if second == "auto":
                # The smallest other bundle that has a file in common with the first.
                mine = {m["sha256"] for m in models_of(first)}
                others = [b for b in manifest["bundles"] if b["id"] != first and mine & {m["sha256"] for m in b["models"]}]
                assert others, f"no other bundle shares a model with {first}"
                second = min(others, key=lambda b: sum(m["size_bytes"] for m in b["models"]))["id"]
                a.also_bundle = second
            shared = [m for m in models_of(second) if any(m["sha256"] == x["sha256"] for x in models_of(first))]
            assert shared, f"{second} shares no model with {first}; pick another pair"
            ls = Listener()
            sdk.download(second, ls)
            assert sdk.is_downloaded(second)
            assert any("reusing the copy another bundle downloaded" in s for s in ls.statuses), ls.statuses
            for m in shared:
                x, y = bundle_folder(first) / m["file"], bundle_folder(second) / m["file"]
                store = data_dir / "models" / m["sha256"].lower()
                assert os.path.samefile(x, y), f"{m['file']}: the two bundles hold separate copies"
                assert os.path.samefile(x, store), f"{m['file']}: not the shared store's file"
            return f"{second}: {len(shared)} file(s) held once and linked: {[m['file'] for m in shared]}"

        if a.also_bundle:
            check("two bundles that share a model hold one copy", c_shared, needs=("download fetches and verifies every file",))

        ready = "download fetches and verifies every file"
    else:

        def c_seed():
            sdk, bid = ctx["sdk"], ctx["bundle"]
            assert not sdk.is_downloaded(bid), "the bundle is already present before anything was placed"
            src = Path(a.seed_from) / bid
            dst = bundle_folder(bid)
            dst.mkdir(parents=True, exist_ok=True)
            for m in models_of(bid):
                shutil.copyfile(src / m["file"], dst / m["file"])
            assert sdk.is_downloaded(bid), "files copied into place are not recognised"
            return f"{len(models_of(bid))} files copied into {dst}; recognised by fingerprint, nothing downloaded"

        check("model files placed by hand are recognised", c_seed, needs=("recommend picks a bundle this machine can run",))

        def c_damaged():
            sdk, bid = ctx["sdk"], ctx["bundle"]
            m = models_of(bid)[0]
            f = bundle_folder(bid) / m["file"]
            original = f.read_bytes()[:64]
            with open(f, "r+b") as fh:
                fh.write(b"\0" * 64)
            try:
                assert not sdk.is_downloaded(bid), "a file with 64 bytes overwritten still counts as present"
            finally:
                with open(f, "r+b") as fh:
                    fh.write(original)
            assert sdk.is_downloaded(bid)
            return f"{m['file']} with 64 bytes changed is not accepted; restored, accepted again"

        check("control: a damaged placed file is not accepted", c_damaged, needs=("model files placed by hand are recognised",))

        def c_policy_blocks():
            other = next(b["id"] for b in manifest["bundles"] if b["id"] != ctx["bundle"] and b["models"] and not (data_dir / "bundles" / b["id"]).exists())
            try:
                ctx["sdk"].download(other, Listener())
            except v.VerdureError.Policy as e:
                return f"download of {other} refused by policy: {str(e)[:80]}"
            raise AssertionError("a download ran although the policy forbids downloads")

        check("the policy forbids downloads and the SDK obeys", c_policy_blocks, needs=("the signed manifest verifies",))
        ready = "model files placed by hand are recognised"

    def c_load():
        t0 = time.time()
        ctx["sdk"].load(ctx["bundle"])
        ctx["load_seconds"] = time.time() - t0
        caps = ctx["sdk"].capabilities()
        return f"loaded in {ctx['load_seconds']:.1f} s; {caps.available_ram_mb} MB RAM free after"

    check("load brings both engines into memory", c_load, needs=(ready,))

    def c_transcribe():
        samples, rate = read_wav_f32(Path(a.audio))
        seconds = len(samples) / rate
        stream = ctx["sdk"].transcribe(v.AsrConfig(lang_hints=["en"], keyterms=[], replacements=[]))
        finals = []
        t0 = time.time()
        step = rate // 2
        for i in range(0, len(samples), step):
            finals += [ev for ev in stream.push_pcm(samples[i : i + step], rate) if isinstance(ev, v.TranscriptEvent.FINAL)]
        finals += [ev for ev in stream.finish() if isinstance(ev, v.TranscriptEvent.FINAL)]
        took = time.time() - t0
        text = " ".join(e.text for e in finals)
        ctx["transcript"] = text
        ctx["asr_rtf"] = took / seconds
        for phrase in JFK_PHRASES:
            assert phrase in plain(text), f"expected {phrase!r} in the transcript; got {text!r}"
        # Correct but implausibly slow is a broken build, not a pass: the first
        # Windows library transcribed this perfectly at 19.7x real time because
        # the engine had been compiled without optimisation.
        if a.max_rtf is not None:
            assert took / seconds <= a.max_rtf, f"{took / seconds:.1f}× real time, limit {a.max_rtf}: correct words, but far too slow — check how the engines were compiled"
        return f"{seconds:.0f} s of speech in {took:.1f} s ({took / seconds:.2f}× real time): {text!r}"

    check("a known recording is transcribed correctly", c_transcribe, needs=("load brings both engines into memory",))

    def c_chat():
        sink = Sink()
        t0 = time.time()
        reply = ctx["sdk"].chat([v.ChatMessage(role=v.Role.USER, content="What is the capital of France? Answer with one word.")], sink)
        took = time.time() - t0
        assert "paris" in reply.lower(), f"expected Paris; got {reply!r}"
        assert sink.pieces, "nothing was streamed"
        assert "".join(sink.pieces).strip() == reply.strip(), "the streamed pieces do not add up to the reply"
        if a.max_chat_seconds is not None:
            assert took <= a.max_chat_seconds, f"a one-word answer took {took:.1f} s, limit {a.max_chat_seconds}: check how the engines were compiled"
        return f"{reply.strip()!r} in {took:.1f} s, streamed in {len(sink.pieces)} pieces"

    check("a known question is answered correctly", c_chat, needs=("load brings both engines into memory",))

    def c_extract():
        schema = json.dumps(
            {
                "type": "object",
                "properties": {"name": {"type": "string"}, "age": {"type": "integer"}},
                "required": ["name", "age"],
                "additionalProperties": False,
            }
        )
        t0 = time.time()
        try:
            out = json.loads(ctx["sdk"].extract("The patient is Ada Obi. She is 34 years old.", schema).json)
        except v.VerdureError.SchemaInvalid as e:
            # The SDK's contract is "valid JSON or an error naming the fields".
            # A small model failing the form is that contract, not a platform fault.
            note("extract: the model's answer did not fit the form", str(e)[:300])
            return f"refused with a typed error in {time.time() - t0:.1f} s (allowed by the contract; see the note)"
        assert isinstance(out.get("name"), str) and isinstance(out.get("age"), int), out
        if "ada" not in out["name"].lower() or out["age"] != 34:
            note("extract: valid form, unexpected values", out)
        return f"{out} in {time.time() - t0:.1f} s"

    check("extract returns a valid form or a typed refusal", c_extract, needs=("load brings both engines into memory",))

    def c_search():
        sdk = ctx["sdk"]
        n = sdk.index(
            "guides",
            [
                v.Doc(id="epi", title="Immunisation", text="The first measles vaccine is given at nine months of age."),
                v.Doc(id="ors", title="Diarrhoea", text="Give oral rehydration salts and zinc for ten days."),
                v.Doc(id="mal", title="Malaria", text="Test with a rapid diagnostic test before treating a fever as malaria."),
            ],
        )
        assert n == 3, n
        hits = sdk.query("guides", "When is the measles vaccine given?", 2)
        assert hits and hits[0].doc_id == "epi", [(h.doc_id, h.score) for h in hits]
        return f"top passage from {hits[0].doc_id!r}: {hits[0].text!r}"

    check("documents are indexed and the right passage is found", c_search, needs=("start with the sandbox licence",))

    def c_export():
        out = Path(a.export_bundles)
        for bid in [b for b in (ctx["bundle"], a.also_bundle) if b and b != "auto" and bundle_folder(b).exists()]:
            (out / bid).mkdir(parents=True, exist_ok=True)
            for m in models_of(bid):
                shutil.copyfile(bundle_folder(bid) / m["file"], out / bid / m["file"])
        return f"copied to {out}"

    if online and a.export_bundles:
        check("the downloaded files are set aside for the offline run", c_export, needs=("download fetches and verifies every file",))

    def c_delete():
        sdk = ctx["sdk"]
        sdk.delete(v.DeleteScope.ALL())
        left = [p.name for p in data_dir.iterdir()] if data_dir.exists() else []
        assert not (data_dir / "bundles").exists(), f"the bundle folders survived delete: {left}"
        assert not (data_dir / "models").exists(), f"the shared model store survived delete, so no space was freed: {left}"
        assert not (data_dir / "vault.key").exists(), left
        assert not sdk.is_downloaded(ctx["bundle"])
        return f"left in the data folder: {left}"

    check("delete(All) removes the models, loaded or not", c_delete, needs=("load brings both engines into memory",))

    for key in ("download_seconds", "load_seconds", "asr_rtf"):
        if key in ctx:
            note(f"measured: {key}", round(ctx[key], 2))
    return finish(a)


def finish(a) -> int:
    failed = [r for r in RESULTS if r["status"] == "fail"]
    skipped = [r for r in RESULTS if r["status"] == "skip"]
    passed = [r for r in RESULTS if r["status"] == "pass"]
    print(f"\n{len(passed)} passed, {len(failed)} failed, {len(skipped)} skipped ({a.mode} mode, {sys.platform})")
    if a.report:
        Path(a.report).write_text(json.dumps({"mode": a.mode, "platform": sys.platform, "python": sys.version, "results": RESULTS}, indent=2, ensure_ascii=False), encoding="utf-8")
    # A skipped check is a check that did not run: not a pass.
    return 1 if failed or skipped else 0


if __name__ == "__main__":
    sys.exit(main())
