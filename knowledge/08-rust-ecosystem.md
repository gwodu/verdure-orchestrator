# 08 — Rust Ecosystem

**Status:** 🟡 Provisional — primary binding is settled; alternative paths need more research

This file documents Rust crates for ML inference. For the underlying runtime backends, see `07-runtime-backends.md`.

> **Decision content behind the `Backend` seam** (`14-architecture.md`) — specifically the crate/FFI implementation choices. Structure lives there; *which* crate and FFI strategy lives here.

## Production-ready

**`llama-cpp-2`** (utilityai) — actively maintained low-level Rust bindings closely mirroring the llama.cpp C API. Tool-calling support, OpenAI-compatible server option. **This is the primary binding for the Phase 1 binary.**

**`llama-cpp-4`** — a newer iteration in the same lineage. Worth watching but `llama-cpp-2` is the safer v1 choice unless the newer crate offers a specific feature we need.

## Promising, less mature

**`mistral.rs`** — pure Rust, no C++ dependency. Supports quantized models for popular LLM architectures with Apple Silicon, CPU, and CUDA support. Beautiful API. **[GAP — needs research]** Mobile (Android NDK) maturity unclear; production readiness at scale unconfirmed. Tracked as `[OQ-15]`.

**`candle`** (Hugging Face) — well-engineered Rust ML framework. Solid for desktop, weaker for mobile as of latest review. Good for inference workloads where pure-Rust deployment matters more than peak performance.

## Mentioned in the project summary, lower priority

**`burn`** — Rust deep learning framework. **[GAP — needs research]** Current state in 2026 and applicability to LLM inference. Tracked as `[OQ-16]`.

**`tract`** — Rust ONNX/TensorFlow inference engine. Possibly useful for ONNX-format models, not GGUF-focused. **[GAP — needs research]** Maturity and relevance for Verdure's use case. Tracked as `[OQ-17]`.

## FFI strategy

For v1: use `llama-cpp-2` and accept the C++ dependency through llama.cpp. Cross-compilation to Android NDK and iOS toolchain is well-documented for llama.cpp itself. **[GAP — needs research]** Specifics of cross-compiling `llama-cpp-2` to Android NDK for ARMv8-A with NEON. Tracked as `[OQ-18]`.

Long term, if pure-Rust paths mature, Verdure can swap out the binding without changing the orchestration layer above. The architecture isolates the backend choice from the rest of the codebase.

## Recommendation for v1

Start with `llama-cpp-2`. It wraps the most-supported C++ runtime and gives Verdure access to every llama.cpp backend (CPU NEON, AVX, Metal, Vulkan, eventually Hexagon NPU) through a single Rust crate.

## Open questions

- [OQ-15] `mistral.rs` mobile maturity and production readiness.
- [OQ-16] `burn` current state and applicability to LLM inference workloads.
- [OQ-17] `tract` maturity and relevance vs. llama.cpp / GGUF focus.
- [OQ-18] `llama-cpp-2` cross-compilation to Android NDK — specifics and pitfalls.
