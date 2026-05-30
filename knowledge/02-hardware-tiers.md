# 02 — Hardware Tiers

**Status:** ✅ Settled (tier model); 🟡 Provisional (specific device archetypes may evolve)

This file answers: **given a device, what tier is it?**

It is paired with `03-model-matrix.md`, which answers: **given a tier, what model do we recommend?** Together these two files are the matching brain. Every recommendation Verdure produces touches both.

## Laptop tiers (Phase 1 primary targets)

**Tier L1 — the floor.** 4 GB RAM laptops with Intel Core i3 or i5 processors from 2015–2018, integrated graphics. Used Dell Inspiron, HP Pavilion archetypes. Common in emerging markets as used or refurbished imports.

**Tier L2 — the volume.** 6–8 GB RAM laptops, Intel Core i5 or i7 from 2017–2020. Common refurbished MacBooks (Intel-era), mid-tier Windows machines, base-spec ThinkPads. This is the largest practical addressable market for Phase 1.

**Tier L3 — aspirational.** 16 GB+ RAM, Apple Silicon M1/M2/M3/M4 or modern AMD/Intel. Has headroom for 7B+ parameter models. The "I want the best local experience" tier.

## Phone tiers (Phase 2 primary targets)

**Tier P1 — the floor.** 4 GB RAM, Unisoc T603 or T606 chipset, Itel A70 archetype. Sub-$100 devices, ubiquitous in Africa and South Asia. Represents hundreds of millions of users globally.

**Tier P2 — the volume.** 4–8 GB RAM, MediaTek Helio G81 or G85, Tecno Spark 40 or Infinix Hot 40i archetype. $100–$200 segment, ~42% of African shipments in 2025. This is the global volume sweet spot.

**Tier P3 — aspirational.** 6–8 GB RAM, MediaTek Dimensity 6300 or Snapdragon mid-range. Redmi Note 14C, Samsung Galaxy A06 5G. The ceiling we respect; not the primary target.

## Why we target down

Following the RAM crisis (see `01-thesis.md`), the device floor is moving down through 2026–2027. Verdure designed for 4 GB is aligned with the trend. A product that only works well at 8 GB+ is fighting where the market is going.

There is also a strategic point: whatever runs well on Tier L1 / P1 will run brilliantly on Tier L3 / P3. The reverse is not true. Targeting down captures more users.

## Mapping a user-described device to a tier

In Phase 0, the website asks the user to pick from a small set of **plain-language device cards** (see `05-website-spec.md`). The cards never show a spec — no RAM, no "GPU", no "Apple Silicon" — because the audience is non-technical (see `../skills/design.md`, "Assume zero technical knowledge"). Each card is described in lived experience and maps to one internal tier; the spec columns below stay internal and are the source of truth we reason with. In Phase 1+, the binary reads device specs directly via OS APIs and maps to a tier automatically.

The card → tier mapping (cards are the user-facing layer; RAM/graphics stay internal):

| User picks (card) | Internal tier | RAM (internal) | What the card quietly signals |
|---|---|---|---|
| "A newer Mac" (≈2021 or later) | L3 | 16 GB+ unified | Apple Silicon — can generate images |
| "An older Mac" (before ≈2021) | L2-high | ~8 GB | Intel Mac, no real GPU |
| "A regular laptop" (everyday Windows) | L2-low | 6–8 GB | no dedicated GPU |
| "A gaming computer" | L3+ | 16–32 GB + GPU | dedicated GPU — unlocks image/video |
| "An older or basic computer" | L1 | ~4 GB | the floor |
| "I'm not sure" | L2-low (safe default) | 6–8 GB | confirmed/adjustable on the next screen |

The "quietly signals" column is the key Phase-0 win: the cards capture the graphics signal the multimodal engine needs (see `12-multimodal-models.md`) **without ever asking the user a technical question.** Implemented in `website/lib/tiers.ts` (`DEVICE_CARDS`). Distinguishing graphics-memory *amount* for the heaviest tasks remains `[OQ-29]`.

## Open questions

- [OQ-03] Do we have physical access to a Tier P1 / P2 / L1 device for real benchmarking? (Critical — blocks publishing real-world performance numbers.)
- [OQ-04] What is the practical floor — do we even attempt to support 4 GB phones, or start at 6 GB?
