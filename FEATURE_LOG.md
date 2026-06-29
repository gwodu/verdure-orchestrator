# FEATURE_LOG.md

Living record of features built across the Verdure monorepo — the **Verdure desktop app** (`app/`), the shared decider package (`packages/decider/`), the Phase 0 website (`website/`), and the long-term Rust core (`src/`, `tests/`). Newest first. The knowledge base (`knowledge/`) is the shared brain for all of them.

## Verdure desktop app (`app/`) — unified interface (post-pivot)

> Pivoted 2026-06-18 (`knowledge/18`) from "install separate apps" to **one interface that runs models in-app**, then to **Verdure as the personal AI** (`knowledge/20`). The "porter" naming in code/below is legacy; see `knowledge/21-handoff.md` for current state + next steps.

### 2026-06-26 — Chat: real sampling (fixes the `uids!!!!` garbage + slowness)

**What:** Replaced the pure-greedy sampler in `llm::decide` with a proper chain — repetition penalty → top-k → top-p → light temperature → distribution pick (`penalties(64, 1.1, 0, 0)`, `top_k(40)`, `top_p(0.9)`, `temp(0.6)`, fixed seed). Lowered `MAX_NEW_TOKENS` 512 → 256.

**Problem it solves (observed 2026-06-26):** with greedy argmax, the q4 0.5B chat model collapsed into a repetition loop — every reply was `uids!!!!!!!!!!!…`. Greedy is deterministic argmax with no penalty, so once a tiny quantized model (pushed off-distribution by the JSON-forcing system prompt) latches onto a token it repeats it forever. Worse, the loop **never reaches an end-of-turn token**, so generation ran the full 512-token budget every message — which is *also* why it was slow. One cause, both symptoms. A repetition penalty breaks the loop; a light temperature keeps replies natural; the model can then actually stop (EOG fires early). This stays within the no-grammar constraint from the crash fix below — plain sampling, lenient parse, UI cross-check unchanged.

**Not the whole speed story:** `llm::decide` still reloads the model from disk every message (~0.4–1 GB) — `[OQ-59]` (resident model in Tauri state) is the rest of the latency fix.

**Verification:** could not compile on this Mac (build gated on Xcode CLT — see `[OQ-61]`/handoff); the existing `parse_decision` tests are unaffected and the sampler API was checked against `llama-cpp-2` 0.1.150 docs. On-device run still pending.

### 2026-06-26 — Model picking now weighs *active* compute, not just total params (`knowledge/03`)

**What:** Added an "Active parameters, not just total: the speed axis" section to `03-model-matrix.md` and a Gemma 4 E2B model entry. The matrix keyed only on total params (= footprint); this captures that **generation speed tracks the params activated per token**, which sparse-activation models (MoE; Gemma 4's Per-Layer Embeddings) keep far below their stored size. Net rule: *total params → does it fit; active params → does it feel fast.*

**Why (user request):** evaluating Gemma 4 E2B for the chat front door because sparse activation should give snappy short-prompt replies. Research surfaced the catch worth recording: E2B is ~2.3B *effective* but a **~3.1 GB** Q4_K_M file — the <1.5 GB active footprint needs runtime PLE offload (unverified in our pinned `llama-cpp-2`), and it uses Gemma's chat template, not ChatML. So it's a *roomy-tier* pick, **not** a floor replacement for the 0.5B. Tracked for adoption as `[OQ-61]`.

**Code wiring (decision: "add as roomy-tier, verify first"):** registered `GEMMA4_E2B` in `model.rs` (in `MODELS` so Storage manages it uniformly), but held OUT of the live pick behind a `GEMMA4_E2B_VERIFIED` flag (currently `false`) so the picker never auto-selects an unverified model that could OOM-abort the app. Added a `PromptStyle` (ChatMl | Gemma) that travels with each model; `llm::decide` + `format_tool_prompt` now branch on it (Gemma uses `<start_of_turn>`/`<end_of_turn>` + a tokenizer-added BOS, no system role). The template formatting is **the part testable without a model**, so it's unit-tested now; flipping the model on after on-device verification is then a one-line change. The current shipping default is unchanged (the Qwen pair).

### 2026-06-23 — The real chat-crash fix: remove the GBNF grammar sampler

**What:** Removed the grammar-constrained sampler from `llm::decide`. The model now samples plainly (greedy) and `parse_decision` is **lenient** — it extracts the first `{...}` JSON object if present, otherwise treats the whole output as a plain answer. Tool routing does not depend on perfect JSON: the UI's deterministic `classifyIntent` cross-check is the real router.

**Problem it solves — the actual chat crash.** A **symbolicated** crash report (2026-06-23) showed chat's `SIGABRT` was **not** memory: the trace is `ggml_abort ← llama_grammar_reject_candidates ← llama_grammar_apply_impl ← llama_sampler_sample ← llm::decide`, with only 716 MB malloc'd. llama.cpp's GBNF grammar aborts the **whole process** when it reaches a state with no valid continuation (e.g. a multi-byte token the `char` rule can't extend). The grammar we'd added "for reliability" was the crash. (My first diagnosis blamed RAM off an *unsymbolicated* report + the prior 1.5B OOM history — wrong; corrected here.)

**Why removing it is right:** the grammar was redundant — `classifyIntent` already routes transcription deterministically regardless of the model's output — and actively dangerous (an in-process `abort()` is uncatchable). Plain sampling + lenient parse + the cross-check is safer and simpler. A grammar can return only once the model runs in an **isolated process** (knowledge/21 — now the top robustness P0).

**Tests:** 3 new `llm.rs` parse tests (plain prose → answer; JSON wrapped in prose → extracted; unknown action → answer) alongside the existing two.

### 2026-06-23 — Live machine-context + memory-aware model picking

**What:** Verdure reads a **live snapshot of the machine's free resources** — available RAM, free disk, cores, arch, Low Power Mode (`device::system_resources`, `system_resources` command) — and uses it to **pick the chat model that fits the memory free *right now***, declining honestly when nothing fits. A "Your computer" readout on the Storage screen makes that context visible.

**What it does and doesn't fix:** this is the machine-as-context foundation the user asked for, and it prevents a *genuine* OOM-abort when memory is truly tight. It did **not** fix the chat crash that prompted it — that was the grammar sampler (see the entry above); memory was fine (~2 GB free, 0.5B fit). Keeping it anyway: it's the right substrate for honest orchestration ("you at the core of context", knowledge/20), extending the static `device.rs` specs + the transcription predictor (knowledge/19) to *live* resources.

**Architecture (`skills/architecture.md`):** the `vm_stat`/`df`/`pmset` parsers are **pure** so they're unit-tested; the I/O wrappers shell out to fixed paths (PATH-safe, like the existing sysctl). `runtime_mb` is a per-model footprint estimate (0.5B ≈ 1100 MB, 1.5B ≈ 2400 MB) — **provisional, to calibrate on-device**.

**Tests:** 4 new `device.rs` parser tests (vm_stat available-memory math, df column, low-power, missing-page-size fallback) + 1 `model.rs` test (picks larger when roomy, small when tight, declines when nearly full). Frontend typechecks; Storage screenshot shows the readout.

### 2026-06-23 — Live (microphone) transcription — speak, watch text appear

**What:** A second way into on-device transcription, alongside drop-a-file: **record live from the mic**. The transcribe panel gained an **Upload a file / Record live** toggle. In live mode, a Start/Stop button captures the microphone (`cpal`, pure-Rust CoreAudio) and streams finished sentences into the transcript as you speak, entirely on-device. A red recording button + pulsing dot + elapsed timer make the recording state obvious.

**Problem it solves:** the file path only helps if you already *have* a recording. Live capture is the everyday case — a meeting, a voice note, dictation — without any other app. It's the foundation the roadmap's "AI-first dictation" (knowledge/21 P2) builds on.

**Architecture (`skills/architecture.md`):**
- **Whisper isn't streaming, and the floor device is CPU-bound** (Intel Mac, base.en ≈ 3.9× real-time). Re-transcribing a growing buffer every second would fall behind. So `live.rs` **segments and commits**: a pure, unit-tested `Segmenter` (energy-gated VAD-lite) cuts the mic stream at natural pauses — never mid-word — and each segment is transcribed **once** as it completes. This keeps up with real time on this hardware; text lands a few seconds after each pause (the normal live-captioning cadence).
- **Resident model per session.** The Whisper context is loaded **once** when recording starts (`transcribe::load`) and reused for every segment — no per-segment reload. A step toward the broader "resident model" debt in knowledge/21.
- **cpal `Stream` is `!Send`**, so the capture stream is built, used, and dropped on one owned thread; samples flow to the worker over an `mpsc` channel; an `AtomicBool` + a join (in Tauri managed state) make Start/Stop clean. Mic frames reuse the file path's `downmix_to_mono`/`resample_linear` (one tested audio core).
- **Events mirror the existing pattern:** `live:status` / `live:segment` / `live:error` / `live:stopped`, consumed in `bridge.ts` exactly like `transcribe:progress`.

**UX (`skills/design.md`):** one task ("Turn speech into text"), two inputs behind a calm toggle; the file emoji moved to 📄 so 🎙️ now means *record*. Honest browser-preview simulation so the flow is reviewable without a mic.

**Permissions:** `src-tauri/Info.plist` adds `NSMicrophoneUsageDescription` (plain, no-harvesting language) — required or macOS kills the app on mic access instead of prompting.

**Robustness caveat (carried, knowledge/21 gap #4):** ggml can `abort()` the whole process on a model error, and live mode runs Whisper repeatedly — raising that exposure. The durable fix (runtime in a separate process, P0 #3) is **not** in this change; the model-file validation in `model::ensure` is the only guard for now. Logged as the next hardening step.

**Tests:** 5 new Rust unit tests for the pure `Segmenter` (cuts on a pause after speech, respects the minimum length, force-cuts a non-stop talker, discards pure silence, flushes trailing speech). Frontend typechecks + a new `2b-live.png` visual-review shot of a mid-session live transcript.

**Status:** 🟡 Built; **on-device confirmation pending** (a tagged Intel build on the founder's Mac). CI can't exercise a microphone, but it compiles + cross-compiles the new `cpal` path and runs the segmenter tests. A specific risk to verify on-device: an unsigned/ad-hoc `.app` must still be granted macOS Microphone permission. See TEST_REPORT.md.

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
