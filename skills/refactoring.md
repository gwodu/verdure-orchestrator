# Refactoring

Software at decade-scale isn't written once. It's continuously rewritten while still working. Refactoring is that discipline.

## The premise

Code rots if untouched. New requirements warp old abstractions, accumulated decisions stop being optimal, and "we'll clean it up later" never arrives. The only defense is constant, small, safe rewrites — refactoring as a continuous practice, not a quarterly event.

## The non-negotiable precondition: tests are green

You cannot refactor code that isn't tested. Without tests, you aren't refactoring — you're rewriting and hoping. Every refactoring move follows the loop:

1. Tests green.
2. Apply one named refactoring move.
3. Tests still green.
4. Commit.
5. Repeat.

If step 3 fails, revert immediately. Do not try to fix forward. The point of small steps is that each step is small enough to undo cleanly.

See `testing.md` — the test suite is the harness that makes refactoring safe.

## Named moves, not vibes

Refactoring has vocabulary. Use it. From Fowler's catalog, the moves most relevant for Verdure:

- **Extract Function** — pull a coherent block of code into its own function with a clear name
- **Inline Function** — fold a trivial wrapper back into its only caller
- **Rename** — names that no longer describe their referent are debt; pay it
- **Replace Conditional with Polymorphism** — when a long `match` on a type tag appears, the types want to be traits
- **Move Function / Move Field** — when a function lives in the wrong module, move it before adding to the wrong module
- **Replace Magic Literal with Named Constant** — `8` becomes `MIN_RAM_GB_FOR_TIER_L2`
- **Extract Trait** — when two structs grow parallel methods, the shared behavior wants a trait

The point of names: they make refactoring discussable in code review. "I'd extract function on lines 40-58" is communication; "the code's kind of messy here" isn't.

## Smells as triggers

Don't refactor on aesthetics. Refactor when a smell tells you the design is decaying. Smells worth acting on:

- **Long function** that does five unrelated things — split it (but see `architecture.md` on the Ousterhout-vs-Clean-Code tension; don't fragment for its own sake)
- **Shotgun surgery** — one change requires edits in many places (change amplification — the design decision leaked)
- **Primitive obsession** — passing `(u8, String, bool)` everywhere instead of a named domain type — introduce the type
- **Repeated literal** — the same magic number in three places — extract a constant
- **Long parameter list** — usually means several parameters belong together as a struct
- **Speculative generality** — abstractions added "in case we need them later" without a concrete use case — delete them
- **Dead code** — unreachable or unused — delete it

## Small steps, always working

The mistake is the heroic refactor: a week-long branch that rewrites half the codebase. Don't. Refactor in steps small enough that the codebase compiles and passes tests at every commit. If the refactor is genuinely large, do it in a series of small PRs each independently safe.

## When NOT to refactor

- During a debugging session you don't fully understand yet
- Right before a release deadline, on code unrelated to the release
- On code you'll be deleting soon
- On code nobody touches, just to clean it up cosmetically — the change can introduce bugs the existing stability doesn't have

Refactor when you're about to add a feature in that area, when fixing a bug exposes a structural problem, or when reading code surfaces a smell worth addressing before it grows.

## Self-check before declaring a refactor done

- Did the tests stay green through every step?
- Is the public interface still the same (or, if intentionally changed, is the change documented)?
- Has the smell that triggered this been resolved, or just moved?
- Is `FEATURE_LOG.md` updated if this changed any user-visible behavior?
