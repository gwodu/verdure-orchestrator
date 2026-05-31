# 01 — Thesis

**Status:** ✅ Settled

## What Verdure is

Verdure is the orchestration layer for local AI. Given any device, Verdure detects what it can run, picks the right model, downloads it, configures the right backend, and exposes a unified inference interface. The user picks a model and uses it — everything else is invisible.

## What Verdure recommends

**Scope extended 2026-05-29:** Verdure recommends across modalities — text, audio, image, and video — not just text LLMs. The thesis always said "local AI," but the matching brain began text-only; this makes the broader scope explicit. Crucially, the orchestration includes **honesty about feasibility**: many image and most video tasks need graphics power a typical laptop lacks, so for those devices the right recommendation is "this isn't practical locally yet — here's where to go instead," not a model that would crawl. Honest decline is a feature of the decider, not a gap. The per-modality matrix and feasibility logic live in `12-multimodal-models.md`.

## What Verdure is not

Verdure is not a model. It is not an inference engine. It is not a replacement for llama.cpp, Ollama, ExecuTorch, MLC LLM, Jan, or LM Studio. Verdure *uses* these as backends, or refers to them as install paths during Phase 0. The value sits above them.

## The seven decisions Verdure makes for the user

Every recommendation Verdure produces is the result of seven sequential decisions, made invisibly:

1. **Hardware fingerprinting** — what does this device actually have?
2. **Model selection** — given the hardware, which model class fits?
3. **Quantization tier** — Q4_K_M, Q5_K_M, Q6_K — which tradeoff?
4. **Runtime backend routing** — llama.cpp CPU path, NPU delegation, Metal, Vulkan?
5. **Source resolution and download** — pull from the right place, handle flaky networks
6. **Lifecycle management** — storage budgets, swapping, eviction, updates
7. **Unified inference interface** — one API regardless of underlying backend

## North star UX

User opens Verdure → picks a model (or accepts the recommended default) → clicks install → uses it. No technical setup. No model size discussions. No quantization choices. No backend configuration.

## The moat: intent-first orchestration

Every existing tool (Ollama, LM Studio, Jan, Hugging Face) starts with "browse the model library." They answer *how do I run a model?* Verdure starts with *what are you trying to do?* and answers *which model should you run, and why?*

This inversion is the differentiation. The runner is commoditizable; the decider is not. Phase 0 plants the flag on this position before competitors notice the gap.

**Known gap — the demand side (`[OQ-33]`).** The decider has two halves: knowing what people *want to do*, and knowing which model *does it*. So far the model half is built (`03`, `12`); the use-case half was bootstrapped from the available models rather than from real usage. To make the moat real, the orchestration should be *planned from validated use cases* — researched for breadth and prevalence — not just from the model catalog. This feeds the intent taxonomy and the planned intent cards (`[OQ-32]`).

## Phasing strategy

Verdure ships in three phases. The orchestration brain — this knowledge base — is constant across all of them. Only the delivery surface changes.

**Phase 0 — the recommendation website.** A public site at verdureai.com that takes the user through a 60-second flow and produces a personalized recommendation with honest performance expectations and tiered install paths. No binary. No installation. See `05-website-spec.md`.

**Phase 1 — the laptop binary.** A Rust daemon on macOS / Windows / Linux. No app sandbox. Other processes call Verdure via local socket. Closes the loop from recommendation to one-click install and run.

**Phase 2 — phones.** Android first (larger emerging-market footprint), iOS second if feasible. Verdure as a system-permissioned app whose background daemon serves inference to other apps via local IPC.

Implementation language for Phase 1 and Phase 2 is Rust — non-negotiable for cross-platform reach. Phase 0 is whatever web stack ships fastest.

## Market context: the RAM crisis

DRAM contract prices roughly doubled through 2025–2026. Mid-range phones are expected to drop from 12 GB to 8 GB. Budget phones may drop from 6–8 GB back to 4 GB. The global device floor is moving *down*, not up.

This makes intelligent orchestration more valuable, not less. Generic instructions like "just run Ollama" break on these devices. Verdure designed for 4 GB targets is aligned with where the market is heading.

## Open questions

- [OQ-01] Do we commit to laptops-first formally, or develop Phase 1 and Phase 2 in parallel?
- [OQ-02] For Phase 2: Android first or iOS first?
