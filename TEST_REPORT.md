# TEST_REPORT.md

Test discipline record across the Verdure monorepo. Decider: `npm test --workspace @verdure/decider`. Website: `npm test --workspace nextjs`. Porter app frontend: `cd app && npm test` (vitest, since 2026-08-28) + `npm run build` (typecheck). **`app/scripts/shots.mjs` is broken** — it waits for the pre-July home screen and times out; take screenshots with a direct Playwright call until it is repaired or deleted. Porter Rust: `cd app/src-tauri && cargo test`. Long-term binary: `cargo test`. Newest first.

## Verdure desktop app (`app/`) — post-pivot

### 2026-10-20 — Web search (`knowledge/24`)

- **Rust (`web.rs`, 12 new):** scrub drops emails and 7+-digit number strings (`+234-803-123-4567,` with its trailing comma — the first version missed it; the token is now judged without trailing punctuation), keeps years/prices/model numbers, caps at 200 chars by *characters* (300 × "é" → 200), collapses whitespace. Budget = n_ctx/3 clamped 120–600. `fit_snippets` keeps rank order, trims the straddling snippet rather than dropping it, honours the count cap. `sources_block` numbers and dates each source and ends with the citation instruction. `parse_reply` drops empty snippets; a broken reply surfaces "try again", not serde's "expected …". `net_state_from`: unreachable or connection-required → Offline, WWAN → Cellular, else Wi-Fi. `relay_host` is the bare hostname.
- **Rust (`llm.rs`, 2 new):** `compose_user_turn` puts the sources block first and the question last; the prompt framed with history contains the block exactly once and the earlier turn without it. The offline note composes ahead of the question; `None` is a no-op. The `#[ignore]`d repro harness takes the new `None` argument.
- **Third-party notices guard caught the new crates:** `system-configuration` and `system-configuration-sys` were missing; regenerated (`generate-notices.py`: 520 components).
- **App vitest (`web.test.ts` 11, `format.test.ts` +4):** default mode is `wifi`, corrupt stored values fall back; off → never searches on any network; wifi/cellular → ask, wifi/offline → offline, `unknown` → ask (Android must not spend data on a guess); `any` → search except offline. Offer heuristic: 7 positive fixtures, 7 negatives including "nowhere" (whole-word) and the four example questions. Disclosure text carries the four named parties and no absolute "nothing is logged". Citations: only `[n]` within range becomes a marker, `[12]` and `[3]` stay literal with 2 sources, none without sources, code spans untouched; `citedSources` dedups in order of first use.
- **Website vitest (`lib/search.test.ts` 16):** query validation (type, empty, char cap); budget clamp incl. `NaN`/strings; the LLM Context URL carries exactly `q` + `maximum_number_of_tokens`, no location/language; headers send the key and no cookie/forwarded-for; both reducers on documented shapes plus tolerant variants and unknown shapes → `[]`; trim keeps 5 in order, caps per-snippet at 700 and the straddler to the remainder; a maximal 5-snippet reply serialises under 4 KB.
- **Route, exercised on `next dev`:** GET → 405; POST without a key → 503 "Web search isn't set up on the server yet." (checked before the body, so a bad body also gets 503 until the key exists). **Not exercised:** a live Brave call — the key isn't on this machine; `[OQ-90]`.
- **Visual review (Playwright, 390×844):** offer card (first time, disclosure open), answer with `[1]`/`[2]` markers and the sources footer, second offer with the collapsed toggle, Storage's Web search section. First attempt's `text=Check the web` selector hit the paragraph, not the button — script bug, scoped to `button:has-text`.
- Rust 67 passed + 1 ignored, zero warnings; app vitest 50; website 25; tsc clean on both.
- **`testflight-13` failed at Xcode's link step** (~3.5 min wall, ≈35 billable): `_SCNetworkReachabilityCreateWithName` / `_SCNetworkReachabilityGetFlags` undefined for arm64. The crate's build script emits `cargo:rustc-link-lib=framework=SystemConfiguration`, which only reaches cargo's own link line; on iOS Tauri hands Xcode a static library, so the framework must be declared in `tauri.conf.json` (`bundle.iOS.frameworks`) to land in the generated project.yml. Neither `cargo test` here nor `ci.yml`'s iOS `cargo check` links an iOS binary, so nothing local or in CI could have caught it. Fixed; the real-context integration test still builds with the new key (Rust 67).

### 2026-10-20 — Web search on the iPad (`testflight-14`): two defects, one honesty bug

- **On-device (founder, iPad, Wi-Fi):** approved search → badge "Couldn't check the web — no connection" and the model wrote "the device is offline". The device was online. Reproduced from here with the app's exact request: `POST https://verdureai.com/api/search` → **307** to `www.verdureai.com`, which ureq does not follow for a POST (spec-correct), so the app parsed a "Redirecting..." page; and at `www.` the route is **404** because Vercel's production deployment predates `e4e8c39` (`[OQ-81]`). The local `next build` succeeds and lists `ƒ /api/search`, so the failure is in Vercel's build, not the code — logs not readable from this machine (MCP 403 for the team scope; CLI not logged in).
- **Fixed in the app:** `RELAY_URL` → `https://www.verdureai.com/api/search` (test asserts the `www.` host); `search` rejects any non-200 instead of parsing it; `UnavailableReason { Offline, Failed, Empty }` flows from the UI's own knowledge into the model's note AND the badge, so the stated cause can't be invented (`the_model_is_told_only_the_reason_the_ui_knows`: a relay failure never says "offline"). Rust 69, app vitest 50, tsc clean.
- **Not yet re-verified on device** — needs the relay actually deployed first; a `testflight-N` before that would only reproduce the failure with better wording.

### 2026-10-20 — Install paths can run the model they're offered for

- New invariant `never offers Fooocus for a model it cannot run`: every tier × image task. Wherever Fooocus is offered, the model must be Stable Diffusion. **Positive control:** run against the old code it fails with "L3+ image generate: expected 'FLUX.1 schnell' to match /Stable Diffusion/"; it passes on the fix.
- Matrix fixture updated deliberately with the knowledge change: L3+ image generate is SDXL.
- decider 83, website 9.

### 2026-10-20 — The intent classifier matches whole words

- **Negative fixtures (7), from the inspection's probe:** "immune system", "ecosystems", "go to the store", "fix my resume", "photosynthesis", "function of the liver", "a TV program". Each used to land on a wrong specialist (stem separation, code, image generation) and now falls through to general text.
- **Positive guards (5):** plurals ("images", "recordings"), symbols ("c++"), phrases ("speech to text"), and a real code request without the dropped words ("fix this python bug").
- **Caught by an existing test:** the card lock `classifyIntent(card.example) === card.classification` failed on "create images…", because whole-word matching lost plurals. Fixed with an optional trailing "s" on every keyword, not a per-word patch.
- decider 74 → 82; website 9; decider typecheck clean.

### 2026-10-20 — Characters split across tokens no longer vanish

- `one_decoder_per_reply_keeps_characters_split_across_tokens`: "é" fed as two one-byte halves through one persistent decoder gives "é"; through fresh decoders per half (the deprecated `token_to_str`'s behaviour) it doesn't. It exercises the same `decode_to_string` call `token_to_piece` uses.
- **Not run against a real model here:** there's no `.gguf` on disk. The `#[ignore]`d `repro_oq71_generates_from_a_real_model` harness is the place to confirm with an emoji- or CJK-heavy prompt.
- Rust 53. The three deprecation warnings are gone; the crate builds with **zero warnings**.

### 2026-10-20 — Dead code out, guided by the compiler

- **`pub` → `pub(crate)` across the modules first,** so rustc could name dead code. It immediately listed `choose_chat_model`, `preferred_chat_model` and `ChatModelTooBig::model_name`. After each removal it named the next layer (`DeviceSpec`, `detect_spec`, `detect_tier`, file `transcribe`), until only the deprecated decoder call (item 10) remained.
- **Removed tests:** 14 that covered only deleted code. Rust 66 → 52, all green.
- **`handlers()`**: the integration test now builds the real app with the real command list. Its private copy had drifted, missing `chat_model_options` and `open_third_party_notices`.
- **Notices:** regenerated. `the_engines_we_wrap_are_credited` now checks that every `| MPL-2.0 |` row has a crates.io source link, replacing a hard-coded "Symphonia". **Positive control:** deleting the `cssparser` link made it fail with "MPL-2.0 component cssparser has no source pointer"; restoring it made it pass.
- **Not compiled here:** the iOS and Android `cfg` branches of `device.rs` (no iOS or Android target on this Mac). The removed functions were self-contained per platform; CI's iOS/Android checks cover them on the next PR.

### 2026-10-20 — Copy fixes, with their logic made testable

- `picker.test.ts` (4): recommends the most capable standard model that fits; never the experiment; falls back to the smallest when nothing fits; null with no options.
- `micPermissionHelp` (2): each OS gets its own path, and never another platform's.
- **Cross-language contract:** the download-failure sentence must contain "connection" (Rust `download_failure_reads_plainly_and_keeps_the_resume_hook`), and the UI's `looksInterrupted` must recognise that exact sentence (vitest). Rewording it can no longer silently remove the Resume button.
- **Visual check:** screenshotted the first-run picker in `vite preview` with Playwright. **Found while doing it:** `app/scripts/shots.mjs` still drives the pre-pivot intent grid and would fail on today's app, so the screenshot loop is broken. It's queued in `21`.
- vitest 30 → 36, Rust 66.

### 2026-10-20 — The UI's formatting gets its first tests

- `app/src/format.test.ts` (24): `renderMarkdown` against seven injection payloads, plus "only its own fixed tag set ever appears"; formatting (bold, emphasis, code, lists, headings, a lone `*` left alone); `escapeHtml` for all five characters; `fmtSize`/`fmtBytes` boundaries (1023 MB vs 1.0 GB, negative, fractional, never "0 MB" for a real file); `friendlyError` never blank or `[object Object]`; `parseProgress` rejects near-misses; `looksInterrupted` vs an unrelated error.
- **Bug caught by the first test:** "does not bold inside a code span" failed. The code-span step ran first, but bold/emphasis then ran over the `<code>` contents too, so the comment claimed an ordering guarantee the code never had. Committed as two steps: the pure move with the test held back (behaviour unchanged), then the fix with that test plus a mixed code/bold case.
- vitest 5 → 29. tsc + vite build clean.

### 2026-10-20 — Checking the shipped iOS and Android artifacts

- **iOS** (`build-testflight.yml`, before upload): unzips the `.ipa`, runs `codesign --verify --strict`, and asserts both kernel entitlements are `true` **in the signed app** and `CFBundleIdentifier` is the App Store ID. Tested locally both ways: the unsigned Mac build is rejected, naming the two missing keys and the wrong bundle ID; a correct plist passes. One fix came out of that: empty `codesign` output (an unsigned app) used to crash `plistlib`, and now it reports cleanly.
- **Android Play** (`.aab`): `jarsigner -verify` exits 0 on an unsigned jar ("jar is unsigned."), so the step now requires `jar verified`. Tested locally: an unsigned jar is rejected and a self-signed one accepted. **Rejected the inspection's `-strict` suggestion:** a self-signed upload key raises warnings (self-signed, no timestamp, invalid chain), and `-strict` would have made every Play build fail.
- **Android APK:** `aapt2 dump badging` asserts `package: name='ai.verdure.app'` and `RECORD_AUDIO` on the built APK. No `aapt2` on this Mac, so its first run is the next `android-build-N`.
- `concurrency` groups on all four release workflows (no cancel-in-progress). actionlint clean.

### 2026-10-20 — `verify-release.py` can no longer pass by checking nothing

- New `test_verify_release.py` (6), the script's first committed tests. `dmg_name` follows the manifest version (and strips `v`); no version is an error, not a guess. `find_latest_json` uses an explicit path outside the bundle (where CI writes it), returns None when the path is missing, finds one nested in the bundle, and returns None when there is nothing, which the caller now treats as a **failed** check rather than a skip.
- **Run for real:** `verify-release.py --url https://verdureai.com/updates` gave **25/25** against the live release. The `.dmg` name now comes from `latest.json` instead of the hard-coded `0.1.0`.
- Script tests: 10 → 16. The `--dir` path with `--latest-json` first runs in CI on the next `app-build-N`.

### 2026-10-20 — CI runs the tests that already existed

- New `ci.yml` job `app-checks` (Linux): `npm test --workspace @verdure/porter-app` (vitest 5), `python3 -B -m unittest discover -s app/scripts` (10), and `actionlint` v1.7.7 with `-shellcheck=`.
- **actionlint checked locally first, with a positive control:** a clean pass on all six workflows (exit 0), and a deliberately broken file is flagged (`property "nope" is not defined`). Without the control, a clean pass could mean a broken linter.
- **Not run yet in CI:** the job's first run is the next PR or `workflow_dispatch` (Linux, ~2 billable min).

### 2026-10-20 — One memory model for the gate and the window

- `a_model_admitted_at_the_gate_edge_gets_no_extra_window`: the 4B with `available == runtime_mb` gets 0 spare and `MIN_CONTEXT`. Before the fix it got ~264 MB of "spare" (~1900 tokens of KV), past the gate that admitted it.
- `spare_memory_subtracts_the_full_resident_footprint` replaces the weights-plus-headroom test; it still checks for no underflow when memory is tight.
- Rust 64 → 65 passed. `CONTEXT_HEADROOM_MB` was deleted: `runtime_mb` already carries the margin.

### 2026-10-20 — 27B scope fix

- `every_device_keeps_the_4b_as_powerful`: covers iOS at 4/8/16 GB and non-iOS at 8/64 GB; the three standard choices come first, in order.
- `the_27b_is_offered_only_on_16gb_ios`: yes on 16 GB iOS; no on an 8 GB iPad, an iPhone, or a 64 GB non-iOS machine. The platform is a parameter (`chat_model_menu_for`), so **the iOS branch now runs on this Mac**. The old `cfg`-swapped constant's iOS assertion never ran anywhere.
- `a_27b_choice_on_a_small_device_falls_back_instead_of_failing`: a stored 27B id where it isn't offered auto-picks a model that fits.
- `every_menu_choice_is_in_the_registry` now iterates the largest menu (16 GB iOS), so the 27B is covered too.
- Replaced `powerful_slot_is_the_4b_off_ios` (it tested the swap that was the bug). Rust 62 → 64 passed.

### 2026-10-08 — `ios-signing.py` (one certificate for every iOS build)

`python3 -m unittest app/scripts/test_ios_signing.py`: **10/10 pass.**
- **JWT signed for real:** a throwaway P-256 key, signed by `openssl`, converted from DER to raw `r||s`, then verified back with `openssl dgst -verify`. Also checked by hand with a PKCS#8 key (`BEGIN PRIVATE KEY`, the format Apple's `.p8` uses).
- **DER edge cases:** handles a sign-padded 33-byte integer and a short 30-byte one; rejects garbage.
- **Exact matching:** the App ID filter is a prefix match, so a longer identifier is ignored. The certificate is matched on its exact DER bytes, not its name.
- **Entitlement check:** reports exactly the keys a profile doesn't grant. A key that's only a prefix of a granted one doesn't count as granted. (Replaced the capability-enabling tests: Apple's API rejects kernel capabilities, so that code was removed.)
- **Profile:** the previous `Verdure CI App Store` profile is deleted and other profiles are left alone. The new one names only our certificate and is installed to both Xcode profile folders.
- **Live runs found two bugs the fakes couldn't:** `testflight-9` failed because `limit` is rejected on the App ID capabilities endpoint (HTTP 400). `testflight-10` failed because the API's `capabilityType` doesn't include the kernel capabilities (HTTP 409). Both failed in the first minute, before the build. Both runs confirmed the JWT authenticates, the App ID resolves, and the `.p12` imports with its password.
- **`testflight-11`:** the certificate was found on the account and the profile was created, and the profile grants both kernel entitlements, all confirmed live. The archive then failed on a workflow bug: `xcodebuild` splits `CODE_SIGN_IDENTITY[sdk=iphoneos*]=…` at its first `=`, so it looked for an identity named `iphoneos*]=Apple Distribution`. Removed; the plain command-line `CODE_SIGN_IDENTITY` already outranks the project. It failed after ~3 min wall (~30 billable). **Correction (2026-10-20):** the cache was *not* hit. The logs say `Cache not found` on -11 and -12; `actions/cache` saves only on a green job, so failed iterations stay cold, and `testflight-12` was a cold ~5 min wall ≈ 55 billable. The four runs together cost ~100 billable minutes, not the ~15 reported at the time (wall minutes were reported as billable).

### 2026-10-08 — Qwen3.8 27B iPad experiment

- `qwen38_27b_is_opt_in_only`: the auto-pick never hands out the 27B, even with 64 GB free. It's in `MODELS`, so it can be downloaded and removed. Its `runtime_mb` is above an 8 GB iPad's jetsam headroom (so the memory check refuses it there) and above its own weights. It disables thinking.
- `powerful_slot_is_the_4b_off_ios`: guards the swap to iOS only; the 4B stays everywhere else, and the Powerful slot is the last menu entry.
- `chat_options_report_fit_against_the_device` now checks whichever model fills the Powerful slot, not a hardcoded 4B id.
- **Caught before shipping:** the picker drew every entry in its UI list, whether or not the backend offered it. Adding the 27B's description would have shown two Powerful cards on the Mac. Fixed: once the backend's menu loads, only the models it offers are drawn.
- **Not run:** the 27B has never been loaded. The real file is 8.8 GB and this Mac has ~11 GB free disk and 8 GB RAM. The iPad run is the test (`[OQ-84]`).

### 2026-09-11 (later) — Verifying a publish path that cannot yet be run

**The constraint again.** `publish-to-play.py` cannot be executed end-to-end: no service account exists, and Google refuses an API upload for a new app's first bundle regardless. So everything testable was tested, and the untestable part was written down rather than assumed green (`[OQ-83]`).

| Case | Expected | Result |
|---|---|---|
| Missing `.aab` | refuse before opening a Play edit | ✅ |
| 2 KB `.aab` (truncated build) | refuse — "not a real app bundle" | ✅ |
| Implausible package name | refuse | ✅ |
| Invalid track (`closed`) | argparse rejects; valid set named | ✅ |
| Valid input | passes, defaults to `alpha` = **closed** track | ✅ |
| versionCode unset (local build) | `tauri.properties` untouched | ✅ |
| versionCode 42 → 43 | replaced, not appended (exactly one line) | ✅ |
| versionCode non-numeric / > 2.1e9 | refuse with the limit stated | ✅ |

**Why validation runs before the first network call.** The API's `edits` flow opens a transaction; aborting mid-way leaves a dangling edit and an opaque server-side error. Every predictable mistake is caught locally first, and the workflow runs `--dry-run` immediately before the real invocation.

**Track correctness was verified, not assumed.** Google's docs confirm the 12-testers/14-day requirement counts **closed** testing only — internal testing does not qualify. Defaulting to the wrong track would have cost two weeks of dwell time with nothing to show. That is why `alpha` is the default and `internal` is not.

**Artifact-level verification of the earlier change.** `android-build-10` (green) was not trusted on its log. The APK was downloaded and its *binary* manifest decoded — and the first attempt at that returned zero hits, because `strings` defaults to ASCII while Android stores manifest strings as UTF-16. **The positive control caught a broken instrument rather than a broken build.** Re-done correctly: `ai.verdure.app` and `ai.verdure.app.fileprovider` present, `ai.verdure.porter.MainActivity` intact.

**Not tested, stated plainly:** no call has ever been made to Google's API — auth, edit lifecycle, bundle upload, track assignment and commit are all unexercised (`[OQ-83]`). No `.aab` has been built by the Play workflow. Nothing installed on an Android device from this build.

### 2026-09-11 — Testing a build-time patch with no way to run the real generator

**The constraint.** `app/scripts/patch-android-project.py` edits a project that only exists after `tauri android init`, and this Mac has no Android SDK (`~/Library/Android/sdk` holds only `licenses`). So the code under test could not be run against its real input here.

**What was done instead of guessing.** The Gradle template was **extracted from the Tauri CLI binary itself** (`strings` over `cli.darwin-x64.node`, v2.11.2), giving the exact generated shape — `namespace` and `applicationId` on separate lines, both from `{{app.identifier}}`. A fixture was built from that verbatim text and the patch run against it.

| Case | Expected | Result |
|---|---|---|
| Good template | `applicationId` → `ai.verdure.app`, `namespace` untouched | ✅ |
| Re-run | no-op, exactly one `applicationId` | ✅ idempotent |
| `namespace` changed by a future Tauri version | fail loudly, name the JNI symbol | ✅ fires |
| Second `applicationId` appears (build flavors) | refuse rather than guess | ✅ fires |
| Gradle file moves | fail with the expected path | ✅ fires |
| Control after each failure case | still passes | ✅ |

**Why the negative cases were run at all.** A guard that has never fired is indistinguishable from a guard that cannot fire. This is the `[OQ-80]` lesson — every check was green about the wrong object — and the `verify-release.py` precedent of proving the instrument works before trusting a pass.

**The failure mode being guarded.** Not a crash. If the Java package and the JNI symbol diverge, Android's mic capture thread dies **silently** and the UI hangs on "preparing" until the watchdog. Nothing in a build log would say so. The assertion converts an invisible runtime failure into a loud build failure.

**Stated plainly — not tested:** no real `tauri android init` run, no APK/AAB built, nothing installed on a device. CI is the first real execution, and an `android-build-N` tag should run before any `android-play-N` tag.

### 2026-09-10 — Testing a model swap: six tests, and the one that would have caught a shipping bug

**Unit tests written (6 new, 54 → 60 passing).**

| Test | What it locks down |
|---|---|
| `no_think_style_prefills_an_empty_thinking_block` | The reasoning off-switch is present AND the no-think prompt is byte-identical to plain ChatML apart from the suffix — so history framing cannot drift between the two arms |
| `reasoning_models_disable_thinking` | Both Qwen 3.5 entries carry `ChatMlNoThink`; a copy-pasted `ChatMl` is how this regresses |
| `a_cheaper_kv_cache_buys_a_bigger_window` | The KV win is spent where it matters — `context_for_budget` yields a longer window at 12 KB/token than 144 |
| `qwen35_context_is_far_cheaper_than_the_dense_4b` | The 10× KV advantage is a property of the registry, not a comment |
| `unverified_qwen35_4b_stays_out_of_the_live_pick` | Abundant RAM cannot smuggle in an unmeasured model |
| `every_menu_choice_is_in_the_registry` | Every menu/candidate id resolves via `find()` — a model missing from `MODELS` is both undownloadable and unremovable (`[OQ-73]`) |

**The edge case that mattered — caught by running, not by reading.** Qwen 3.5 is a reasoning model. Framed as plain ChatML it produced **256 tokens of `<think>` scratchpad and no answer**. Every existing assertion passed: the model loaded, generation succeeded, the reply was non-empty. A reply that is 100% visible chain-of-thought is *exactly* the shape of output that a "non-empty reply" test certifies as healthy. **The lesson is the `[OQ-71]` lesson again — assert on the artifact the user receives, not on the call succeeding.**

**A measurement that corrected the record.** The load log shows `CPU_REPACK` at 490 MB on a 1211 MB model, plus a ~500 MB compute buffer: peak resident runs ~1 GB above file size. `[OQ-64]`/`[OQ-70]` had recorded `runtime_mb: 2300` as "~900 MB too conservative" against a measured ~1.40 GB — that figure was **weights-only**. Corrected in `21-handoff.md` (×2) and `open-questions.md` before it could justify lowering an OOM guard that is doing its job.

**A hypothesis falsified.** Head-to-head on the same probe, same build: the shipping 2.77 bpw model answered well. The "low bit-rate is degrading answers" theory is **not supported** and is not claimed. Settling it needs `[OQ-74]`'s eval harness, not another anecdote.

**Harness improvement.** `repro_oq71_generates_from_a_real_model` now takes `VERDURE_REPRO_STYLE=nothink|gemma`. The prompt style is a variable of the experiment; hardcoding ChatML is what would have hidden the reasoning-model failure.

**Infrastructure fix.** CI's model-URL check was a hand-maintained copy of the URLs in `model.rs`, so every newly added model was silently exempt until someone remembered to paste it in — green precisely for the newest, least-proven URL. It now derives the list from `model.rs`, with a floor check (`< 5 URLs` fails) so a refactor that moves them turns into a failure rather than a silent no-op. All 7 URLs verified 200.

**Not tested (stated plainly):** no on-device run on iOS or Android; no speed measurement (tok/s) for the hybrid architecture on any device; the 4B is unmeasured and gated off.

### 2026-08-28 (later) — `verify-release.py`: the regression test for `[OQ-80]`

**What it prevents.** A signed, notarized `.app` inside an **unnotarized `.dmg`** — shipped in `v0.1.0` while every check we had reported success (`[OQ-80]`). Also the two search failures from `[OQ-79]` in the same session.

**Why the existing checks could not catch it.** CI asserted `spctl --assess --type execute` on the `.app` and passed. The `.dmg` was located by a *separately written* `find` in the upload step, so the assertion and the shipped artifact were never bound together. Everything asserted was true, about the wrong object. **Adding more assertions of that shape would only have produced more green.**

**Assertions now made** (21 on a macOS release):
- `.dmg` — stapled, and `spctl --assess --type open --context context:primary-signature` accepted. `--type open` matters: it is the check Gatekeeper runs on a disk image, and `--type execute` does not exercise it.
- The `.dmg` is given a **quarantine flag first**, because without it Gatekeeper does not assess at all and a broken build looks fine locally. This is what a browser download produces.
- `.app` mounted *from that `.dmg`* — signature intact, stapled, Gatekeeper-accepted, universal (`x86_64` + `arm64`).
- Notices bundled and non-trivial; the ggml copyright present.
- Updater public key compiled in; **no secrets embedded**.
- `latest.json` parses, declares `darwin-universal`, and its signature matches the `.sig` byte-for-byte (a mismatch makes every client silently reject updates).

**Positive controls — the part that addresses `[OQ-79]`.** The script begins by proving its instruments work: `spctl` must **reject** a synthesised unsigned app, and `strings` must **find** a known symbol in the binary. Without these, a disabled Gatekeeper or a mis-pathed search yields all-green and means nothing. This is `debugging.md` §7 promoted from a debugging habit to a testing rule.

**Validated against a known failure, not only a success.** Run against the bad `v0.1.0` bundle it scores **19/21**, failing precisely `dmg is stapled` and `dmg passes Gatekeeper on double-click` — and nothing else. A verifier that has only ever seen a good build is untested.

**`--url` mode** re-downloads the published bytes and runs the same contract. That is the only check with no blind spot, since it covers build, sign, staple, upload *and* host. Preferred after publishing a release.

**Edge case caught while writing it:** the first run reported `exit=0` on a failing release — that was `tail`'s status coming through a pipe, not the script's. The same trap had already made a failed local `tauri build` look clean earlier the same day. Recorded in `skills/testing.md`; check for the artifact or `${PIPESTATUS[0]}`, never a piped exit code.

### 2026-08-28 — Update consent (`updates.ts`) + the available-memory identity (`device.rs`)

**A frontend test harness now exists.** `app/` had none — `tsc` was the only automated check, and a typechecker cannot assert behaviour. Added vitest + jsdom for one reason: the update-consent rule is a **privacy promise**, and promises that are easy to break silently need a test more than features do.

**Unit tests written (5, `updates.test.ts`) — the valuable ones are negative:**
- `asks before it ever checks` — an *unanswered* question must not be read as consent. This is the one a future refactor is most likely to break, by adding a `check()` to a launch path.
- `never touches the network when the user said don't check` — asserts `checkForUpdate` is **not called**, not merely that nothing was shown.
- `checks on launch for both modes that opted in` — the positive path, both modes.
- `stays quiet when a check fails` — resolves rather than throws; a person with no wifi should not get a broken launch.
- `keeps the choice reversible and ignores a corrupted stored value` — a hand-edited `localStorage` value reads as "never asked", failing toward the private side.

**Why these are unit tests and not an integration test:** the rule lives in `updates.ts` and the network call in `bridge.ts`, deliberately separated (`knowledge/23`). Mocking the bridge is therefore testing the real seam, not a stub of one.

**`device.rs` — a test that encodes the reason, not the number.** Fixing `[OQ-77]` meant dropping `purgeable` from the available-memory sum. A test asserting a new constant would have recorded *what* changed and lost *why*. `available_ram_is_the_complement_of_used` instead asserts the identity that makes the fix correct — `free + inactive + speculative ≡ total − (active + wired + compressor)` — so re-adding the tempting `Pages purgeable` line fails with an explanation attached. Verified against live `vm_stat` first: both forms agreed to the megabyte (1895 MB), categories summing to 8190 MB on an 8 GB machine.

**Two bugs caught by looking rather than reasoning:**
1. `btn-secondary` — a class I used that exists nowhere in `styles.css`. It typechecked, built clean, and rendered as a raw white browser button. Only the screenshot showed it.
2. `[target.'cfg(desktop)'.dependencies]` in `Cargo.toml` silently matches nothing, because Cargo resolves dependencies before Tauri defines that cfg. The symptom appeared far away, as `Permission updater:default not found` from `tauri-build`.

**Verified locally:** `cargo test` 52 passed / 0 failed / 1 ignored; `npm test` (app) 5 passed; `npm run build` clean; decider 74 passed.

### 2026-08-22 — Build stamp (`vite.config.ts`, `main.ts`, `styles.css`) + corrected `approx_mb`

**Verified by looking at it**, per `design.md`'s visual self-review loop: built, served, and screenshotted. The stamp renders bottom-right as `dev 1097ec8+` at 10px / 38% opacity / `pointer-events: none`, and the full-page shot confirms it disturbs no other element. Asserted programmatically too — element present, text correct, computed `position: fixed`, and the value `dev 1097ec8+` is present in the emitted bundle (proving Vite's `define` substitution actually happened, rather than the identifier surviving as a runtime lookup that would throw).

**`approx_mb`: 51 Rust tests stayed green,** because none of them assert a specific size — they assert *relationships* (does this fit, is the list consistent). Correct behaviour-not-implementation testing, but it also means **no test would have caught the wrong numbers.** They were only found by curling each URL for `Content-Length` while verifying something unrelated.

**The gap, stated plainly:** `approx_mb` is unverifiable offline — the truth lives on Hugging Face. A test can't reach it, and the CI URL check (which could) now costs money and runs rarely. So these five numbers are correct as of 2026-08-22 and will silently rot if a publisher re-quantizes a file. The cheap fix when someone next touches the CI job: have it compare `Content-Length` against `approx_mb` rather than just checking the URL resolves — same request, one extra assertion.

**Tooling finding: `app/scripts/shots.mjs` is broken and has been since July.** It waits for `.grid .card`, the legacy home screen that stopped being the entry point in the chat-first redesign — so it times out on every run. `TEST_REPORT` has been citing it as the frontend's verification step the whole time. Screenshots for this change were taken with a direct Playwright call instead. Worth repairing or deleting; a test harness nobody can run is `debugging.md` §7 in miniature — a broken tool whose silence reads as a clean result.

### 2026-08-21 — Removing the `[OQ-71]` diagnostic models (`model.rs`, `main.ts`, `ci.yml`)

**No test changes needed — and that is the result worth recording.** Deleting two entries from `MODELS` and two from `chat_candidates` left all 51 tests green with no edits, because the model tests assert *behavior against the lists* (`entries.len() == MODELS.len()`, `opts.len() == CHAT_MODEL_CHOICES.len()`, "the preferred model is whatever the top candidate is") rather than hard-coding which models exist. That is `testing.md`'s "test behavior, not implementation" paying out: a whole category of change — adding or retiring a model — costs zero test churn.

**Gap this exposed, not yet closed:** nothing asserts that the frontend's `MODEL_TIERS` (picker copy, by model id) and the backend's `CHAT_MODEL_CHOICES` describe the same set. They drifted for a week — the diagnostics were in `MODEL_TIERS` but never in `CHAT_MODEL_CHOICES`, so they rendered as cards with copy but no size or fit line, and the `opt` lookup silently fell through to "assume it fits." The two lists live on opposite sides of the Tauri boundary with no shared schema; either could grow an entry the other lacks. Worth a contract test once `main.ts` has a harness.

**Verified locally** (the new inner loop): `cargo test` → 51 passed, 0 failed, 1 ignored; `npm run build` (`tsc` + `vite build`) clean.

### 2026-08-14 — Beta-fix round (`llm.rs` batch/reserve, `main.ts` listener lifecycle)

**Unit tests written (2 new):**
- `prompt_cap_always_leaves_room_to_answer` — across `MIN_CONTEXT`/1024/2048/`MAX_CONTEXT`, the prompt cap always reserves the reply budget and never clamps the prompt to nothing. This is the guard for the beta-reported hard failure.
- Extended `history_is_kept_newest_first_and_whole` with a large-reserve case: a bigger system instruction must cost **history**, never the reply — the regression guard for growing the prompt again.

**Why the original tests missed the batch bug (worth recording):** they cover the pure *selection* logic and never allocate a `LlamaBatch`. The failure lived in the gap between "which turns fit the window" (tested, correct) and "can the batch hold them" (untested). The lesson is that a budget test is not a capacity test — they are different limits and both need asserting.

**Not unit-testable here:** the chunked prefill itself (needs a loaded model + real batch — FFI territory per `testing.md` rule 1) and the frontend listener lifecycle (`main.ts` has no unit harness). Both are verified by CI compilation plus on-device use; the listener fix is deliberately belt-and-braces (release on every exit path *and* at session start) precisely because it cannot be asserted in a test.

**Verified:** CI green — macOS `cargo test`, `aarch64-apple-ios` and `aarch64-linux-android` cross-compile checks; `tsc` + `vite build` clean.

### 2026-08-12 — Chat memory (`llm.rs`: budget, eviction, prompt assembly)

**Unit tests written (5 new, 8 total in `llm.rs`).** All over pure logic, no model loaded:
- `context_scales_with_what_the_device_can_spare` — a roomy device with a cheap KV cache is capped by `MAX_CONTEXT` (prefill time, not memory); a tight device with the 4B's expensive cache gets a real but smaller window; zero spare never drops below `MIN_CONTEXT`; and the window never exceeds what the model was trained for.
- `spare_memory_subtracts_weights_and_headroom` — including the underflow case: a machine tighter than the headroom yields 0, not a panic.
- `history_is_kept_newest_first_and_whole` — what's retained is always a **suffix** of the conversation (recency, no holes in the middle), and a window with no room for history still answers the question.
- `prompt_carries_prior_turns_in_order` — history precedes the new message, and turn markers are balanced (the assertion that would catch a dangling half-turn).
- `gemma_prompt_carries_history_without_a_system_role` — the instruction rides the first user turn and appears **exactly once** however long the history grows (the bug a naive per-turn loop would introduce).

**Design choice that made this testable:** `first_kept_turn` takes the token-cost function as a parameter. Production passes the model's real tokenizer (an undercount would overflow the window mid-generation); tests pass a word counter, so eviction is verified deterministically without a 400 MB download.

**Edge cases caught while writing:** the Gemma branch originally used an empty-slice trick to handle "no history," which was correct but easy to break — restructured to a single `split_first` match with one exit. The reserve (`MAX_NEW_TOKENS + 128`) is subtracted *before* history selection so memory can never crowd out the reply itself.

**Verified:** `cargo test` in CI (macOS), plus `cargo check` for `aarch64-apple-ios` and `aarch64-linux-android` — the cross-target checks added earlier the same day, doing exactly the job they were added for.

### 2026-08-12 — CI coverage: all three platforms compile-checked on every push

**Change (harness, not product):** the porter job gained `cargo check --target aarch64-apple-ios`; a new `porter-android` job (Linux, NDK clang wired explicitly for bare cargo) runs `cargo check --target aarch64-linux-android`. Both are path-filtered on `app/**`/`packages/decider/**` like the existing job.

**Gap this closes:** `cfg(target_os = "ios"/"android")` code — the JNI mic + storage bridges, the iOS jetsam probes, the non-macOS `ureq` download path — previously compiled *only* when a tag build ran. A type error there merged green and failed days later at ship time (the pattern that cost Android build round 1). Now it fails on the push that introduces it.

**Deliberately check-only:** no bundling, signing, or packaging in CI — those stay tag-triggered. `cargo check` still runs the C++ build scripts (whisper.cpp/llama.cpp compile), which is where the real cross-platform risk lives.

**Verified:** YAML structure asserted locally (4 jobs, each with `runs-on`); the checks themselves verified by the push — CI is the compiler here, per `testing.md`.

### 2026-08-12 — Space recovery ([OQ-67]: partials, old-model offer, Android make-room)

**Unit tests written (1):** `list_surfaces_an_interrupted_partial_download` — writes a fake `.part`, asserts `list()` reports its exact bytes with `installed: false`, then asserts `remove()` reclaims it even though the model was never fully installed (the reclaim path users actually need). Existing `list_reports_not_installed_for_empty_dir` extended to assert `partial_bytes == 0`.

**Deliberately not unit-tested:** the Android `allocateBytes` JNI path — it's an FFI boundary into the OS (testing.md rule: tests run against wrappers, not raw FFI; the OS behavior can't be mocked meaningfully). Verified by compilation in the Android workflow + pending on-device run (Pixel 8a). The macOS `open` deep-link is a process spawn with a built-in fallback; verified manually on the next Mac build.

**Edge cases caught in design:** `getAllocatableBytes` checked before `allocateBytes` (asking beyond the ceiling always throws — the pre-check turns a generic Java exception into an honest "can clear X of Y MB" message); a pending Java exception is always cleared so it can't poison the JVM bridge; the partial-download row says "resumes if kept" so removal is an informed choice, not a surprise restart.

**Verified:** `tsc` (strict) + `vite build` clean. macOS `cargo test` runs in CI on push; Android compile-check via the `android-build-N` tag workflow.

### 2026-08-10 — Friendly error fallback (`main.ts` `friendlyError`)

**Change:** frontend-only — the three `String(err)` display sites route through a `friendlyError` helper (string/Error messages pass through; anything else falls back to a calm generic line). No new pure logic beyond a two-branch coercion, and `main.ts` has no unit harness — consistent with prior frontend-only entries, **no new unit tests**; the behavior contract is the type signature plus the backend's already-tested error phrasing (`llm.rs` `map_err` strings).

**Verified:** `tsc` (strict) + `vite build` clean. Edge case considered: the chat resume-detection regex reads the same string it did before for string errors, so the "download paused → resume" path is unchanged.

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
