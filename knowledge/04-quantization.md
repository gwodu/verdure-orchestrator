# 04 — Quantization

**Status:** ✅ Settled

## The rule

**Always pre-quantized. Never on-device.**

This is non-negotiable. Quantization is a property of the model file Verdure chooses to download, never an operation Verdure performs on the user's machine.

## Why never on-device

Quantization is computationally expensive. Converting an FP16 model to Q4_K_M requires loading the full unquantized model into memory first. A 3B model at FP16 is ~6 GB. The Itel A70 (Tier P1) has 4 GB total RAM. The device that most needs the quantized model is, by definition, the device that cannot perform the quantization.

Even on devices that could do it, the user would wait 20–40 minutes on first install while their device heated up. That destroys the one-click-install promise.

The community has already done the quantization work. Hugging Face hosts pre-quantized GGUF files at every standard level for every major model. Verdure pulls them.

## Format

**GGUF** is the standard. Universal across the llama.cpp ecosystem (see `07-runtime-backends.md`). The vast majority of published quantizations are GGUF.

## Quantization tier — Q4_K_M is the default

**Q4_K_M** preserves approximately 95% of the unquantized model's quality at roughly 25% of the file size. It is the community standard for mobile and edge deployment.

## Decision tree

- **Default**: Q4_K_M. Use unless there is a specific reason not to.
- **Tight RAM headroom** (Tier L1, P1, P2): stay at Q4_K_M. Do not go below — quality degrades rapidly past Q4.
- **Comfortable RAM** (Tier L2, P3): optionally Q5_K_M for a ~3% quality bump if memory budget allows.
- **High bandwidth / unified memory** (Tier L3, Apple Silicon): Q6_K is acceptable when bandwidth is not the bottleneck.

## Where to find quantized GGUFs

Pre-quantized GGUFs are published by:
- Official model authors (Google, Microsoft, Meta, Alibaba, etc.)
- The `bartowski` Hugging Face account
- The `unsloth` Hugging Face account
- Historical: TheBloke (legacy archive)

See `10-distribution.md` for distribution and CDN strategy.
