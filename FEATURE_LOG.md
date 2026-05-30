# FEATURE_LOG.md — verdure_web

Living record of features built in the Phase 0 website repo. The Rust binary
keeps its own log in `verdure-orchestrator/FEATURE_LOG.md`.

## 2026-05-29 — Multimodal recommendations (audio · image · video)

**What:** The recommendation engine went from text-LLM-only to multimodal. The classifier (`lib/intent.ts`) now resolves a request to a `(modality, task)` pair across text/audio/image/video (`lib/modality.ts`); the engine (`lib/recommend.ts`) dispatches by modality and returns one of three honest outcomes — `recommended`, `works-but-slow`, or `not-local-yet`.

**Problem it solves:** Typing "transcribe my recording" or "make a video" used to silently return a generic chat model. Now audio/image/video map to real local tools (Whisper, Kokoro, Real-ESRGAN, Stable Diffusion, LTX-2…) where the device can run them — and to an honest "your device can't do this locally yet" where it can't (e.g. video on a laptop), instead of a fake recommendation.

**Key design decisions:**
- **Feasibility is graphics-bound, not RAM-bound** for non-text work, so the five device tiers collapse into three capability classes (cpu-laptop / apple-silicon / gpu).
- **Copy is verb-first and jargon-free** per the new "Assume zero technical knowledge" principle in `verdure-orchestrator/skills/design.md`: the headline says what the user will *do*; the model name is demoted to an ignorable "the tool we'd use" detail with a learn-more link.
- **Install paths are modality-specific data** returned by the engine (Whisper apps for audio, Draw Things/Fooocus for image, etc.) — Ollama/LM Studio only ever apply to text. The UI renders them generically.

**Knowledge base:** grounded in research captured in `verdure-orchestrator/knowledge/12-multimodal-models.md` (the new source of truth); `03` scoped to text; thesis, licensing, runtime-backends cross-referenced; OQ-28..31 opened.

**Tests:** `lib/intent.test.ts` and `lib/recommend.test.ts` rewritten for the multimodal shape (40 tests total, all passing). Text model picks are unchanged and still asserted.

**Not yet done:** visual self-review (screenshot loop) of the new recommendation page — no browser tooling installed locally. Device-picker still uses RAM/Apple-Silicon jargon (deferred; see design note).

## 2026-05-29 — Waitlist persistence via Google Sheets

**What:** `/api/waitlist` now appends each signup as a row in a Google Sheet
instead of logging to the console. New deep module `lib/sheets.ts` owns auth
(service-account JWT), private-key parsing, column order, and the append call.

**Problem it solves:** Phase 1 launch list capture was a `console.log` stub —
signups weren't durable. Closes the persistence half of `[OQ-10]`.

**Behavior:**
- Configured (env vars set): appends a row; an append failure returns a
  retryable 502 so no real signup is silently lost.
- Not configured: logs the signup server-side and returns success, keeping the
  four-screen flow exercisable before the Google Cloud setup is done.

**Setup:** `SETUP_SHEETS.md` (Sheet + service account + env vars), template in
`.env.example`. The user owns the Google Cloud setup.

**Tests:** `lib/sheets.test.ts` — 9 unit tests over the pure helpers
(key normalization, config loading, column order). The networked append is not
unit-tested; it needs live credentials (see `TEST_REPORT.md`).
