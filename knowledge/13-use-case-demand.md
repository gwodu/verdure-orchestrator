# 13 — Use-Case Demand Map

**Status:** 🟡 Provisional (grounded in 2024–2026 secondary research, 2026-06-01; no rigorous survey of *local-AI* users by use case exists publicly — treat rankings as directional, not statistical).

This file is the **demand side** of the matching brain. The other legs answer supply: `02-hardware-tiers.md` (device → tier) and `03-model-matrix.md` + `12-multimodal-models.md` (tier → model). This file answers the question those can't: **what do people actually want to do?** — so the intent layer (the Screen 1 cards in `05-website-spec.md`, the `(modality, task)` taxonomy in `website/lib/modality.ts`) is planned from real demand, not guessed from the model catalog. Resolves the moat gap flagged in `01-thesis.md` and `[OQ-33]`.

## The one insight that matters

**For *local* AI, demand ranks differently than for AI in general.** Mainstream "how people use ChatGPT" telemetry (the best volume data) says the top uses are advice/practical-guidance (~29%), writing (~24%), and information-seeking (~24%), with coding small (~5%) and media tiny. But Verdure's users run AI *locally*, and the things that push someone to go local are **privacy, no subscription cost, no rate limits, offline, and no content filters.** Those motivations concentrate demand into a different, narrower set — and several of those use cases are *systematically under-counted* by chat-based surveys:

- **Transcription (speech-to-text)** is one of the most-used practical AI tasks and the single largest *local* non-text workload (Whisper: top open ASR model, ~4M downloads/month; audio ≈ 15% of all Hugging Face downloads). Driven by privacy ("confidential audio never leaves the machine"). Nearly invisible in chat surveys.
- **Coding** is small in consumer ChatGPT data (~5%) but the top *local-native* use — devs run local models to keep proprietary code private and avoid API cost (84–90% of developers use AI; local coding is a real, privacy-driven niche).
- **Image generation** is the most prevalent deliberate non-text task (~34% of US adults) and has a large local tier (Stable Diffusion / ComfyUI / Civitai), driven by uncensored output, unlimited free generation, and custom models/LoRAs.
- **Private document Q&A / RAG** ("chat with my files") and **roleplay / companionship / uncensored content** are large *local-native* categories — the latter near-invisible in surveys (under-disclosed) yet huge in tooling (SillyTavern, the "uncensored model" genre).

**Corollary that changed the card design: video generation is the weakest case, not a headline.** It has the most hype but the least consumer adoption (OpenAI shut down the Sora app in 2026 after demand cratered), and it's the least local-feasible on Verdure's hardware tiers (`12-multimodal-models.md`). Making it a prominent Screen 1 card would funnel users toward a "your device can't do this yet" dead-end. It stays reachable via free-text (and gets the honest decline), but it is **not** a card.

## Demand taxonomy → Verdure's `(modality, task)`

Ranked by relevance to a *local-AI* audience (a blend of overall prevalence and "local-native" pull). "Card?" = surfaced as a Screen 1 intent card.

| Use case | `(modality, task)` | Demand (local-weighted) | Why local | Card? |
|---|---|---|---|---|
| Everyday questions, advice, chat | `text/general` | Very high (advice is the #1 general use) | Weak (cloud is fine) — but the universal entry point | ✅ |
| Writing & ideas (stories, posts, email, brainstorm) | `text/creative` | Very high | Privacy of drafts; unlimited; uncensored | ✅ |
| Coding & technical help | `text/code` | High (top local-native) | **Strong** — proprietary code privacy, cost | ✅ |
| Turn recordings into text (transcription) | `audio/transcription` | High (top local non-text) | **Strong** — confidential audio stays local | ✅ |
| Create images | `image/generate` | High | **Strong** — uncensored, free, custom models | ✅ |
| Languages & translation | `text/multilingual` | Medium-high (steady) | Offline; document privacy | ✅ |
| Private document Q&A / RAG | (no task yet → `text/general`) | High local-native | **Strong** — files never leave device | ⏳ candidate (needs a task) |
| Roleplay / companionship / uncensored | (`text/creative` / `general`) | High local, under-reported | **Strong** — no content filter | ⏳ deferred (sensitive; validate) |
| Summarization | (sub-task of the above) | High but rarely standalone | follows transcription/RAG | ➖ via free-text |
| Speech / text-to-speech | `audio/speech` | Medium (creator-led) | offline/private narration | ➖ via free-text |
| Image edit / upscale | `image/edit`, `image/upscale` | Medium | local, free | ➖ via free-text |
| Home automation / agentic "do it for me" | (none) | Emerging; gated by trust | **Strong** (private always-on) | ➖ future |
| Music generation | `audio/music` | Growing (cloud-led: Suno) | weak locally | ➖ via free-text |
| Video generation | `video/generate` | **Lowest** adoption; most hype | not local-feasible on our tiers | ❌ free-text → honest decline |

**Long tail → free text.** Screen 1 keeps a "describe it in your own words" fallback so the unsurfaced tasks (edit/upscale, TTS, music, separation, video, RAG, automation) still route through the classifier (`website/lib/intent.ts`) to the right `(modality, task)` — including the honest "not-local-yet" verdicts.

## The Screen 1 intent cards (the actionable output → `[OQ-32]`)

Six cards, ordered to lead with the universal/high-demand and the strongest local-native cases, mirroring the device cards' recognition-first pattern. Each maps to a distinct `(modality, task)` and ships in `website/lib/modality.ts` as `INTENT_CARDS`:

1. **Everyday questions & chat** → `text/general`
2. **Writing & ideas** → `text/creative`
3. **Coding & tech help** → `text/code`
4. **Turn recordings into text** → `audio/transcription`
5. **Create images** → `image/generate`
6. **Languages & translation** → `text/multilingual`

Plus a free-text "Something else — describe it" path for the long tail. Each card carries a canonical example phrase that the keyword classifier maps to the card's declared `(modality, task)`; a unit test locks `classifyIntent(card.example) === card.classification` so cards and classifier can't drift.

## What to revisit (research is directional, not final)

- **Add a `document`/RAG task and a `companionship` task?** Both are large *local-native* categories the current taxonomy doesn't represent (RAG maps to nothing; companionship folds into `text/general`). Strong candidates for the next card + a model-matrix entry — but RAG needs an embeddings/retrieval story and companionship is sensitive; validate before surfacing. See `[OQ-34]`.
- **Validate against real users** — every ranking here is inferred from secondary sources; no representative survey of local-AI users by use case exists. Pair with `[OQ-06]` (intent→model assumptions) and `[OQ-31]` (classifier vs real phrasings).
- **Local-vs-cloud share** for image/music/video is estimated, not measured.

## Key sources

- OpenAI / NBER, *How People Use ChatGPT* (WP 34255, 2025) — best volume taxonomy. https://www.nber.org/papers/w34255
- Filtered.com / HBR, *How People Are Really Using Gen AI* (2024 top-100; 2025 update). https://hbr.org/2025/04/how-people-are-really-using-gen-ai-in-2025
- a16z, *Top 100 Gen AI Consumer Apps* (6th ed., 2026) & *State of Consumer AI 2025*. https://a16z.com/100-gen-ai-apps-6/
- Pew Research, *34% of U.S. adults have used ChatGPT* (2025). https://www.pewresearch.org/short-reads/2025/06/25/
- Stack Overflow 2025 & JetBrains 2025/26 developer surveys (coding). https://survey.stackoverflow.co/2025/ai
- Hugging Face model-download stats (modality mix; LLMs ≈ 9.5% of downloads). https://huggingface.co/blog/lbourdois/huggingface-models-stats
- Whisper adoption (transcription as top local non-text task). https://www.aboutchromebooks.com/whisper-statistics/
- Menlo Ventures *State of Consumer AI 2025* & Artificial Analysis *Generative Media Survey 2025* (image > video). https://menlovc.com/perspective/2025-the-state-of-consumer-ai/
- Local-AI usage (community/tooling, directional only): SillyTavern, AnythingLLM, Civitai, Ollama library pull counts.

## Open questions

- [OQ-34] Should `document`/RAG and `companionship` become first-class tasks (taxonomy + model-matrix entries + cards)? Both are large local-native uses the current taxonomy omits. (From `13-use-case-demand.md`.)
