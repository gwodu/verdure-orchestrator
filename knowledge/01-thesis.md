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

**The two halves of the decider.** It must know what people *want to do* and which model *does it*. The model half lives in `03`/`12`; the demand half — what people actually use (local) AI for, by prevalence — is researched in `13-use-case-demand.md` (added 2026-06-01, resolving `[OQ-33]`). The intent layer (Screen 1 cards, the `(modality, task)` taxonomy) is planned from `13`, not from the model catalog. Key demand finding: local-AI use concentrates on transcription, coding, image generation, private document Q&A, and roleplay/uncensored content — driven by privacy, cost, and the absence of content filters.

## The decider that finishes the job (plug-n-play, never bespoke)

The decider's promise isn't complete at "here's the right local AI for you" — the user's next question is "okay, now make it work." Verdure closes that loop: it turns the recommendation into a **running tool, in one click.** This is the local answer to directory sites like *There's An AI For That*, which organize AI by task but stop at an outbound link to a (mostly cloud) sign-up. Nobody occupies the intersection Verdure already owns: curated + intent-organized + **local** + 1-click + for non-technical users.

**Plug-n-play, never bespoke.** Verdure *configures and launches* tools — installing the best existing local app, or pushing a per-use-case config to a *ported* open-source shell (e.g. AnythingLLM) — and **never authors or brands its own tools.** The catalog of what's portable is `15-port-catalog.md`. The reason is the moat: the moment Verdure ships "Verdure Transcribe," it can no longer honestly say "actually, MacWhisper is better for you." Neutrality *is* the moat; building apps would destroy it (and drag Verdure onto an unbounded maintenance treadmill). A "tool" is therefore the recommendation *pre-assembled and runnable* — model + persona + optional local RAG + a thin surface + the honesty verdict — and it stays **data, not a codebase**. The architecture of this layer is in `14-architecture.md` (the Tool layer).

**Phasing the tools.** Ship the decider first; then turn the highest-demand, *non-agentic* intents into runnable tools (writing, doc-Q&A, transcription). Autonomous, multi-step **agents** (the travel/tax-assistant vision) stay deferred and aspirational — only ~17% of people trust AI to act for them, and small local models are unreliable at multi-step tool-calling. We earn that with reliability and a confirm-not-autonomous UX before shipping it.

How this grows into a network-effect business — the atomic unit (a tool/persona), seeding supply from the open-source ecosystem, and the ASSET roadmap — is in `16-growth-asset.md`.

## Phasing strategy

Verdure ships in three phases. The orchestration brain — this knowledge base — is constant across all of them. Only the delivery surface changes.

**Phase 0 — the recommendation website.** A public site at verdureai.com that takes the user through a 60-second flow and produces a personalized recommendation with honest performance expectations and tiered install paths. No binary. No installation. See `05-website-spec.md`.

**Phase 1 — the laptop binary.** A Rust daemon on macOS / Windows / Linux. No app sandbox. Other processes call Verdure via local socket. Closes the loop from recommendation to one-click install and run.

**Phase 2 — phones.** Android first (larger emerging-market footprint), iOS second if feasible. Verdure as a system-permissioned app whose background daemon serves inference to other apps via local IPC.

Implementation language for Phase 1 and Phase 2 is Rust — non-negotiable for cross-platform reach. Phase 0 is whatever web stack ships fastest.

## Market context: the RAM crisis

DRAM contract prices roughly doubled through 2025–2026. Mid-range phones are expected to drop from 12 GB to 8 GB. Budget phones may drop from 6–8 GB back to 4 GB. The global device floor is moving *down*, not up.

This makes intelligent orchestration more valuable, not less. Generic instructions like "just run Ollama" break on these devices. Verdure designed for 4 GB targets is aligned with where the market is heading.

## Market validation: Apple proved the thesis (WWDC 2026)

At WWDC 2026, Apple restructured its Foundation Models framework around a public `LanguageModel` protocol — a backend abstraction that lets any conforming model (Apple's on-device system model, Private Cloud Compute, custom Core AI weights, open-weight MLX models, and third-party cloud models from Anthropic and Google) sit behind one Swift API. This is Apple independently arriving at the same conclusion Verdure encoded from day one: the runner is commoditizable; the abstraction seam is where value sits.

What Apple did not build is the decider. Nothing in their stack asks "what are you trying to do?" — it answers *how to call a model*, never *which model you should run and why*. Apple is also structurally disincentivized to build a neutral decider: their framework exists to make Apple's model the default, and a model vendor cannot credibly referee its own model against competitors. Verdure's neutrality is a moat Apple cannot copy without undermining its own stack.

**This makes Apple Silicon the beachhead, not a distraction.** By commoditizing the runner on Mac — a resident system model, MLX, and the `LanguageModel` protocol — Apple removed the most expensive part of Verdure's build (daemon, FFI, model distribution) *on that platform*. So Verdure ships the decider first where the runner is already solved: prove the intent-first thesis on Apple Silicon Macs at low cost, then carry it to the core market. The emerging-market mission (the RAM crisis, Tier P1/P2, billions of users) remains the destination — Mac is the entry wedge that funds and de-risks the road there, not a change of mission.

The pattern to actually watch is Google shipping the analogous architecture on Android (a system model plus an open backend protocol in AICore), because that would land directly on Verdure's core Tier P1/P2 market — and there, unlike on Apple's premium hardware, the decider is the whole game. This argues for moving fast while the Android side of this pattern remains unbuilt. Tracked as `[OQ-44]`.

## Open questions

- [OQ-01] Do we commit to laptops-first formally, or develop Phase 1 and Phase 2 in parallel?
- [OQ-02] For Phase 2: Android first or iOS first?
- [OQ-44] Track Google/Android analog risk: signs of a system model + open backend protocol in AICore that would replicate Apple's WWDC 2026 pattern on Verdure's core market.
