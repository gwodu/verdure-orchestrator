# Open Questions — Master List

This is the living index of every unresolved question across the Verdure knowledge base. Each entry has a stable `[OQ-XX]` reference code that is cross-linked from the file where the question first appeared.

When a question is resolved:
1. Update the originating knowledge file with the resolution and remove the `[OQ-XX]` marker from the body of the file (keep it in the file's "Open Questions" section as resolved, or remove if no longer relevant).
2. Move the entry here from "Open" to "Resolved" with a one-line note about the resolution.
3. Do not renumber. `[OQ-XX]` codes are permanent.

## Critical — resolve this week

- [ ] **[OQ-57]** AI-first dictation (`20`): a system-wide dictation/command button (accessibility surface) that feeds transcription→personal-context AND commands→Verdure — voice as a command channel, not just a text field. The Wispr Flow disruptor, AI-first. Builds on tool-calling (`[OQ-55]`) + the context store (`[OQ-56]`). (From `20-personal-ai.md`.)
- [ ] **[OQ-56]** The personal context core (`20`): a local, user-owned store of the user's dictations/preferences/history that every model + agent reads and writes, keeping the user at the center of context. Shape, privacy model (`06`), portability. (From `20-personal-ai.md`.)
- [ ] **[OQ-55]** Conversational front door (`18`): the LLM modality (llama.cpp-in-binary crate + GGUF model download), grammar-constrained (GBNF) tool-calling over a small closed tool set, a tool-calling-capable small model (Qwen 2.5 / Llama 3.2 3B), deterministic cross-check + confirm, and the chat-first UI. The interface through which capabilities (incl. dictation) get invoked. (From `18`.)
- [ ] **[OQ-54]** Live on-device dictation (`19`): mic-capture crate + permission, VAD choice, chunk/segment strategy, and the optional local-LLM cleanup pass (needs the LLM modality). The next build after the performance predictor. (From `19-performance-and-dictation.md`.)
- [ ] **[OQ-53]** Confirm the first wrapped slice for the unified interface (`18`): transcription via **whisper.cpp** (light, fixes the Buzz handoff) vs chat via **Ollama** (highest demand, heavier). Plus the shell's panel/navigation model for spanning model families. (From `18-unified-interface.md`.)
- [ ] **[OQ-52]** Reconcile `01`/`14`/`15`/`16`/`17` to the 2026-06-18 pivot (`18`): the reversed bright line (build one surface over headless engines, never build a model/engine), the redefined atomic unit, Tool layer → "engine layer". Do before deep build. (From `18-unified-interface.md`.)
- [ ] **[OQ-07]** Which LLM API for Phase 0 intent classification? Evaluate Claude Haiku 4.5, Gemini Flash, GPT-5 nano, Groq Llama for latency, structured-output reliability, vendor stability. (From `05-website-spec.md`.)
- [ ] **[OQ-09]** Phase 0 tech stack pick. Any reasonable choice works; pick one and commit. (From `05-website-spec.md`.)
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
- [ ] **[OQ-34]** Should `document`/RAG and `companionship` become first-class tasks (taxonomy + model-matrix entries + Screen 1 cards)? Both are large local-native uses the current `(modality, task)` taxonomy omits. (From `13-use-case-demand.md`.)
- [ ] **[OQ-35]** `Executor` on Android — is `InProcessExecutor` (FFI, no isolation) the only viable strategy, or can a foreground-service worker give partial crash isolation? Phase 2 process-model blocker. (Related to `[OQ-25]`.) (From `14-architecture.md`.)
- [ ] **[OQ-36]** `Transport` cross-app IPC — the API surface (streaming, cancellation, concurrent sessions) AND the per-app authorization model (socket file-perms vs. explicit user consent per calling app), and whether either leaks across Unix socket / named pipe / Binder into the trait. (From `14-architecture.md`.)
- [ ] **[OQ-37]** Core-purity litmus — gate the core on compiling to `wasm32` in CI as cheap proof no platform detail leaked? Decide if the gate is worth the build cost. (From `14-architecture.md`.)
- [ ] **[OQ-38]** Eviction policy under memory pressure — LRU, intent-priority, or pinned-plus-LRU? Core logic; needs a default before the lifecycle manager. (From `14-architecture.md`.)
- [ ] **[OQ-39]** Concurrency & resource arbitration for the daemon — resident-model count, request queueing/fairness across concurrent callers, per-session context, interaction with eviction and the `Executor`. (From `14-architecture.md`.)
- [ ] **[OQ-40]** Model catalog/registry — who resolves a model ID to `(hash, metadata, file)`, and how it updates/versions across an always-latest website and pinned installed apps. Maybe its own seam. (Ties to `[OQ-21]`/`[OQ-22]`.) (From `14-architecture.md`.)
- [ ] **[OQ-41]** `mlx-rs` production maturity for LLM inference vs. the Swift sidecar worker route for `MlxBackend`. (From `07-runtime-backends.md`.)
- [ ] **[OQ-42]** `MLXLanguageModel` iOS availability — Mac-only or iPhone too? Determines whether open-weight MLX models are reachable on iOS through Apple's framework. (From `07-runtime-backends.md`.)
- [ ] **[OQ-45]** The Tool layer — when to introduce the Tool manifest (gated on the 2nd concrete tool), the two port classes (standalone-app vs assistant-config on a hosted shell), and which intents become the first runnable Mac tools (writing assistant + doc-RAG). Leading approach: port AnythingLLM/Jan and configure via API, never build a shell (see `15-port-catalog.md`). (From `14-architecture.md`.)
- [ ] **[OQ-46]** Verify **AnythingLLM / Jan on the Intel-Mac target**, and prove the unattended REST/file config as the port mechanism for assistant-style tools. (From `15-port-catalog.md`.)
- [ ] **[OQ-47]** Composition layer (pluggable tools) — build the first concrete chain (**Buzz → AnythingLLM**, watch-folder → REST), then a second (**whisper.cpp → Ollama**) to earn the typed-port / 3-field pipe contract. Hard cap at 2 nodes — no branches, transforms, or model-directed routing (that's the deferred agentic line). GUI-only tools (UVR) need a headless equivalent ported. (From `14-architecture.md`.)
- [ ] **[OQ-48]** Does a privacy-preserving, user-controlled **persona-sharing** layer ever get built (the only candidate network effect), or does Verdure stay a pure utility? Optional and far-future; the data-harvesting moat is rejected outright (`06`). Atomic unit = one OSS tool installed & running. (From `16-growth-asset.md`.)

## Defer — Month 2 and later

- [ ] **[OQ-16]** `burn` current state and applicability to LLM inference. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-17]** `tract` maturity and relevance vs. GGUF focus. (From `08-rust-ecosystem.md`.)
- [ ] **[OQ-22]** Model metadata schema — extend HF format or roll our own. (From `10-distribution.md`.)
- [ ] **[OQ-43]** `VerdureLanguageModel` Swift package — ship decision and phasing. Strategic distribution channel, not just engineering. (From `11-platform-deployment.md`.)
- [ ] **[OQ-44]** Track Google/Android analog of Apple's `LanguageModel` pattern (a system model + open backend protocol in AICore) landing on the core P1/P2 market. (From `01-thesis.md`.)

## Resolved

- [x] **[OQ-50]** Porter end-to-end verification — resolved 2026-06-16 via CI: a GitHub Actions macOS runner compiled the Tauri backend, ran `cargo test` (9 porter tests), and ran the real `brew install --cask buzz` — all green (run `27621986664`). Moved off the founder's Mac (no Xcode CLT + tight disk). Only the GUI launch (`open -a Buzz`) is uncovered (needs a desktop session). (From `17-build-plan.md`.)
- [x] **[OQ-49]** MVP app stack/location + decider sharing — resolved 2026-06-16: **Tauri v2 in `app/`** (Vite + vanilla-TS frontend), decider shared as a **workspace package `@verdure/decider`** (`packages/decider/`), imported by both website and porter (not a copy). Ships TS source; npm workspaces symlink it. See `17-build-plan.md` "Decided while building". (From `17-build-plan.md`.)
- [x] **[OQ-08]** Domain — resolved 2026-05-26: the Phase 0 site is verdureai.com. (From `05-website-spec.md`.)
- [x] **[OQ-14]** MLC vs. llama.cpp+Metal — superseded (WWDC 2026): MLX replaced MLC as the Apple Silicon candidate; 30–40% gap over llama.cpp on M5 confirmed meaningful. See `07-runtime-backends.md`.
- [x] **[OQ-26]** iOS daemon feasibility — resolved (WWDC 2026): daemon infeasible, confirmed; strategy pivoted to a thin decider over Apple's Foundation Models stack (the OS now ships the shared inference service). See `11-platform-deployment.md`.
- [x] **[OQ-32]** Intent cards on Screen 1 — done 2026-06-01: six research-grounded cards (`INTENT_CARDS` in `website/lib/modality.ts`) — Everyday questions/chat, Writing & ideas, Coding & tech help, Turn recordings into text, Create images, Languages & translation — plus a free-text fallback for the long tail. Set chosen from `13-use-case-demand.md`; deliberately **no** video-generation card (lowest demand, least local-feasible). A unit test locks each card's example phrase to its `(modality, task)`. (From `05-website-spec.md`.)
- [x] **[OQ-33]** Demand-side use-case research — done 2026-06-01: researched (4-source fan-out) and synthesized into `13-use-case-demand.md`, the demand leg of the matching brain. Key finding: local-AI demand concentrates on transcription, coding, image generation, private RAG, and roleplay/uncensored content (privacy / cost / no-filter drivers); video generation is the weakest case. Drove the Screen 1 intent cards (`[OQ-32]`). Ongoing user validation tracked by `[OQ-06]` / `[OQ-31]`. (From `01-thesis.md`.)
- [x] **[OQ-30]** Music & video generation for non-technical users — resolved 2026-05-29: **recommend them, keeping the honest "this is hard / for comfortable users / needs ComfyUI" framing.** The friction is on-strategy: showing how difficult these are to set up yourself is exactly what demonstrates Verdure's value. Do not hide or over-soften the difficulty. (From `12-multimodal-models.md`.)
- [x] **[OQ-10]** Launch-list capture mechanism — resolved 2026-05-29: Google Sheets, via a service account appending one row per signup in the `/api/waitlist` route. Chosen over Mailchimp/Buttondown/etc. for zero cost, zero new vendor, and a directly readable list at Phase 0 scale. Implemented in `website/lib/sheets.ts`; setup in `website/SETUP_SHEETS.md`. (From `05-website-spec.md`.)
