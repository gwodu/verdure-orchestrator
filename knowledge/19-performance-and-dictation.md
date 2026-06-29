# 19 — Performance prediction & the dictation roadmap

**Status:** 🟢 Active (decided 2026-06-19). Performance predictor built (v1); live dictation is next.

## Predict the experience, don't make users benchmark
Verdure estimates how well a local model will run **from the device's specs**, before any download — then **calibrates with the real measurement** from the user's first run. This is the "honest performance numbers" promise (`skills/design.md`) generalized.

- **Where:** `packages/decider/src/performance.ts` (pure, shared, tested) — knowledge operationalized, like the matrix.
- **Inputs:** a `DeviceSpec { arch, cores, ramGb }`, **detected** not asked (desktop reads `sysctl` in `device.rs` → `device_spec` command).
- **Formula:** a compute proxy (`perCore[arch] × diminishing(cores)`) against an **anchor** (a 4-core Intel running `base` at ~3× real-time, grounded in published whisper.cpp CPU benchmarks). `predicted_RTF = REF_RTF × model_cost × (REF_score ÷ device_score)`. RAM gates feasibility.
- **Output:** a category (`fast / comfortable / slow / very-slow / wont-fit`) + a rough "× real-time" **range** — never false precision; always labelled *estimate* until measured.
- **Calibration:** `transcribe_audio` returns `{audioSeconds, processingSeconds}`; the UI shows the **measured** factor and `estimateTranscriptionSpeed(spec, size, measuredRtf)` returns the truth. Real measurements are better anchors — collected **opt-in only**, never passive telemetry (`06`).
- **Honesty:** constants are PROVISIONAL and documented as such. The prediction is a category + range; the one real run corrects it.

This resolves the data gap behind **[OQ-05]** (real tokens/sec per device — the predictor now estimates it and self-corrects from one real run; see `bestModelFor`) and informs **[OQ-03]** (device anchors). *(This previously cited `[OQ-51]`; that code is the separate "pin vs track-latest versions" question — corrected 2026-06-29.)*

### Anchors (real measurements)
- **2026-06-19 — founder's 2019 Intel Mac (8 logical cores), `base.en`: 59s of audio in 15s → RTF ≈ 0.254 (~3.9× real-time).** The formula is now anchored to this (replacing the earlier published-benchmark guess of ~0.33). Insight: the machine has ~8 logical cores but Whisper effectively used ~4 (default thread count), so the spec-only estimate over-counted cores — the real anchor corrects it. Threads are a future speed lever (relevant to dictation latency).

## Whispr/Wispr Flow — what it is (researched 2026-06-19)
The fast-dictation product people compare us to is **cloud-based** (audio leaves the device; no offline mode) and pairs **proprietary ASR with an LLM that rewrites speech in real time**. It does **not** use Whisper. ([Wikipedia](https://en.wikipedia.org/wiki/Wispr_Flow))

Takeaways:
- The "smart, clean" feel = an **LLM post-processing pass** (punctuation, capitalization, filler removal, formatting) on top of raw ASR. The "prompt engineering" intuition was right.
- Their speed = **server GPUs**. That's their moat *and* their compromise (privacy + subscription) — the exact opposite of Verdure's local/private stance.

## Live dictation — the next build (on-device, private)
Match the *"speak, pause, text appears"* feel locally:
1. **Mic capture** (e.g. `cpal` + a mic permission) — new surface area.
2. **VAD-chunked transcription** — transcribe each utterance on pause; perceptually near-live. whisper.cpp ships a sliding-window `stream` example for true partials.
3. **`tiny.en` for latency** (the predictor picks tiny vs base per device).
4. **Optional local LLM cleanup** — punctuation/formatting/filler removal via the coming LLM modality (Ollama). Audio + text modalities **compose** — the unified-interface thesis (`18`).

Differentiator: local, private, no subscription vs Wispr's cloud. Honest caveat: on an older Intel CPU, `tiny.en` + transcribe-on-pause is the realistic target — which is exactly what the predictor tells each user.

**Order (decided):** predictor first (so dictation honestly picks the model + sets expectations), then live dictation. "Honest by construction."

## Open questions
- [OQ-54] Live dictation: mic-capture crate + permission, VAD choice, chunk/segment strategy, and the optional LLM-cleanup pass (needs the LLM modality). (From `19`.)
- Calibration sharing (opt-in anchor contribution) — design within `06` (no telemetry). Tracked with [OQ-03].
