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

Tests and the build run on every push and every pull request. CI is the firebreak that catches the failure mode "I forgot to run tests before pushing" — exactly the mode the discipline above is designed to prevent and that CI enforces structurally.

Verdure is a monorepo over one brain (the shared decider, `packages/decider`), so CI lives in one `.github/workflows/ci.yml` with **path-filtered** jobs — each runs only when it or a dependency changes:

- **Decider + website** (Linux): the decider tests (the single source of truth), the website tests, and the website production build. Triggers on `packages/decider/**` or `website/**`.
- **Desktop porter** (macOS runner): the MVP. **This is where the app gets compiled.** It runs the frontend typecheck+build, then `cargo test` (which compiles the Tauri Rust backend), then `cargo check --target x86_64-apple-darwin` to cross-compile for the founder's Intel Mac (the pivot's main C++/whisper risk), then a check that every model-download URL in `model.rs` still resolves. Triggers on `app/**` or `packages/decider/**`.
- The long-term Rust core (`src/`/`tests/`) has **no job yet** — it's a scaffold with a malformed manifest; it gets one when it becomes real.

Never merge red. When CI fails, the fix lives in the same change.

**CI is the gate, not the compiler (since 2026-08-21).** Xcode CLT 16.4 + `cmake` are installed on the founder's Mac: `cd app/src-tauri && cargo test` runs the suite locally in seconds, and that is now the inner loop. What follows described the era when it wasn't possible — kept because it still explains the CI design: The Tauri backend needs the macOS toolchain (Xcode Command Line Tools) to build, so a dev machine without them can't `cargo build`/`cargo test` the porter locally — that's expected, not a blocker. Pushing a branch runs the macOS job, which compiles `app/src-tauri` and runs the tests for you. For porter changes the loop is therefore **push to compile-check**; write the pure logic (parsers, prompt formatting, model gating) to be unit-testable so it's actually exercised there. Don't report "can't build locally" as "unverifiable" — it's verified in CI on push.

## CI cost discipline — compile continuously, package on request

CI minutes are a real budget on a private repo, and **macOS runners bill at 10×**. The rule that keeps the bill small without giving up safety:

**Compile-checking is continuous and cheap. Packaging is deliberate and expensive.** Never conflate them.

- **Compile checks** answer "does the code still build for this platform." They run on push, on Linux where possible, and cost ~1–2 billable minutes. Keep them.
- **Packaging builds** produce an installable artifact (`.dmg`, `.ipa`, `.apk`). They are triggered by a **tag you type on purpose**, never automatically. Measured on `release-14` (2026-08-22): macOS `.dmg` **66 billable min**, TestFlight **57**, Android `.apk` **19** (before caching). One ship-everywhere tag ≈ **144 billable minutes**.

**Pick the narrowest trigger that answers your question:**

| What you need | How | Billable |
|---|---|---|
| Test macOS yourself | **Build locally** — `cd app && npm run tauri build` | **0** |
| Rust/logic correctness | **Locally** — `cd app/src-tauri && cargo test` | **0** |
| Signed + notarized macOS for someone else | tag `app-build-N` | ~66 |
| Test on iPad | tag `testflight-N` | ~57 |
| Test on Android | tag `android-build-N` | ~8 cached |
| Genuinely ship everywhere | tag `release-N` | ~144 |

`release-N` exists because iOS once sat three weeks behind (2026-08-12) — but it is a **shipping** tag, not a testing one. Reaching for it to test one platform buys the other two builds nobody asked for. That happened on `release-14`: macOS was wanted, and the tag also spent 57 minutes on an iOS archive that failed on an unrelated Apple account limit.

**The default loop is local.** The founder's Mac has the toolchain (since 2026-08-21), so a macOS build needs no runner at all. Reach for CI when you need a *signature*, a *platform this machine isn't*, or the pre-merge gate.

**Other levers already in place** (don't undo them): `concurrency: cancel-in-progress` so three quick pushes don't queue three builds; `dorny/paths-filter` diffing the actual push so a docs commit triggers nothing; `timeout-minutes` on every job so a hung runner can't bill for hours; cargo caching on every compiling job; and `retention-days: 14` on artifacts, since private-repo storage bills by GB-month.

**A known gap this creates, tracked honestly:** the macOS CI job is gated to PRs and manual runs, so on a long-lived branch that never opens a PR, the **iOS and x86_64 cross-compile checks effectively never run**. iOS can therefore break silently until a `testflight-N` tag fails. Before tagging iOS after a long gap, trigger CI manually (`workflow_dispatch`) — that path is not gated to PRs and compile-checks iOS for ~40 billable minutes instead of discovering it inside a 57-minute signed build.

When you add a new component, add its CI job in the same change that adds the component. A component without CI is unprotected. The known remaining CI gaps are the FFI sanitizer net (`[OQ-63]`) and a job for the long-term `src/` core once it's real. When you add a downloaded model to `model.rs`, add its URL to the porter job's resolve check in the same change (so a rotted release URL fails in CI, not on a user's machine).

## Self-check before declaring a feature done

- Are there unit tests for the pure logic?
- Are there integration tests for the boundaries this feature crosses?
- If this is FFI code: are the sanitizers green?
- If this is UI: has Claude actually looked at the screenshot?
- If a future Claude refactors the internals, will these tests still pass?
- Has `TEST_REPORT.md` been updated with what was tested and any edge cases caught?
- Is CI green on the PR? (For porter/Rust changes, "green" means pushed — CI is the porter's compiler; see "Continuous integration" above.)
