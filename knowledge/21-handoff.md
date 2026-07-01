# 21 — Handoff: where we are, alignment, and next steps

*Role: current **state + next steps** — "where are we now / what do I do next." Overwritten as the build moves; it does not narrate chronology. For the dated history of how we got here, see `KNOWLEDGE.md`'s changelog.*

**Status:** 🟢 Current working state as of 2026-06-30. **Entering a redesign** — see "Next steps" below; the inspection that precedes it is `KNOWLEDGE.md`'s 2026-06-29 changelog entry + `skills/inspection.md`. **New session? Read this first** (then `18` pivot, `20` personal-AI, `19` predictor). This is the single source of truth for *where the build is*; older files (`01/14/15/16/17`) predate the pivot — see "Doctrine" below.

## What exists and works (branch `mvp-porter-buzz`, not merged to main)
The MVP is a **Tauri desktop app** (`app/`) — Verdure, the local personal AI — that runs models **inside one interface**, on an Intel Mac, shipped via CI (no local toolchain; see "How to build/ship").

Verified on the founder's 2019 Intel Mac (8 GB, macOS 15):
- **Transcription (file)** ✅ — drop audio → text, on-device via whisper.cpp (`base.en`). Real run measured ~3.9× real-time.
- **Transcription (live mic)** ✅ — record from the microphone, text streams in as you speak (`live.rs`: `cpal` capture + a pure energy-gated `Segmenter` that cuts at pauses + transcribe-once-per-segment with a resident Whisper context). Built 2026-06-23, segmenter unit-tested + cross-compiles in CI; **confirmed working on-device 2026-06-30** (mic permission grant cleared; unsigned-app TCC was not a blocker in practice).
- **Performance predictor** ✅ — estimates speed from device specs (`packages/decider/performance.ts`), calibrated by the real measurement; anchored to the founder's Mac.
- **Storage management** ✅ — per-model size + Remove (in-panel) and a global "Manage storage" screen; models download on first use, removable, re-download on demand.
- **Chat tool-calling** 🔴 **not working on-device — folded into the redesign, not a standalone P0.** Local Qwen via llama.cpp; "I need to dictate" → confirm → opens transcription; `classifyIntent` cross-check is the real router. Prior crash/quality fixes (grammar-sampler removal, RAM-aware pick, real sampling) and the parked **Gemma 4 E2B** wiring have **not** made it usable. **Base chat model is now ByteShape's low-bit Qwen3 4B** (`Q3_K_S-2.77bpw`, ~1.4 GB, Apache 2.0; `USE_BYTESHAPE_QWEN3_4B = true`), top of the pick with the Qwen 0.5B/1.5B floor as fallback — the bet being a real 4B beats the 0.5B garbage within the RAM budget. **Pending on-device:** (a) does it load on our pinned llama.cpp; (b) does it fit the 8 GB Mac at all (with ~1.5 GB free the gate falls back to Qwen — `N_CTX` already lowered to 1024 + `runtime_mb` ~2300 to help, but the 1.4 GB weights are a hard floor so RAM must be freed to ~2.3 GB to exercise it). `[OQ-64]`. *(How it got here: KNOWLEDGE.md changelog, 2026-06-23 / 06-29 / 07-01.)*

Green: decider 74 tests; porter Rust 40 tests (incl. live-transcription segmenter + machine-context/model-fit + sampling/prompt-style tests); CI (Linux decider+website, macOS porter incl. x86_64 cross-compile + model-URL checks). Last green CI: `78aaa54` (2026-06-29).

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
**▶ The next move is a redesign.** The pre-redesign inspection (2026-06-29) is done and its structural fixes are committed; what's left feeds *into* the redesign rather than racing ahead of it. The redesign's own scope is **not yet written** — define it before building. The items below are the standing inputs it must absorb (statuses current as of 2026-06-30); they are no longer an independent execution queue.

**Done since this list was first written:**
- ✅ **Live mic transcription confirmed on-device** (2026-06-30) — was P0 item 0. TCC/mic-permission cleared.
- ✅ **Chat `SIGABRT` (GBNF grammar) removed** + available-RAM-aware model pick (2026-06-23); real sampling chain + Gemma 4 E2B wiring parked behind `GEMMA4_E2B_VERIFIED` (2026-06-29). **Chat still does not work on-device** — folded into the redesign (below), not a standalone task.
- ✅ **Pivot-doctrine reconciliation** (old P1 item 4, `[OQ-52]`) — discharged by the inspection (superseded-doctrine banners on `01/14/15/16/17`).

**Standing inputs the redesign must absorb:**
- **Make chat actually work** — the open robustness/quality knot: **verify the ByteShape Qwen3 4B base model on-device** (loads on pinned llama.cpp + fits/feels usable; calibrate its `runtime_mb`) [OQ-64]; process-isolate the model runtime so a ggml `abort()` can NEVER crash the UI (surface failures as errors) [gap #4, OQ-58] — this also removes the 4B's load-abort risk; resident chat model + conversational memory in Tauri state (fast replies, cheap prompt-history memory — NOT RAG) [OQ-55, OQ-59]; resolve `[OQ-62]` (fold chat's RAM-aware pick into `@verdure/decider` vs document as a separate stack).
- **UX findings the inspection deferred to the redesign:** app UX jargon, the Storage spec panel, raw error strings surfaced to the user.
- **Rename `porter` → Verdure** across the crate/binary (decide identifier/data-dir migration so downloaded models aren't orphaned). [gap #1]
- **Decide the legacy-porter's fate** (keep transitional vs remove) and make un-wrapped intents show an honest "coming soon" rather than the app-install recommendation. [gap #2]

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
