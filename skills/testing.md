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

Verdure's `infrastructure/` crate calls into C++ via `llama-cpp-2`. This is where memory safety bugs come from. Rust's compiler cannot help across the FFI boundary.

The rules:

1. **All `unsafe` is wrapped.** Every FFI call sits inside a safe Rust abstraction that validates inputs, owns memory, and converts C error codes to `Result`. Tests run against the safe wrapper, not the raw FFI.
2. **Use binding generators.** Don't hand-write `extern "C"` blocks when `bindgen` or `cxx` exists. CXX in particular is designed to make whole categories of FFI bugs impossible by controlling both sides of the boundary.
3. **Sanitizers in CI.** AddressSanitizer and MemorySanitizer must run against FFI integration tests in the CI pipeline. Shipping FFI code without sanitizer coverage is the single highest-risk omission Verdure could make. Memory corruption from llama.cpp will be silent until it isn't.
4. **Document ownership at every FFI boundary.** Who allocates? Who frees? When? Every `unsafe` block needs a comment answering these.

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

## Self-check before declaring a feature done

- Are there unit tests for the pure logic?
- Are there integration tests for the boundaries this feature crosses?
- If this is FFI code: are the sanitizers green?
- If this is UI: has Claude actually looked at the screenshot?
- If a future Claude refactors the internals, will these tests still pass?
- Has `TEST_REPORT.md` been updated with what was tested and any edge cases caught?
