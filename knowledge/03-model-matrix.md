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
| L3 (Apple Intelligence devices) | 8 GB+ unified | Resident system model (~3B) | Apple Foundation Model | **0 GB — pre-installed by OS** |

## Expected throughput

Conservative estimates. Real benchmarks pending physical-device testing.

- Tier P1 (Unisoc T606, Gemma 3 1B Q4_K_M): 6–10 tokens/sec generation
- Tier P2 (Helio G85, Llama 3.2 3B Q4_K_M): 8–14 tokens/sec generation
- Tier L1 (older Intel i5, Llama 3.2 3B Q4_K_M): 12–18 tokens/sec generation
- Tier L2 / L3 (M1+ MacBook, Phi-4-mini Q4_K_M): 20–30 tokens/sec generation

For interactive use, anything above 15 tokens/sec feels responsive. Below 8 tokens/sec starts to feel slow for chat. Below 4 tokens/sec is impractical for live interaction.

## Active parameters, not just total: the speed axis

The matrix above keys on **total** parameters, because total params set the two things that gate a download: the **disk footprint** and the **RAM** to hold the weights. But total params are *not* what sets generation **speed**. Speed is set by the parameters the model actually **activates per token**. These two numbers come apart in sparse-activation architectures, and that gap is now a first-class consideration when picking a model:

- **Dense models** (Qwen 2.5, Llama 3.2, Phi-4-mini) activate every parameter on every token. Total params = active params: one number sets footprint *and* speed together.
- **Sparse-activation models** store more parameters than they compute per token:
  - **Mixture-of-Experts (MoE)** routes each token through a few experts, so a model with large total capacity runs at the speed of its (much smaller) active slice.
  - **Per-Layer Embeddings (PLE)** — Gemma 4's E-series (E2B, E4B) — keeps a large embedding table that can live outside the hot compute path, so per-token work tracks the *effective* size (the "E" in E2B = ~2.3B **effective**), not the full stored size.

**Why this matters for the conversational front door (`18`):** the user feels latency most on *short* messages ("hey", "what's your name") — the case where fixed per-token cost dominates. A sparse model gives the snappy first-token-to-done feel of a small model while keeping more of a larger model's quality. So for chat, prefer the model with the **smallest active compute that still meets the quality bar**, not simply the smallest total.

**The catch — footprint does not shrink with active compute.** Sparse models still have to *store* all their parameters: Gemma 4 E2B is ~2.3B effective but ships as a **~3.1 GB Q4_K_M file** — heavier on disk and naive-load RAM than a dense 1.5B. The advertised "<1.5 GB" active footprint is only realised **if the runtime implements the offload** (PLE / expert paging); without it you pay the full resident cost. So a sparse model is the right pick for *speed-on-roomy-machines*, **not** for the memory floor — and only after the runtime's offload support is verified (it directly gates whether the model fits, and loading one that doesn't OOM-aborts the whole app — `21`). See [OQ-61].

Net rule for picking: **total params → does it fit (download + RAM); active params → does it feel fast.** Check both. On the floor tier, fit wins and dense-small stays; with headroom, a sparse model can beat a dense one of the same speed class on quality.

## Intent shifts the recommendation within a tier

The user's stated intent (captured in Phase 0 via natural-language input and LLM-parsed) shifts the recommended model *within* the device's capability ceiling — not across it. The tier defines the model class; the intent picks within that class.

Examples on an 8 GB MacBook (Tier L2-high):

- **Creative writing** → Gemma 3 4B. Quality over speed; the user reads as the model generates.
- **Quick chat / Q&A** → Llama 3.2 3B. Speed over quality; faster response feels better for short interactions.
- **Code help** → Phi-4-mini. Reasoning-strong on technical tasks.
- **Multilingual** → Qwen 2.5 3B. Strong across 35+ languages.

## The zero-download recommendation

On Apple Intelligence–capable devices (M1+ Macs, recent iPhones, on iOS 27 / macOS 27+), the OS ships a resident ~3B system model accessible via Apple's Foundation Models framework. For many everyday intents (summarize, classify, rewrite, short Q&A), the honest recommendation is: **download nothing — you already have a model.** Footprint zero, storage zero, instant availability.

This matters beyond Apple devices as a principle: a decider that knows when the answer is "install nothing" is more trustworthy than one that always recommends a download (see `06-privacy-principles.md`, honest expectations). The matrix must therefore support resident models — models with no download artifact. Architectural handling is in `14-architecture.md` (resident `ModelChoice`). Backend details are in `07-runtime-backends.md` (`AppleSystemBackend`). *(Design intent, not yet implemented: neither `@verdure/decider` nor the app models a zero-download choice today — this becomes real with the Apple backend, and stays flagged here so it isn't silently dropped.)*

The system model is closed-weight and fixed-size; for intents that exceed its quality ceiling (deep creative work, hard reasoning, specific domain needs), the recommendation shifts to a downloaded open-weight model per the matrix above.

## The in-app chat registry — a deliberately separate stack (resolves [OQ-62])

The shipping app picks its **chat** model through its own small registry (`app/src-tauri/src/model.rs`: ByteShape Qwen3 4B on top, then Qwen 3.5 2B, with Qwen2.5 1.5B/0.5B as floor/fallback, gated on **live available RAM** measured at the moment of use), not through this matrix. That is a decision, not drift — resolved 2026-08-10:

- **Different input.** This matrix maps a *static hardware tier* to a recommendation before anything is installed. The chat gate reads *available* memory at runtime — a number that changes minute to minute and only exists inside the running app (keying on available-not-total RAM is what fixed the 8 GB OOM crash).
- **Different moment.** The matrix answers "what should this device download?"; the registry answers "what can this device load *right now*?" — including honestly declining when nothing fits.
- **One caller.** Only the app has a live RAM reading. Folding a runtime axis into `@verdure/decider` would add an abstraction with a single consumer — the over-engineering the coding principles forbid. If the website ever needs runtime-aware picks, revisit then.

The boundary: **this file owns which models are worth recommending and why** (the registry's candidates should be justifiable from the sections above — the 4B-over-0.5B bet is the speed/quality axis applied to the memory floor); **`model.rs` owns the runtime gate**. A model added to the registry without a rationale expressible here is the smell that this decision needs revisiting. The registry's code comments cross-reference this section.

## Hybrid attention: the KV-cache axis (2026-09-10)

The matrix keys on **total params** (fit) and **active params** (speed). A third axis now matters as much as either, and it is the one that decides how much of a *conversation* a device can hold: **how many layers carry a KV cache.**

A classic transformer caches K and V on **every** layer, so conversation memory costs `2 × layers × kv_heads × head_dim × 2 B` per token — and that cost is what `llm.rs::context_for_budget` divides the spare RAM by. **Hybrid** models (Qwen 3.5, Qwen3-Next, Granite 4, Falcon-H1) interleave linear-recurrent layers — Gated DeltaNet, Mamba-2 — which carry a **fixed-size** recurrent state instead, and put full attention on only every *n*-th layer.

Measured on our own build (Intel Mac, 2026-09-10):

| | Qwen3 4B (dense) | Qwen3.5 2B (hybrid) |
|---|---|---|
| Layers with a KV cache | 36 of 36 | **6 of 24** |
| KV cost per token | 144 KB | **12 KB** |
| Recurrent state | — | 19 MB, fixed |
| Trained context | 32K | 262K |

**Why this is the most valuable axis for Verdure specifically.** Our devices are RAM-starved, and a beta tester's literal complaint was *"it ran out of context."* On the floor tier the KV cache — not the weights — is what caps the window: `MAX_CONTEXT` is 4096, and reaching it at 144 KB/token needs ~576 MB of spare RAM, versus ~48 MB at 12 KB/token. A hybrid buys back roughly a factor of ten of conversation memory **at the same footprint**, without a bigger download or a slower model. It is the cheapest quality-per-byte lever available to us, and it is arithmetic rather than a benchmark claim.

**The catch — reasoning models.** Qwen 3.5 emits a `<think>` scratchpad by default; framed as plain ChatML it spends its whole reply budget narrating and the answer never arrives (measured: 256 tokens of reasoning, no answer). The documented off-switch is a **pre-filled empty thinking block** after the assistant marker, carried as `PromptStyle::ChatMlNoThink`. Any future reasoning model needs the same treatment — check before adopting, because the failure is invisible to a test that only asserts a non-empty reply.

**Runtime footprint is bigger than the weights.** The same session found llama.cpp's `CPU_REPACK` buffer (weights re-laid-out for CPU SIMD) is a **second resident copy of a large fraction of the weights** — 490 MB on a 1211 MB model — on top of a ~500 MB compute buffer. Estimates built from file size alone understate peak resident by ~1 GB. This is why `runtime_mb` in the registry is deliberately pessimistic, and it retro-justifies the 4B's `2300` figure that `[OQ-64]` had flagged as over-cautious against a "measured 1.40 GB" that was weights-only.

**Not adopted, with reasons:** MoE (LFM2-8B-A1B, Granite-4 H-Tiny, gpt-oss-20B) — sparsity cuts *compute*, never *resident memory*; all experts must be in RAM, so the smallest useful one (~5 GB) exceeds every tier below L2. Expert offload to flash is bandwidth-bound and collapses on phone storage. LFM2 also ships under a custom `lfm1.0` licence, not redistributable (`09`). **Meta Muse Glimmer** (Aug 2026, Apache 2.0, agentic) — 30B, smallest GGUF 16.8 GB, targets 24 GB VRAM, and our vendored llama.cpp has no architecture entry for it, so it would not load. Track for L3+ when tool-calling returns (`20`); not a candidate at any tier we serve.

## Model details

**Gemma 3 1B** — Google's smallest current Gemma. Apache 2.0 license issues with Gemma 3 (custom terms; see `09-licensing.md`). Strong general-purpose for the floor tier.

**Gemma 3 4B** — Google's 4B class. Good creative writing, instruction-following. Custom Gemma terms.

**Gemma 4 E2B** — Google, released 2026-04-02. Apache 2.0 (a licensing improvement over the custom Gemma 3 terms). ~2.3B **effective** parameters via Per-Layer Embeddings (sparse activation — see the speed-axis section above), but a **~3.1 GB Q4_K_M** file. Multimodal (text/image/video, + audio on the small variants); we use text only today. The appeal for the chat front door is *speed at quality*: short-prompt latency closer to a 1.5B with more capability behind it. **Caveats before adoption:** (1) the sub-1.5 GB active footprint depends on runtime PLE offload — unverified in our pinned `llama-cpp-2` (`[OQ-61]`); (2) it uses Gemma's chat template (`<start_of_turn>` / `<end_of_turn>` + BOS), not ChatML, so the prompt builder must change; (3) at 3.1 GB it does **not** fit the 8 GB-with-1-GB-free floor — it belongs in the roomier slot, not as the floor model.

**Llama 3.2 3B** — Meta. Strong general performance. Community license with 700M MAU cap — fine for Verdure until massive scale (see `09-licensing.md`).

**Llama 3.1 8B** — Meta. The 8B class workhorse for L3 tier.

**Phi-4-mini (3.8B)** — Microsoft. Reasoning-strong, MIT license, ideal for technical and code tasks.

**Qwen 2.5 1.5B / 3B** — Alibaba. Strong multilingual support, Apache 2.0, no friction. **Superseded** by Qwen 3.5 2B in the in-app registry (below); retained as the floor fallback.

**Qwen 3.5 2B** — Alibaba, Apache 2.0. Hybrid (Gated DeltaNet + full attention every 4th layer), 262K trained context, ChatML template, **reasoning model — thinking must be disabled** (see the hybrid section above). Verified on our own build 2026-09-10: loads through the pinned `llama-cpp-2 0.1.154` (which vendors `LLM_ARCH_QWEN35` and `GGML_OP_GATED_DELTA_NET`), `graph splits = 1`, all CPU, coherent output. 1221 MB download, ~2218 MB measured peak resident. **12 KB/token** conversation memory — the same as the 0.5B floor model, for a 1.88 B model.

**Qwen 3.5 4B** — the roomy-tier sibling: same architecture, 32 KB/token, 2613 MB download. Architecture proven via the 2B; **footprint not yet measured**, so it is held out of the live pick (`[OQ-82]`).

**SmolLM 2 1.7B** — Hugging Face. Apache 2.0, designed for edge deployment. Strong candidate for on-device intent classification in Phase 1+.

**Mistral Small** — Mistral. 7B class, Apache 2.0.

## Open questions

- [OQ-05] Real tokens/sec on physical Unisoc T606 and MediaTek Helio G81 devices — currently unknown in public literature. Opportunity to publish the first dataset.
- [OQ-06] Validation of intent → model preference assumptions with real users. Does "creative writing" actually score better on Gemma 3 4B vs Llama 3.2 3B for our target audience?
