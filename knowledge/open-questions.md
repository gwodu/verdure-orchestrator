# Open Questions — Master List

This is the living index of every unresolved question across the Verdure knowledge base. Each entry has a stable `[OQ-XX]` reference code that is cross-linked from the file where the question first appeared.

When a question is resolved:
1. Update the originating knowledge file with the resolution and remove the `[OQ-XX]` marker from the body of the file (keep it in the file's "Open Questions" section as resolved, or remove if no longer relevant).
2. Move the entry here from "Open" to "Resolved" with a one-line note about the resolution.
3. Do not renumber. `[OQ-XX]` codes are permanent.

## Critical — resolve this week

- [ ] **[OQ-07]** Which LLM API for Phase 0 intent classification? Evaluate Claude Haiku 4.5, Gemini Flash, GPT-5 nano, Groq Llama for latency, structured-output reliability, vendor stability. (From `05-website-spec.md`.)
- [ ] **[OQ-09]** Phase 0 tech stack pick. Any reasonable choice works; pick one and commit. (From `05-website-spec.md`.)
- [ ] **[OQ-26]** iOS feasibility for the daemon model. Strategic blocker for Phase 2 on iOS. (From `11-platform-deployment.md`.)
- [ ] **[OQ-03]** Physical device access for Tier P1 / P2 / L1 testing. Blocks publishing real-world benchmarks. (From `02-hardware-tiers.md`.)
- [ ] **[OQ-19]** Hugging Face TOS at scale — rate limits, bandwidth costs. Phase 1 blocker. (From `10-distribution.md`.)

## Important — resolve in Month 1

- [ ] **[OQ-01]** Commit formally to laptops-first, or develop Phase 1 and Phase 2 in parallel. (From `01-thesis.md`.)
- [ ] **[OQ-02]** Phase 2 first OS — Android or iOS? (From `01-thesis.md`.)
- [ ] **[OQ-04]** Practical floor — support 4 GB phones or start at 6 GB? (From `02-hardware-tiers.md`.)
- [ ] **[OQ-05]** Real tokens/sec on physical Unisoc T606 and Helio G81. (From `03-model-matrix.md`.)
- [ ] **[OQ-06]** Validate intent → model preference assumptions with real users. (From `03-model-matrix.md`.)
- [ ] **[OQ-11]** Visual design language for the website. (From `05-website-spec.md`.)
- [ ] **[OQ-12]** Spec database for free-text device parsing — LLM-only or integrate EveryMac / Notebookcheck. (From `05-website-spec.md`.)
- [ ] **[OQ-13]** ExecuTorch 1.0 integration specifics on Android. (From `07-runtime-backends.md`.)
- [ ] **[OQ-15]** `mistral.rs` mobile maturity and production readiness. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-18]** `llama-cpp-2` cross-compilation to Android NDK. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-20]** CDN economics for global model mirroring. (From `10-distribution.md`.)
- [ ] **[OQ-21]** Curated catalog governance — criteria, version pinning. (From `10-distribution.md`.)
- [ ] **[OQ-23]** Apple Developer Program pricing and 2026 notarization policy. (From `11-platform-deployment.md`.)
- [ ] **[OQ-24]** Windows code signing pricing, CA recommendations, SmartScreen timelines. (From `11-platform-deployment.md`.)
- [ ] **[OQ-25]** Android 14+ background service policies and Play Store requirements. (From `11-platform-deployment.md`.)
- [ ] **[OQ-27]** Add a path-filtered GitHub Actions CI job for the website (`website/`) — run `npm test` + `npm run build` whenever `website/` changes. (The monorepo already has Rust CI for `cargo test`; this OQ tracks the parallel gap on the website side. Monorepo as of 2026-05-30.) (From `skills/testing.md`.)
- [ ] **[OQ-28]** Real on-device throughput for non-text models (Whisper sizes, SD/SDXL, LTX-2) across the three capability classes. Parallels `[OQ-05]` for text. (From `12-multimodal-models.md`.)
- [ ] **[OQ-29]** The five-tier → three-capability-class collapse can't capture VRAM / unified-memory *amount*, which decides whether image editing and video actually run. Does the device picker need a graphics-memory signal, asked in non-technical terms? (From `12-multimodal-models.md`.)
- [ ] **[OQ-31]** Validate the modality keyword classifier against real user phrasings; the eventual LLM classifier (`[OQ-07]`) must classify modality, not just text intent. (From `12-multimodal-models.md`.)
- [ ] **[OQ-32]** Intent cards on Screen 1 — let users pick what they want to do from cards (mirroring the device cards), e.g. Audio, Speech-to-text, Video generation, General/automation, with a free-text fallback. Recognition over recall for non-technical users. Follow the `website/lib/tiers.ts` `DEVICE_CARDS` pattern; each card maps to the `(modality, task)` taxonomy in `website/lib/modality.ts`. The card set should be grounded in `[OQ-33]`, not guessed. (From `05-website-spec.md`.)
- [ ] **[OQ-33]** **Demand-side use-case research (strategic).** We researched the *supply* side — which models exist and how they perform (`03`, `12`) — but never the *demand* side: the real breadth AND prevalence of what people actually use AI for (coding, writing, speech-to-text, image gen, video gen, translation, summarization, automation, …). The current intent/modality taxonomy was derived from available models, not real usage. Research actual demand so the orchestration is *planned from use cases*, then build the intent layer (cards `[OQ-32]`, taxonomy in `website/lib/modality.ts`) and validate it (`[OQ-06]`, `[OQ-31]`) from that. This is the missing half of intent-first orchestration. (From `01-thesis.md`.)

## Defer — Month 2 and later

- [ ] **[OQ-14]** MLC LLM vs. llama.cpp+Metal benchmark gap on Apple Silicon. (From `07-runtime-backends.md`.)
- [ ] **[OQ-16]** `burn` current state and applicability to LLM inference. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-17]** `tract` maturity and relevance vs. GGUF focus. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-22]** Model metadata schema — extend HF format or roll our own. (From `10-distribution.md`.)

## Resolved

- [x] **[OQ-08]** Domain — resolved 2026-05-26: the Phase 0 site is verdureai.com. (From `05-website-spec.md`.)
- [x] **[OQ-30]** Music & video generation for non-technical users — resolved 2026-05-29: **recommend them, keeping the honest "this is hard / for comfortable users / needs ComfyUI" framing.** The friction is on-strategy: showing how difficult these are to set up yourself is exactly what demonstrates Verdure's value. Do not hide or over-soften the difficulty. (From `12-multimodal-models.md`.)
- [x] **[OQ-10]** Launch-list capture mechanism — resolved 2026-05-29: Google Sheets, via a service account appending one row per signup in the `/api/waitlist` route. Chosen over Mailchimp/Buttondown/etc. for zero cost, zero new vendor, and a directly readable list at Phase 0 scale. Implemented in `website/lib/sheets.ts`; setup in `website/SETUP_SHEETS.md`. (From `05-website-spec.md`.)
