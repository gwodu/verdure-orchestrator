# 16 — Growth Strategy: the ASSET Framework

**Status:** 🟡 Provisional — adopted 2026-06-16 as the growth / network-effect roadmap.

This file maps Verdure's growth onto the **ASSET framework** by Eva Lau (Two Small Fish Ventures; *"network effect = assets"*), to build Verdure as a defensible, network-based business rather than a one-shot utility. It sequences the two sides of the network that the product files define: `13-use-case-demand.md` (demand) and `15-port-catalog.md` (supply).

Source: Eva Lau, *The ASSET Framework* — https://twosmallfish.vc/what-is-the-asset-framework (she scaled Wattpad's community before founding Two Small Fish).

## The framework (Wattpad's playbook)
- **A — Atomic unit** — the smallest unit users congregate around. Wattpad = a *story*. It defines the kind of network you're building.
- **S — Seed the supply side** — the hardest task: kick-start the network by bootstrapping initial supply from *existing free resources* before chasing demand. Wattpad imported **20,000 public-domain books** (Moby Dick, Pride and Prejudice) to bring readers in before recruiting writers.
- **S — Scale the demand side** — once supply is stable, reduce friction and reach users. Wattpad built **mobile apps everywhere** so readers could read anytime — which then attracted writers seeking an audience.
- **E — Enlarge the network effect** — amplify the interactions that compound value (comments, conversations); the readers↔writers flywheel.
- **T — Track proprietary insights** — harvest user-behavior data into a moat and adjacent opportunities. Wattpad's data on which stories had entertainment potential turned it into an entertainment company (TV/film).
- Coefficient: *network effect ≈ users × attention × connections.*

## A — Atomic unit: a **Tool**
Verdure's atomic unit is a **Tool** — the installable unit of local AI capability, bound to one `(modality, task)`. Two kinds (`15`): a **ported app** (Buzz, Upscayl…) and — crucially — a **persona**: a config (model + system prompt + optional RAG) that runs on the ported shell (AnythingLLM/Jan), exactly like a custom GPT but **local**. The persona is the *user-creatable, shareable* atomic unit — Verdure's "story." When many tools/personas exist and users create + share them, Verdure is a two-sided network (creators ↔ users), not just a utility.

> The MVP delivers ONE atomic unit end-to-end (intent → installed → running, local, 1-click). That proves the unit; it does not yet create the network. The strategic MVP unit is a **persona** (e.g. a local writing agent), because it's the unit that later scales into supply.

## S — Seed the supply side (where tools come from)
Wattpad's lesson: bootstrap from **existing free resources**, don't wait for creators. Verdure's "20,000 public-domain books" is the **existing open-source local-AI ecosystem**:
1. **Curated ported apps.** The free/OSS tools already exist (`15`). Seed supply by recording an **install recipe per tool in the catalog manifest** — from the tool's official source (GitHub release / Homebrew / app site). Adding a tool is a **data entry, not code** — the manifest IS the "easily port more tools" mechanism.
2. **Verdure-authored starter personas.** A small set (writing agent, doc-Q&A persona…) written cheaply as configs on the ported shell. Seeds the persona library before users contribute.
3. **(Later — the E stage) user/community personas.** Custom-GPT-style contribution that turns the seed into self-growing supply.

**MVP scope:** a minimal, hand-curated seed for ONE use case (a local writing persona + a couple of ported apps). Manual seeding is *correct* here — Wattpad hand-imported too.

## S — Scale the demand side
Reduce friction to users. Verdure already has the pieces: the website funnel (intent cards), 1-click install, "runs on the device you already have — private, free, no jargon." Wattpad's "mobile apps everywhere" = Verdure's "no new hardware, no cloud, no setup." Demand scales after the supply is seeded and the atomic unit works.

## E — Enlarge the network effect
The flywheel turns when users can **create and share personas** (and discover/rate tools): creators make personas → more users → more creators. The **persona library is the network-effect engine** — which is *why* persona-GPTs are strategically central, not just a feature. Deferred until the atomic unit + seed are proven.

## T — Track proprietary insights
Verdure's proprietary dataset: **which use cases → which tools/personas actually work on which devices** — the "what local AI real people use, by intent and device" map that no one else has. It's the demand × feasibility data (`13` + the matrix), grown by real usage; the moat and the source of adjacent opportunities. Always **opt-in only, never default telemetry** (`06-privacy-principles.md`) — refusing to share never produces a worse experience.

## Where we are: the A stage
The MVP *is* the atomic unit (one persona/tool, 1-click, local). Supply seeding is minimal and curated; the network-effect stages (E, T) come after the unit and seed are proven. Sequence: **prove the unit → seed supply from the OSS ecosystem + starter personas → scale demand via the funnel → open persona creation/sharing (network effect) → harvest the usage map (insights).**

## Open questions
- [OQ-48] Define the atomic unit precisely and the **persona create/share mechanism** (the network-effect engine): is a persona a first-class shareable artifact, and from what phase? (From `16-growth-asset.md`.)
