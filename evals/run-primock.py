#!/usr/bin/env python3
"""The SDK's first real speech run: a PriMock57 consultation through the
Python binding — device → bundle → load → transcribe → extract facts → score.

    python3 -I evals/run-primock.py day1_consultation01 [--bindings sandbox/py] [--audio ~/Verdure-bisect/fixtures/primock57]

Needs the library built WITH engines (`cargo build -p verdure-ffi --features
llama,whisper`) and the Python binding regenerated from it (docs/sdk-integration-
guide.md §5). The bundle's model files are expected in
`<data_dir>/bundles/<bundle id>/`; if absent they are copied from
~/Verdure-bisect/models when present, else downloaded through the SDK.
"""
from __future__ import annotations

import argparse
import json
import shutil
import struct
import sys
import time
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "evals"))
import score  # noqa: E402


def read_wav_f32(path: Path) -> tuple[list[float], int]:
    with wave.open(str(path), "rb") as w:
        assert w.getsampwidth() == 2 and w.getnchannels() == 1, f"{path}: expected 16-bit mono"
        rate = w.getframerate()
        raw = w.readframes(w.getnframes())
    n = len(raw) // 2
    ints = struct.unpack(f"<{n}h", raw)
    return [s / 32768.0 for s in ints], rate


class Listener:
    def on_progress(self, done, total):
        if total:
            print(f"\r  download {done * 100 // total}%", end="", flush=True)

    def on_status(self, message):
        print(f"\n  {message}")

    def is_cancelled(self):
        return False


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("consultation")
    ap.add_argument("--bindings", default=str(ROOT / "sandbox" / "py"))
    ap.add_argument("--audio", default=str(Path.home() / "Verdure-bisect" / "fixtures" / "primock57"))
    ap.add_argument("--data-dir", default=str(Path.home() / "Verdure-bisect" / "sdk-data"))
    ap.add_argument("--out", default=str(ROOT / "evals" / "out"))
    ap.add_argument("--task", default="scribe")
    ap.add_argument("--transcript", help="skip speech recognition and extract from this saved transcript")
    ap.add_argument("--window-words", default=150, help="transcript words per extraction window (≈60 s of speech)")
    a = ap.parse_args()

    sys.path.insert(0, a.bindings)
    import verdure as v  # noqa: E402

    token = (ROOT / "bundles" / "sandbox.token").read_text().strip()
    pub = (ROOT / "crates" / "verdure-ffi" / "keys" / "issuer.pub").read_text().strip()
    policy = v.PolicyConfig(network_downloads=True, allowed_hosts=[], local_counters=False, encrypted_storage=True, retention_days=None)
    sdk = v.Verdure.start(token, policy, a.data_dir)
    caps = sdk.capabilities()
    print(f"device: tier {caps.tier}, {caps.available_ram_mb} MB available, engines {list(caps.backends)}")
    if not caps.backends:
        print("this library has no engines compiled in — build with --features llama,whisper first")
        return 2

    sdk.load_manifest((ROOT / "bundles/manifests/example.json").read_text(), (ROOT / "bundles/manifests/example.json.sig").read_text(), pub)
    task = {"scribe": v.Task.SCRIBE, "chw": v.Task.CHW_FORMS, "tutor": v.Task.TUTOR}[a.task]
    bundle = sdk.recommend(task)
    print(f"recommend({a.task}) → {bundle.id} (tier {bundle.tier}, needs {bundle.required_mb} MB); skipped: {list(bundle.skipped)}")

    # Seed the bundle directory from the local model cache so the run needs no network.
    bdir = Path(a.data_dir) / "bundles" / bundle.id
    bdir.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((ROOT / "bundles/manifests/example.json").read_text())
    for b in manifest["bundles"]:
        if b["id"] == bundle.id:
            for m in b["models"]:
                src = Path.home() / "Verdure-bisect" / "models" / m["file"]
                if src.exists() and not (bdir / m["file"]).exists():
                    shutil.copy(src, bdir / m["file"])
    if not sdk.is_downloaded(bundle.id):
        sdk.download(bundle.id, Listener())
        print()
    t0 = time.time()
    sdk.load(bundle.id)
    print(f"loaded in {time.time() - t0:.1f}s")

    if a.transcript:
        transcript = Path(a.transcript).read_text()
        audio_seconds = asr_seconds = 0.0
    else:
        transcript, audio_seconds, asr_seconds = run_asr(sdk, v, Path(a.audio), a.consultation)

    # Facts the way the product does it: over each new stretch of transcript
    # (the platforms emit facts about every 60 s), accumulated — not one pass
    # over a whole consultation, which is beyond a floor-tier model.
    schema = (ROOT / "bundles/manifests/schema-facts.json").read_text()
    words = transcript.split()
    window = int(a.window_words)
    facts: list = []
    windows_ok = windows_failed = 0
    t0 = time.time()
    for i in range(0, len(words), window):
        piece = " ".join(words[i : i + window])
        try:
            out = sdk.extract(piece, schema)
            new = json.loads(out.json)["facts"]
            windows_ok += 1
            for f in new:
                facts.append(f)
                print(f"  [{f['group']}] {f['text']}")
        except v.VerdureError.SchemaInvalid as e:
            windows_failed += 1
            print(f"  window {i // window + 1}: {str(e)[:220]}")
    extract_seconds = time.time() - t0
    print(f"facts: {len(facts)} from {windows_ok} window(s), {windows_failed} failed, {extract_seconds:.0f}s")
    if windows_ok == 0:
        facts = None

    outdir = Path(a.out)
    outdir.mkdir(parents=True, exist_ok=True)
    (outdir / f"{a.consultation}.transcript.txt").write_text(transcript)
    if facts is not None:
        (outdir / f"{a.consultation}.facts.json").write_text(json.dumps({"facts": facts}, indent=2))
    report = score.score(score.reference_text(ROOT / "evals/fixtures/primock57" / a.consultation), transcript, facts, audio_seconds or None, asr_seconds or None)
    report.update({"bundle": bundle.id, "tier": caps.tier, "os": caps.os, "available_ram_mb": caps.available_ram_mb, "extract_windows_ok": windows_ok, "extract_windows_failed": windows_failed, "extract_seconds": round(extract_seconds, 1)})
    (outdir / f"{a.consultation}.score.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0


def run_asr(sdk, v, audio_dir: Path, consultation: str) -> tuple[str, float, float]:
    """Transcribe the two PriMock channels (doctor, patient) mixed into one conversation."""
    doc, rate = read_wav_f32(audio_dir / f"{consultation}_doctor.wav")
    pat, rate2 = read_wav_f32(audio_dir / f"{consultation}_patient.wav")
    assert rate == rate2
    n = max(len(doc), len(pat))
    doc += [0.0] * (n - len(doc))
    pat += [0.0] * (n - len(pat))
    mixed = [(x + y) * 0.5 for x, y in zip(doc, pat)]
    audio_seconds = n / rate
    print(f"audio: {audio_seconds:.0f}s at {rate} Hz")
    stream = sdk.transcribe(v.AsrConfig(lang_hints=["en"], keyterms=[], replacements=[]))
    chunk = rate // 2
    finals = []
    t0 = time.time()
    first_word = None
    for i in range(0, n, chunk):
        for ev in stream.push_pcm(mixed[i : i + chunk], rate):
            if isinstance(ev, v.TranscriptEvent.FINAL):
                finals.append(ev)
                if first_word is None:
                    first_word = time.time() - t0
                print(f"  {ev.start_ms / 1000:7.1f}s  {ev.text}")
    for ev in stream.finish():
        if isinstance(ev, v.TranscriptEvent.FINAL):
            finals.append(ev)
            print(f"  {ev.start_ms / 1000:7.1f}s  {ev.text}")
    asr_seconds = time.time() - t0
    transcript = " ".join(e.text for e in finals)
    print(f"transcribed in {asr_seconds:.1f}s (RTF {asr_seconds / audio_seconds:.2f}); first text after {first_word:.1f}s" if first_word else "no speech recognised")
    return transcript, audio_seconds, asr_seconds


if __name__ == "__main__":
    sys.exit(main())
