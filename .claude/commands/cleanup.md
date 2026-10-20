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

- **Everything verifies locally, and a push runs no CI.** `cd app/src-tauri &&
  cargo test`, `cd app && npx tsc --noEmit && npx vitest run`, `npm test
  --workspace @verdure/decider`, and `python3 -B -m unittest discover -s
  app/scripts` — all green before each commit (`skills/testing.md`, "Continuous
  integration"). Only an iOS/Android-only `cfg` branch can't run here: keep that
  logic behind a pure function with the platform as a parameter so it *can*.
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
