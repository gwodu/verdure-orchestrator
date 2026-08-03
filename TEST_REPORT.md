# TEST_REPORT.md

Test discipline record across the Verdure monorepo. Decider: `npm test --workspace @verdure/decider`. Website: `npm test --workspace nextjs`. Porter app frontend: `cd app && npm run build` (typecheck) + `node app/scripts/shots.mjs` (screenshots). Porter Rust: `cd app/src-tauri && cargo test`. Long-term binary: `cargo test`. Newest first.

## Verdure desktop app (`app/`) — post-pivot

### 2026-08-02 — Android mic capture: `ndk_context` initialization [OQ-66]

**Diagnosis method (on-device, no local toolchain):** `adb logcat --pid=$(adb shell pidof ai.verdure.porter)` while tapping the mic. Isolated the fault in three steps — (1) `adb pm grant` + `appops set … allow` force-granting `RECORD_AUDIO` did **not** fix it → not permission; (2) the log shows Whisper `base.en` loading fully → not the model; (3) immediately after load, `thread 'verdure-live' panicked … ndk-context: android context was not initialized` → cpal/oboe's JNI device enumeration on an unpopulated `ndk_context`.

**Fix coverage:** native glue (`Java_..._initAudioContext`) is Android-only FFI — not unit-testable; CI compiles it via the `android-build-7` cross-build. A build-time **assertion** guards the hard-coded JNI package (`package ai.verdure.porter` must be present in the generated `MainActivity.kt`), converting a would-be runtime `UnsatisfiedLinkError` into a loud build failure.

**Mac regression:** the new deps are under `[target.'cfg(target_os = "android")'.dependencies]`, so the macOS porter build + tests are unaffected (target-gated; nothing extra compiled).

**On-device confirmation (Pixel 8a, 2026-08-03, `android-build-7`):** ✅ passed. `adb logcat` shows the `ndk-context: not initialized` panic gone (grep count 0) and `AudioRecord` reached for the first time; oboe negotiates a channel config (`getMinBufferSize: Invalid channel configuration` probe lines are benign) and live transcription streams. Method as above (install, tap mic, watch the PID-filtered log).

### 2026-07-31 — Android test build (`build-android.yml`, cfg-gated `device.rs`) [OQ-66]

**Verified in CI (2 rounds, `android-build-1..2`):**
- **`aarch64-linux-android` cross-compile succeeds** — whisper.cpp, llama.cpp, cpal, ureq/rustls, libc, and the tauri android runtime all build via the NDK. The port's decisive unknown, answered yes.
- Full pipeline green: `tauri android init` → manifest/permission patch → NDK build → zipalign + apksigner (verified) → 9 MB test-signed `.apk`.
- **Mac regression:** branch CI green after the cfg split — macOS porter compiles, all porter tests pass (incl. the new `parses_meminfo_available` parser test), iOS cross-compile + model-URL checks hold.

**New pure-logic test:** `parses_meminfo_available` — the Android available-RAM gate parses `MemAvailable` from `/proc/meminfo` and ignores `MemTotal`/`MemFree`.

**Failure history:** round 1 — native compile fully succeeded; Kotlin failed (`Expecting a top level declaration`) because the MainActivity patch rewrote a class that already had an `onCreate`. Round 2 — inject at the stable `super.onCreate` anchor → green.

**Not covered (on-device only, [OQ-66]):** whether the model runs within Android's LMK limits on a real device; cpal/AAudio mic capture + the runtime permission grant; UI at true phone width.

### 2026-07-25 — Dictation-into-chat + model picker (`model.rs`, `lib.rs`, frontend)

**New pure-logic tests (`model.rs`):**
- `explicit_choice_used_when_it_fits_declined_when_it_does_not` — a chosen model is loaded when it fits and returns a strict `ChatModelTooBig` (no silent downgrade) when it doesn't.
- `unknown_choice_falls_back_to_auto_pick` — empty/stale id → largest that fits; only a non-strict error when nothing fits.
- `chat_options_report_fit_against_the_device` — the picker options report `fitsMemory` correctly (Powerful drops out on a tight machine, Small stays), and none show installed for an empty data dir.

**Frontend:** `tsc --noEmit` clean (strict); `vite build` clean. The dictation flow and the picker were reviewed in `vite preview` (the browser build simulates live segments + returns plausible model options, so the UI is exercisable); real mic + real download are on-device (next TestFlight build).

**Note:** the `chat` command signature changed (`allow_fallback: bool` → `model_id: String`) and a `chat_model_options` command was added — both compile-checked in CI.

### 2026-07-25 — TestFlight pipeline verified end-to-end (`build-testflight.yml`) [OQ-65]

**Verified in CI** (`testflight-4`, run 30163433105, all steps green): signed archive → export signed App-Store `.ipa` → **upload to TestFlight accepted by App Store Connect**. This exercises the whole distribution path: Xcode automatic signing via the App Store Connect API key, provisioning-profile creation, and altool upload authentication.

**Failure ladder (each a real config gate, fixed in order — useful for the next platform):**
- `testflight-1/2`: `exportArchive Cloud signing permission error` → App Manager API key lacks cloud *distribution* signing; **Admin** role required. (Accepting the pending Paid-Apps/EU agreements was also necessary — both block distribution signing.)
- `testflight-3`: export passed; upload failed `Cannot determine the Apple ID from Bundle ID 'ai.verdure.porter'` → build bundle ID ≠ app-record bundle ID.
- `testflight-4`: bundle ID overridden to `ai.verdure.porter.898PY37K5S` (the record's) → green.

**Not automatable here:** the on-device install + run from TestFlight (Apple-side processing + the founder's iPad) — that's the human verification step, same class as the sideload runs. On-device chat/transcription results still feed `[OQ-64]`/`[OQ-65]`.

### 2026-07-17 — Chat-first redesign + prairie aesthetic (`main.ts`, `styles.css`)

**Change:** presentation only — boot into chat, mic button, gear→Storage, back-nav to chat, warm gradient theme via CSS tokens. No backend/logic touched (model picker, `choose_chat_model`, chat command, parsers all unchanged), so **no new unit tests** — there is no new pure logic to assert.

**Verified:** `tsc --noEmit` clean (strict, `noUnusedLocals`); `vite build` clean. Existing decider/porter test suites unaffected (no imports changed in shared code).

**Not done — honestly:** the design.md **visual self-review loop** (render + look) could not be automated in this environment — the Chrome extension wasn't connected and no Playwright/Puppeteer is installed (and installing a headless browser on the tight-disk founder Mac wasn't warranted). Reviewed manually via `vite preview`; the authoritative visual check is on-device in the next iPad build. Follow-up worth doing: wire an `app/scripts/shots.mjs` (referenced in `.gitignore` but absent) so app UI has the same screenshot loop the website has.

### 2026-07-04 — iPad test build (`build-ipad.yml`, cfg-gated backend) [OQ-65]

**Change:** iOS-compatible backend (`model.rs` download split, `device.rs` syscall probes) + a CI workflow producing an unsigned iPad IPA.

**Verified in CI (3 build rounds, `ipad-build-1..3`):**
- **`cargo build --lib --release --target aarch64-apple-ios` succeeds** — whisper.cpp, llama.cpp, cpal, ureq/rustls, and the tauri mobile entry all cross-compile for iOS. This was the port's decisive unknown.
- The full pipeline is green end-to-end: `tauri ios init --ci` → plist mic-key injection → prebuilt-lib link (`libapp.a`, the template's naming, learned from round 2's link line) → unsigned xcodebuild → 5 MB `.ipa` artifact.
- **Mac regression:** branch CI green after the cfg split — macOS porter compiles, all porter unit tests pass, x86_64 cross-compile + model-URL checks hold. The Mac dependency tree is unchanged (`ureq`/`libc` are `cfg(not(macos))`).

**Deliberately not unit-tested:** the iOS syscall probes (`sysctlbyname`/`statvfs`/`os_proc_available_memory`) — `cargo test` runs on the macOS host where the `cfg(target_os = "ios")` code doesn't compile; they're thin, documented wrappers over audited `libc` + one public Apple API (testing.md FFI rules 1 & 4 followed: SAFETY comments state ownership at each boundary — nothing is allocated across it).

**Failure history (for the record):** round 1 — generated xcode-script phase requires a parent tauri-CLI process (server-addr file) absent under bare xcodebuild; round 2 — linker wanted `libapp.a` (template renames every crate's staticlib); round 3 — green.

**Not covered (on-device only, [OQ-65]):** whether iOS's per-app memory ceiling admits the 4B; mic capture via cpal/AVAudioSession; TCC on a sideloaded app.

### 2026-07-01 — "Best model only" chat toggle (`model.rs`, `lib.rs`, frontend)

**Change:** added `choose_chat_model(available_mb, allow_fallback)` (pure) + `preferred_chat_model()`, refactored the candidate order into a shared `chat_candidates()`, threaded an `allow_fallback` arg through the `chat` command and `chatSend`, and added a persisted UI toggle.

**Tests (pure logic — three new):**
- `preferred_chat_model_is_the_top_candidate` — the strict pick equals the head of the RAM picker's order (one source of truth for ordering), and is the 4B while enabled.
- `best_model_only_uses_the_preferred_or_explains_why_not` — at the preferred model's footprint it's chosen; one MB under, it returns `Err(strict=true)` carrying the model name + needed MB and **does not** downgrade, even though a smaller model would have fit. This is the whole point: no silent substitution.
- `fallback_downgrades_and_only_fails_when_nothing_fits` — with fallback on and room only for the floor, a smaller model that fits is returned (never the preferred, never a non-fitting one); below the floor's footprint it errors with `strict=false` (the "close some apps" path).

**Edge case caught:** the strict/no-fallback branch must reject the preferred model when it's one MB too big *without* consulting smaller models — the test pins that, so a future refactor can't accidentally re-introduce the silent downgrade.

**Frontend:** `tsc --noEmit` clean; `vite build` clean. Not covered: visual/behavioral review of the toggle — on-device in the tagged build (no browser/Tauri toolchain locally).

**Could not run `cargo test` on this Mac** — build gated on Xcode CLT; CI compiles + runs these on push.

### 2026-07-01 — ByteShape Qwen3 4B base chat model (`model.rs`)

**Change:** added `BYTESHAPE_QWEN3_4B` to the registry and made it the top candidate in `chat_model_for_available` (behind `USE_BYTESHAPE_QWEN3_4B`), Qwen pair as fallback; refactored the picker to a flag-built list.

**Tests (pure logic — three new + one updated):**
- `byteshape_4b_is_the_base_when_it_fits_and_falls_back_when_it_does_not` — at exactly the 4B's `runtime_mb` it's the pick; at one MB under, the 4B is skipped and whatever IS picked genuinely fits (`pick.runtime_mb ≤ available`). Guards the core invariant: **never auto-select a model larger than available RAM** (loading one OOM-aborts the app). Written value-robustly (asserts the invariant, not a specific fallback target) so it survives `runtime_mb` re-tuning — e.g. the 1024-ctx footprint drop from 2600 → 2300.
- `byteshape_4b_carries_the_chatml_prompt_style` — the model carries `PromptStyle::ChatMl` (Qwen3 = ChatML, so no new framing) and `find()` resolves its id to the Chat modality.
- `chat_model_picks_by_available_memory` (updated) — the roomy-machine pick is now the 4B when enabled (was Qwen 1.5B); the tight-machine (1500 MB → 0.5B) and near-empty (500 MB → decline) cases are unchanged, so the RAM-gate floor behavior is preserved. Written flag-aware so it stays honest if the base model is disabled.

**Edge case caught:** the fallback-boundary test pins that a model just *over* available RAM is skipped in favor of the largest that fits — the exact discipline that keeps a too-big pick from reaching `load_from_file` and aborting.

**Not covered (same constraint as prior chat work):** whether the GGUF actually loads/decodes on the pinned `llama-cpp-2` and whether it fits the founder's 8 GB Mac — both are on-device (CI has no model, no runtime memory). `[OQ-64]`.

**Could not run `cargo test` on this Mac** — build gated on missing Xcode CLT; CI compiles the backend, runs these tests, and resolves the new download URL on push.

### 2026-06-26 — Chat sampler fix (greedy → real sampling)

**Change:** swapped pure-greedy for a penalties→top-k→top-p→temp→dist chain in `llm::decide` to kill the `uids!!!!` repetition loop (and the slowness it caused — the loop never hit EOG, so every reply ran the full token budget).

**Tests (sampler):** no new unit test for sampling itself — quality is a property of the loaded model + sampler at runtime, nothing pure to assert without a GGUF in-process (CI has no model, same constraint as the no-crash behavior below). The existing five `llm.rs` `parse_decision` tests are unaffected and still lock the lenient parser. The sampler chain was checked against the `llama-cpp-2` 0.1.150 `LlamaSampler` API (signatures for `penalties`/`top_k`/`top_p`/`temp`/`dist` confirmed).

**Tests (Gemma 4 E2B wiring — roomy-tier, gated):** the parts that *are* pure got tests:
- `llm::chatml_prompt_frames_the_turn` / `gemma_prompt_uses_turn_markers_not_chatml` — assert each family's prompt framing (ChatML `<|im_start|>` vs Gemma `<start_of_turn>`, Gemma carries no system role / no literal `<bos>`) and the matching `add_bos_for` convention (`AddBos::Never` vs `Always`). This locks the template formatting so adoption is a flag flip.
- `model::unverified_gemma_stays_out_of_the_live_pick` — with the `GEMMA4_E2B_VERIFIED` flag false, even 64 GB free does not auto-select Gemma. `model::gemma_carries_the_gemma_prompt_style` — the family travels with the model (Gemma→Gemma, Qwen→ChatMl, Whisper→None). Existing `chat_model_picks_by_available_memory` unaffected (Qwen pair still the live default).

**Could not run `cargo test` on this Mac** — build gated on missing Xcode CLT (`[OQ-61]`); the sampler fix needs on-device confirmation that replies are coherent and stop early, and Gemma needs an on-device load + PLE-offload check before its flag flips.

### 2026-06-23 — Chat-crash fix (grammar sampler) + live machine-context

**The crash, correctly diagnosed (second pass).** Chat `SIGABRT`-crashed the whole app on the 8 GB Intel Mac. My **first** diagnosis (RAM/available-memory) was **wrong** — based on an *unsymbolicated* crash report + the prior 1.5B OOM history. The **symbolicated** report was unambiguous: `ggml_abort ← llama_grammar_reject_candidates ← llama_grammar_apply_impl ← llama_sampler_sample ← llm::decide`, only 716 MB malloc'd. It's the **GBNF grammar sampler**, not memory. Lesson logged: don't infer an abort cause from raw offsets — symbolicate first.

**Fix:** removed the grammar sampler; plain greedy sampling + lenient parse. 3 new `llm.rs` tests: plain prose → answer, JSON-in-prose → extracted, unknown action → answer (+ the 2 existing JSON tests). These lock the lenient parser; the actual no-crash behavior is on-device (CI has no model).

**Also shipped (separate, still valuable): live machine-context + memory-aware picking.** 5 new pure-logic tests:
- `device::parses_available_ram_from_vm_stat` — free + inactive + speculative + purgeable pages × page size; active/wired ignored.
- `device::available_ram_is_none_without_a_page_size` — malformed `vm_stat` → `None` (caller falls back to half of total).
- `device::parses_df_available_column` / `detects_low_power_mode` — `df -k` 4th column, `pmset -g` flag.
- `model::chat_model_picks_by_available_memory` — roomy → 1.5B; tight → 0.5B only; nearly full → `None`.

This did **not** fix the crash (memory was fine), but it's the right orchestration substrate and prevents a real OOM in tighter conditions. `runtime_mb` footprints are provisional, to calibrate on-device.

**Still on-device / not guaranteed:** that plain sampling never aborts on this model (expected, but unproven without the weights in CI); the `vm_stat`/`df`/`pmset` execution; gate thresholds. The only *guarantee* that a model can't crash the app is **process isolation** — not yet built, now the top robustness P0.

### 2026-06-23 — Live (microphone) transcription

**Automated (runs in CI):** 5 new Rust unit tests over the pure `Segmenter` in `live.rs` — the only part of live capture that *can* be tested without audio hardware, and the part that holds the real logic:
- `cuts_on_a_pause_after_speech` — speech then ≥ 0.6 s of silence completes a segment.
- `does_not_cut_before_the_minimum_length` — a short word + a long pause stays buffered (no sub-1 s fragments).
- `force_cuts_a_nonstop_talker_at_the_max` — 13 s of unbroken speech still yields a segment (the 12 s force-cut).
- `discards_pure_silence` — silence past the max length is dropped, never sent to the model.
- `flush_returns_trailing_speech` — stop hands back buffered speech, then resets.

The segmenter is **pure by design** (no audio device, no I/O) precisely so the cut logic — the thing most likely to be wrong — is unit-tested. CI also compiles + **cross-compiles to x86_64** the new `cpal` capture path (the main new-dependency risk), so a linking/SDK problem fails in CI, not on the user's Mac.

**Not unit-testable (verified on-device or not at all):** the microphone capture itself (cpal opening CoreAudio), the resident-context streaming loop, and the macOS Microphone **permission** flow — headless CI has no audio device and no TCC prompt. A specific risk to confirm on-device: an unsigned/ad-hoc `.app` (what build-app.yml ships) must still be **granted Microphone access** by the user; if macOS denies it outright for unsigned apps, signing/notarization becomes a prerequisite. The `NSMicrophoneUsageDescription` string (`Info.plist`) is in place.

**Frontend:** typechecks (`tsc` strict) + the screenshot loop gained `2b-live.png` — a mid-session live transcript (browser simulation) visually reviewed: red Stop button, pulsing recording dot + timer, status line, streaming transcript, Copy. The drop-a-file flow is unchanged and still captured in `2-transcribe.png` (now with the Upload/Record toggle).

**On-device confirmation pending** (tag a build → install on the founder's Intel Mac → grant mic permission → speak). This is the same verification loop the rest of the app uses.

### 2026-06-20 — State of testing (transcription, predictor, chat, tool-calling)

**Automated (CI green):** decider **74** tests (incl. `performance.ts`: arch/cores ordering, RAM gate, measured-overrides-estimate). Porter Rust **23** tests (catalog, device tier, porter `tail`/errors, audio resampler+downmix, model registry, llm JSON-decision parsing). CI compiles + **cross-compiles to x86_64** the whole stack (whisper.cpp + llama.cpp + tauri) and checks the model-download URLs resolve.

**What is NOT unit-testable (verified on-device or not at all):** model inference itself (whisper/llama) needs the weights + real hardware; the live Tauri IPC byte-routing (mock harness denies app commands — see the integration-test note below); GUI render + drag-drop. These are verified by shipping a CI-built Intel app to the founder's Mac.

**On-device findings (founder's 2019 Intel Mac, 8 GB):**
- Transcription works; base.en ≈ **3.9× real-time** (59 s audio in 15 s) — became the predictor's real anchor.
- Whisper/Buzz mismatch caught in visual review (hero named the model, not the installed app) — fixed.
- **Chat OOM crash**: the 1.5B model + WebView exceeded 8 GB RAM; ggml `abort()` (SIGABRT) killed the app even at 2048 context. Lesson: an 8 GB Intel Mac is a 0.5B-class machine for in-app chat → **RAM-gated** the model. Deeper lesson (open): ggml aborting the process is a robustness hole — needs runtime process isolation. 0.5B build (app-build-15) **pending on-device confirmation**.

**Build/ship is the verification loop:** no local toolchain → CI cross-builds a downloadable Intel `.app`/`.dmg`; install + drive it on the real Mac. (See `knowledge/21-handoff.md`.)

## MVP desktop porter (`app/`)

### 2026-06-16 — Porter verified in CI (GitHub Actions macOS runner)

**Green on a clean macOS machine** (run `27621986664`, job "Desktop porter (macOS)", 3m25s). The founder's Mac lacked Xcode CLT and disk, so verification moved to CI (`.github/workflows/ci.yml`). The macOS job proved, in order:
- **Frontend typecheck + build** — `npm run build --workspace @verdure/porter-app` (tsc strict + Vite), also producing `app/dist` for `generate_context!`.
- **`cargo test`** — compiled the entire Tauri backend (tauri 2, serde, the app crate) and passed all 9 porter unit tests.
- **`brew install --cask buzz`** — the porter's actual install command, run for real. It succeeded, proving the one-click install recipe end-to-end on a clean machine.

The Linux job ("Decider + website", 47s) was green too: decider 64 tests, website 9 tests + production build.

**Only thing not covered:** the literal GUI launch (`open -a Buzz`) needs a desktop session, so it isn't exercised headlessly. The install — the part a non-technical user can't do — is verified. `[OQ-50]` closed.

### 2026-06-16 — Porter command-surface integration tests (`app/src-tauri/src/lib.rs`)

**Green in CI** (11 Rust tests total). Added `command_tests` to cover the gap that isolated unit tests + the simulated browser preview leave open — the actual Tauri command surface the frontend's `invoke()` targets:
- `app_builds_with_both_commands_registered` — builds the **full app with the real context** (`generate_context!`): `tauri.conf.json`, `capabilities/default.json`, and the icons all parse and load, and both `detect_device`/`port_tool` register in the real `invoke_handler`. This is real integration coverage — it **caught `port_tool` not being runtime-generic** (a `CommandArg`/`AppHandle<R>` bug) and the duplicate-`generate_context!` macOS link error during development.
- `detect_device_command_returns_a_valid_tier` — calls the exact function the `detect_device` IPC dispatches to and asserts a tier the decider recognizes.

**Deliberately not asserted — the literal IPC byte-routing.** An attempt to drive `tauri::test::get_ipc_response` was abandoned after it denied the call with `"detect_device not allowed. Plugin not found"` — both with and without capabilities, and even matching Tauri's own docs example verbatim. The Tauri docs are authoritative that **application commands are allowed by default on all webviews** (no permission needed), and our `capabilities/default.json` is exactly what `create-tauri-app` ships — so this is a quirk of Tauri's *mock* IPC harness, not our code, and the real "Set it up" is authorized. Verifying it further would test Tauri's internals, not Verdure. The true end-to-end (real webview render + IPC + GUI launch) is the on-device run.

**Still genuinely untested:** the assembled real desktop app — the webview rendering in WKWebView and a real click flowing through IPC to a launched Buzz. That needs a desktop session (the founder's Mac once CLT + disk allow); CI can't launch a GUI app. Everything up to and including the **real install** (`brew install --cask buzz`) is verified.

### 2026-06-16 — Porter Rust backend unit tests (`app/src-tauri/src/*`)

- `catalog.rs` (5): `find("buzz")` resolves and `find("unknown")` is `None`; the Buzz install recipe builds `brew install --cask buzz`, the check builds `brew list --cask buzz`, and launch builds `open -a Buzz`. These lock the recipe→command contract as **pure** functions (the brew path is injected), so they need no Homebrew on the test machine.
- `porter.rs` (3): `tail()` keeps short output whole and truncates long output from the front (asserts the 500-char + ellipsis bound); `PortError::BrewMissing` renders human, jargon-free copy (no "Err").
- `device.rs` (1): `detect_tier()` always returns a tier the decider recognizes — never an empty/bogus string — whatever the host machine is.

**Design choice that made this testable:** command construction lives on the `InstallMethod`/`LaunchMethod` enums (pure), separate from execution in `porter.rs` (I/O). So the high-value logic (what we'd run) is covered without sanitizer/FFI concerns — there is no FFI here, only `std::process::Command`.

**Not unit-tested (deliberate):** the actual `brew install` / `open` execution and the Tauri command wiring — that's the end-to-end run, verified by launching the app and porting Buzz on the Mac (pending CLT).

### 2026-06-16 — Porter frontend (`app/src/*`)

**Verified by typecheck + build + the screenshot loop.** `npm run build` (= `tsc` strict, incl. `noUnusedLocals/Parameters`, then `vite build`) passes — this also proves the frontend consumes `@verdure/decider` correctly. The visual self-review loop (`app/scripts/shots.mjs`, Playwright) captured all four states at 720px: home (intent grid), recommendation, setup-working, done.

**Edge case caught (visual review):** the recommendation hero first showed **Whisper** (the decider's model) as "We'll set up …", but the porter installs **Buzz** (the app). That contradicts what lands on the machine and what the progress/done copy said. Fixed: the hero names the tool we install (Buzz) with the engine (Whisper) as context — caught only because the design skill's "actually look at the screenshot" step was followed.

## Shared decider (`packages/decider/`)

### 2026-06-16 — Decider package extraction (`packages/decider/src/*`)

**Suite: 64 tests passing**, run from the package (`vitest`). These are the same matrix/modality/recommend/intent tests previously under `website/lib`, moved verbatim with the code — so the relocation is proven behavior-preserving by the tests passing in their new home. The website's full production build and its remaining 9 sheets tests also pass against the imported package, confirming no consumer broke.

## Website (Phase 0)

### 2026-05-29 — Multimodal recommendation engine (`website/lib/intent.test.ts`, `website/lib/recommend.test.ts`)

**Suite: 40 tests passing** (rewritten from the prior 13 text-only tests + 9 sheets tests).

`classifyIntent` — covers every modality and task: text (creative/code/multilingual/general + fallback), audio (transcription/speech/separation/music + the bare-"recording" → transcription default), image (generate/edit/upscale), video (generate/enhance).

`recommend` — covers the full feasibility matrix:
- Text picks are unchanged from the original matrix and still asserted model-by-model (regression lock during the refactor).
- Audio: transcription/speech recommended on every tier; separation works-but-slow on laptops; music honestly declined on laptops, available on GPU.
- Image: upscaling recommended everywhere; generation/editing declined on plain laptops, recommended on Mac/GPU.
- Video: generation declined on everything except a dedicated GPU (and only "works-but-slow" there); enhancement works-but-slow on laptops.
- **Invariants over the whole matrix:** every recommendation has a non-empty headline/body/label; `not-local-yet` ⇒ no model and no install options; otherwise a model is always present.

**Edge cases caught:** "make my photo sharper and bigger" classified as image-generation until "sharper"/"clearer" were added to the upscale keywords — fixed in the same change. The cross-modal trap "make music" (the word "write"/creative vs. audio) is locked by an explicit test: music wins.

**Not unit-tested (deliberate):** copy wording itself, and the visual rendering of the recommendation page (the design skill's screenshot loop) — pending browser tooling. The engine that *drives* the page is fully covered.

### 2026-05-29 — Waitlist Google Sheets backend (`website/lib/sheets.test.ts`)

**Unit tests written (9).** Covered (pure logic, no network):
- `normalizePrivateKey` — escaped `\n` → real newlines; real newlines untouched. This is the field that most often breaks service-account auth, so it's tested directly.
- `loadSheetsConfig` — returns null if any required var is missing or blank; resolves and normalizes the key when complete; defaults the range to `Sheet1!A:F` and honors an override.
- `isSheetsConfigured` — true only on a complete config.
- `buildWaitlistRow` — emits cells in the documented column order, one per declared column (locks the Sheet's column contract).

**Edge cases caught:** whitespace-only env values are treated as missing (a blank var in a deploy dashboard shouldn't read as "configured"). Column count is asserted against `WAITLIST_COLUMNS` so adding a column without updating the row builder fails a test rather than silently misaligning the Sheet.

**Not unit-tested (deliberate):** the live `spreadsheets.values.append` call — it requires real Google credentials and is integration territory. Manual verification steps are in `website/SETUP_SHEETS.md` §5.

## Orchestration binary (Phase 1)

_No tests logged yet — Rust scaffold only (`tests/integration_tests.rs` placeholder)._
