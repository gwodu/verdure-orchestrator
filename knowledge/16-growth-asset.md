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

## A — Atomic unit: the 1-click value loop
Verdure's atomic unit is the **core value loop**: *tell us what you want to do → the decider picks the right local tool → it's installed and running on your device, in one click.* That intent→install→run action is the irreducible unit of value, and proving it is the MVP.

*(Precision on the term: in Eva Lau's framework the atomic unit is the OBJECT users congregate around — Wattpad's "a story," with reading and writing as the two sides. Verdure's object is a **Tool** in the catalog. But a ported Tool (Buzz, Upscayl…) is **not user-generated**, so — unlike a Wattpad story — it doesn't form a network on its own. So for the MVP we center the value loop above, not an object.)*

**Mind the Wattpad analogy — it has real limits.** Wattpad is a UGC content network; its stories are user-created, which is the whole engine. Verdure's MVP is a **utility** — it installs third-party tools — and a utility has **no network effect by default**. Whether Verdure becomes a network-effect business is a deliberate later choice (below), not something to force by stretching the analogy.

## S — Seed the supply side (where tools come from)
Wattpad's lesson: bootstrap from **existing free resources**, don't wait for creators. Verdure's "20,000 public-domain books" is the **existing open-source local-AI ecosystem**:
1. **Curated ported apps.** The free/OSS tools already exist (`15`). Seed supply by recording an **install recipe per tool in the catalog manifest** — from the tool's official source (GitHub release / Homebrew / app site). Adding a tool is a **data entry, not code** — the manifest IS the "easily port more tools" mechanism.
2. **Verdure-authored starter personas.** A small set (writing agent, doc-Q&A persona…) written cheaply as configs on the ported shell. Seeds the persona library before users contribute.
3. **(Later — the E stage) user/community personas.** Custom-GPT-style contribution that turns the seed into self-growing supply.

**MVP scope:** a minimal, hand-curated seed for ONE use case (a local writing persona + a couple of ported apps). Manual seeding is *correct* here — Wattpad hand-imported too.

## S — Scale the demand side
Reduce friction to users. Verdure already has the pieces: the website funnel (intent cards), 1-click install, "runs on the device you already have — private, free, no jargon." Wattpad's "mobile apps everywhere" = Verdure's "no new hardware, no cloud, no setup." Demand scales after the supply is seeded and the atomic unit works.

## E — The network-effect question (deferred — choose deliberately, don't force it)
A utility has no network effect by default. Verdure has two honest paths from utility to network, to pick *later*, not in the MVP:
1. **Data network effect — the natural fit for a decider.** Every opt-in recommendation + outcome (intent + device → which tool → did it stick) makes the decider smarter and the catalog better-ranked for the *next* person. More use → better recommendations → more users. This is the **T → A** loop; it needs no user-generated content and is the most defensible fit for "the decider." Opt-in only, never default telemetry (`06`).
2. **UGC marketplace (the genuine Wattpad analog, optional).** If users can create + share **personas** (custom-GPT-style local configs), supply becomes user-generated and a creators ↔ users flywheel forms. The persona is the *supply object* here — **not** the atomic unit — and this is an optional later layer, with real risk of over-fitting Wattpad.

The MVP commits to neither; it proves the utility loop. Which path (or both), and when, is `[OQ-48]`.

## T — Track proprietary insights
Verdure's proprietary dataset: **which use cases → which tools/personas actually work on which devices** — the "what local AI real people use, by intent and device" map that no one else has. It's the demand × feasibility data (`13` + the matrix), grown by real usage; the moat and the source of adjacent opportunities. Always **opt-in only, never default telemetry** (`06-privacy-principles.md`) — refusing to share never produces a worse experience.

## Where we are: the A stage
The MVP *is* the atomic unit — the 1-click intent→install→run loop, proven for one use case. It is a **utility** at this stage. Supply seeding is minimal and curated; the network-effect choice (data vs UGC personas) is deliberate and deferred. Sequence: **prove the loop → seed supply from the OSS ecosystem → scale demand via the funnel → choose a network-effect path → harvest the usage map.**

## Open questions
- [OQ-48] Define the atomic unit precisely and the **persona create/share mechanism** (the network-effect engine): is a persona a first-class shareable artifact, and from what phase? (From `16-growth-asset.md`.)
