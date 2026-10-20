# Documentation

Documentation isn't a phase. It's a habit that runs through every task. Without it, the knowledge base goes stale and a future Claude has to re-derive what we already know.

## The principle

**If a decision matters enough to make, it matters enough to write down.**

The corollary: silent divergence from the documented system is worse than no system. If you decide to do something different from what `../knowledge/` says, update `../knowledge/` in the same change. Don't leave the docs lying.

## Open questions — the [OQ-XX] discipline

Whenever you encounter an unanswered question during any task, your first move is to add it to `../knowledge/open-questions.md` with a stable `[OQ-XX]` reference code. Do not guess. Do not silently proceed on an assumption.

The discipline:

1. **Stable codes.** Open questions get a permanent ID like `[OQ-27]`. The number is never reused. Even when resolved, the code stays — it might be referenced from history.
2. **Mirrored placement.** Each open question appears in two places: inline at the bottom of the originating knowledge file ("Open Questions" section), and centrally in `../knowledge/open-questions.md` with full context.
3. **Resolution, not deletion.** When a question is resolved, move it to the "Resolved" section in `open-questions.md` with a one-line note about how it was resolved. Update the originating file. Don't just delete it.
4. **Source noted.** Every entry in `open-questions.md` notes its originating file in parentheses, e.g. "(From `02-hardware-tiers.md`.)"

If a knowledge file has a `**[GAP — needs research]**` marker, it must correspond to an `[OQ-XX]` entry. See `research.md` for what to do with gaps.

## Status markers

Every knowledge file opens with one of:

- `**Status:** ✅ Settled` — researched, decided, unlikely to change
- `**Status:** 🟡 Provisional` — current best understanding, may revise as we learn
- `**Status:** 🔴 Open` — actively unresolved, blocking decisions downstream
- `**Status:** 🟢 Current` — a living file describing the present state (the handoff, the pivot and north-star files); overwritten as things change rather than settled once

When the status changes (e.g., a Provisional file becomes Settled after validation), update the marker in the same change.

## Cross-references — always explicit

When a file references another, say so directly: "see `02-hardware-tiers.md` for tier definitions." Never assume a reader (human or future Claude) has loaded another file. This applies in both directions — knowledge files referencing each other, skill files referencing knowledge files, code comments referencing knowledge files when relevant.

A code module implementing a knowledge-file concept should reference back: `// Tier mapping defined in knowledge/02-hardware-tiers.md`.

## Feature log discipline

Every feature added gets an entry in `FEATURE_LOG.md` with:

- What was built
- What problem it solves (a sentence — connects to a user need or knowledge-file decision)
- Test results (passing? edge cases caught?)
- User impact statement (from `design.md`)

If a feature was built without a problem worth stating, ask why it was built.

## Test report discipline

Every change that touches code updates `TEST_REPORT.md`:

- Unit tests written
- Integration tests written
- Edge cases caught
- Failures encountered and fixed

This is the participatory layer in the four-ways-of-knowing structure. Without it, the testing skill becomes ritual without memory.

## Living state: history vs. handoff

Two docs track "where the project is," and they must not overlap — overlap is the only way they drift. Each owns one role:

- **`KNOWLEDGE.md` changelog** = append-only **history**: dated events, "what happened when." Never edited retroactively.
- **`knowledge/21-handoff.md`** = current **state + next steps**: "where are we now / what to do next." Overwritten as the build moves; it does *not* narrate chronology — it links back to the changelog for how we got here.

When you finish a unit of work, do both: append one dated line to the changelog (history), and refresh the handoff's state + next-steps (current). If you catch yourself writing chronology into the handoff, that belongs in the changelog with a pointer instead.

## Conventions across all files

- Plain prose. Tables only where they meaningfully aid scanning (the model matrix, the license audit). Avoid bullets where prose reads better.
- Minimal decorative emoji beyond the three status markers.
- Files self-contained — write for a future Claude opening this in a fresh session.
- Changelog at the bottom of major files (root `KNOWLEDGE.md`, anything substantially revised).

## Self-check before declaring documentation done

- Are all `[OQ-XX]` codes I created mirrored in `open-questions.md`?
- If I appended a changelog entry, did I refresh `21-handoff.md` in the same change (or verify it is still accurate)? History without a state refresh is exactly how the handoff went five weeks stale (found 2026-08-10) — the changelog and the handoff move together or not at all.
- Are all gap markers I left referenced by an open question?
- Does every cross-reference point to a file that exists with the exact filename?
- Did I update the originating file's "Open Questions" section if I added one?
- Did I update `FEATURE_LOG.md` if this was a feature?
