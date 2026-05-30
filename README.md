# Verdure

The orchestration layer for local AI. Given any device, Verdure detects what it can run, picks the right model, and gives the user one-click access to AI on their own machine. The moat is **intent-first orchestration** — starting from "what are you trying to do?" instead of "browse the model library."

This is a private monorepo with two surfaces over one shared brain.

## Layout

| Path | What it is |
|------|------------|
| `knowledge/` | The propositional knowledge base — the "brain." Shared source of truth for both surfaces. Start at `KNOWLEDGE.md`. |
| `skills/` | Procedural skills — *how* we work (architecture, design, testing, …). |
| `website/` | **Phase 0** — the recommendation website (Next.js + TypeScript), deployed at verdureai.com. The brain operationalized as a recommender. |
| `src/`, `tests/` | **Phase 1** — the Rust orchestration binary (scaffold). |
| `CLAUDE.md` | Who Claude is on this project, and the repository layout in full. |
| `FEATURE_LOG.md`, `TEST_REPORT.md` | Project-wide living logs, organized by component. |

The website and the binary are two deliveries of the same decision engine. The recommendation logic lives once, as knowledge in `knowledge/`, and the code follows it.

## Working in this repo

- **Website:** `cd website && npm install && npm run dev` (tests: `npm test`). See `website/SETUP_SHEETS.md` for the waitlist backend.
- **Binary:** `cargo build` / `cargo test` from the root.

Before changing anything, read `KNOWLEDGE.md`, then load the relevant `knowledge/` and `skills/` files. Knowledge and code stay in lockstep — never let them diverge silently.
