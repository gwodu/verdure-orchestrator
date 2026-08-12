# 21 — Handoff: where we are, alignment, and next steps

*Role: current **state + next steps** — "where are we now / what do I do next." Overwritten as the build moves; it does not narrate chronology. For the dated history of how we got here, see `KNOWLEDGE.md`'s changelog.*

**Status:** 🟢 Current working state as of 2026-08-10. The app now runs **on-device on three platforms** (Intel Mac, iPad via TestFlight, Pixel 8a via CI APK) with chat, live dictation, and resumable model downloads working. **The next gate is distribution:** the Play open-testing workflow is ready, but the first upload makes the Android package name permanent — the `porter` rename decision (`[OQ-60]`) must be made first. **New session? Read this first** (then `18` pivot, `20` personal-AI). This is the single source of truth for *where the build is*; older files (`01/14/15/16/17`) predate the pivot — see "Doctrine" below.

## What exists and works (branch `mvp-porter-buzz`, not merged to main)

The MVP is a **Tauri app** (`app/`) — Verdure, the local personal AI — that runs models **inside one interface**, shipped entirely via CI (no local toolchain; see "How to build/ship").

Verified on-device (founder's 2019 Intel Mac 8 GB, iPad via TestFlight, Pixel 8a via sideloaded APK):

- **Chat** ✅ — a plain conversation with a local model via llama.cpp. **Confirmed loading and generating on the iPad** (2026-07-27, answering `[OQ-64]`'s core question). The JSON tool-calling wrapper was **removed** on-device evidence (the tiny model over-triggered tools and leaked malformed JSON); capabilities-as-tools returns later, constrained + process-isolated (`[OQ-55]`). Model lineup: **ByteShape Qwen3 4B** (`Q3_K_S-2.77bpw`, ~1.4 GB) as the top RAM-gated pick, Qwen2.5 0.5B/1.5B as floor/fallback. A first-launch **model picker** (Small/Balanced/Powerful, with download size + per-device memory fit, persisted, changeable from the header) replaced the earlier "best model only" toggle. The 8 GB Mac exercises only the fallback tier (~1.5 GB free vs the 4B's ~2.3 GB floor); the 4B's `runtime_mb` still needs on-device calibration (`[OQ-64]` residual).
- **Transcription (file)** ✅ — drop audio → text via whisper.cpp `base.en`; ~3.9× real-time measured on the Intel Mac.
- **Transcription (live mic)** ✅ **on all three platforms** — `live.rs`: `cpal` capture + a pure energy-gated `Segmenter` + transcribe-once-per-segment with a resident Whisper context. macOS confirmed 2026-06-30; iPad with the TestFlight builds; **Android confirmed 2026-08-03** (Pixel 8a) after root-causing the `ndk_context` initialization gap in Tauri v2 (changelog 2026-08-02).
- **Dictation into chat** ✅ — the mic button dictates straight into the chat box; the standalone transcription page was removed (2026-07-25). First mic tap without Whisper installed shows a size-and-consent prompt before downloading.
- **Downloads** ✅ — streamed to disk with a live progress bar, **resumable** (progress survives interruption; one-tap resume), keep-screen-awake on mobile during large downloads.
- **Out-of-space help** 🟡 (`[OQ-67]`, built 2026-08-12, on-device confirmation pending) — unfinished `.part` downloads visible + reclaimable in Storage; switching chat models offers removing the old one; Android's picker can ask the OS to clear other apps' cached junk (`StorageManager.allocateBytes`); macOS opens the system storage screen, iOS gets honest instructions. Principle: Verdure never deletes a byte it didn't create.
- **Performance predictor** ✅ — estimates speed from device specs (`packages/decider/src/performance.ts`), calibrated by real measurements.
- **Storage management** ✅ — per-model size, Remove (in-panel + global screen), disk fail-state warnings, free-space readout.
- **UI** — chat-first "prairie" redesign shipped 2026-07-17 (app opens straight into chat; mic in the input row; gear → Storage).
- **Distribution** — macOS Intel .dmg via `app-build-N` tags; **iOS via TestFlight** (signed, no expiry, `testflight-N` tags); **Android test APK** via `android-build-N` tags; **Play open-testing workflow ready** (`build-android-play.yml`, signed .aab) but ungated founder-side steps remain — see Next steps.

Green: decider + website suites pass locally; porter Rust suite (44 unit tests incl. segmenter, model gating, prompt styles, Android meminfo parser) compiles and runs in CI. **Last green CI: `d4e861d` (2026-08-04), the branch tip.**

## Doctrine (current — supersedes the pre-pivot files)

- **Build apps for AI, with the user at the core of context** (`20`). Verdure is the personal AI, not an assistant.
- **One interface over headless OSS engines** (`18`). We compile engines in (whisper.cpp, llama.cpp) and wrap them; we never build a model/engine, but we DO build the one surface. This **reverses** the old "port whole apps, never build a surface" line in `14/15/17`.
- Local, private, no telemetry, owned by the user (`06`). Honest performance, sized to the device. Cheapest/safest/easiest (`18`).

## Alignment with core principles

**Aligned ✅**
- Local / private / on-device / no telemetry — fully, on all three platforms.
- One interface over headless engines — whisper + llama compiled in everywhere; no separate apps.
- Honest performance — predictor + real measurement + RAM-gating on *available* memory; the picker shows per-device fit before download; nothing silently downgrades without the UI saying which model answered.
- Honest consent — every model download asks first, with its size.
- Tests ship with features; CI green; living logs current.

**Gaps / debt ⚠️ (fix to stay in line)**
1. **Naming — now deadline-bound.** The crate/binary/identifier are still `verdure-porter` / `ai.verdure.porter`, and **the first Play upload makes the Android package name permanent** (`[OQ-66]` 2026-08-03 note). Decide the rename (`[OQ-60]`) *before* that upload; the desktop crate/data-dir rename can follow with a migration plan.
2. **Robustness — ggml aborts the whole app.** A model OOM/error calls `abort()` (SIGABRT) and kills the GUI instead of surfacing an error. The durable fix is **process isolation** for the model runtime (`[OQ-58]`). P0 before real users.
3. **Chat reloads the model from disk every message.** `llm::decide` still does `load_from_file` + new context per call. Needs a **resident model/session** in Tauri state, which also unlocks conversational memory (`[OQ-59]`).
4. **Legacy porter is transitional scaffolding.** `catalog.rs`, `porter.rs`, `port_tool` remain registered; the old recommendation flow (`renderHome`) is retained but unreachable. Decide keep-as-fallback vs remove (`[OQ-60]`); un-wrapped intents should show an honest "coming soon."
5. **Capabilities-as-tools is currently OFF.** Tool-calling was removed on on-device evidence (2026-07-27); the "apps for AI" step one (`20`) is deferred until it can come back constrained + process-isolated (`[OQ-55]`, depends on `[OQ-58]`).

## Next steps (prioritized — pick up here)

1. **Decide the rename, then ship Play open testing.** The rename decision (`[OQ-60]`, see its 2026-08-10 gate note) blocks the first `.aab` upload. Then the founder-side Play steps: upload keystore + secrets, create the Play app, Data Safety / content rating / privacy-policy, first upload, open the test track (`FEATURE_LOG` 2026-08-03 has the checklist).
2. **Process-isolate the model runtime** (`[OQ-58]`) — a ggml `abort()` must never take down the UI. Also removes the 4B load-abort risk and re-opens the door to grammar-constrained tool-calling.
3. **Resident chat model + conversational memory** (`[OQ-59]`) — keep the loaded model + context in Tauri state; carry recent history in the prompt (cheap memory, not RAG).
4. **Calibrate the 4B on a capable device** (`[OQ-64]` residual) — measure its real resident footprint, set `runtime_mb` from measurement, and judge reply quality.
5. **Legacy-porter decision** (gap #4) and the deferred UX polish (raw-error copy is fixed as of 2026-08-10; remaining: Storage spec panel jargon).

**P2 — the vision**
- **Personal context core** — local, user-owned store every model reads/writes (`[OQ-56]`).
- **AI-first dictation** — system-wide voice button → commands into Verdure + transcription into the context core (`[OQ-57]`). Foundation exists in `live.rs`; what's left is the system-wide surface and routing.

## How to build / ship (no local toolchain needed)

The founder's Mac has **no Xcode CLT and tight disk**, so everything builds in CI:
- Push to `mvp-porter-buzz` → CI runs (`.github/workflows/ci.yml`): decider+website (Linux), porter (macOS, incl. `cargo test` + x86_64 cross-compile check + model-URL reachability).
- **macOS Intel app:** `git tag app-build-N && git push origin app-build-N` → `build-app.yml` → `verdure-porter-macos-intel` (.dmg/.app). Install: `gh run download <run-id> -n verdure-porter-macos-intel`, mount, copy to /Applications, `xattr -cr`, open.
- **iPad via TestFlight** (the current iOS channel, `[OQ-65]`): `git tag testflight-N` → `build-testflight.yml` signs with the founder's App Store Connect API key (GitHub secrets: `ASC_API_KEY_P8`/`ASC_API_KEY_ID`/`ASC_API_ISSUER_ID`/`APPLE_TEAM_ID`) and uploads. iOS bundle ID is `ai.verdure.porter.898PY37K5S` to match the App Store record. Test vehicle only — the shippable iOS strategy stays `11`'s thin-decider.
- **Android test APK** (`[OQ-66]`): `git tag android-build-N` → `build-android.yml` (Linux runners) → test-signed `.apk` (throwaway CI key — uninstall before installing a differently-signed build).
- **Android Play open testing:** `git tag android-play-N` → `build-android-play.yml` → signed `.aab` via the persistent upload keystore (secrets pending founder-side). Both Android workflows share `app/scripts/patch-android-project.py` (manifest + mic wiring), so they cannot diverge.
- Legacy unsigned iOS path (`ipad-build-N` → unsigned .ipa, 7-day sideload) is superseded by TestFlight.
- Key fix in place: `app/src-tauri/.cargo/config.toml` forces `MACOSX_DEPLOYMENT_TARGET=10.15` so whisper/llama cross-compile.

## Environment constraints (carry forward)

2019 Intel Mac (x86_64), **8 GB RAM** (hard ceiling — exercises only the fallback chat tier), no Xcode CLT, ~3 GB free disk. Build/verify in CI; test the 4B on the iPad or Pixel 8a. See the `build-env-constraints` memory.
