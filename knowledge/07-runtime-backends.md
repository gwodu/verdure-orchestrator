# 07 — Runtime Backends

**Status:** 🟡 Provisional — primary backend choice is settled; secondary backends need more research before commitment

This file documents the inference runtime backends Verdure uses or routes to for **text** models. For the Rust bindings that wrap these backends, see `08-rust-ecosystem.md`.

> **Decision content behind the `Backend` seam** (`14-architecture.md`). Structure (the trait, how it composes) lives there; *which* engine and why lives here.

**Multimodal note (2026-05-29):** non-text models use different runtimes — whisper.cpp / faster-whisper (audio transcription), ONNX (TTS), diffusers / ComfyUI and ncnn-vulkan (image), ComfyUI (video). In Phase 0, Verdure only *refers* users to GUI apps that bundle these (MacWhisper, Upscayl, Draw Things, etc.; see `12-multimodal-models.md`), so no backend integration is needed. Wiring non-text backends into the Phase 1 binary is out of scope for v1 and not yet an open question — flag it when Phase 1 scope is set.

## Primary backend: llama.cpp

**llama.cpp** is the de facto standard for GGUF-based inference. It runs on every chipset Verdure targets:
- ARM Cortex-A53 / A55 and up via NEON kernels (covers all Phase 2 phone tiers)
- x86-64 with AVX, AVX2, AVX-512, AVX-VNNI (covers Phase 1 laptops)
- Apple Silicon via Metal
- Cross-platform GPU via Vulkan

Verdure binds to llama.cpp via the `llama-cpp-2` Rust crate (see `08-rust-ecosystem.md`). This is the workhorse for v1 of the Phase 1 binary.

## Secondary backend: ExecuTorch (Android NPU path)

**ExecuTorch** is Meta's edge inference runtime. As of ExecuTorch 1.0 (released October 2025), it supports 12+ hardware backends and over 80% of popular edge models work out of the box. Relevant for Verdure: NPU acceleration on Snapdragon (Hexagon), MediaTek (APU), and Apple Neural Engine.

**[GAP — needs research]** Specifics of ExecuTorch integration on Android: which exact chipsets benefit, packaging requirements, NDK build complexity, runtime size on disk. Tracked as `[OQ-13]`.

For v1 of the Phase 1 laptop binary, ExecuTorch is not yet wired in. It becomes relevant when Phase 2 targets Android devices with NPUs.

## Secondary backend: MLX (Apple Silicon path)

**MLX** is Apple's open-source ML framework, now the performance leader on Apple Silicon — reported 30–40% faster than llama.cpp on M5-generation hardware, with Metal 4 support. Ollama itself now routes through MLX on Apple Silicon. As of WWDC 2026, Apple is open-sourcing `MLXLanguageModel`, a Foundation Models–conforming wrapper that runs open-weight models from the Hugging Face mlx-community.

This supersedes the earlier MLC LLM consideration. `[OQ-14]` is resolved: the relevant Apple Silicon comparison is MLX vs. llama.cpp+Metal, and the gap is large enough to justify integration for Tier L3.

Two integration routes for a Verdure `MlxBackend`, unresolved:
1. **`mlx-rs` from the Rust core** — unofficial Rust bindings over Apple's `mlx-c` C API. Preserves single-binary purity. **[GAP — needs research]** production maturity of `mlx-rs` for LLM inference. Tracked as `[OQ-41]`.
2. **Swift sidecar worker** — a small Swift binary linking MLX Swift, spawned by the supervisor and speaking the worker protocol. Requires the heterogeneous-worker `Executor` extension (see `14-architecture.md`). More moving parts, but rides Apple's first-party code path.

Note `MLXLanguageModel`'s platform availability on iOS (vs. Mac-only) is unconfirmed from beta documentation. Tracked as `[OQ-42]`.

## Zero-download backend: Apple system model

On Apple Intelligence devices, the OS provides a resident ~3B model via the Foundation Models framework (`SystemLanguageModel`, Swift API). A Verdure `AppleSystemBackend` adapter exposes it through the `Backend` trait like any other engine — except there is nothing to download, store, or manage. See `03-model-matrix.md` (zero-download recommendation) and `14-architecture.md` (resident `ModelChoice`).

Caveats: closed weights, fixed size, updated on Apple's schedule, Swift-only API (implies the sidecar route or a thin Swift shim). All Foundation Models / MLXLanguageModel APIs are iOS 27 / macOS 27 developer beta as of June 2026 — treat signatures as provisional until the fall public release.

## Long-term: pure-Rust backends

If `mistral.rs` or `candle` (see `08-rust-ecosystem.md`) mature to production-ready status on mobile, they offer a pure-Rust path that removes the C++ FFI dependency. This is a future direction, not a v1 commitment.

## What Verdure adds above the backend

The user never picks a backend. Verdure detects the device and routes to the fastest legal path:

- Default: llama.cpp via `llama-cpp-2`
- Snapdragon with Hexagon NPU (Phase 2): ExecuTorch with NPU delegation
- MediaTek with APU (Phase 2): ExecuTorch with APU delegation
- Apple Silicon (Tier L3): MLX via `MlxBackend` when justified; `AppleSystemBackend` (resident system model) for intents within its quality ceiling; llama.cpp with Metal as the universal fallback

To the caller, every model speaks one API. Backend selection is invisible.

## Open questions

- [OQ-13] ExecuTorch 1.0 integration specifics on Android — exact chipset coverage, packaging, NDK complexity.
- [OQ-14] Resolved (WWDC 2026) — superseded: MLX replaced MLC as the Apple Silicon candidate, and the 30–40% gap over llama.cpp+Metal on M5 is confirmed meaningful.
- [OQ-41] `mlx-rs` production maturity for LLM inference — viable from the Rust core, or is the Swift sidecar worker the safer route?
- [OQ-42] `MLXLanguageModel` platform availability — Mac-only or also iOS? Determines whether open-weight MLX models are reachable on iPhone through Apple's framework.
