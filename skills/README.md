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
- `inspection.md` — auditing the project against its own principles: the four modes, the truck-driver test, drift

## Quick routing

This table is the canonical routing. `../CLAUDE.md`'s Skills section summarizes it for the always-loaded context; when they differ, this table wins — keep them in sync.

- Writing or editing Rust code → `architecture` + `testing` + (`refactoring` if touching existing code)
- Writing or editing UI → `design` + `testing`
- Writing any user-facing text (website, app, emails, error messages) → `copywriting` + `design`
- Updating knowledge files → `documentation`
- Investigating an open question → `research` + `documentation`
- Code review / cleanup pass → `refactoring` + `testing`
- Auditing architecture/principles, or pre-redesign alignment check → `inspection`
