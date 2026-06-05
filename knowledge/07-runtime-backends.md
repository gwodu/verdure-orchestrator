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

## Secondary backend: MLC LLM (Apple Silicon path)

**MLC LLM** uses compiled inference kernels and is among the fastest options on Apple Silicon. Potentially useful for Tier L3 macOS deployments if benchmarks justify the integration complexity.

For v1, default to llama.cpp with Metal. Revisit MLC if benchmarks show a meaningful gap that affects the user experience on M-series Macs.

## Long-term: pure-Rust backends

If `mistral.rs` or `candle` (see `08-rust-ecosystem.md`) mature to production-ready status on mobile, they offer a pure-Rust path that removes the C++ FFI dependency. This is a future direction, not a v1 commitment.

## What Verdure adds above the backend

The user never picks a backend. Verdure detects the device and routes to the fastest legal path:

- Default: llama.cpp via `llama-cpp-2`
- Snapdragon with Hexagon NPU (Phase 2): ExecuTorch with NPU delegation
- MediaTek with APU (Phase 2): ExecuTorch with APU delegation
- Apple Silicon: llama.cpp with Metal (or MLC if justified)

To the caller, every model speaks one API. Backend selection is invisible.

## Open questions

- [OQ-13] ExecuTorch 1.0 integration specifics on Android — exact chipset coverage, packaging, NDK complexity.
- [OQ-14] MLC LLM vs. llama.cpp+Metal benchmark gap on Apple Silicon — does the integration cost justify the speedup?
