# FEATURE_LOG.md

Living record of features built across the Verdure monorepo — the **Verdure desktop app** (`app/`), the shared decider package (`packages/decider/`), the Phase 0 website (`website/`), and the long-term Rust core (`src/`, `tests/`). Newest first. The knowledge base (`knowledge/`) is the shared brain for all of them.

## Verdure desktop app (`app/`) — unified interface (post-pivot)

> Pivoted 2026-06-18 (`knowledge/18`) from "install separate apps" to **one interface that runs models in-app**, then to **Verdure as the personal AI** (`knowledge/20`). The "porter" naming in code/below is legacy; see `knowledge/21-handoff.md` for current state + next steps.

### 2026-08-12 — "Midnight glass" visual identity (styles.css + design.md)

**What:** full retheme from the prairie-daylight palette to the brand video's world — deep-space navy gradient background, frosted-glass panels (`backdrop-filter` blur + mint-tinged borders + glow shadows), mint CTAs as glowing pills with dark ink (never white-on-mint), the VERDURE wordmark as wide-tracked serif with a mint gradient and a one-shot glint sweep, drifting glass orbs in the background, and a staggered fade-up on entry. Palette measured from the video with ffmpeg palettegen (`#021329` navy family, `#34c69a`/`#4cd8ad`/`#b7f3e6` mint family, `#479e96` sea-teal), not eyeballed. Warn/danger moved to warm coral (`#ff9d8a`) — the brick-red vanished on navy. Fonts unchanged and now documented as a decision: `ui-rounded` body + `ui-serif` display is the exact pairing the video uses, and system faces keep the app free of font downloads.

**Why:** founder direction with a brand video — "sleek and simple, Apple but warm." One stylesheet themes all three platforms (macOS/iOS/Android share the webview UI). The canonical identity now lives in `skills/design.md` ("Visual identity — midnight glass"): palette, glass rules, type, motion rules, contrast floor.

**User impact:** after this change, the app feels like one deliberate product with the brand's own atmosphere — and every existing affordance (fit warnings, progress bars, recording states) stays exactly where it was, retinted.

**Motion & floor-machine care:** all animation is entry-only, transform/opacity (GPU-cheap on the 2019 Intel Mac), and fully disabled under `prefers-reduced-motion`. Recording/preparing mic states keep their distinct look (coral pulse vs neutral breathing).

**Verification:** tsc + vite clean. Visual self-review loop actually run this time (Chrome + vite preview): model picker, chat empty state, and a chat exchange reviewed on-screen — wordmark, glass, CTA glow, bubbles all render as intended. On-device confirmation on the next tag builds (blur performance on the Intel Mac is the one thing preview can't prove).

### 2026-08-12 — Help users make room for models ([OQ-67], steps 1–4)

**What:** four space-recovery features, own-sandbox first — Verdure never deletes a byte it didn't create:
1. **Unfinished downloads surfaced** (`model.rs` `partial_bytes`, Storage screen): an abandoned `.part` (up to ~1.4 GB) now shows as "Unfinished download · N MB so far — resumes if kept" with Remove, instead of invisibly eating the disk. Totals include it.
2. **Old-model offer on switch** (`renderFreeOldModel`): switching chat models offers one optional tap to remove the now-redundant old model ("free 1.1 GB"), honest that switching back means re-downloading. Never automatic.
3. **Android "make room"** (`free_up_space` command, JNI): when a model doesn't fit the disk, one tap asks Android's `StorageManager.allocateBytes` to clear *other apps' clearable cached files* — the OS decides what's junk; we never see or touch anyone's data. `getAllocatableBytes` is checked first so the failure message is honest ("can clear about X MB — needs Y MB").
4. **macOS/iOS honest pointers**: macOS gets "See what's filling your disk" (opens the System Settings Storage pane, falls back to System Settings if the community-documented pane id is refused); iOS gets instructions in the warning copy (Settings → General → iPhone Storage), since no public API or deep link exists.

**Why:** real user reports — not enough space to install any model. The app detected and declined honestly but left the user alone with the problem. Platform survey and the no-cleaner decision (duplicate-finders rejected) are in `[OQ-67]`.

**Problem it solves:** the ~550 MB (minimum) to ~1.5 GB (Powerful) a user needs free, recovered from space Verdure itself created first, then from OS-sanctioned mechanisms only.

**Verification:** new Rust unit test (partial-download surfacing + reclaim); tsc + vite clean; macOS Rust compiles in CI on push. The Android JNI path compiles only in the Android workflow (tag build) and needs on-device confirmation on the Pixel 8a — tracked in `[OQ-67]`. macOS deep-link pane id is provisional with a safe fallback.

### 2026-08-10 — Friendly error fallback at the display boundary (`main.ts`)

**What:** the three places that showed a caught error verbatim (`String(err)` — mic setup, chat send, legacy porter button) now route through one `friendlyError` helper: backend messages (already human-written sentences from `llm.rs`/`model.rs`) pass through, but a thrown object or empty string shows "Something went wrong. Please try again." instead of `[object Object]` or a blank.

**Why:** the truck-driver test — a raw JS error representation is jargon by definition. This was an inspection finding (2026-06-29, deferred to the redesign; re-flagged 2026-08-10) — the backend half was already honest, the frontend just leaked whatever it caught.

**Verification:** tsc + vite clean. The chat-send path's resume-detection regex is unaffected (string errors pass through unchanged). Frontend-only; no Rust touched.

### 2026-08-03 — Android Play distribution: signed-AAB workflow (open-testing path)

**What:** the Android app now has a Play-ready build path. New `build-android-play.yml` (tag `android-play-N`) produces a **signed Android App Bundle (`.aab`)** using a **persistent upload keystore** held as GitHub secrets (`ANDROID_KEYSTORE_BASE64` + password/alias secrets — founder-owned, same trust model as the iOS App Store Connect keys). Gradle emits an unsigned `.aab`; we sign it with `jarsigner` (the JAR-signing scheme AABs use) so Google **Play App Signing** can take over (upload key → Play re-signs delivered APKs). This is the Android analog of `build-testflight.yml`; the throwaway test-APK workflow stays for quick sideload smoke tests.

**Why now:** the APK is proven on-device (mic works), which was `[OQ-66]`'s gate for moving to a shareable link. Target is Play **open testing** — a public "click → install → auto-update" link, no tester-email collection. Verdure's local/no-telemetry design makes the Data Safety form the best case ("no data collected/shared") and `RECORD_AUDIO` trivially justified (on-device dictation).

**Refactor (no drift):** the manifest + `MainActivity` patch (RECORD_AUDIO + the `initAudioContext`/`ndk_context` mic fix) moved out of the inline workflow heredoc into one shared script — `app/scripts/patch-android-project.py` — that **both** Android workflows run, so the test-APK and Play-AAB paths can never diverge on the mic wiring.

**Founder-side (account, not code):** generate the upload keystore + add the four secrets; then in Play Console create the app under the permanent package name (`ai.verdure.porter` unless changed first — permanent once published), fill the store listing / Data Safety / content rating / privacy-policy URL, upload the first `.aab` by hand to establish Play App Signing, and open the **open-testing** track for the public link. Automated Play upload (a service-account step) folds in after the app exists.

**Verification:** both workflow YAMLs parse; the shared patch script syntax-checks. The `--aab` cross-build + `jarsigner` step run once the keystore secrets are set and an `android-play-N` tag is pushed (CI is the compiler here). `[OQ-66]`.

### 2026-08-02 — Android mic fix: initialize the `ndk_context` oboe reads (root cause found on-device)

**Problem (on-device Android, `[OQ-66]`):** the live mic never worked — tap the mic, it hangs on "preparing" and the 30s watchdog fires; no text, no error. Debugged on-device with `adb logcat`. Ruled out permission (force-granting `RECORD_AUDIO` via `adb` didn't help) and the model (Whisper `base.en` loads cleanly in the log). The smoking gun:

```
thread 'verdure-live' panicked at ndk-context-0.1.1/src/lib.rs:72:30:
android context was not initialized
```

**Root cause:** cpal's Android backend (oboe) enumerates the microphone over JNI and reads the app's `JavaVM` + `Context` from the process-global `ndk_context`. **Tauri v2 never populates that global.** So the first mic-open on the capture thread panics, the thread dies, and — because a panic emits no `live:error` — the UI just hangs until the watchdog. macOS/iOS never hit this (CoreAudio, no JNI); Android is the only platform on this path.

**Fix:** initialize `ndk_context` ourselves at launch. `MainActivity.onCreate` now calls a native `initAudioContext(applicationContext)` (`lib.rs` `Java_ai_verdure_porter_MainActivity_initAudioContext`, `#[cfg(android)]`), which stores the `JavaVM` + a global ref to the Application context into `ndk_context` (once, leaked deliberately — the Application context is process-lifetime). New Android-only deps (`jni`, `ndk-context`), decoupled from cpal via raw pointers; the macOS dependency tree is untouched (target-gated). The CI `MainActivity` patch gained the `external fun` declaration + the call, plus a package assertion so a future Tauri package rename fails the build loudly instead of an `UnsatisfiedLinkError` at runtime.

**Verification:** the porter compiles on macOS CI (Android code is cfg-gated out there); the Android cross-compile + APK build is exercised by the `android-build-7` tag. **Confirmed working on-device (Pixel 8a, 2026-08-03):** the `ndk-context` panic is gone, `AudioRecord` opens (oboe negotiates a channel config — the `getMinBufferSize: Invalid channel configuration` probe lines are benign), the mic goes to "Listening…", and live transcription streams. Live dictation now works on all three platforms (macOS/iOS/Android). Closes the core mic unknown of `[OQ-66]`.

### 2026-08-01 — Download UX: live progress bar + friendly one-tap resume; mic button states

On-device Android polish:
- **Progress bar.** The resumable download now emits `PROGRESS <done> <total>` MB lines (`model.rs` `download` computes total from Content-Range/Content-Length, falls back to the model's approx size); the chat install card renders a filling bar + "377 of 400 MB · 94%" instead of a static "installing…" line.
- **Friendly pause + resume.** An interrupted download no longer shows a raw "os error 103" — it shows "Download paused — your progress is saved" with a **Resume download →** button that re-attempts (which resumes from the `.part` on disk). Refactored `submit` → `runChat(text)` so resume re-runs without re-posting the question.
- **Mic button states** (from the same review): idle → **preparing** (neutral dim) → **recording** (red) — the mic only goes red once capture is really live, and a 30s watchdog replaces an infinite "Starting the microphone…" hang with an actionable "allow microphone in Settings" message. The Android build also re-requests mic permission on `onResume`.

**Verification:** frontend `tsc` + `vite` clean; the `download`/`ensure` signature change compiles in CI. On-device confirmation in the next Android build. `[OQ-66]`.

### 2026-07-31 — Resumable model downloads + keep-screen-awake (mobile reliability)

**Problem (on-device Android):** big-model downloads (1.4 GB) never finished — when the screen sleeps or the app backgrounds, Android suspends the process and tears down the socket (`ECONNABORTED` / "Software caused connection abort, os error 103"). Worse, the old flow *deleted* the partial file on any interruption and restarted from zero, so a big model over mobile never caught up.

**Fix — resumable downloads (the robust foundation, all platforms):** `model.rs` `download()` now resumes from the existing `.part` file — macOS via `curl -C -`, iOS/Android via an HTTP `Range: bytes=N-` request (served by Hugging Face's CDN; 206 → append, 200 → restart). `ensure()` keeps the `.part` across attempts (no longer deletes on interruption) and reports "Resuming — N of ~M MB…". So every interruption just costs a pause: reopen Verdure and it continues from where it stopped, instead of restarting.

**Fix — keep screen awake during download:** a best-effort Screen Wake Lock (`withScreenAwake`, no-op where unsupported) is held while a model downloads, so an idle screen doesn't suspend the app mid-download. Install copy updated ("Keep Verdure open; this can take a few minutes").

**Deferred (the gold standard):** a true background download (Android foreground service / DownloadManager) so it continues with the app fully closed — real Kotlin/plugin work, noted for later. Resume + wake-lock covers the common case now.

**Verification:** frontend `tsc` + `vite` clean; the `download`/`ensure` changes compile in CI. On-device confirmation (big model finishing across an interruption) in the next Android build. `[OQ-66]`.

### 2026-07-31 — Android test build green (APK via CI) [OQ-66]

**What:** Verdure builds for Android — the same one-interface app, same shared Rust core and GGUF flow, cross-compiled for `aarch64-linux-android` via the NDK. Same-branch `cfg(target_os = "android")` gating; macOS/iOS paths untouched (Mac CI stayed green). New `build-android.yml` (tag `android-build-N`, **Linux runners — 1/10 the macOS cost**): `tauri android init`, manifest `RECORD_AUDIO` + runtime permission request patched into MainActivity, NDK build, zipalign + apksigner with a throwaway key → 9 MB installable `.apk` artifact.

**Key finding: whisper.cpp + llama.cpp + cpal cross-compile for Android** — proven in CI (the decisive `[OQ-66]` unknown; **NOT MediaPipe** — the shared Rust runtime ports directly). Confirms the same "compile the engines in, wrap them" approach works on a third platform.

**Backend (cfg-gated `device.rs`):** total RAM via `libc::sysinfo`, available via `/proc/meminfo MemAvailable` (pure parser + unit test), disk via `statvfs`, ARM tier default. Frontend: Storage screen names ARM correctly (was hardcoded Intel/Apple), plus a phone-width CSS pass (~360–412px) since Android phones are narrower than the iPad.

**Two rounds (iOS-style iteration):** round 1 — native compile fully succeeded, Kotlin failed because the MainActivity permission patch rewrote the class (the template already has an `onCreate`); round 2 — inject at the stable `super.onCreate` anchor → green.

**Install:** sideload the APK (enable "install unknown apps"). Test-signed with a per-run throwaway key, so uninstall an old build before installing a new one. Play-Console testing link (shareable URL) comes next with a persistent upload keystore.

### 2026-07-27 — On-device fixes: plain chat (no JSON tool-calling), picker delete, disk fail-states

**First real on-device run (TestFlight build 6) — big signal:** the chat model **loads and generates on the iPad** (wrote a poem, answered "Hey"). `[OQ-64]`'s core question — does the model run on-device — is **answered YES**. Two bugs surfaced, both from the JSON tool-calling wrapper:

**Bug 1 — chat replied with a transcription prompt to normal questions** (a poem request → "I can set you up to turn recordings into text"). The tiny model over-triggered the `set_up_transcription` tool-action. **Bug 2 — raw JSON leaked into the chat** (the poem came out as `{"action":"answer","reply":…}`) when the model emitted malformed/nested JSON that the lenient parser dumped verbatim.

**Fix — chat is now a plain conversation.** Removed the JSON tool-calling entirely: `llm.rs` system prompt is plain ("answer directly, clearly, concisely"), `decide()` returns the generated text as the reply (no JSON parse, no `parse_decision`/`extract_json_object`), and the UI always shows the reply (removed the `set_up_transcription` action branch + `classifyIntent` cross-check). Transcription is the mic button; capabilities-as-tools returns later, constrained + process-isolated (`[OQ-55]`). Also filter Whisper's non-speech markers (`[BLANK_AUDIO]`, `[ Silence ]`) out of dictated text.

**Model management (founder ask):** added a **Remove** button to each downloaded model **in the picker** (delete one to free room for another, in one screen), a **disk fail-state warning** ("Not enough free space to download (needs X). Remove a model to make room"), and a **free-space readout** at the top of the picker (space + memory free) — the Storage-screen transparency, where you choose.

**Verification:** `tsc` + `vite build` clean; `llm.rs` prompt-framing tests kept, obsolete parser tests removed; Rust compiled in CI. Shipped in TestFlight build 7.

### 2026-07-25 — Dictation-into-chat, model picker, install notice

Three chat improvements:

**1. Mic dictates into the chat box** (not a separate screen). The chat mic is now a record toggle: tap → live transcription streams words into the input as you pause, tap again to stop, then edit + Send. Reuses the `live.rs` engine (`startLiveTranscription`/`onSegment`); the mic pulses red while listening. The standalone transcription screen stays for file transcription (still reached from the "set it up" chat action), just not behind the mic. Closes the core of the AI-first dictation vision (`[OQ-57]`).

**2. First-launch model picker** (Small · Balanced · Powerful). New `Choose your model` screen shows the three chat models with plain tier names, one-line blurbs, **download size, and whether each fits this device's free memory** (design.md — honest space, model name demoted). Choice persists (`localStorage`), is shown as a chip in the chat header, and is changeable anytime. Backend: `chat_model_options` command (size + `fitsMemory`/`fitsDisk` per device) and `chat_model_choice(id, available)` — loads the picked model if it fits, else declines honestly; unknown/empty id falls back to the auto-picker. Replaces the old "best model only" toggle (the explicit pick supersedes it). The lineup is all-lightweight (0.4 / 1.1 / 1.4 GB) per the founder's call.

**3. "Installing your model" notice.** On the first message with an un-downloaded model, a prominent one-time notice names the tier + size ("Setting up your Powerful model — a one-time 1.4 GB download…") instead of a silent wait.

**User-impact statement:** *the user talks or types to Verdure, sees clearly which model they're using and what it costs in space, and is told plainly when a one-time download is happening.*

**Verification:** `tsc` + `vite build` clean; new `model.rs` unit tests for the explicit-choice + options logic. Rust compiled in CI (below); on-device behavior (real dictation, real download) confirmed in the next TestFlight build.

**Follow-up (same day, on founder review):** deleted the standalone transcription screen entirely (`renderTranscribe` + file-drop/tabs/speed-estimate helpers, ~280 lines) — dictation lives in chat now, so the page was vestigial. The mic now **asks permission before installing the speech model**: first tap (when Whisper isn't downloaded) shows a consent message with the size ("~140 MB, runs on your device") and an "Install & start talking" button, matching the chat-model install prompt; confirm → download → record. The chat "set up transcription" action routes into that same mic flow instead of the deleted page. Unused transcription CSS left for a later sweep.

### 2026-07-25 — TestFlight distribution working (signed iOS build via CI) [OQ-65]

**What:** `build-testflight.yml` (tag `testflight-N`) now builds a **code-signed** iOS app with the founder's paid Apple Developer account and uploads it to **TestFlight** — replacing the unsigned-sideload path (no more 7-day cert, no Sideloadly). Reuses the proven iOS build steps, then: patch build number (run number) + `ITSAppUsesNonExemptEncryption=false`, stage the App Store Connect API key, archive with Xcode-managed automatic signing, export a signed App-Store `.ipa`, upload via `xcrun altool`.

**Problem it solves:** the founder's 8 GB Mac can't run the models; the iPad can, but sideloaded builds expired weekly and needed a Mac to re-sign. TestFlight installs through the TestFlight app, no expiry, and can add testers — the sustainable test channel.

**The four-round debug (all account-side, no code bugs):** (1) export failed "Cloud signing permission error" — the **App Manager** API key can't do cloud *distribution* signing; fixed by regenerating the key as **Admin**. (2) Two agreements (Paid Apps + EU trader) were unaccepted, which also blocks distribution signing. (3) Upload failed "Cannot determine the Apple ID from Bundle ID `ai.verdure.porter`" — the build signed as `ai.verdure.porter` (Xcode-managed XC App ID) but the "Verdure AI" App Store record is bound to the explicit App ID `ai.verdure.porter.898PY37K5S`; fixed by overriding the iOS bundle ID at archive time to match (macOS build + `tauri.conf.json` untouched). (4) Green end-to-end.

**Boundary honored:** all signing credentials are the founder's, created by them and added as GitHub secrets (`ASC_API_KEY_P8` / `ASC_API_KEY_ID` / `ASC_API_ISSUER_ID` / `APPLE_TEAM_ID`); Claude never handled a private key.

**Follow-up:** the dual-App-ID mess (`ai.verdure.porter` vs `…898PY37K5S`) is cosmetic debt to fold into the identifier rename (`[OQ-60]`). `build-ipad.yml` (unsigned) kept as a fallback.

### 2026-07-17 — Chat-first redesign + "prairie" aesthetic

**What:** Reworked the app's front door and visual language. (1) **Opens straight into chat** — the 4/6-tile task grid is no longer the entry (`boot()` → `renderChat()`); chat is the home. (2) A **microphone button** in the chat input row opens dictation (the Whisper flow) in one tap. (3) A quiet **gear icon** in the header reaches Storage; back-nav from Storage/Transcribe now returns to chat. (4) New **"prairie on a summer day" palette** — a layered gradient background (summer sky → sun-glow → meadow-green → wheat-cream, fixed), warm meadow green replacing cold emerald, earth-olive ink, a sun-dot brand mark, soft gradient bubbles/buttons, frosted warm-paper panels. Retinted through the CSS design tokens so the change cascades everywhere at low risk, with targeted gradient overrides on the hero surfaces (background, bubbles, buttons, mic). Icons are inline SVG (CSP-safe, no remote assets).

**User-impact statement:** *After this change, the user lands where the value is — a calm, warm chat they can talk or type to — instead of a cold grid of choices; dictation is one tap away.*

**Problem it solves:** the old front door made the user choose a "task" before doing anything, and the visual tone was hard-edged/clinical. The redesign removes a whole decision step (design.md — "remove a step") and softens the feel to something inviting.

**Legacy:** the task grid + recommendation/porter flow (`renderHome` and friends) is retained but **unwired** — the only path to un-wrapped modalities (image/video) — pending the legacy-porter keep-or-remove decision (`[OQ-60]`). Flagged in-code.

**Verification:** `tsc --noEmit` clean; `vite build` clean. **Visual self-review (design.md) NOT automated** — the browser extension wasn't connected and no headless browser is installed; reviewed via local `vite preview` and to be confirmed on-device in the next iPad build. No new unit tests (pure presentation; no logic changed — the model picker, chat command, and parsers are untouched).

### 2026-07-04 — iPad test build: the app runs on iOS (unsigned IPA via CI) [OQ-65]

**What:** A temporary iPad port of the whole app as a **test vehicle** — the founder's 8 GB Mac can't exercise the ByteShape 4B (`[OQ-64]`); his iPad can. Deliberate, flagged deviation from `knowledge/11`'s thin-decider iOS strategy (the shippable-iOS decision stays with `11`). Backend made iOS-compatible behind `cfg` gates with the **macOS paths byte-identical**: `model.rs` downloads via in-process `ureq`/rustls on iOS (no process spawning there; macOS keeps its on-device-verified `curl`); `device.rs` reads syscalls instead of shell-outs — `sysctlbyname` (RAM), `statvfs` (disk), and `os_proc_available_memory()` — the app's **real headroom before iOS jetsam-kills it**, feeding the existing chat RAM gate so the picker sizes against iOS's true constraint. New `build-ipad.yml` (tag `ipad-build-N`): `tauri ios init` fresh each run, mic-permission plist injected, Rust lib pre-built with cargo (bypassing the generated xcode-script phase, which requires a parent tauri-CLI process CI can't provide), project rewritten to link the prebuilt lib, xcodebuild unsigned → `verdure-ipad-unsigned.ipa` artifact.

**User-impact statement:** *After this change, the founder can test chat and live transcription on the one device he has that can actually run the models — because CI now produces an installable iPad build without needing a local toolchain or an Apple Developer account.*

**Problem it solves:** every on-device unknown (`[OQ-64]`: does the 4B load, is it usable) was blocked on a machine that can no longer run the app. This unblocks testing entirely.

**Key finding:** **whisper.cpp + llama.cpp + the full backend cross-compile for `aarch64-apple-ios`** — proven in CI (the decisive unknown of the port, answered in round 2 of 3).

**Install:** sideload with a free Apple ID (Sideloadly/AltStore; 7-day cert) — no $99 account. See `knowledge/21` "How to build / ship".

**Not verified (on-device, tracked in [OQ-65]):** jetsam ceiling vs the ~2.3 GB 4B footprint; cpal/AVAudioSession mic capture; TCC mic grant for a sideloaded app; WebView UI fit on iPadOS.

### 2026-07-01 — "Best model only" chat toggle (kills silent fallback for testing + honesty)

**What:** A chat setting — *"Use the best chat model only — tell me if it won't fit, don't switch to a smaller one"* (persisted in `localStorage`, off by default). When on, chat uses the preferred model (the ByteShape 4B) or **declines with an honest message naming the model and the memory shortfall**, instead of silently downgrading to Qwen 0.5B. Backend: a pure `model::choose_chat_model(available_mb, allow_fallback)` returning either the chosen model or a `ChatModelTooBig` (carries model name + needed vs available MB + a `strict` flag); the `chat` command gained an `allow_fallback` arg and formats the message in human GB. Factored the candidate ordering into one `chat_candidates()` list shared by the RAM picker and the preferred/strict pick, so they can't drift.

**User-impact statement:** *After this change, the user can tell exactly which chat model is running — because best-only mode either uses the 4B or says why it can't, instead of quietly substituting a weaker one.*

**Problem it solves:** silent fallback made the 4B impossible to test — a reply might be the 4B or the 0.5B and you couldn't tell. Now best-only mode is a deterministic lever: a reply means the 4B ran; the decline message means it didn't fit (and says how much memory it needs). It's also better UX per "honest performance / feedback" — no silent downgrade behind the user's back.

**Tests:** three new `model.rs` unit tests — preferred = top candidate; best-only uses-preferred-or-errors-strict (no downgrade even when a smaller model would fit); fallback downgrades and only errors (non-strict) when nothing fits. Frontend `tsc` + `vite build` clean.

**Not done:** visual self-review of the toggle (no browser tooling / Tauri toolchain locally) — verify on-device in the tagged build. A general "which model is active" indicator (for fallback mode too) is a natural follow-up, deferred to the redesign's chat UX pass.

### 2026-07-01 — ByteShape Qwen3 4B as the base chat model (`model.rs`)

**What:** Wired [ByteShape](https://byteshape.com/)'s ShapeLearn-quantized **Qwen3-4B-Instruct-2507 `Q3_K_S-2.77bpw`** (~1.4 GB, Apache 2.0) as the top-of-pick chat model in the RAM-aware picker, with the Qwen 0.5B/1.5B pair kept as the always-present floor/fallback (`USE_BYTESHAPE_QWEN3_4B = true`). Refactored `chat_model_for_available` from nested conditionals to a flag-built candidate list so adding a gated model no longer amplifies edits. Added its URL to the CI resolve check. Qwen3 uses ChatML, so the existing `PromptStyle::ChatMl` framing applies unchanged — no new prompt plumbing.

**Problem it solves:** the 8 GB Intel Mac forced chat onto Qwen 0.5B, which produced garbage; a *real* 4B is far more coherent, and ByteShape's low-bit quant squeezes it to ~1.4 GB so it can plausibly fit the machine's available RAM. The direct lever on "chat doesn't work."

**Why it's the safe route:** the GGUF's base type is a *standard* llama.cpp K-quant (ByteShape's "KQ" = per-tensor mixing on stock GGML kernels — no engine fork), and Qwen3 (2025) is within our pinned `llama-cpp-2 0.1.150`'s arch support (it already runs Apr-2026 Gemma 4). Enabling it can't strand a tight machine: if the 4B doesn't fit available RAM it's simply not picked and Qwen answers instead. One-line revert (`USE_BYTESHAPE_QWEN3_4B = false`) if an on-device run shows it fails to load.

**Not yet confirmed (on-device):** (a) that it *loads* on our pinned llama.cpp — low risk, but a load failure OOM-aborts the app until process isolation lands (`[OQ-58]`); (b) whether it *fits* the founder's 8 GB Mac at all — with ~1.5 GB free the gate falls back to Qwen, so the 4B needs RAM freed to be exercised. To help, `N_CTX` was lowered 2048 → 1024 (`llm.rs`) and `runtime_mb` set to match (~2300, provisional, to calibrate on-device like the predictor `19`) — but the 1.4 GB of weights is a hard floor, so no context cut makes it fit under ~2.3 GB free. Tracked as `[OQ-64]`.

**Verification:** can't compile on this Mac (build gated on Xcode CLT); pure-logic covered by new `model.rs` unit tests (base-when-it-fits + fallback-when-it-doesn't + ChatML style); CI compiles the backend, runs the tests, and resolves the new URL on push.

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
