---
description: Audit Verdure against its own principles — produces findings, changes nothing
argument-hint: [scope — e.g. "rust backend", "knowledge base", or empty for all layers]
allowed-tools: Read, Grep, Glob, Bash, Task, TodoWrite
---

Run an inspection of Verdure against its own stated principles.

**Scope:** $1 (if empty, inspect all layers.)

Follow the procedure in the skill below exactly. It is the rubric — do not
substitute a generic code-review standard for Verdure's own.

@skills/inspection.md

## Context to load first

- @KNOWLEDGE.md — the orientation index and changelog (what happened when)
- @knowledge/21-handoff.md — current working state, known debt, next steps
- @skills/README.md — the canonical skill routing table

Read the knowledge files relevant to the scope before judging anything by them.
Where the scope touches code, read the skill that governs that code first
(`architecture.md` for Rust, `design.md` for UI, `copywriting.md` for user-facing
text) — judge by the project's standard, not a generic one.

## Hard rules for this command

1. **Report, do not edit.** This command produces findings only. If you spot a
   one-line fix, record it as a finding — do not apply it. Editing mid-inspection
   conflates "what's wrong" with "what I changed."
2. **Cite evidence.** Every finding needs a `file:line` or filename. A finding
   without evidence is an opinion.
3. **Lead with root causes.** The findings that recur across several layers from
   a single cause outrank the easy nitpicks.
4. **Note what's healthy.** Confirming a principle holds tells the next session
   what not to re-litigate.

## Output

Use the finding format from the skill:

**AREA | FINDING | PRINCIPLE or MODE violated | SEVERITY | EVIDENCE (`file:line`) | RECOMMENDATION**

Close with:
- **Healthy** — principles confirmed holding, so the next session skips them.
- **Queue** — findings ordered for `/cleanup`, structural and drift findings first.
- **Defer** — anything worth recording but not fixing now, phrased as a proposed
  `[OQ-XX]` entry for `knowledge/open-questions.md`.
