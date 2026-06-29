# 16 — Growth Strategy: the ASSET Framework

**Status:** 🟡 Provisional — growth lens, mostly intact post-pivot; one definition updated by `18` (`[OQ-52]`).

> **Read through the pivot.** The growth *strategy* here survives the 2026-06-18 pivot (`18`): seed supply from the OSS ecosystem, **reject the data/network-effect moat as off-mission**, mission over moat. The one thing the pivot **redefined** is the **atomic unit**: this file calls it "one OSS tool installed & running"; `18` redefines it as **one everyday task done with a local model, in one place** (no separate tool to install). Read the atomic-unit passages below as that updated definition.

This file uses the **ASSET framework** by Eva Lau (Two Small Fish Ventures; *"network effect = assets"*) as a **lens** for growth — and is explicit about where Verdure **deliberately diverges**, because Verdure optimizes for **user agency and benefit, not a defensible data/network moat**. We adopt A and the two S's; we reject the network-effect/data-harvesting end (E and especially T) as off-mission. Pairs with `13-use-case-demand.md` (demand) and `15-port-catalog.md` (supply).

Source: Eva Lau, *The ASSET Framework* — https://twosmallfish.vc/what-is-the-asset-framework (she scaled Wattpad's community before founding Two Small Fish).

**Mission over moat.** Verdure wins by being the most useful and trustworthy way for a non-technical person to run real local AI — not by locking in a network or hoarding data. Differentiation (neutral, private, agency-giving) is a *consequence* of maximizing user benefit, not the goal. The north star: a **base local AI that recommends, sets up, and connects your tools — on your device, owned by you** (see `01-thesis.md`).

## The framework (Wattpad's playbook)
- **A — Atomic unit** — the smallest unit users congregate around. Wattpad = a *story*. It defines the kind of network you're building.
- **S — Seed the supply side** — the hardest task: kick-start the network by bootstrapping initial supply from *existing free resources* before chasing demand. Wattpad imported **20,000 public-domain books** (Moby Dick, Pride and Prejudice) to bring readers in before recruiting writers.
- **S — Scale the demand side** — once supply is stable, reduce friction and reach users. Wattpad built **mobile apps everywhere** so readers could read anytime — which then attracted writers seeking an audience.
- **E — Enlarge the network effect** — amplify the interactions that compound value (comments, conversations); the readers↔writers flywheel.
- **T — Track proprietary insights** — harvest user-behavior data into a moat and adjacent opportunities. Wattpad's data on which stories had entertainment potential turned it into an entertainment company (TV/film).
- Coefficient: *network effect ≈ users × attention × connections.*

## A — Atomic unit: one OSS tool installed & running
Verdure's atomic unit is **a user getting one real local AI tool working, from their intent** — the core value transaction: *tell us what you want to do → the decider picks the right **OSS tool** → it's installed and running on your device, in one click.* That's the Uber-"ride" of Verdure (Uber's atomic unit is a completed ride, not a driver profile). The **first** atomic unit is an existing **open-source tool** (e.g. Buzz) — not a persona. A persona is just one *flavor* of tool, and leading with it over-indexes on differentiation; lead with the simplest, highest-benefit thing instead.

**Mind the Wattpad analogy — it has real limits.** Wattpad is a UGC content network; its stories are user-created, which is the whole engine. A ported OSS tool (Buzz, Upscayl…) is **not** user-generated, so it doesn't form a network the way a story does. Verdure is a **utility that empowers the user** — and that's the point, not a deficiency. We are not trying to manufacture a network effect.

## S — Seed the supply side (where tools come from)
Wattpad's lesson: bootstrap from **existing free resources**, don't wait for creators. Verdure's "20,000 public-domain books" is the **existing open-source local-AI ecosystem**:
1. **Curated ported apps.** The free/OSS tools already exist (`15`). Seed supply by recording an **install recipe per tool in the catalog manifest** — from the tool's official source (GitHub release / Homebrew / app site). Adding a tool is a **data entry, not code** — the manifest IS the "easily port more tools" mechanism.
2. **Verdure-authored starter personas.** A small set (writing agent, doc-Q&A persona…) written cheaply as configs on the ported shell. Seeds the persona library before users contribute.
3. **(Later — the E stage) user/community personas.** Custom-GPT-style contribution that turns the seed into self-growing supply.

**MVP scope:** a minimal, hand-curated seed for ONE use case (a local writing persona + a couple of ported apps). Manual seeding is *correct* here — Wattpad hand-imported too.

## S — Scale the demand side
Reduce friction to users. Verdure already has the pieces: the website funnel (intent cards), 1-click install, "runs on the device you already have — private, free, no jargon." Wattpad's "mobile apps everywhere" = Verdure's "no new hardware, no cloud, no setup." Demand scales after the supply is seeded and the atomic unit works.

## E — We deliberately don't chase a network effect
Verdure is content to be a utility that *empowers* users; growth comes from being genuinely useful + word of mouth, not from lock-in. We explicitly reject the two standard paths:
- **A data network effect** (the decider improving from harvested usage) is **off-mission** — it makes Verdure's value depend on accumulating user data, the opposite of "your data never leaves your device" (`06`). **Killed.**
- **A UGC persona marketplace** (Wattpad-like) is at most a *possible far-future* layer if users ever want to share local persona-configs — **not a goal, not a focus**, and never a reason to centralize or harvest anything. A persona is one flavor of tool, not the strategy. `[OQ-48]`.

## T — We do NOT harvest proprietary insights from users
ASSET's final stage turns user-behavior data into a moat. **Verdure rejects this outright** — it is incompatible with `06-privacy-principles.md` and with the mission of giving people agency, not mining them. Our durable asset is *public and open*: the curated knowledge base (`02`/`03`/`12`/`13`) and the tool catalog (`15`) — knowledge we research and write down, not data we extract from users. The decider improves because *we* do the research, not because we watch you. **No data conglomerate.**

## Where we are: the A stage
The MVP *is* the atomic unit — **one OSS tool installed & running**, from the user's intent, in one click. Verdure is a **utility that empowers users**, and that is the goal — not a stepping stone to a data/network moat. Sequence: **prove the loop with one OSS tool → seed more supply from the OSS ecosystem → scale demand via the funnel (the website + word of mouth) → connect tools (`14` composition, v1.1).** There is no usage-harvesting stage.

## Open questions
- [OQ-48] Define the atomic unit precisely and the **persona create/share mechanism** (the network-effect engine): is a persona a first-class shareable artifact, and from what phase? (From `16-growth-asset.md`.)
