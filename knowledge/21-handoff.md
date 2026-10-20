# 21 — Handoff: where we are, alignment, and next steps

*Role: current **state + next steps** — "where are we now / what do I do next." Overwritten as the build moves; it does not narrate chronology. For the dated history of how we got here, see `KNOWLEDGE.md`'s changelog.*

**Status:** 🟢 Current working state as of **2026-10-20**, refreshed after a full-project `/inspect`. **Verdure for macOS is PUBLISHED and shareable** (universal, signed, notarized, self-updating): <https://github.com/gwodu/Verdure-Releases/releases/latest>. **iOS signing is fixed** (one stored certificate, first green signed build `testflight-12`). **New session? Read this first**, then `18` (pivot), `20` (personal AI), `22` (chat memory), `23` (updates + consent).

## Start here — next session

**Ready to do, in order:**

1. **Fix the scope of the 27B experiment (`[OQ-84]`) — user-facing, do first.** `model.rs` swaps the Powerful slot to the Qwen3.8 27B on **every iOS build**, so iPhones and 8 GB iPads on TestFlight lost the 4B and are offered a model they can never run. Their memory warning also says "close some apps", which can never make it fit. Fix: keep the 4B everywhere, and offer the 27B as an *extra* choice only when total RAM ≥ 16 GB. Then re-tag `testflight-N`.
2. **Run the 27B on the founder's iPad (`[OQ-84]`).** `testflight-12` delivered it. Record whether it loads, its real footprint (`runtime_mb = 10000` is an estimate), and the time to the first reply.
3. **Work the cleanup queue below** (`/cleanup` takes the top item). The top three items correct things that mislead the next session.
4. **Website: make the monorepo build on Vercel (`[OQ-81]`).** Until it's fixed, verdureai.com runs a *copied* decider from `main`, so a decider change on this branch can't reach the site. `main` has also gained `app/privacy/page.tsx` since `[OQ-81]` was written, so merge `main` into the branch before deploying. `[OQ-87]` (what the site is for after the pivot) and `[OQ-86]` (a wrong L3 model recommendation) are the next website items.
5. **Play: the manual first upload, then `[OQ-83]`'s confirming test.** The Android rename is done (`ai.verdure.app`) and the upload is automated. Play's rule for an unpublished app is draft-only releases, and the 12-testers / 14-days requirement counts **closed** testing.
6. **`[OQ-58]` process isolation**, the P0 before a *public* link. It matters more now: the 27B experiment is the most likely thing yet to trigger a ggml `abort()`.
7. **`[OQ-64]` / `[OQ-70]`**: peak-RSS measurement across a full context window. **`[OQ-72]`** / **`[OQ-73]`**: diagnosed, not fixed. Dictation on the notarized macOS build is still unexercised on a device.

**Then the standing debt:** the desktop half of the `porter` rename (`[OQ-60]`, now optional because Android is decoupled), the evals harness (`[OQ-74]`), `[OQ-78]` (the update check exposes an IP), and the iOS 15 minimum before April 2027 (`[OQ-85]`).

**Local loop:** everything verifies free on this machine, and **CI no longer runs on push** — see "How to build / ship" below.
- `cd app/src-tauri && cargo test` — the Rust suite, seconds.
- `cd app && npm run build` — frontend typecheck + build.
- `npm test --workspace @verdure/decider` — the decider suite.
- `python3 -B -m unittest discover -s app/scripts` — release-script tests (`-B`: write no `__pycache__`). **No CI job runs these** (cleanup queue).
- `cd app && npm run tauri build` — the whole macOS app (~8 min cold).

The `#[ignore]`d `repro_oq71_generates_from_a_real_model` needs a `.gguf` to point at; its own doc comment says how to get one (the 469 MB copy was deleted 2026-08-22 to reclaim disk).

**Kept on disk for diagnostics:** `~/Verdure-bisect/llama-cpp-reference/` (26 MB — upstream's prebuilt macOS x64 `llama-cli`, `llama-tokenize`, `llama-perplexity`, `llama-bench`). This is the known-good reference that exonerated everything outside our binary during `[OQ-71]`, and `llama-tokenize` is the tool `[OQ-74]`'s tokenizer-parity check needs. Worth its space.

**Disk is tight** (~5–7 GB free; `target/debug` ~2 GB, `target/release` ~1 GB, and a universal build adds an `aarch64` tree on top). npm and Homebrew caches are safely clearable and were worth 4.3 GB.

## What exists and works (branch `mvp-porter-buzz`, not merged to main)

The MVP is a **Tauri app** (`app/`) — Verdure, the local personal AI — that runs models **inside one interface**. Built and tested locally; CI is used only for signatures and the platforms this Mac isn't (see "How to build / ship").

Verified on-device (founder's 2019 Intel Mac 8 GB, iPad via TestFlight, Pixel 8a via sideloaded APK):

- **Chat** ✅ — a plain conversation with a local model via llama.cpp. **Confirmed loading and generating on the iPad** (2026-07-27, answering `[OQ-64]`'s core question). The JSON tool-calling wrapper was **removed** on-device evidence (the tiny model over-triggered tools and leaked malformed JSON); capabilities-as-tools returns later, constrained + process-isolated (`[OQ-55]`). Model lineup: **ByteShape Qwen3 4B** (`Q3_K_S-2.77bpw`, ~1.4 GB) as the top RAM-gated pick, then **Qwen 3.5 2B** (added 2026-09-10 — hybrid, 12 KB/token conversation memory vs the 4B's 144, verified on this Mac), with Qwen2.5 1.5B/0.5B as floor/fallback. **Qwen 3.5 4B** is in the registry but gated off pending a footprint measurement (`[OQ-82]`). A first-launch **model picker** (Small/Balanced/Powerful, with download size + per-device memory fit, persisted, changeable from the header) replaced the earlier "best model only" toggle. **The 8 GB Mac runs the 4B** — confirmed 2026-08-22, weights resident, coherent replies; the old "fallback tier only" claim was an artefact of the Metal bug. `runtime_mb` (2300) was thought ~900 MB too conservative against a measured ~1.40 GB — but **2026-09-10 correction:** that ~1.40 GB was **weights-only**. A real load log shows llama.cpp also holds a `CPU_REPACK` buffer (weights re-laid-out for CPU SIMD — 490 MB on a 1211 MB model) plus a ~500 MB compute buffer, so peak resident runs ~1 GB ABOVE the file size. The conservative gate was right; lowering it would have caused OOM aborts. A *peak* RSS measurement across a full context window is still owed. (`[OQ-64]`)
- **Transcription (file)** ✅ — drop audio → text via whisper.cpp `base.en`; ~3.9× real-time measured on the Intel Mac.
- **Transcription (live mic)** ✅ **on all three platforms** — `live.rs`: `cpal` capture + a pure energy-gated `Segmenter` + transcribe-once-per-segment with a resident Whisper context. macOS confirmed 2026-06-30; iPad with the TestFlight builds; **Android confirmed 2026-08-03** (Pixel 8a) after root-causing the `ndk_context` initialization gap in Tauri v2 (changelog 2026-08-02).
- **Dictation into chat** ✅ — the mic button dictates straight into the chat box; the standalone transcription page was removed (2026-07-25). First mic tap without Whisper installed shows a size-and-consent prompt before downloading.
- **Downloads** ✅ — streamed to disk with a live progress bar, **resumable** (progress survives interruption; one-tap resume), keep-screen-awake on mobile during large downloads.
- **Out-of-space help** 🟡 (`[OQ-67]`, built 2026-08-12, on-device confirmation pending) — unfinished `.part` downloads visible + reclaimable in Storage; switching chat models offers removing the old one; Android's picker can ask the OS to clear other apps' cached junk (`StorageManager.allocateBytes`); macOS opens the system storage screen, iOS gets honest instructions. Principle: Verdure never deletes a byte it didn't create.
- **Performance predictor** ✅ — estimates speed from device specs (`packages/decider/src/performance.ts`), calibrated by real measurements.
- **Storage management** ✅ — per-model size, Remove (in-panel + global screen), disk fail-state warnings, free-space readout.
- **Chat memory** ✅ (`22`) — the conversation is remembered; **confirmed on-device on iPad 2026-08-14**. Beta round found and fixed a batch-overflow regression that surfaced as a raw "ran out of context" error.
- **UI** — chat-first layout (2026-07-17); **"midnight glass" visual identity shipped 2026-08-12** (deep navy + aurora mint + frosted glass, from the founder's brand video; canonical spec in `skills/design.md` "Visual identity"). Entry animations are CSS-only and reduced-motion-safe. Blur performance on the 2019 Intel Mac unverified until the next `app-build-N`.
- **macOS signing + notarization** ✅ (2026-08-17) — `build-app.yml` signs with the founder's Developer ID cert, notarizes via Apple, and staples; verified in CI by `codesign` + `spctl` (`accepted / source=Notarized Developer ID`) + `stapler validate`. **No more `xattr -cr`** — the .dmg opens with a double-click. Notarization reuses the existing App Store Connect key; only the signing certificate was new. `Entitlements.plist` grants microphone + outgoing network (the hardened runtime would otherwise break dictation).
- **Distribution** — macOS Intel .dmg via `app-build-N` tags; **iOS via TestFlight** (signed, no expiry, `testflight-N` tags); **Android test APK** via `android-build-N` tags; **Play open-testing workflow ready** (`build-android-play.yml`, signed .aab) but ungated founder-side steps remain — see Next steps.

Green locally (2026-10-20): Rust 62 passed + 1 ignored, decider 74, ios-signing 10. **CI runs only on a pull request or a manual dispatch — never on push** (`ci.yml`); nothing verifies a commit except running the suites locally before pushing. Triggers, tags and costs are owned by `11` (pipeline map).

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
3. ~~Chat reloads the model every message~~ — **resolved 2026-08-12** (`[OQ-59]`, `knowledge/22`): weights are resident in Tauri state across messages. Remaining refinement: the llama *context/KV cache* is still rebuilt per turn, so history is re-prefilled; holding it needs a self-referential struct and is gated on a measurement, not a guess.
4. **Legacy porter is transitional scaffolding.** `catalog.rs`, `porter.rs`, `port_tool` remain registered; the old recommendation flow (`renderHome`) is retained but unreachable. Decide keep-as-fallback vs remove (`[OQ-60]`); un-wrapped intents should show an honest "coming soon."
5. **Capabilities-as-tools is currently OFF.** Tool-calling was removed on on-device evidence (2026-07-27); the "apps for AI" step one (`20`) is deferred until it can come back constrained + process-isolated (`[OQ-55]`, depends on `[OQ-58]`).

## Cleanup queue (from `/inspect`, 2026-10-20)

`/cleanup` works this list top-down, one item per change. Six fresh readers inspected knowledge, skills + logs, Rust, frontend, decider + website, and the pipeline. **Four root causes produced most findings:**
- **(A) Facts restated in many places, refreshed in few.** The CI and release facts live in six files. The 2026-09-16 correction reached three; the handoff then went six weeks stale, the same failure the 2026-08-10 inspection found.
- **(B) The pivot's leftovers were never removed** (`[OQ-60]` undecided). There's dead UI and backend code, an unreachable decider path in the app, a website still selling the pre-pivot funnel, and an empty root `src/` core. `pub` on everything hides it from rustc's dead-code lint.
- **(C) Two vocabularies or two copies of one thing:** model ids in Rust and TS, tier names vs internal names in user copy, two memory models (`runtime_mb` vs `approx_mb + 700`), and a command-registry copy in a test.
- **(D) Tests that exist but no gate runs:** script tests, vitest, the untestable `main.ts`, unchecked `.ipa`/`.apk` artifacts, and `verify-release.py` steps that have never run in CI.

Ordered (structural and misleading items first):
1. **27B scope** — see Start here #1 (`model.rs:405-410`, `main.ts:75-77`, `:453`). Also map a stored 4B id to the current Powerful slot (`main.ts:406`, `model.rs:472`).
2. **Stale CI/verify instructions in procedures** — `/cleanup` itself still says to verify by pushing (`.claude/commands/cleanup.md:41-45`); `skills/testing.md:84,102,155`; `debugging.md:75` (says 90-day artifacts; actual is 14), `:138`. Make `11` the one owner of tags, costs and triggers; others link to it (`testing.md:85-96` vs `:93`).
3. **Memory accounting: one model** — context sizing uses file size + 700 MB while the gate uses `runtime_mb`, so the 4B at the gate edge can exceed the gate (`llm.rs:125-129`, `lib.rs:215`). An OOM-abort risk under `[OQ-58]`.
4. **Run the tests we have in CI** — one cheap Linux job: `python3 -m unittest discover -s app/scripts`, `npm test` for `app/`, `actionlint`. Route release and script work in `skills/README.md` (15 of the last 20 commits had no skill row).
5. **`verify-release.py` blind spots** — `latest.json` is written outside `--dir` and silently skipped; the `0.1.0` dmg name is hard-coded (`build-app.yml:145,218`, `verify-release.py:242,254`). The next `app-build-N` is the first run of the notarize-dmg and verify steps.
6. **Verify the shipped iOS/Android artifacts** — `codesign -d --entitlements` on the `.ipa`; `aapt2`/`bundletool` on the APK/AAB; `jarsigner -verify -strict` (`build-android-play.yml:135`). Add `concurrency` groups to the release workflows (two TestFlight runs would delete each other's profile, `ios-signing.py:187`).
7. **Make `main.ts`'s pure logic testable** — `void boot()` on import blocks tests (`main.ts:1398`). Extract `renderMarkdown` (the model-output safety boundary), `friendlyError`, the progress parser and one `fmtSize`, then add vitest cases. Fix the preview mock ids (`bridge.ts:74-76`).
8. **User-facing copy** — the "too big" error names an internal model and a nonexistent Settings screen (`lib.rs:190-195`); raw `{e}` reaches the UI (`model.rs:571,590,630`, `llm.rs:327,401`, `live.rs:216,243`, `main.ts:1261`); "0.4–1.4 GB" is hard-coded (`main.ts:639`); the Android-only mic path is shown on every OS (`main.ts:724`); use tier names in Storage and progress; recommend a default on the first-run picker.
9. **Dead code, after deciding `[OQ-60]`** — `renderHome`/recommendation flow (~200 lines of `main.ts`), `catalog.rs`/`porter.rs`/`port_tool` (registered on iOS and Android too), `transcribe_audio` + `audio.rs` + `symphonia`, `choose_chat_model`/`preferred_chat_model`, `USE_BYTESHAPE_QWEN3_4B`, `ToolDecision.action`, unused decider exports (`isTier`, `findIntentCard`), unused `performance.ts` (knowledge says it shipped; nothing calls it). Root `src/`, `tests/`, `Cargo.toml` are an untouched scaffold. Change `pub` → `pub(crate)` first, so the compiler lists the rest.
10. **Decoder correctness** — `token_to_str` per token drops multi-byte characters split across tokens (`llm.rs:432`, 3 deprecation warnings). Use `token_to_piece` with a UTF-8 decoder.
11. **Decider correctness** — the intent classifier matches substrings ("eco**system**" → audio separation; `intent.ts:45,96`); use word boundaries plus negative tests. Fooocus is offered for FLUX/Qwen-Image-Edit, which it can't run (`recommend.ts:387,400`, `12:40`). `textCopy` duplicates `decideText`'s tree.
12. **Knowledge hygiene** — `23` is missing from the `KNOWLEDGE.md` index; `[OQ-59]` holds misfiled `[OQ-60]` bullets and should move to Resolved; the changelog's "Role" line sits mid-list; CLAUDE.md's `app/` row is pre-pivot ("Buzz", "pending Xcode CLT"); 🟢 isn't in the status-marker vocabulary; `05` needs a superseded banner.
13. **Small hygiene** — SAFETY comments on Android/`mem::zeroed` unsafe blocks (`lib.rs:447,469`, `device.rs:182,197,257,274`); the command-registry test checks a drifted copy (`lib.rs:648-707`); the updater key is inlined into a script (`build-app.yml:125`); pin the Rust toolchain; drop the unused `aarch64-apple-ios-sim` target; download SHA256 + a macOS progress bar (`model.rs:546-576`).

**Confirmed healthy (don't re-inspect):** the routing tables match and there are no orphaned skills; commands `@`-include skills; OQ codes are unique and every reference resolves; `11`'s pipeline map matches the YAML; release workflows have timeouts, 14-day retention and `if-no-files-found: error`; iOS signing fails fast and holds no secret on argv; model URLs are checked from `model.rs`; pure logic in Rust (segmenter, context/eviction maths, parsers) is extracted and tested; FFI goes through wrapper crates; the registry is data and the unverified-model gates are tested; Metal pinning has a regression harness; the licence guard covers `Cargo.lock`; the decider matrix matches `03`/`12`/`02`/`13`, the website imports it unforked on this branch, and "not local yet" is handled honestly; `renderMarkdown` escapes first; update consent is honest, reversible and tested; reduced motion is respected.

## Next steps (prioritized — pick up here)

1. **Play closed testing.** ~~Decide the rename~~ done for Android (`ai.verdure.app`, 2026-09-11); keystore, secrets and the Play app exist, and the upload is automated (`[OQ-83]`). Remaining: the manual first upload, then 12 testers for 14 days on the **closed** track before production access.
2. **Process-isolate the model runtime** (`[OQ-58]`) — a ggml `abort()` must never take down the UI. Also removes the 4B load-abort risk and re-opens the door to grammar-constrained tool-calling.
3. ~~Resident chat model + conversational memory~~ — **done 2026-08-12** (`22`). Residual: the KV cache is still rebuilt per turn.
4. **Calibrate the 4B on a capable device** (`[OQ-64]` residual) — measure its real resident footprint, set `runtime_mb` from measurement, and judge reply quality.
5. **Legacy-porter decision** (gap #4) and the deferred UX polish (raw-error copy is fixed as of 2026-08-10; remaining: Storage spec panel jargon).

**P2 — the vision**
- **Personal context core** — local, user-owned store every model reads/writes (`[OQ-56]`).
- **AI-first dictation** — system-wide voice button → commands into Verdure + transcription into the context core (`[OQ-57]`). Foundation exists in `live.rs`; what's left is the system-wide surface and routing.

## How to build / ship

Rust builds and tests run **locally** since 2026-08-21 (Xcode CLT 16.4 + `cmake`) — that's the inner loop. **Shipping** still runs entirely in CI, because the runners hold the signing identities (Developer ID, App Store Connect, Android keystores) and cross-compile the platforms this Mac can't:
**Decision rule (set 2026-08-22): compile continuously, package on request.** Pick the narrowest trigger that answers your question — full costs and rationale in `skills/testing.md` ("CI cost discipline").

| What you need | How | Billable min |
|---|---|---|
| Test macOS yourself | **local** `cd app && npm run tauri build` | **0** |
| Rust/logic correctness | **local** `cd app/src-tauri && cargo test` | **0** |
| Signed+notarized macOS for someone else (universal) | tag `app-build-N` | ~130 |
| Test on iPad | tag `testflight-N` | ~57 |
| Test on Android | tag `android-build-N` | ~8 cached |
| Genuinely ship everywhere | tag `release-N` | ~200 |

- **macOS is built on this machine, not in CI.** `cd app && npm run tauri build` → **`target/release/bundle/macos/Verdure.app`**. Verified 2026-08-22: compiles in **~8 min** cold (llama.cpp + whisper.cpp at `-O3`; incremental rebuilds are far quicker), produces a valid x86_64 Mach-O, and **launches**. Unsigned, which is fine for the founder's own testing — a locally built app carries no quarantine flag, so Gatekeeper doesn't prompt and it won't translocate. CI's macOS build exists only to *sign and notarize* for handing to someone else.
  **The `.dmg` step fails locally and that's OK.** `bundle_dmg.sh` errors after the `.app` is already built (it drives Finder via AppleScript to style the disk image, which needs an interactive session + Automation permission). The `.app` is the deliverable — copy it to /Applications and run it. If it fails, `hdiutil detach` any leftover `/Volumes/dmg.*` and delete the `rw.*.dmg` intermediate it strands, or the next run inherits the mess. Want a real `.dmg`? That's what `app-build-N` is for.
  **Trap when scripting this:** `npm run tauri build | tail` reports **`tail`'s** exit status, so a failed build looks like success. Check for the artifact, not the exit code (`skills/debugging.md` §7 — a null result from a broken tool looks exactly like a clean one).
- **iOS and Android are built only on request** — when the founder asks, or when enough has changed that they should be re-verified on device. Never as a side effect of shipping something else.
- **`release-N` is a SHIPPING tag, not a testing one.** It fires all three platforms (~144 billable min). It was added 2026-08-12 after iOS silently sat three weeks behind, and it still earns its place for a real release — but reaching for it to test *one* platform buys two builds nobody asked for. That is exactly what happened on `release-14`. The Play `.aab` is deliberately *not* on `release-*`.
- **Pushing a branch runs no CI** (no `push` trigger since 2026-09-16). CI runs on a pull request or `workflow_dispatch`. The x86_64 and iOS cross-compile checks only run there, so before tagging iOS after a long gap, either dispatch CI manually (~40 billable min) or accept that a `testflight-N` fails early on a compile error. **Caveat:** `main`'s `ci.yml` still has a push trigger that fails every time (`[OQ-88]`).
- **macOS app from CI (signed + notarized, UNIVERSAL):** `git tag app-build-N && git push origin app-build-N` → `build-app.yml` → `verdure-macos-universal` (.dmg/.app). Runs natively on both Apple Silicon and Intel ([OQ-76]). Install with `gh run download <run-id> -n verdure-macos-universal`, then copy to /Applications. **Use `gh`, not the browser** — a Safari download carries a quarantine flag that makes macOS run the app translocated from `/private/var`, so you can end up testing a stale copy (this bit us 2026-08-22). No `xattr -cr` needed since notarization landed.
- **iPad via TestFlight** (the current iOS channel): `git tag testflight-N` → `build-testflight.yml`. Signs **manually** with one stored distribution certificate; the secrets, the one-time portal steps and the yearly renewal are in `11` (iOS, "Signing reuses ONE certificate"), the single runbook. iOS bundle ID is `ai.verdure.porter.898PY37K5S`. **Costs:** macOS bills 10×; `testflight-12` was 5 min wall ≈ 55 billable on a cold cache, and the cache saves only on a green run, so failed iterations stay cold. Test vehicle only — the shippable iOS strategy stays `11`'s thin-decider.
- **Android test APK** (`[OQ-66]`): `git tag android-build-N` → `build-android.yml` (Linux runners) → test-signed `.apk` (throwaway CI key — uninstall before installing a differently-signed build).
- **Android Play open testing:** `git tag android-play-N` → `build-android-play.yml` → signed `.aab` via the persistent upload keystore (secrets pending founder-side). Both Android workflows share `app/scripts/patch-android-project.py` (manifest + mic wiring), so they cannot diverge.
- The legacy unsigned iOS path (`build-ipad.yml`, `ipad-build-N` → 7-day sideload) was **retired 2026-08-12** — TestFlight superseded it and `[OQ-65]`'s exit condition was met. To restore it: `git show 2026-08-12 -- .github/workflows/build-ipad.yml` (git history is the archive; the file is not kept as dead code).
- Key fix in place: `app/src-tauri/.cargo/config.toml` forces `MACOSX_DEPLOYMENT_TARGET=10.15` so whisper/llama cross-compile.

## Distribution — how Verdure reaches people (2026-08-28)

**The link to share:** <https://github.com/gwodu/Verdure-Releases/releases/latest>
Direct download, skipping the release page: `.../releases/latest/download/Verdure_0.1.0_universal.dmg`
**Do NOT share `verdureai.com/updates/...`** — that is the machine-facing updater endpoint; a human clicking it gets JSON.

- **`gwodu/Verdure-Releases`** is a **public** repo holding downloads only, no source. It carries **no licence file** deliberately: a public repo with none is "all rights reserved", the right default for a proprietary binary, and adding one would licence the product away (`[OQ-79]`). It exists because release assets on a *private* repo require an auth token — verified: anonymous requests 404.
- **Updater endpoint:** `https://verdureai.com/updates/latest.json`, redirected by `website/next.config.ts` (on **`main`**) to the GitHub release. **The endpoint is compiled into every binary** — a copy of Verdure asks that exact address for the rest of its life, so it points at our own domain rather than GitHub, and moving hosting later must never strand an install. Redirect, not rewrite, so GitHub serves the bytes.
- **Cutting a release:** `git tag app-build-N && git push origin app-build-N` → signed, notarized, stapled universal `.dmg` + `.app.tar.gz` + `.sig` + `latest.json`. Then `gh release create vX.Y.Z --repo gwodu/Verdure-Releases <artifacts>`.
- **Before publishing, always:** `python3 app/scripts/verify-release.py --dir <bundle>`; **after publishing:** `--url https://verdureai.com/updates`. The second is the one with no blind spot — it re-downloads the published bytes through the live redirect chain. It is wired into CI as of `app-build-26` onward; `app-build-25` predates it and was checked by hand.
- **`v0.1.0` shipped once with a Gatekeeper-rejected `.dmg`** and was pulled within minutes (`[OQ-80]`). Tauri notarizes the `.app` and merely *signs* the `.dmg` around it; CI verified only the `.app`. Both are now notarized and both are asserted, with `--type open` for the disk image.

## Environment constraints (carry forward)

2019 Intel Mac (x86_64), **8 GB RAM**, Xcode CLT 16.4 + `cmake` installed (since 2026-08-21), ~7 GB free disk. Build and test locally — including the macOS app bundle; CI is for signatures and other platforms.

**The "this Mac only runs the fallback chat tier" constraint is retired (2026-08-22).** It ran the Qwen3 4B ("Powerful") with the weights resident. That belief formed while the Metal bug (`[OQ-71]`) made every model emit garbage here, so a correctness failure was recorded as a memory ceiling — see `[OQ-64]`'s note, which also has the first real footprint measurement. Disk stays the live constraint: `target/debug` ~2.1 GB and `target/release` adds ~1 GB more. See the `build-env-constraints` memory.
