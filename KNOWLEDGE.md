# Verdure — KNOWLEDGE.md

Verdure is the orchestration layer for local AI. It detects what a device can run, picks the right model, configures the right backend, and gives the user one-click access to AI on their own machine. The competitive moat is intent-first orchestration: starting with "what are you trying to do?" rather than "browse the model library."

This file is an orientation index, not a content store. Knowledge lives in the `knowledge/` subfolder as numbered, modular files. Claude should read this file at the start of every session, then load only the knowledge files relevant to the current task.

This knowledge base is shared across the monorepo: it informs both the Phase 1 Rust binary (`src/`) and the Phase 0 website (`website/`). The website's recommendation engine is this knowledge operationalized in code — see `CLAUDE.md` ("Repository layout").

For perspective — *who Claude is while working on this project* — see `CLAUDE.md`.

## Knowledge index

★ marks the most-referenced files (the matching brain).

- `knowledge/01-thesis.md` — what Verdure is, the moat, the seven decisions, phasing strategy
- `knowledge/02-hardware-tiers.md` ★ — device-to-tier mapping (L1–L3 laptops, P1–P3 phones)
- `knowledge/03-model-matrix.md` ★ — tier-to-**text**-model recommendation matrix, intent-shifts-within-tier
- `knowledge/12-multimodal-models.md` ★ — audio / image / video model matrix, graphics-bound feasibility, honest "not-local-yet" verdicts
- `knowledge/13-use-case-demand.md` ★ — the demand side: what people actually use (local) AI for, grounded in research → the Screen 1 intent cards
- `knowledge/04-quantization.md` — pre-quantized only, GGUF, Q4_K_M default
- `knowledge/05-website-spec.md` — Phase 0 recommendation website full spec
- `knowledge/06-privacy-principles.md` — ask don't sniff, local by default, no telemetry
- `knowledge/07-runtime-backends.md` — llama.cpp, ExecuTorch, MLC, pure-Rust paths
- `knowledge/08-rust-ecosystem.md` — llama-cpp-2, candle, mistral.rs, FFI strategy
- `knowledge/09-licensing.md` — model license audit
- `knowledge/10-distribution.md` — Hugging Face source, CDN considerations
- `knowledge/11-platform-deployment.md` — code signing, app stores, OS specifics
- `knowledge/14-architecture.md` ★ — the structural spine: pure core + replaceable adapters (seams), the one-click pipeline, the Phase 0↔1 boundary, the Tool layer. Composes 07/08/10/11.
- `knowledge/15-port-catalog.md` ★ — the supply-side database: portable free/open-source local tools by use case, the two port classes, and the "port AnythingLLM, don't build a shell" decision.
- `knowledge/16-growth-asset.md` — the growth / network-effect roadmap (Eva Lau's ASSET framework): atomic unit, seed supply, scale demand, network effect, proprietary insights.
- `knowledge/open-questions.md` — master open-questions list, organized by urgency

## Where to start by task

- Recommending a text model for a specific device → `02` + `03`
- Recommending an audio / image / video model (or deciding it isn't local-feasible) → `02` + `12`
- Designing the intent layer / deciding what use cases to surface → `13` (demand) + `12`/`03` (supply)
- Designing or modifying the Phase 0 website → `05` + `06`
- Writing Rust binary code → `07` + `08`
- License or distribution questions → `09` + `10`
- Platform-specific deployment → `11`
- How the binary is structured / adding a device or engine / the one-click pipeline → `14`
- Which existing tool to port for a use case (the local-tool database) → `15` (+ `13` demand, `09` licensing)
- Growth / how the network effect compounds / where to seed supply → `16`
- Tracking what is still open → `open-questions.md`

## Conventions

Each knowledge file opens with a status marker (✅ Settled, 🟡 Provisional, 🔴 Open) and ends with an "Open Questions" section where relevant. Open questions also appear in the centralized `open-questions.md` with stable `[OQ-XX]` reference codes. Cross-references between files are always explicit.

## Changelog
- 2026-06-16 — adopted the **ASSET growth roadmap** (Eva Lau / Two Small Fish) as `16-growth-asset.md`. Atomic unit = a Tool (esp. a user-creatable **persona**); seed supply from the open-source ecosystem (Verdure's "20,000 public-domain books") via the catalog manifest; persona library = the network-effect engine; proprietary insight = the "what local AI people use, by device" map (opt-in only). `[OQ-48]`.
- 2026-06-16 — designed the **composition layer**: user-directed, deterministic 2-tool chaining (transcription → writing) via typed I/O ports + a 3-field pipe contract; bright line "wire existing I/O, never build a missing surface"; hard-capped at 2 nodes (not n8n); deferred after the single-tool MVP. `14` + `15` (pipeability) + `[OQ-47]`.
- 2026-06-15 — added `15-port-catalog.md` (the supply-side local-tool database) + the two port classes; decided to **port AnythingLLM/Jan as the configurable shell rather than build one**; aligned `14`'s Tool layer to "port, never build." Seed from OSS lists (TAAFT is ~95% cloud). `[OQ-46]`.
- 2026-06-15 — adopted "the decider that finishes the job": a deferred, data-driven, **plug-n-play Tool layer** (configure & launch existing local tools / the device's own model, **never build bespoke apps**) — the local answer to "There's An AI For That". First Mac tools = writing assistant + doc-RAG. In `01`, `14` (Tool layer), `[OQ-45]`.
- 2026-06-15 — Apple-stack integration (WWDC 2026), distributed by concern across `01/03/07/11/14`: MLX supersedes MLC, the zero-download/resident-model recommendation, the iOS daemon→thin-decider pivot, and Apple Silicon reframed as the **beachhead** (mission still emerging-market). Resolved `[OQ-14]`/`[OQ-26]`; opened `[OQ-41]`–`[OQ-44]`. No vendor file — content lives in the concern-owning files.
- 2026-06-05 — added `14-architecture.md` (the structural spine: pure core + adapters/seams, the one-click pipeline, the Phase 0↔1 single-source-of-truth boundary). `07/08/10/11` reframed as the decision-content behind specific seams.
- 2026-06-01 — added `13-use-case-demand.md` (the demand side); intent layer / Screen 1 cards now planned from researched use cases, not the model catalog.
- 2026-05-29 — added `12-multimodal-models.md`; Verdure now recommends across audio/image/video, not just text. `03` scoped to text.
- 2026-05-24 — initial knowledge base assembled from research and project summary
