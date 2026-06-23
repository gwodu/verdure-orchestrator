# 21 — Handoff: where we are, alignment, and next steps

**Status:** 🟢 Current working state as of 2026-06-20. **New session? Read this first** (then `18` pivot, `20` personal-AI, `19` predictor). This is the single source of truth for *where the build is*; older files (`01/14/15/16/17`) predate the pivot — see "Doctrine" below.

## What exists and works (branch `mvp-porter-buzz`, not merged to main)
The MVP is a **Tauri desktop app** (`app/`) — Verdure, the local personal AI — that runs models **inside one interface**, on an Intel Mac, shipped via CI (no local toolchain; see "How to build/ship").

Verified on the founder's 2019 Intel Mac (8 GB, macOS 15):
- **Transcription (file)** ✅ — drop audio → text, on-device via whisper.cpp (`base.en`). Real run measured ~3.9× real-time.
- **Transcription (live mic)** 🟡 — record from the microphone, text streams in as you speak (`live.rs`: `cpal` capture + a pure energy-gated `Segmenter` that cuts at pauses + transcribe-once-per-segment with a resident Whisper context). Built 2026-06-23, segmenter unit-tested + cross-compiles in CI; **on-device confirmation pending** (needs a tagged Intel build + the macOS Microphone-permission grant — unsigned-app TCC is the specific risk to watch).
- **Performance predictor** ✅ — estimates speed from device specs (`packages/decider/performance.ts`), calibrated by the real measurement; anchored to the founder's Mac.
- **Storage management** ✅ — per-model size + Remove (in-panel) and a global "Manage storage" screen; models download on first use, removable, re-download on demand.
- **Chat + grammar tool-calling** 🟡 — local Qwen2.5 via llama.cpp; GBNF-grammar-locked tool calls; "I need to dictate" → confirm → opens transcription; `intent.ts` cross-check. **Crash history:** OOM-aborted on the 8 GB Mac even at 0.5B (only ~2 GB free). Now model selection is **available-RAM-aware** (`model::chat_model_for_available` + `device::system_resources`, 2026-06-23): picks a model that fits free memory or declines honestly. On-device confirmation pending; the *guaranteed* fix (process isolation) is the next P0.

Green: decider 74 tests; porter Rust 33 tests (incl. 5 live-transcription segmenter tests + 5 machine-context/model-fit tests); CI (Linux decider+website, macOS porter incl. x86_64 cross-compile + model-URL checks).

## Doctrine (current — supersedes the pre-pivot files)
- **Build apps for AI, with the user at the core of context** (`20`). Verdure is the personal AI, not an assistant.
- **One interface over headless OSS engines** (`18`). We compile engines in (whisper.cpp, llama.cpp) and wrap them; we never build a model/engine, but we DO build the one surface. This **reverses** the old "port whole apps, never build a surface" line in `14/15/17`.
- Local, private, no telemetry, owned by the user (`06`). Honest performance, sized to the device. Cheapest/safest/easiest (`18`).

## Alignment with core principles
**Aligned ✅**
- Local / private / no telemetry / on-device — fully.
- One interface over headless engines — whisper + llama compiled in, wrapped in Verdure's own UI.
- Capabilities-as-tools — grammar-constrained tool-calling is "apps for AI" step one.
- Honest performance — predictor + measured + RAM-gating by detected specs.
- Tests ship with features; CI green; living logs kept.

**Gaps / debt ⚠️ (fix to stay in line)**
1. **Naming drift.** The crate/binary/identifier are still `verdure-porter` / `ai.verdure.porter`. The product is no longer a "porter" (app installer) — it's the personal AI. Rename (mind the data-dir/identifier migration so downloaded models aren't orphaned).
2. **Legacy porter is transitional scaffolding.** `catalog.rs`, `porter.rs`, the `port_tool` command, and the Buzz DMG path are now only a *fallback* for un-wrapped modalities (image/video still route to the old recommendation/porter screen). This contradicts the "build, don't port" doctrine. Decide: keep as explicit transitional fallback, or remove as modalities get wrapped. Document either way.
3. **Stale knowledge.** `01/14/15/16/17` still describe the pre-pivot "port apps" doctrine (tracked by `[OQ-52]`). They'd mislead a fresh reader. Reconcile to `18/20`.
4. **Robustness — ggml aborts the whole app.** A model OOM/error calls `abort()` (SIGABRT) and kills the GUI instead of surfacing an error (this caused the 8 GB chat crash). Violates "guardian of the UX." The durable fix is **running the model runtime in a separate process** so a model failure can never take down Verdure. High priority before real users.
5. **Chat reloads the model from disk every message.** `llm::decide` does `load_from_file` + new context per call (~0.4–1 GB load each time) → slow, wasteful. Needs a **resident model/session** (Tauri managed state holding the loaded model + context), which also unlocks conversational memory.

## Next steps (prioritized — pick up here)
**P0 — correctness & robustness (before this touches real users)**
0. Confirm **live mic transcription** on-device (built 2026-06-23): tag an Intel build, install, **grant Microphone permission**, and speak — verify segments stream in and the cut cadence feels right. Watch for the unsigned-app TCC risk (macOS may refuse mic access to an unsigned `.app`); if so, signing/notarization is the fix. Also a chance to tune the `Segmenter` defaults (threshold/silence/min/max) against a real mic + room.
1. ~~Confirm the 0.5B chat build runs without crashing on the 8 GB Mac.~~ **Resolved/superseded 2026-06-23:** it still crashed (SIGABRT) even at 0.5B — diagnosed as *available*-RAM exhaustion (~2 GB free of 8 GB; the gate keyed on total RAM). Fixed by **memory-aware model picking**: `device::system_resources` reads live free RAM/disk, `model::chat_model_for_available` picks a model that fits or declines honestly (no load → no abort). Built; on-device confirmation pending (tag a build, chat, expect either a working 0.5B reply OR the honest "not enough free memory" message — NOT a crash). Calibrate the `runtime_mb` footprints to what actually loads. Still: have the user Remove the orphaned 1.5B model (1 GB) via Storage — disk is at ~2.5 GB free.
2. **Isolate the model runtime in a separate process** so a ggml `abort()` can NEVER crash the UI; surface failures as errors. [gap #4] **Now the top robustness priority** — the memory gate (step 1) shrinks the crash window but is mitigation, not a guarantee; this is the real fix. The live `system_resources` context feeds it (decide before spawning, kill/restart the child cleanly).
3. **Resident chat model + conversational memory** (design together): keep the loaded model + context in Tauri state across messages (fast replies), and carry recent turn history in the prompt (the cheap memory fix — NOT RAG). [OQ-55 memory; the user explicitly asked.] Pairs naturally with #2 (the resident model lives in the isolated runtime).

**P1 — alignment & cleanup**
4. Reconcile `01/14/15/16/17` to the pivot doctrine. [OQ-52]
5. Rename `porter` → Verdure across the crate/binary (decide identifier/data-dir migration). [gap #1]
6. Decide the legacy-porter's fate (keep transitional vs remove) and make un-wrapped intents show an honest "coming soon" rather than the app-install recommendation. [gap #2]

**P2 — the vision**
7. **Personal context core (RAG)** — local, user-owned store every model reads/writes; the "you at the core of context" substrate. [OQ-56]
8. **AI-first dictation** — a system-wide voice button → commands into Verdure + transcription into the context core (the Wispr Flow disruptor). [OQ-57] *Foundation now exists:* live mic capture + streaming Whisper (`live.rs`, 2026-06-23); what's left is the system-wide hotkey/surface and routing dictation into the context core.

## How to build / ship (no local toolchain needed)
The founder's Mac has **no Xcode CLT and tight disk**, so everything builds in CI:
- Push to `mvp-porter-buzz` → CI runs (`.github/workflows/ci.yml`): decider+website (Linux), porter (macOS, incl. `cargo test` + x86_64 cross-compile check + model-URL reachability).
- To produce a runnable Intel app: `git tag app-build-N && git push origin app-build-N` → `build-app.yml` cross-compiles x86_64 and uploads `verdure-porter-macos-intel` (.dmg/.app).
- Install on the Mac: `gh run download <run-id> -n verdure-porter-macos-intel`, then mount the dmg, copy `Verdure.app` to /Applications, `xattr -cr` it, `open` it. (See the session's install one-liner.)
- Key fix already in place: `app/src-tauri/.cargo/config.toml` forces `MACOSX_DEPLOYMENT_TARGET=10.15` so whisper/llama cross-compile (ggml `std::filesystem`).

## Environment constraints (carry forward)
2019 Intel Mac (x86_64), **8 GB RAM** (hard ceiling — a 0.5B-class machine for in-app chat), no Xcode CLT, ~3 GB free disk. Build/verify in CI; ship a downloadable Intel app. See the `build-env-constraints` memory.
