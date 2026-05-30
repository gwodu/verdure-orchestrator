# 09 — Model Licensing Audit

**Status:** ✅ Settled

This file documents the licenses of the **text** models Verdure recommends. For the model-tier mapping itself, see `03-model-matrix.md`. **Non-text models (audio/image/video) and their licenses are audited inline in `12-multimodal-models.md`** — the same Apache/MIT-first posture applies, with two flags worth repeating: **MusicGen is CC-BY-NC (non-commercial) — avoid**, and FLUX.1 *dev* / HunyuanVideo carry non-commercial or jurisdictional restrictions, so Verdure leads with FLUX.1 *schnell*, Whisper, Kokoro, Real-ESRGAN, and Qwen-Image-Edit instead.

## The audit

| Model | License | Verdure-safe? | Notes |
|---|---|---|---|
| Gemma 4 | Apache 2.0 | ✅ Ideal | No custom clauses, no restrictions on redistribution or commercial deployment |
| Gemma 3 / 3.5 | Custom Gemma terms | ⚠️ Usable, adds friction | Use restrictions must be passed downstream |
| Qwen 2.5 / 3 / 3.5 | Apache 2.0 | ✅ Ideal | Clean redistribution |
| Phi-4 / Phi-4-mini | MIT | ✅ Ideal | Clean redistribution |
| Llama 3.2 / 4 | Community license | ⚠️ Fine until scale | 700M MAU cap, EU restrictions |
| SmolLM 2 | Apache 2.0 | ✅ Ideal | Clean redistribution |
| Mistral Small 4 | Apache 2.0 | ✅ Ideal | Clean redistribution |

## Strategic posture

Lead with Apache 2.0 and MIT models. They redistribute cleanly, have no MAU caps, no jurisdictional restrictions, and no downstream-friction terms. Specifically: Gemma 4, Qwen, Phi-4 / Phi-4-mini, SmolLM 2, Mistral Small.

Llama models remain usable as a secondary option. Verdure is nowhere near the 700M MAU threshold, and Llama's quality at the 3B and 8B classes is excellent. Just track the cap as the user base grows.

Gemma 3 is acceptable but requires passing Google's custom terms through to end users — friction worth noting.

## Why this matters

Phase 0 only links to models (the user installs them through Ollama, LM Studio, or Jan). License friction is borne by those tools.

Phase 1 changes this. If Verdure ships a curated mirror or pre-packaged bundles, Verdure becomes a distributor. License clarity is non-negotiable. Lead with permissive-license models in the curated catalog (see `10-distribution.md`).

## Open questions

None. Licenses are documented and the strategic posture is clear. If models update their licenses (it has happened — Llama license has tightened over versions), update this file.
