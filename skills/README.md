# Skills

Procedural skills — *how* Claude works on Verdure. These complement the knowledge files in `../knowledge/` (the *what*) and `../CLAUDE.md` (the *who*).

Before starting any task, identify which skills apply and load them. A typical task touches more than one.

## Skill index

- `architecture.md` — system structure: deep modules, hexagonal seams, domain modeling
- `design.md` — human-centered design: affordances, user-impact, visual self-review
- `testing.md` — test discipline: three pillars, FFI safety, behavior over implementation
- `refactoring.md` — continuous improvement: named moves, smell-driven, tests-locked
- `documentation.md` — knowledge maintenance: open questions, feature logs, cross-references
- `research.md` — honest inquiry: filling gaps without inventing content
- `copywriting.md` — user-facing text: conversion through trust, awareness stages, honest specificity
- `debugging.md` — locating a defect that already escaped: Agans' nine rules as the spine, with what this codebase has actually cost us under each — verify the artifact not the source, differential diagnosis against upstream's own binary, bisecting over CI artifacts, the FFI boundary, and telling a broken harness apart from a merely weak model
- `inspection.md` — auditing the project against its own principles: the four modes, the truck-driver test, drift

## Quick routing

This table is the canonical routing. `../CLAUDE.md`'s Skills section summarizes it for the always-loaded context; when they differ, this table wins — keep them in sync.

- Writing or editing Rust code → `architecture` + `testing` + (`refactoring` if touching existing code); SDK crates also `knowledge/26` (the rules for adding to the crates)
- Writing or editing UI → `design` + `testing`
- Writing any user-facing text (website, app, emails, error messages) → `copywriting` + `design`
- Updating knowledge files → `documentation`
- Investigating an open question → `research` + `documentation`
- **Something behaves impossibly / a regression appeared → `debugging`** (+ `testing` once the cause is known)
- Code review / cleanup pass → `refactoring` + `testing` (shortcut: `/cleanup`)
- Auditing architecture/principles, or pre-redesign alignment check → `inspection` (shortcut: `/inspect`)
- Editing CI workflows, build/release scripts (`app/scripts/`), signing or store publishing → `testing` ("CI cost discipline", "Verifying artifacts, not intentions") + `knowledge/11` (the pipeline map, its costs, and the runbooks)

## Slash-command shortcuts

Two of the rows above (cleanup, inspection) have a Claude Code command in `../.claude/commands/`, so the
routing runs on demand instead of relying on a session remembering to load the
skill. The command **inlines the skill's full text** (`@skills/…`) rather than
copying it — the skill file stays the single source of truth, and editing it
updates the command automatically.

- `/inspect [scope]` → `inspection.md`. Produces findings, changes nothing. Its
  allowed-tools lean read-only (Bash stays, for git evidence), so "Report, do not
  edit" is the command's hard rule, discouraged — not made impossible — by the
  tool list. Closes with Healthy / Queue / Defer sections.
- `/cleanup [target]` → `refactoring.md` + `testing.md`. Works the queue one named
  move at a time, tests green between each. Bare or `next` takes the top item from
  `../knowledge/21-handoff.md`'s debt list.

The pairing matters: `inspection.md` diagnoses and refuses to edit, `refactoring.md`
treats. Run `/inspect` first, then `/cleanup` against its Queue.

Adding a command for another row? Point it at the skill with `@`, never restate the
skill's content — a second copy is a single-source-of-truth violation the next
inspection will find.
