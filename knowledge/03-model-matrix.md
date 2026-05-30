# 03 — Model Matrix (text)

**Status:** 🟡 Provisional (throughput numbers extrapolated from adjacent benchmarks; needs real-world validation)

This file answers: **given a tier, what *text* model do we recommend?**

It is paired with `02-hardware-tiers.md`, which answers: **given a device, what tier is it?** Together these two files are the matching brain.

**Scope note (2026-05-29):** Verdure now recommends across modalities. This file covers **text** (chat, writing, code, multilingual). For **audio, image, and video** tasks — and the graphics-bound feasibility logic that decides when a task is honestly *not* local-capable — see `12-multimodal-models.md`. The intent classifier first picks a modality, then the task within it.

Quantization defaults are documented in `04-quantization.md`. Licensing is documented in `09-licensing.md`.

## The matrix

All footprints assume Q4_K_M quantization (the default — see `04-quantization.md`).

| Tier | RAM | Model class | Example | Footprint |
|---|---|---|---|---|
| L1 / P1 | 4 GB | 1B params | Gemma 3 1B | ~0.7–1.0 GB |
| L2-low / P2 | 6–8 GB | 1.5B–3B params | Qwen 2.5 1.5B, Llama 3.2 3B | ~1.0–2.2 GB |
| L2-high / P3 | 8 GB | 3B–4B params | Phi-4-mini 3.8B, Gemma 3 4B | ~2.2–2.9 GB |
| L3 | 16 GB+ | 7B+ params | Mistral Small, Llama 3.1 8B | ~4.5–5.5 GB |

## Expected throughput

Conservative estimates. Real benchmarks pending physical-device testing.

- Tier P1 (Unisoc T606, Gemma 3 1B Q4_K_M): 6–10 tokens/sec generation
- Tier P2 (Helio G85, Llama 3.2 3B Q4_K_M): 8–14 tokens/sec generation
- Tier L1 (older Intel i5, Llama 3.2 3B Q4_K_M): 12–18 tokens/sec generation
- Tier L2 / L3 (M1+ MacBook, Phi-4-mini Q4_K_M): 20–30 tokens/sec generation

For interactive use, anything above 15 tokens/sec feels responsive. Below 8 tokens/sec starts to feel slow for chat. Below 4 tokens/sec is impractical for live interaction.

## Intent shifts the recommendation within a tier

The user's stated intent (captured in Phase 0 via natural-language input and LLM-parsed) shifts the recommended model *within* the device's capability ceiling — not across it. The tier defines the model class; the intent picks within that class.

Examples on an 8 GB MacBook (Tier L2-high):

- **Creative writing** → Gemma 3 4B. Quality over speed; the user reads as the model generates.
- **Quick chat / Q&A** → Llama 3.2 3B. Speed over quality; faster response feels better for short interactions.
- **Code help** → Phi-4-mini. Reasoning-strong on technical tasks.
- **Multilingual** → Qwen 2.5 3B. Strong across 35+ languages.

## Model details

**Gemma 3 1B** — Google's smallest current Gemma. Apache 2.0 license issues with Gemma 3 (custom terms; see `09-licensing.md`). Strong general-purpose for the floor tier.

**Gemma 3 4B** — Google's 4B class. Good creative writing, instruction-following. Custom Gemma terms.

**Llama 3.2 3B** — Meta. Strong general performance. Community license with 700M MAU cap — fine for Verdure until massive scale (see `09-licensing.md`).

**Llama 3.1 8B** — Meta. The 8B class workhorse for L3 tier.

**Phi-4-mini (3.8B)** — Microsoft. Reasoning-strong, MIT license, ideal for technical and code tasks.

**Qwen 2.5 1.5B / 3B** — Alibaba. Strong multilingual support, Apache 2.0, no friction.

**SmolLM 2 1.7B** — Hugging Face. Apache 2.0, designed for edge deployment. Strong candidate for on-device intent classification in Phase 1+.

**Mistral Small** — Mistral. 7B class, Apache 2.0.

## Open questions

- [OQ-05] Real tokens/sec on physical Unisoc T606 and MediaTek Helio G81 devices — currently unknown in public literature. Opportunity to publish the first dataset.
- [OQ-06] Validation of intent → model preference assumptions with real users. Does "creative writing" actually score better on Gemma 3 4B vs Llama 3.2 3B for our target audience?
