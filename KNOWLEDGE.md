# Verdure — KNOWLEDGE.md

Verdure is the **cheapest, safest, and easiest way to use AI in daily life** — the user's **local AI conductor**: **one simple interface that runs the right local model** for whatever the user wants to do, on their own device, owned by them. Cheapest (free, local), safest (on-device, no data leaves), easiest (one interface — no new apps or workflows to learn). It starts from "what are you trying to do?", picks the right local model, and runs it behind a single Verdure UI. **Agency, not extraction:** Verdure conducts existing OSS models/engines, never builds its own; mission over moat; no data harvesting.

> **👉 Here to build? Start at `knowledge/18-unified-interface.md`** — the 2026-06-18 pivot: **one interface over local models** (not a launcher of apps). Then `17` for what carries over.

> **Note:** As of 2026-06-18 Verdure pivoted from a *launcher that installs separate apps* (the "porter", `15`/`17`) to **one interface that drives headless model engines directly** (`18`). Files `01/14/15/16/17` predate the pivot and are being reconciled — read them through the lens of `18`.

This file is an orientation index, not a content store. Knowledge lives in the `knowledge/` subfolder as numbered, modular files. Claude should read this file at the start of every session, then load only the knowledge files relevant to the current task.

This knowledge base is shared across the monorepo: it informs the Phase 0 website (`website/`, the decider/funnel), the **MVP desktop porter** (`app/`, the near-term build — see `17`), and the long-term Rust core (`src/`). The recommendation engine is this knowledge operationalized in code — now a shared workspace package, **`@verdure/decider`** (`packages/decider/`), imported by both the website and the porter so the decision can never differ between surfaces. See `CLAUDE.md` ("Repository layout").

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
- `knowledge/16-growth-asset.md` — the growth lens (Eva Lau's ASSET): adopts atomic-unit + seed-supply; **explicitly rejects** the data/network-effect moat as off-mission.
- `knowledge/17-build-plan.md` ★ — the (pre-pivot) build plan: the Tauri MVP + roadmap. Superseded as the *primary* surface by `18`; still the reference for what carries over (decider, shell, CI, install machinery).
- `knowledge/18-unified-interface.md` ★ — **the current direction (2026-06-18 pivot): one interface over local models, not a launcher of apps.** The mission, the doctrine shift (build one surface over headless engines), the architecture, and the first slice. **Start here to build.**
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
- Growth lens / where to seed supply / why we reject the data moat → `16`
- **What we're building now and why (the 2026-06-18 pivot) → `18`** (then `17` for carryover)
- Tracking what is still open → `open-questions.md`

## Conventions

Each knowledge file opens with a status marker (✅ Settled, 🟡 Provisional, 🔴 Open) and ends with an "Open Questions" section where relevant. Open questions also appear in the centralized `open-questions.md` with stable `[OQ-XX]` reference codes. Cross-references between files are always explicit.

## Changelog
- 2026-06-18 — **major pivot (`18-unified-interface.md`): one interface over local models, not a launcher of apps.** Triggered by the Buzz handoff friction (separate UI + Homebrew→Xcode-CLT prompt + 512 MB). Verdure becomes a single shell that drives **headless OSS engines** (Ollama, whisper.cpp, …) behind one consistent UI — *conduct models, not apps*. **Doctrine reversed:** never build a model/engine, but **do** build the one unified surface. Mission sharpened: **the cheapest, safest, easiest way to use AI in daily life.** Atomic unit redefined: one everyday task done with a local model, in one place. Decider + shell + CI carry over; standalone-app porting (Buzz-as-app) demoted → transcription will run on whisper.cpp inside the shell. v1 surfaces are dead-simple ("drop audio → text"). `[OQ-52]`/`[OQ-53]`.
- 2026-06-16 — **built the MVP first slice (the Buzz port).** Scaffolded the Tauri porter in `app/`; extracted the decider into a shared workspace package `@verdure/decider` (`packages/decider/`) imported by both website and porter (resolves `[OQ-49]`: shared package, not copy). Catalog-as-data + a pure-then-execute porter (`app/src-tauri/`). Loop proven in browser preview; real on-Mac run pending Xcode CLT (`[OQ-50]`). See `17-build-plan.md`, FEATURE_LOG.md, TEST_REPORT.md.
- 2026-06-16 — alignment pass + handoff: added `17-build-plan.md` (the MVP = a Tauri porter on a portable engine, first slice Buzz; + roadmap). Reframed the orientation files to the **conductor/porter** identity (not "orchestration moat"); reconciled phasing (Phase 1 = the Tauri MVP, not the Rust daemon); added an `OllamaBackend` note. `[OQ-49]`.
- 2026-06-16 — `16-growth-asset.md`: ASSET as a *lens* with deliberate divergence. Atomic unit = **one OSS tool installed & running** (the Uber-"ride"), not a persona. **Reject** the data-harvesting moat (off-mission, `06`) and don't chase a network effect — Verdure is a **utility that empowers users**; north star = the user's **local AI conductor** (recommend + set up + connect tools, on-device, owned by them). Seed supply from the OSS ecosystem; mission over moat. `[OQ-48]`.
- 2026-06-16 — designed the **composition layer**: user-directed, deterministic 2-tool chaining (transcription → writing) via typed I/O ports + a 3-field pipe contract; bright line "wire existing I/O, never build a missing surface"; hard-capped at 2 nodes (not n8n); deferred after the single-tool MVP. `14` + `15` (pipeability) + `[OQ-47]`.
- 2026-06-15 — added `15-port-catalog.md` (the supply-side local-tool database) + the two port classes; decided to **port AnythingLLM/Jan as the configurable shell rather than build one**; aligned `14`'s Tool layer to "port, never build." Seed from OSS lists (TAAFT is ~95% cloud). `[OQ-46]`.
- 2026-06-15 — adopted "the decider that finishes the job": a deferred, data-driven, **plug-n-play Tool layer** (configure & launch existing local tools / the device's own model, **never build bespoke apps**) — the local answer to "There's An AI For That". First Mac tools = writing assistant + doc-RAG. In `01`, `14` (Tool layer), `[OQ-45]`.
- 2026-06-15 — Apple-stack integration (WWDC 2026), distributed by concern across `01/03/07/11/14`: MLX supersedes MLC, the zero-download/resident-model recommendation, the iOS daemon→thin-decider pivot, and Apple Silicon reframed as the **beachhead** (mission still emerging-market). Resolved `[OQ-14]`/`[OQ-26]`; opened `[OQ-41]`–`[OQ-44]`. No vendor file — content lives in the concern-owning files.
- 2026-06-05 — added `14-architecture.md` (the structural spine: pure core + adapters/seams, the one-click pipeline, the Phase 0↔1 single-source-of-truth boundary). `07/08/10/11` reframed as the decision-content behind specific seams.
- 2026-06-01 — added `13-use-case-demand.md` (the demand side); intent layer / Screen 1 cards now planned from researched use cases, not the model catalog.
- 2026-05-29 — added `12-multimodal-models.md`; Verdure now recommends across audio/image/video, not just text. `03` scoped to text.
- 2026-05-24 — initial knowledge base assembled from research and project summary
