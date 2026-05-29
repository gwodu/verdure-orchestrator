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
- [ ] **[OQ-10]** Email capture mechanism for Phase 1 launch list. (From `05-website-spec.md`.)
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
- [ ] **[OQ-27]** Set up GitHub Actions CI for the `verdure_web` repo — run `npm test` + `npm run build` on every push and pull request to `main`. (The `verdure-orchestrator` repo already has CI for `cargo test`; this OQ tracks the parallel gap on the website side.) (From `skills/testing.md`.)

## Defer — Month 2 and later

- [ ] **[OQ-14]** MLC LLM vs. llama.cpp+Metal benchmark gap on Apple Silicon. (From `07-runtime-backends.md`.)
- [ ] **[OQ-16]** `burn` current state and applicability to LLM inference. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-17]** `tract` maturity and relevance vs. GGUF focus. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-22]** Model metadata schema — extend HF format or roll our own. (From `10-distribution.md`.)

## Resolved

- [x] **[OQ-08]** Domain — resolved 2026-05-26: the Phase 0 site is verdureai.com. (From `05-website-spec.md`.)
