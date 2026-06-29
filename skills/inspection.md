# Inspection

Inspection is how Verdure checks itself against its own stated principles — *before* a redesign, a big addition, or whenever you suspect the project has drifted from what `../CLAUDE.md` and `../knowledge/` say it should be. It is not a bug hunt and not a code review. It asks one question: **does the project still match the intentions it has written down?**

## When to run one

- Before a redesign or a large change — confirm the foundation is solid before building on it.
- When the same kind of mistake recurs across sessions — a sign a guideline isn't being loaded.
- Periodically, as hygiene — drift accumulates silently; it is cheapest to catch early.

## What you inspect against — the rubric

Verdure's structure rests on **four modes of knowing**, each with one home. An inspection checks that each home is complete, current, and not leaking into the others:

1. **Propositional** (facts, research, decisions) → `../knowledge/`. Is each fact in exactly one place? Is it current? Does the code that depends on it still match?
2. **Perspectival** (who Claude is, the stance) → `../CLAUDE.md`. Is the identity and the operational ground truth present and honest?
3. **Procedural** (how to do recurring work) → `../skills/`. Is every task type routed to a skill? Any orphaned skill? Any skill describing a world that no longer exists?
4. **Participatory** (how we actually engage the code) → testing, CI, and the living logs (`FEATURE_LOG.md`, `TEST_REPORT.md`). Is the discipline practiced, or only asserted?

Layered over the four modes, two non-negotiables:

- **The truck-driver test.** Every user-facing surface must be usable by someone with no technical knowledge — jargon-free, one obvious action, zero configuration. Judge UI and copy against `design.md` and `copywriting.md`.
- **The coding principles** (from `../CLAUDE.md`, "What you do not do"): no over-engineering, no abstraction without a concrete *present* reason, no premature optimization, never ship without tests, mark provisional/unknown honestly, never diverge from the knowledge base silently.

## What you are hunting for

- **Drift** — a file, skill, or surface that describes a superseded direction (Verdure's 2026-06-18 pivot is the recurring fault line). Drift is worst when the index still routes readers *into* the stale file without warning.
- **Redundancy** — one fact stated in two places that can diverge. Name the single owner; make the others reference it.
- **Single-source-of-truth violations** — logic duplicated between `knowledge/` and code, or forked across surfaces, so the "one decision" can differ.
- **Over-engineering** — a seam, trait, enum, or exported function with no present caller. Speculative generality.
- **Drift fingerprints** — evidence a past session skipped a guideline because it wasn't loaded: an undocumented decision, a missing test on pure logic, an abstraction the skills forbid, a `must` in the knowledge base the code ignores.

## How to run it

1. **Fix the rubric first.** State explicitly what you are inspecting against (the list above), so findings are measured, not impressionistic.
2. **Fan out, don't serialize.** Inspect each layer with a fresh reader so no single pass has to hold the whole project at once: one for the knowledge base, one for skills + the participatory machinery, one per code surface (Rust backend, frontend UX, the shared decider). Each reader gets the rubric and a bounded scope, and **reads the relevant skill before judging by it** — judge the code by the project's own standard, not a generic one.
3. **Report, do not edit.** An inspection produces findings, never changes. Editing mid-inspection corrupts the picture and conflates "what's wrong" with "what I changed."
4. **Synthesize across layers.** The most important findings are the ones that recur in every layer from a single root cause (e.g. an undischarged pivot showing up in knowledge, skills, code, and UX at once). Lead with those.

## The finding format

Every finding is one row, evidence-cited and actionable:

**AREA | FINDING (one line) | PRINCIPLE or MODE violated | SEVERITY (high/med/low) | EVIDENCE (`file:line`) | RECOMMENDATION (concrete, minimal)**

Rules for good findings:

- **Cite evidence.** A finding without a `file:line` or filename is an opinion, not a finding.
- **Prefer fewer high-confidence structural findings over many nitpicks.** Severity reflects impact on a user or on a future session, not how easy it is to fix.
- **Recommend the minimal fix.** Inspection serves simplification; a recommendation that adds more structure than it removes needs explicit justification.
- **Note what's healthy too.** Confirming a principle holds is as valuable as finding a violation — it tells the next session what not to re-litigate.

## After the inspection

Findings feed work, but the work is a separate, deliberate step — ideally run *through* this skill's recommendations one at a time, each in its own change with its own living-log entry (`documentation.md`). Resolve the structural, single-source-of-truth, and drift findings first: they are the ones that mislead the next session. A finding worth recording but not fixing now becomes an `[OQ-XX]` (`documentation.md`), so it is tracked rather than forgotten.

## Self-check before declaring an inspection done

- Did I inspect all four modes, plus the truck-driver test and the coding principles?
- Does every finding cite evidence and name the principle or mode it violates?
- Did I lead with the root-cause findings that recur across layers, not the easiest nitpicks?
- Did I resist editing — is this a report, not a diff?
- Did I record what's healthy, so the next session doesn't re-inspect settled ground?
