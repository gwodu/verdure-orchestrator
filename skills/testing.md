# Testing

Tests are how Verdure stays correct over a decade. Treat them as architecture, not afterthought.

## The discipline

**Tests ship with the feature, not after.** A PR without tests is not done. This isn't a guideline; it's the rule. The moment "we'll add tests later" enters the conversation, complexity wins.

## The three pillars

Use all three together, each for what it's best at:

- **Unit tests** — pure logic in isolation. The hardware-tier classifier, the model selector, intent parsing. Fast, deterministic, run on every save. Lives alongside the code in `#[cfg(test)] mod tests` blocks.
- **Integration tests** — module boundaries and real interactions. Does the orchestrator correctly hand off to the backend? Does the recommendation flow produce the right output end-to-end? Lives in `tests/` at the crate root.
- **Property-based tests** (via `proptest`) — invariants over many inputs. For example: "for any valid device spec, the recommended model's memory footprint is less than the device's available RAM." Generates hundreds of inputs, automatically shrinks failing examples to minimal cases.

## Test behavior, not implementation

A test should still pass after a refactor that doesn't change observable behavior. If a test breaks because you renamed a private function or restructured an internal helper, the test was testing the wrong thing.

Concretely: don't assert on call counts, internal sequencing, or private state. Assert on what the function returns, what side effects it produces, what errors it surfaces.

## Snapshot testing for recommendation outputs (via `insta`)

The exact text Verdure produces for "8 GB MacBook + creative writing" is a reviewable artifact. Use snapshot testing to capture it. When it changes, the diff shows up in code review and someone has to consciously approve it. This catches accidental rewording, silent prompt drift, and recommendation logic regressions that pure functional tests miss.

## FFI testing — non-negotiable rules

The MVP app (`app/src-tauri`) calls into C++ via `whisper-rs` (whisper.cpp) and `llama-cpp-2` (llama.cpp). This is where memory safety bugs come from. Rust's compiler cannot help across the FFI boundary, and — observed on-device — a fault in this C++ doesn't return an error, it calls `ggml_abort()`/`SIGABRT` and takes the **whole app** down.

The rules:

1. **Reach C++ only through audited wrapper crates.** We do not hand-write `extern "C"` or raw `unsafe` FFI; `whisper-rs` and `llama-cpp-2` own the boundary (validate inputs, own memory, convert errors to `Result`). Tests run against the safe wrapper, not the raw FFI. If you ever must hand-write FFI, wrap it the same way and use a binding generator (`bindgen`/`cxx`).
2. **Process isolation is the real crash boundary.** Because a C++ abort is uncatchable in-process, the durable safety mechanism is running inference in a **separate process** so a model fault surfaces as an error, not a dead UI. This is the top robustness P0 (`knowledge/21`, `[OQ-58]`) — until it lands, we mitigate by gating model loads on *available* RAM and validating model files (`model.rs`), and by not using crash-prone paths (e.g. the GBNF grammar sampler was removed for exactly this reason).
3. **Sanitizers in CI — currently a GAP, tracked honestly.** AddressSanitizer/MemorySanitizer against FFI integration tests would be the strongest net, but CI runs none today (`ci.yml`), and we link real C++. Don't claim a safety net we don't have: the gap and the decision (add a sanitizer job vs. lean on wrapper-crates + process isolation) are tracked as `[OQ-63]`. If you add hand-written `unsafe`, a sanitizer job stops being optional.
4. **Document ownership at every FFI boundary.** Who allocates? Who frees? When? Every `unsafe` block (and every assumption about what the wrapper crate owns) needs a comment answering these.

## Coverage — chase critical paths, not numbers

Don't aim for 100% line coverage. Aim for 100% of critical paths:

- Hardware tier classification
- Model selection logic
- The FFI boundary (every call wrapped, every wrapper tested)
- Download integrity (SHA256 verification, resume logic)
- Recommendation generation

Logging utilities, debug formatters, and similar accessories do not need exhaustive tests. Spend the coverage budget on code where a bug would mislead the user or corrupt memory.

## The visual self-review loop (for UI code)

For the Phase 0 website and any future UI: after building, render it and look at it. Use Playwright or Puppeteer to screenshot the result. Open the screenshot. Compare to the intent. This is testing, even if it isn't `cargo test`. See `design.md` for the design side of this loop.

## What to do when a test fails on something legitimate

Don't disable. Don't `#[ignore]` and move on. Either:

1. The code is wrong → fix it.
2. The test was wrong → fix the test, write down what was misunderstood in `TEST_REPORT.md`.
3. The expected behavior changed → update the test deliberately, snapshot or otherwise, in the same change as the behavior change.

A skipped test rots faster than no test at all, because it creates the illusion of coverage where there is none.

## Continuous integration

**CI does not run on push (since 2026-08-22) — see "CI cost discipline" below.** It runs on pull requests and on demand. The firebreak against "I forgot to run the tests" is no longer a runner; it is the rule that you run them here, free, before you push. That trade is only honest because every check moved to this machine: `cargo test`, the frontend build, the decider and website suites, and a full `tauri build`.

Verdure is a monorepo over one brain (the shared decider, `packages/decider`), so CI lives in one `.github/workflows/ci.yml` with **path-filtered** jobs — each runs only when it or a dependency changes:

- **Decider + website** (Linux): the decider tests (the single source of truth), the website tests, and the website production build. Triggers on `packages/decider/**` or `website/**`.
- **Desktop porter** (macOS runner, PR/manual only): its remaining reason to exist is the **iOS** cross-compile check, which this Mac cannot do. It also runs the frontend build, `cargo test`, the x86_64 check, and the model-URL reachability check — all of which now duplicate free local work. Triggers on `app/**` or `packages/decider/**`.
- **App checks** (Linux, ~2 billable min): the app's frontend tests (vitest), the release-script tests (`python3 -B -m unittest discover -s app/scripts`; Python stdlib `unittest`, files named `test_*.py` next to the script they test), and `actionlint` over every workflow. Triggers on `app/src/**`, `app/scripts/**` or `.github/workflows/**`.
- **Android compile check** (Linux): `cargo check --target aarch64-linux-android`, so an Android-only `cfg` branch can't break silently. Triggers on `app/**`.
- **SDK** (Linux, `crates/**` or `bindings/**`): `cargo test --workspace` (the pure suite, no models), `cargo check -p verdure-ffi --features llama,whisper`, which compiles llama.cpp + whisper.cpp on the runner — the only place those feature builds are verified while the founder's Mac lacks the disk for a second engine build (`knowledge/26`, `[OQ-112]`) — and the binding checks' own unit tests (`python3 -B -m unittest discover -s bindings/tests`).
- **SDK for Windows** (`build-sdk-windows.yml`, Windows runner at 2×, tag `sdk-windows-N` or dispatch only): the one place the SDK meets Windows, so it is a test bench, not just a build — the pure suite on Windows, the built `verdure.dll` inspected as a file (`bindings/tests/pe_check.py`), then the zip unpacked and driven from Python and Kotlin through real models (`bindings/tests/sdk_smoke.py`, `Smoke.kt`), once online and once with the network blocked at the firewall. `sdk_smoke.py` runs on any desktop: point it at a built library and the bundles folder to repeat the customer path on the Mac.

Never merge red. When CI fails, the fix lives in the same change.

**The Mac is the compiler; CI is the gate and the signing service (since 2026-08-21/22).** Xcode CLT 16.4 + `cmake` are installed, so `cargo test` runs in seconds and `npm run tauri build` produces the whole app in ~8 minutes. *Historical note, because much of this repo was written under the opposite constraint:* from 2026-06-16 to 2026-08-21 this machine had no toolchain, nothing Rust compiled locally, and the loop really was "push to compile-check." Docs from that window still read that way — treat those passages as stale, not as instructions.

## CI cost discipline — compile continuously, package on request

CI minutes are a real budget on a private repo, and **macOS runners bill at 10×**. The rule that keeps the bill small without giving up safety:

**Compile-checking is continuous and cheap. Packaging is deliberate and expensive.** Never conflate them.

- **Compile checks** answer "does the code still build for this platform." For this Mac's own target they run **locally** (`cargo test`, free). The cross-compiles this Mac can't do (iOS) run in CI on a **pull request or `workflow_dispatch`**, never on push.
- **Packaging builds** produce an installable artifact (`.dmg`, `.ipa`, `.apk`). They are triggered by a **tag you type on purpose**, never automatically.

**Pick the narrowest trigger that answers your question.** The tags, triggers and what each one costs (measured) live in one place: `knowledge/11-platform-deployment.md`, "What each trigger costs". Read the number there rather than copying it here; copies of that table drifted apart twice.

`release-N` exists because iOS once sat three weeks behind (2026-08-12) — but it is a **shipping** tag, not a testing one. Reaching for it to test one platform buys the other two builds nobody asked for. That happened on `release-14`: macOS was wanted, and the tag also spent 57 minutes on an iOS archive that failed on an unrelated Apple account limit.

**Before a tag, verify every half that isn't the platform being built (2026-10-20).** The web-search pilot spent three TestFlight builds (~135 billable minutes) where one would have done: the first failed at link on a new native crate; the second reached the iPad before the relay it talks to was even deployed, and found with a $50 build what one `curl` from this Mac would have found for free (an apex redirect and a 404). The rule: a `testflight-N` goes out only when the server side answers the app's exact request with real results, the website is deployed, and the env vars are set — then **one build per verified change set**, never one per discovery. And when a new native dependency enters (anything that links an Apple framework), the choice between a ~40-minute dispatch and the risk of a failed ~50-minute signed build is the founder's to make, asked out loud.

**The default loop is local.** The founder's Mac has the toolchain (since 2026-08-21), so a macOS build needs no runner at all. Reach for CI when you need a *signature*, a *platform this machine isn't*, or the pre-merge gate.

**Other levers already in place** (don't undo them): `concurrency: cancel-in-progress` so three quick pushes don't queue three builds; `dorny/paths-filter` diffing against the PR base so a docs-only PR triggers nothing; `timeout-minutes` on every job so a hung runner can't bill for hours; cargo caching on every compiling job; and `retention-days: 14` on artifacts, since private-repo storage bills by GB-month.

**A known gap this creates, tracked honestly:** the macOS CI job is gated to PRs and manual runs, so on a long-lived branch that never opens a PR, the **iOS and x86_64 cross-compile checks effectively never run**. iOS can therefore break silently until a `testflight-N` tag fails. Before tagging iOS after a long gap, trigger CI manually (`workflow_dispatch`) — that path is not gated to PRs and compile-checks iOS for ~40 billable minutes instead of discovering it inside a 57-minute signed build.

When you add a new component, add its CI job in the same change that adds the component. A component without CI is unprotected. The known remaining CI gaps are the FFI sanitizer net (`[OQ-63]`) and the network-denied sandbox run the SDK promises (`knowledge/26`, privacy section). When you add a downloaded model to `model.rs`, add its URL to the porter job's resolve check in the same change (so a rotted release URL fails in CI, not on a user's machine).

## Verifying artifacts, not intentions

Three defects reached built artifacts on 2026-08-28 while every check reported
success. They shared two root causes, and both are now rules rather than lessons.

**1. A check that can pass by finding nothing needs a positive control.**
A licence search returned zero because its base path was wrong. A crate-root scan
missed `whisper.cpp/LICENSE`, one directory down. Both "succeeded". A search-based
assertion without a control cannot tell *absent* from *my instrument is broken* —
`debugging.md` §7 as a testing rule, not just a debugging one.

So: before trusting that a search found nothing, make it find something you know
is there. `verify-release.py` opens by proving `spctl` rejects an unsigned app and
that `strings` can read the binary. If Gatekeeper were disabled on the runner,
every signature check would pass and mean nothing; the control catches that.

**2. Assert on the artifact the user receives.**
CI verified the `.app` with `spctl --assess --type execute`. It passed. The `.dmg`
wrapped around it — the file a person downloads and double-clicks — shipped
**unnotarized**, and Gatekeeper rejected it. Everything asserted was true, about
the wrong object. Two independently-written `find` expressions, one for verifying
and one for uploading, with nothing forcing them to agree.

So: verify the **shipped** artifact, in the state the user gets it — for macOS
that means applying the quarantine flag first, because without it Gatekeeper does
not assess at all and a broken build looks fine locally. `--type open` for a disk
image, `--type execute` for an app; they are different checks.

**3. Prefer post-publication verification where possible.**
`verify-release.py --url <release>` re-downloads the published bytes and tests
those. It is the only check with no blind spot, because it exercises the whole
chain — build, sign, staple, upload, host. Run it after publishing a release, not
only before.

**4. Beware the exit code you are reading.** A shell pipeline exits with the
status of its *last* command, so `build | tail` reports `tail`'s success and a
failed build looks clean. This bit twice in one session, once while testing the
verifier written to prevent it. Check for the artifact, or check `${PIPESTATUS[0]}`.

**5. A text tool on another OS may not be the tool you know.** The first Windows
run's "no CRLF in the signed files" check used `grep $'\r'` in Git Bash, whose
grep strips carriage returns from text files before matching: it could not have
failed. Its control (a file known to contain CRLF must be seen to) failed in 18
seconds and said so. When a check is about bytes, read bytes (Python, `cmp`),
and keep the control — rule 1 is what turned a silent no-op into a red step.

## Self-check before declaring a feature done

- Are there unit tests for the pure logic?
- Are there integration tests for the boundaries this feature crosses?
- If this is FFI code: are the sanitizers green?
- If this is UI: has Claude actually looked at the screenshot?
- If a future Claude refactors the internals, will these tests still pass?
- Has `TEST_REPORT.md` been updated with what was tested and any edge cases caught?
- Are `cargo test` (app), `cargo test --workspace` (SDK), the frontend build/tests and the decider suite green **locally**, before pushing? (A push runs no CI.) Is CI green on the PR before merge?
