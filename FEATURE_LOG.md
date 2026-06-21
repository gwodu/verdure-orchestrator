# FEATURE_LOG.md

Living record of features built across the Verdure monorepo — the **Verdure desktop app** (`app/`), the shared decider package (`packages/decider/`), the Phase 0 website (`website/`), and the long-term Rust core (`src/`, `tests/`). Newest first. The knowledge base (`knowledge/`) is the shared brain for all of them.

## Verdure desktop app (`app/`) — unified interface (post-pivot)

> Pivoted 2026-06-18 (`knowledge/18`) from "install separate apps" to **one interface that runs models in-app**, then to **Verdure as the personal AI** (`knowledge/20`). The "porter" naming in code/below is legacy; see `knowledge/21-handoff.md` for current state + next steps.

### 2026-06-20 — Chat front door + grammar tool-calling + RAM-gating

**What:** A local chat model (llama.cpp via `llama-cpp-2`, compiled in) is the conversational front door. It both answers and **orchestrates**: it emits a **GBNF-grammar-constrained** tool call (always-valid JSON — `answer` or `set_up_transcription`), `intent.ts` cross-checks, and the user confirms before Verdure acts. Model is **RAM-gated** (`model::chat_model_for`): Qwen2.5 0.5B on ≤8 GB, 1.5B on ≥12 GB.

**Problem it solves:** "apps for AI" step one — capabilities the model invokes, not just buttons. Small-model unreliability is handled by the grammar (can't malform a call) + the deterministic cross-check + a confirm step (human in the loop).

**Crash fixed:** the 1.5B model OOM-aborted an 8 GB Mac (ggml calls `abort()` on a failed alloc, killing the app). Cut chat context 4096→2048 + batch 512, validated model-file size, then **RAM-gated to 0.5B** on ≤8 GB. (Deeper fix — process isolation so ggml can't crash the UI — is a logged P0 next step.)

**Tests:** `llm.rs` parse tests; CI compiles + cross-compiles llama.cpp (the `common` feature gates the grammar samplers; deployment-target fix shared with whisper). On-device 0.5B confirmation pending.

### 2026-06-19 — Performance predictor (predict from specs, calibrate from runs)

**What:** `packages/decider/performance.ts` estimates a model's speed/feasibility from detected specs (`device.rs` reads cores/RAM/arch → `device_spec`), shown before download as a category + "× real-time" range, then **replaced by the real measured speed** after a run (`transcribe_audio` returns timing). Anchored to a real measurement (founder's Mac: base.en ≈ 3.9× real-time, RTF 0.254). `bestModelFor` picks per device.

**Problem it solves:** honest performance without making users benchmark (resolves the data gap behind `[OQ-51]`/`[OQ-03]`). 10 decider tests.

### 2026-06-19 — In-app transcription (whisper.cpp) + storage management

**What:** Drop an audio file → text, entirely in Verdure. `audio.rs` decodes mp3/m4a/wav/flac via symphonia (no ffmpeg) → 16 kHz mono; `transcribe.rs` runs whisper.cpp (`whisper-rs`, compiled in) on CPU. `model.rs` is a **registry** (data) — models download on first use (curl) to app data, with a size-validation guard. **Storage**: per-model size + Remove (in the transcribe panel) and a global "Manage storage" screen.

**Problem it solves:** the first wrapped modality — proves the pivot (model in one UI, no separate app, no Homebrew/CLT). Storage control is first-class for the floor user's tight disk. Verified on-device (~3.9× real-time). Rust unit tests for the resampler/downmix, catalog, model registry.

## MVP desktop porter (`app/`) — the original build (pre-pivot, now legacy)

### 2026-06-16 — The Buzz slice: intent → one-click port (v1.0 first slice)

**What:** A Tauri desktop app (`app/`) that proves Verdure's atomic unit — *one OSS tool installed & running, from the user's intent, in one click* (`knowledge/17-build-plan.md`). Two screens: pick a task (the reused intent cards) → see the recommendation and press **Set it up**. The Rust backend (the "porter") installs the tool from its official source and launches it. First wired tool: **Buzz** (transcription), via the Homebrew cask, launched with `open -a Buzz`.

**Problem it solves:** The website tells a non-technical user *what* to use; it can't *do it for them*. The porter closes that gap — the install/launch a normal person can't be expected to perform unattended happens on one click, locally, free, no account.

**Architecture (`skills/architecture.md`):**
- **The decision is not re-implemented.** The frontend imports `@verdure/decider` (the same `recommend()` the website uses) — single source of truth, no drift (resolves `[OQ-49]`; see the decider extraction below).
- **The catalog is data, not code** (`app/src-tauri/src/catalog.rs`). Adding a ported tool = a `Tool` entry with an `InstallMethod` + `LaunchMethod`; the porter (`porter.rs`) interprets recipes and knows nothing tool-specific. Matches `knowledge/15`/`17` ("more ported OSS tools = data entries").
- **Command construction is pure** (on the method enums), so *what command runs* is unit-testable without touching the system; `porter.rs` does only the I/O.
- **macOS reality handled:** Finder-launched apps get a minimal PATH, so `brew` is located explicitly (`/opt/homebrew`, `/usr/local`, then PATH) rather than trusted on PATH.
- **Device detected, not asked** (`device.rs`): the porter is on the machine, so it infers the tier (Apple Silicon vs Intel) and removes the website's device-picker screen — one less step (`skills/design.md`).

**UX (`skills/design.md`):** verb-first headlines from the decider; the **answer is the tool we actually install (Buzz)**, shown large, with the engine (Whisper) as supporting context — caught and fixed a first pass that showed "Whisper" (the model) as the thing being set up, which contradicted what the porter installs. Honest progress feedback during the (minute-long) download; a clear success state.

**Browser-previewable:** the Tauri calls degrade to a simulated port when served by plain Vite, so the UI runs and screenshots in a browser without the Rust backend compiled (`app/src/bridge.ts`).

**Tests:** Rust unit tests ship with the code — `catalog.rs` (lookup + command construction), `porter.rs` (output truncation, human error copy), `device.rs` (tier is always valid). Frontend verified end-to-end via the screenshot loop (`app/scripts/shots.mjs`): home → recommendation → working → done.

**Status:** ✅ **Verified in CI.** The founder's Mac lacked Xcode CLT + disk, so the build moved to GitHub Actions: a macOS runner compiled the Tauri backend, passed `cargo test` (9 porter tests), and ran the **real `brew install --cask buzz`** — all green (run `27621986664`). The Linux job (decider 64 tests, website 9 + build) is green too. Only the GUI launch (`open -a Buzz`) is uncovered (needs a desktop session). The local desktop run remains a nice-to-have once CLT + disk allow (`cd app && npm run tauri dev`).

## Shared decider (`packages/decider/`)

### 2026-06-16 — Extracted the matching brain into `@verdure/decider`

**What:** The decision logic + taxonomy + cards moved from `website/lib` into a workspace package (`packages/decider/`): `tiers`, `modality`, `matrix` (the pure `decide()`), `intent` (the classifier), `recommend` (presentation). The website and the new desktop porter both import `@verdure/decider`; npm workspaces symlink it. `session.ts`/`sheets.ts` (browser/Google-specific) stayed in `website/lib`.

**Problem it solves:** `CLAUDE.md` requires the matrix be the single source of truth across surfaces. With two surfaces (website + porter), copy-paste would drift. One package, imported by both, makes drift impossible (`[OQ-49]` — chose shared package over copy).

**Key decisions:** the package ships **TypeScript source** (no build step) consumed directly by Next (`transpilePackages`), Vite, and Vitest. A barrel (`src/index.ts`) re-exports everything; removed a duplicate `Feasibility` re-export so `export *` stays unambiguous. Root `package.json` declares the npm workspaces (`packages/*`, `website`, `app`).

**Tests:** the decider's own suite (64 tests — matrix/modality/recommend/intent) moved with it and passes from the package. The website's remaining suite (9 sheets tests) and a full production build both pass against the imported package, proving the refactor preserved behavior.

## Website (Phase 0)

### 2026-05-29 — Multimodal recommendations (audio · image · video)

**What:** The recommendation engine went from text-LLM-only to multimodal. The classifier (`website/lib/intent.ts`) now resolves a request to a `(modality, task)` pair across text/audio/image/video (`website/lib/modality.ts`); the engine (`website/lib/recommend.ts`) dispatches by modality and returns one of three honest outcomes — `recommended`, `works-but-slow`, or `not-local-yet`.

**Problem it solves:** Typing "transcribe my recording" or "make a video" used to silently return a generic chat model. Now audio/image/video map to real local tools (Whisper, Kokoro, Real-ESRGAN, Stable Diffusion, LTX-2…) where the device can run them — and to an honest "your device can't do this locally yet" where it can't (e.g. video on a laptop), instead of a fake recommendation.

**Key design decisions:**
- **Feasibility is graphics-bound, not RAM-bound** for non-text work, so the five device tiers collapse into three capability classes (cpu-laptop / apple-silicon / gpu).
- **Copy is verb-first and jargon-free** per the "Assume zero technical knowledge" principle in `skills/design.md`: the headline says what the user will *do*; the model name is demoted to an ignorable "the tool we'd use" detail with a learn-more link.
- **Install paths are modality-specific data** returned by the engine (Whisper apps for audio, Draw Things/Fooocus for image, etc.) — Ollama/LM Studio only ever apply to text. The UI renders them generically.

**Knowledge base:** grounded in research captured in `knowledge/12-multimodal-models.md` (the source of truth); `03` scoped to text; thesis, licensing, runtime-backends cross-referenced; OQ-28..31 opened.

**Tests:** `website/lib/intent.test.ts` and `website/lib/recommend.test.ts` rewritten for the multimodal shape (40 tests total, all passing). Text model picks are unchanged and still asserted.

**Not yet done:** visual self-review (screenshot loop) of the new recommendation page — no browser tooling installed locally. Device-picker still uses RAM/Apple-Silicon jargon (single-card redesign queued).

### 2026-05-29 — Waitlist persistence via Google Sheets

**What:** `/api/waitlist` now appends each signup as a row in a Google Sheet instead of logging to the console. New deep module `website/lib/sheets.ts` owns auth (service-account JWT), private-key parsing, column order, and the append call.

**Problem it solves:** Phase 1 launch-list capture was a `console.log` stub — signups weren't durable. Closes the persistence half of `[OQ-10]`.

**Behavior:**
- Configured (env vars set): appends a row; an append failure returns a retryable 502 so no real signup is silently lost.
- Not configured: logs the signup server-side and returns success, keeping the four-screen flow exercisable before the Google Cloud setup is done.

**Setup:** `website/SETUP_SHEETS.md` (Sheet + service account + env vars), template in `website/.env.example`. The user owns the Google Cloud setup.

**Tests:** `website/lib/sheets.test.ts` — 9 unit tests over the pure helpers (key normalization, config loading, column order). The networked append is not unit-tested; it needs live credentials (see `TEST_REPORT.md`).

## Orchestration binary (Phase 1)

_No features logged yet — Rust scaffold only (`src/`)._
