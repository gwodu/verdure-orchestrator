---
description: Work the cleanup queue — tests-locked refactors, one finding at a time
argument-hint: [what to clean, or "next" to take the top item from the handoff debt list]
---

Do a cleanup pass on Verdure.

**Target:** $1 (if empty or "next", take the top unresolved item from the
"Gaps / debt" and "Standing inputs" sections of `knowledge/21-handoff.md`.)

This is the treatment step that follows `/inspect`'s diagnosis. Follow both
skills below — `refactoring.md` for the moves, `testing.md` for the harness that
makes them safe.

@skills/refactoring.md
@skills/testing.md

## Context to load first

- @knowledge/21-handoff.md — the standing debt list and current working state
- @CLAUDE.md — the coding principles under "What you do not do"

## The loop — non-negotiable

Per `refactoring.md`, every move follows:

1. Tests green.
2. Apply **one named move** (Extract Function, Rename, Replace Magic Literal…).
3. Tests still green.
4. Commit.
5. Repeat.

If step 3 fails, **revert — do not fix forward.** The point of small steps is
that each is small enough to undo cleanly.

Name the move you are making before you make it. "I'd Extract Function on
`live.rs:180-210`" is communication; "this is messy" isn't.

## Verdure-specific constraints

- **Rust cannot be compiled or tested on this Mac** (no Xcode CLT). That is
  expected, not a blocker. Verify Rust changes by pushing the branch — the
  `porter` job in `.github/workflows/ci.yml` compiles and tests it. Keep pure
  logic (parsers, prompt formatting, model gating) unit-testable so CI exercises
  it. Never report a Rust change as "unverifiable."
- The JS side (`packages/decider`, `app/src`) **does** run locally — `npm test`
  and `tsc`/`vite` before claiming green.
- `@verdure/decider` is the single source of truth for recommendation logic, and
  `knowledge/` is the *why*. If a cleanup changes recommendation behavior, the
  knowledge file changes **first** and the code follows — never the reverse.

## Before declaring done

Per the skills' self-checks:

- Did tests stay green through every step?
- Is the smell that triggered this resolved, or just relocated?
- Is the public interface unchanged, or is the change documented?
- Are `FEATURE_LOG.md` and `TEST_REPORT.md` updated? (`FEATURE_LOG` for
  user-visible behavior; `TEST_REPORT` for tests written and edge cases caught.)
- Did this make Verdure **simpler** for the user? If more complex, justify it
  explicitly.

Finally: state plainly what you cleaned, what you deliberately left, and why.
