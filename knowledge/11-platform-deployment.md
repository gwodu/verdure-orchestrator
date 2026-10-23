# 11 — Platform Deployment

**Status:** 🟡 Provisional — iOS (the largest unknown) resolved at WWDC 2026; remaining gaps are research-grade (signing/store specifics)

This file documents the practical realities of shipping Verdure on each platform. Phase 0 (the website) sidesteps these. Phase 1 and beyond hit them directly.

> **Decision content behind the `Transport` / shell layer** (`14-architecture.md`) — signing, packaging, and per-OS IPC. Structure (the `Transport` trait, the shell's job) lives there; the per-platform packaging realities live here.

## The build and release pipelines at a glance

*Added 2026-09-16 — the three platforms build differently enough that the differences were only discoverable by reading five workflow files. This is the map.*

**One branch: `main` (since 2026-10-20).** The long-running `mvp-porter-buzz` branch was merged and retired; triggers come from the YAML **of the ref being pushed**, which is how `main` kept a stale push-triggered CI for months while the branch had already dropped it. **Every push to `main` also deploys the website to production** (Vercel's production branch), so there is no preview gate.

**Nothing runs on `git push`.** CI triggers on **pull requests and manual dispatch only**; release workflows trigger on **tags**. A plain push to a branch runs nothing at all, which is why `CLAUDE.md` insists on verifying locally before pushing — there is no safety net behind you.

| | macOS | iOS | Android (test) | Android (Play) |
|---|---|---|---|---|
| Workflow | `build-app.yml` | `build-testflight.yml` | `build-android.yml` | `build-android-play.yml` |
| Tags | `app-build-*`, `release-*` | `testflight-*`, `release-*` | `android-build-*`, `release-*` | `android-play-*` |
| Runner | `macos-latest` (**10× cost**) | `macos-latest` (**10× cost**) | `ubuntu-latest` | `ubuntu-latest` |
| Timeout | 60 min | 60 min | 45 min | 45 min |
| Output | universal `.dmg` | `.ipa` | `.apk` (arm64) | `.aab` (arm64) |
| Signed with | Developer ID cert | Apple distribution cert | **throwaway CI key** | **upload keystore** (founder-held) |
| Reaches users by | notarized `.dmg` on GitHub Releases + self-updater | **auto-uploaded to TestFlight** | artifact → sideload | **auto-published to Play closed track** |
| Identity | `ai.verdure.porter` | `ai.verdure.porter.898PY37K5S` | `ai.verdure.app` | `ai.verdure.app` |

**`release-*` fans out to three platforms** — macOS, TestFlight and the Android test APK — which is the "one change → three platforms" property (`KNOWLEDGE.md`, 2026-08-12). The per-platform tags exist for iterating on one platform without paying for the other two.

**Play is deliberately NOT on `release-*`.** A Play release is a distinct, consequential act — it reaches strangers, and a bad one is visible publicly — so it needs its own tag rather than riding along with a routine desktop build.

**Why three different identities.** Tauri derives the app identifier from one config field, but each store wants something different, so two of the three are overridden at build time rather than by changing that field (which would move `app_data_dir()` and orphan installed models — see `[OQ-60]`):
- **macOS** uses `tauri.conf.json`'s `identifier` as-is.
- **iOS** overrides `PRODUCT_BUNDLE_IDENTIFIER` at archive time to match an App Store Connect record created by a tool.
- **Android** patches the Gradle `applicationId` after project generation, leaving the Java `namespace` alone so the JNI symbol keeps resolving.

**Website: the Linux lockfile rule (2026-10-20, `[OQ-81]`).** Vercel builds on Linux; the lockfile is written on a Mac. npm records a native package's other-platform builds (`@tailwindcss/oxide-linux-x64-gnu`, `@next/swc-linux-x64-gnu`, sharp's `@img/*`) as optional entries, and **drops them** when it writes a lockfile while `node_modules` exists. After that, `npm ci` on Linux installs no binding and the build dies on the first CSS file — twelve days of failed deploys before the log was read. `website/lib/lockfile.test.ts` now fails on the Mac if those entries are missing. If it does: do not `npm install` your way out (that is how they vanish); regenerate the lockfile in a scratch tree holding only the `package.json`s (`npm install --package-lock-only`), or re-add the entries at the parents' exact pins from `npm view <name>@<version>`. Reading a failed build: `npx vercel login`, then `npx vercel --scope gwodus-projects list verdure-orchestrator` and `inspect <url> --logs`.

**What CI actually checks** (`ci.yml`, PRs only): a `paths-filter` job decides what changed, then the decider/website suites on Linux, the porter Rust suite plus an `aarch64-apple-ios` `cargo check` on macOS, and an `aarch64-linux-android` `cargo check` on Linux. Model-download URLs are verified by extracting them from `model.rs` (not a hand-kept copy).

**What each trigger costs** — *this table is the one owner; `skills/testing.md` and `21` link here.* Billable = wall minutes × the runner multiplier (macOS 10×, Linux 1×).

| What you need | How | Billable min | Measured |
|---|---|---|---|
| Rust / logic correctness | **local** `cd app/src-tauri && cargo test` | **0** | seconds |
| Test macOS yourself | **local** `cd app && npm run tauri build` | **0** | ~8 min cold |
| Pre-merge checks (incl. the iOS cross-compile) | open a PR, or `workflow_dispatch` on `ci.yml` | ~40 | — |
| Test on iPad | tag `testflight-N` | ~55 | `testflight-12`: 5 min wall, cold cache |
| Test on Android | tag `android-build-N` | ~8 cached, ~19 cold | `release-14` |
| Signed + notarized macOS for someone else | tag `app-build-N` | ~145 | `app-build-24/25`: ~14–15 min wall (universal: both arches) |
| Genuinely ship everywhere | tag `release-N` | ≈ the three above, ~210 | `release-14` was 144, before macOS went universal |

**Caches warm only on success.** `actions/cache` saves at the end of a *green* job, so a run of failing iterations stays cold every time (`testflight-9`…`-11`, 2026-10-08). Before claiming a cache hit, grep the log for `Cache restored`. Put cheap, failure-prone steps (secrets, signing, API calls) before the Rust build so they fail in a minute.

**Cost shape.** macOS runners bill at **10× per minute**, which is why Android moved to Linux and why the macOS CI job is off the per-push path entirely. A three-file docs commit once triggered 7.6 minutes of Android compilation before `paths-filter` was fixed to diff the push rather than the whole branch.

## macOS

- Notarization required for distribution outside the App Store
- Hardened runtime entitlements required for notarization
- Code signing certificate via Apple Developer Program ($99/year as of last check)
- Gatekeeper bypass needed for first-run on unsigned binaries, or properly notarized binary distribution

**[GAP — needs research]** Current Apple Developer Program pricing in 2026, any recent notarization policy changes. Tracked as `[OQ-23]`.

## Windows

- Code signing certificate (~$200–500/year from a CA) — exact pricing fluctuates
- Microsoft SmartScreen builds reputation slowly; early users may see warnings on a new certificate
- Distribution: MSI installer or portable EXE
- Windows Defender may flag unknown binaries until reputation accrues

**[GAP — needs research]** Current Windows code signing certificate pricing and CA recommendations in 2026. SmartScreen reputation-building timelines. Tracked as `[OQ-24]`.

## Linux

- Strategy for v1: AppImage (universal) plus optional distro packages later
- No signing infrastructure required for v1
- Possible future: Flatpak for sandboxed deployment

## Android

- Foreground service for the orchestration daemon
- Storage permissions for the shared model directory
- Background execution policies have tightened across recent Android versions
- Google Play review: system-level access requests may trigger additional scrutiny

**[GAP — needs research]** Current Android 14+ background service policies and Play Store review requirements for system-level access in 2026. Tracked as `[OQ-25]`.

**Update (2026-07-28): the Android build in flight is the app, not the daemon.** The bullets above describe the pre-pivot daemon idea — superseded. What's actually being built is an **Android port of the Tauri test-vehicle app** (same one interface as macOS/iOS, `[OQ-65]`'s sibling): llama.cpp + whisper.cpp cross-compiled for `aarch64-linux-android` via the NDK — **not MediaPipe** (a second runtime + different model formats would abandon the shared Rust core for no gain). Same-branch `cfg(target_os = "android")` gating, `build-android.yml` on Linux runners, distribution via Play Console testing tracks (founder has the developer account). Tracked as `[OQ-66]`; the daemon/system-service questions (`[OQ-25]`) stay open and separate.

### Mobile non-negotiables (founder, 2026-10-23)

Three rules for every build that runs on a phone. They were each broken on the Pixel on 2026-10-23 and are written here, in the file every mobile build change routes through, so they are never broken silently again. `CLAUDE.md` points here; `skills/design.md`'s self-check asks about them.

1. **Work the user started finishes when they switch apps.** A reply being written, a model downloading, a dictation running — none of it dies because the user glanced at WhatsApp. On **Android** the Rust core holds a foreground service (`VerdureWork.kt`, written by the patch script; `background.rs` starts it with the kind of work and stops it when the last guard drops) for exactly as long as the work runs, with the `dataSync` type for a reply or download and `microphone` for dictation, so the process is neither frozen by the cached-app freezer nor killed. One quiet notification names the work; it is hidden on Android 13+ until the user allows notifications, and the service runs either way. On **iOS** the same guard calls `UIApplication.beginBackgroundTask` (roughly half a minute of background run, then the process is suspended and the work *resumes* when the user returns — it is not lost), and dictation gets the `audio` background mode in Info.plist (`build-testflight.yml`). Both iOS halves are **unverified on a device** until the next TestFlight build (`[OQ-116]`); Play's acceptance of the service types for on-device inference is `[OQ-115]`.
2. **The conversation persists.** The transcript lives on disk in the Rust core (`chat.jsonl`, `transcript.rs`): the question is written before the model runs, the reply after, so a reply finished while the webview was suspended is recorded all the same. Every return to the chat screen — from another screen, another app, or a restart — renders from it, reconciled with what the screen already shows (`chat.ts`) so nothing paints twice; the app also catches up on `visibilitychange`, because a suspended webview can drop the `chat:done` event. "New chat" is the only way a conversation ends. The transcript is excluded from cloud backup like the store.
3. **Permissions are asked for in the app, once, where the user is looking.** A powerful permission that only appears on a settings page nobody visits is not consent, it is a trap for the one user who finds it. Android's notification access is asked for in the chat on first open (benefit first, then the exact mechanism, then two honest buttons); "Not now" is final until the user visits the book icon. The same rule applies to calendar and contacts when they come.



`tauri android init` regenerates `gen/android` on every build, so everything Verdure needs in the Android project is applied by `app/scripts/patch-android-project.py` (run by both Android workflows) and asserted on the **built APK** by `build-android.yml`'s badging step: the Play `applicationId`; `RECORD_AUDIO` + the `MainActivity` audio-context init; **`ACCESS_NETWORK_STATE`** (the web search's Wi-Fi-only rule, `24`); **backup exclusion** for Verdure's memory — `fullBackupContent` (Android ≤ 11) and `dataExtractionRules` (12+) excluding `context.sqlite*`, `context.spool*` and `context.apps` at the app's data root (Tauri's `app_data_dir` on Android is `Context.dataDir`), asserted via `aapt2 dump xmltree`; **`VerdureListener.kt`**, the notification listener (`25`, `[OQ-100]`), declared `exported` behind `BIND_NOTIFICATION_LISTENER_SERVICE` — **every app's notifications since 2026-10-23** (no `conversations` filter; an older tree's filter is removed), conversations message by message and anything else as a `notice`, skipping the system's own packages, ongoing notifications and machinery categories, and the apps listed in `context.off`; and **`VerdureWork.kt`**, the foreground service behind the first non-negotiable above, with `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_DATA_SYNC`, `FOREGROUND_SERVICE_MICROPHONE` and `POST_NOTIFICATIONS` declared and `foregroundServiceType="dataSync|microphone"`, asserted on the built APK. The backup exclusion also covers `context.off` and `chat.jsonl`. The script is idempotent and verifies each patch on the file as written; it can be exercised on this Mac against a fixture tree (no SDK needed).

### Publishing to Play — the pipeline (2026-09-11)

**Identity.** Play knows the app as **`ai.verdure.app`** (`[OQ-60]`), set by patching the generated Gradle `applicationId` in `app/scripts/patch-android-project.py` — *not* by changing `tauri.conf.json`'s `identifier`, which macOS and iOS also use and from which `app_data_dir()` is derived. The Java package stays `ai.verdure.porter` so the JNI symbol keeps working. Full reasoning in `[OQ-60]`.

**Signing.** An upload keystore (RSA 2048, valid to 2054) held as four GitHub secrets: `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`. Modern keystores are PKCS12, which **does not support a key password distinct from the store password** — keytool silently discards one, so the two password secrets hold the same value. Google holds the real *app signing* key via Play App Signing; ours is only the *upload* key, and an upload key can be reset through support if lost. Back up the keystore **as a file attachment or as base64 text** — `pbcopy` of a binary keystore is silently lossy (2648 bytes came back as 4153).

**Upload.** `app/scripts/publish-to-play.py` talks to the Google Play Developer API directly from `build-android-play.yml`, restoring the "one tag → three platforms" property that iOS/TestFlight already has. Deliberately **not** a marketplace action: the step holds a credential that can publish to the founder's Play account, and a third-party action would receive it on every run. Skips gracefully (no failure) when `PLAY_SERVICE_ACCOUNT_JSON` is absent, falling back to the artifact.

**Founder-side setup, once:**
1. Google Cloud console → project → enable the **Google Play Android Developer API**.
2. Create a service account, then a JSON key for it.
3. Play Console → **Users and permissions** → invite the service account's email → grant *Release to testing tracks* for this app.
4. Add the JSON file as the GitHub secret **`PLAY_SERVICE_ACCOUNT_JSON`**.

~~**The one step that cannot be automated:** the **first** bundle of a brand-new app must be uploaded by hand in Play Console — Google's API refuses it.~~ **Wrong, measured 2026-10-23:** `android-play-4` was the very first bundle on the app and the API accepted it (`edits.insert` → `bundles.upload` → `tracks.update` → `edits.commit`, versionCode 4 on `alpha`). The real constraint is **draft-only releases while the app has never been rolled out** (`STATUS: draft` in the workflow). The founder rolls the draft out once in Play Console; after that flip `STATUS` to `completed` and the pipeline is hands-off end to end.

**Track matters.** Default is `alpha`, which is Play's **closed** track. Internal testing does **not** count toward the 12-testers/14-day production requirement; closed testing does (`[OQ-66]`).

## iOS

**[OQ-26] is resolved: the daemon model is not feasible on iOS — and no longer needs to be.** Background execution restrictions and the app sandbox rule out a Verdure system service. But as of iOS 27 (WWDC 2026), the OS itself ships the shared inference service Verdure would have had to build: a resident ~3B system model behind Apple's Foundation Models framework, accessible to every app via the public `LanguageModel` protocol.

The iOS strategy therefore pivots from "daemon" to **thin decider over Apple's stack**:
- Verdure on iOS is an app and/or embeddable library, not a service.
- For in-app open-weight models, MLX Swift works but is app-scoped: requires the Increased Memory Limit entitlement, lives under hard per-app memory ceilings (a 4 GB iPhone is limited to roughly 1B-parameter models), and models are killed with the app. Pending `[OQ-42]` on whether `MLXLanguageModel` itself is available on iOS.
- The distribution-channel opportunity: ship a **`VerdureLanguageModel` Swift package** conforming to Apple's `LanguageModel` protocol. Swift developers would get intent-first, device-aware model selection through Apple's own API surface — Verdure deciding between the system model, an MLX model, or a future cloud option, behind the same protocol Anthropic and Google are using to enter the ecosystem. Tracked as `[OQ-43]`.

TestFlight remains viable for early distribution. All of the above reflects iOS 27 developer betas (June 2026); re-verify at public release in the fall.

**Deliberate deviation (2026-07-04, flagged per CLAUDE.md): the iPad test build.** The founder's Mac (8 GB) can no longer exercise the 4B chat model, and his iPad is the only available device that can — so we are building a **temporary iPad port of the existing Tauri app** (bundled whisper.cpp/llama.cpp + downloaded GGUF) purely as a **test vehicle** for the chat/transcription stack. This contradicts the thin-decider strategy above and is *not* a change of strategy: it exists to answer `[OQ-64]` (does the ByteShape 4B load and feel usable) while the Mac can't, and it inherits iOS's real constraints — the per-app memory ceiling (jetsam) may cap the 4B regardless of device RAM, and the in-process ggml-abort risk (`[OQ-58]`) applies unchanged. Tracked as `[OQ-65]`; the strategy of record for a *shippable* iOS product remains thin-decider-over-Apple's-stack.

**Resolution (2026-08-12) — graduated, then retired.** The test vehicle did its job: the 4B loads and generates on the iPad (`[OQ-64]`, 2026-07-27), whisper/llama cross-compile for `aarch64-apple-ios`, and the mic works. iOS graduated to the **signed TestFlight channel** (`build-testflight.yml`); the unsigned-sideload workflow was deleted (git history is the archive). `[OQ-65]` is closed. **Unchanged:** TestFlight is still a *test* channel — the shippable-iOS decision remains this file's thin-decider strategy, not "the Tauri port is the product."

**Signing reuses ONE certificate (2026-10-08).** `build-testflight.yml` used Xcode's *automatic* signing. That needs the certificate's private key in the local keychain, and a runner's keychain is empty every run, so Xcode minted a new certificate per build (development to archive, distribution to export) and the runner threw the key away. They accumulated until Apple refused: *"Your account has reached the maximum number of certificates"* (`release-14`, 2026-08-22). Now one **Apple Distribution** certificate lives in the secrets `IOS_DIST_CERT_P12` + `IOS_DIST_CERT_PASSWORD`, imported into a throwaway keychain, and the build signs **manually**. `app/scripts/ios-signing.py` fetches the App Store profile for that certificate over the App Store Connect API, replacing the single profile named *Verdure CI App Store* each run, and checks that the profile grants every entitlement the build requests, so a missing App ID capability fails in a minute. **Kernel capabilities (Increased Memory Limit, Extended Virtual Addressing) cannot be enabled through the App Store Connect API**: it answers HTTP 409 with a list of supported types that excludes them (testflight-10). They are ticked once in the developer portal. Nothing in the pipeline can create a certificate any more. Signing runs **before** the Rust build, so a bad secret fails in about a minute, not after the 10× minutes. **Renewal:** the certificate expires a year after creation; export the new one as `.p12` and update both secrets. The iOS App ID is `ai.verdure.porter.898PY37K5S` (not `ai.verdure.porter`; see `[OQ-60]`). **First green run: `testflight-12` (2026-10-08)**: signed with the stored certificate and uploaded. Both kernel entitlements are verified **in the profile** (`--require-entitlement`); the exported `.ipa` itself is not yet checked (`codesign -d --entitlements`, cleanup queue in `21`).

**Apple frameworks a crate needs are declared in `tauri.conf.json`, not just by cargo (2026-10-20).** On macOS the app is a Rust binary and a crate's `cargo:rustc-link-lib=framework=X` reaches the link line. On iOS Tauri builds a *static library* that Xcode links, and Xcode knows nothing about cargo's directives — so `testflight-13` failed at `ld` with `_SCNetworkReachabilityCreateWithName` undefined, after every local check and `ci.yml`'s iOS `cargo check` (which never links) passed. The fix is `bundle.iOS.frameworks` (`["SystemConfiguration"]`), which `tauri ios init` writes into the generated `project.yml`. Rule: when a new dependency links an Apple framework, add it there in the same change.

**Deadline: iOS 15 minimum by April 2027.** App Store Connect warned on `testflight-12`: *"Your app targets iOS 14.0. Starting in April 2027, iOS apps must target 15.0 or later to be uploaded."* Tracked as `[OQ-85]`: the target is set in **two** workflow files (`ci.yml`, `build-testflight.yml`), so raise it in one shared place, not just one file.

## Open questions

- [OQ-23] Current Apple Developer Program pricing, notarization policy changes in 2026.
- [OQ-24] Current Windows code signing certificate pricing, CA recommendations, SmartScreen reputation timeline.
- [OQ-25] Android 14+ background service policies and Play Store requirements for system-level apps in 2026.
- [OQ-115] Play's acceptance of `dataSync` as the foreground-service type for on-device inference (the alternative, `specialUse`, needs a declaration and review), and whether the Pixel's battery manager lets the service keep a reply alive; measured on the Pixel 8a with the next `android-build-N`.
- [OQ-116] iOS background work is unverified on a device: does `beginBackgroundTask` from the generation thread buy the half-minute, and does cpal's audio session plus the `audio` background mode keep dictation running off-screen? Answered by the next TestFlight build, not before.
- [OQ-26] Resolved (WWDC 2026) — daemon infeasible on iOS, confirmed; strategy pivoted to a thin decider over Apple's Foundation Models stack (resident system model does the shared-service job).
- [OQ-43] `VerdureLanguageModel` Swift package — do we ship it, and in which phase? Strategic distribution decision, not just engineering.
- [OQ-67] Helping users free disk space for models — per-platform capabilities (iOS sandbox: impossible; Android: OS-mediated `StorageManager.allocateBytes`; macOS: deep-link only, no scan-and-delete) and the own-sandbox-first build order. Full survey + decisions in `open-questions.md`.
