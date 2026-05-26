# Architecture

How to structure code so Verdure can grow for a decade without collapsing under its own weight.

## The metric

**Complexity is the enemy.** Every architectural decision is measured by whether it adds or removes complexity, where complexity means: how hard is it for the next person (or Claude in a new session) to understand and safely change this code?

Two specific failure modes to watch for:

- **Change amplification** — a small conceptual change requires edits in many places. This means the design decision wasn't encapsulated; it leaked.
- **Cognitive load** — understanding any one piece requires holding too much else in your head.

If either is growing, the architecture is rotting. Address it.

## Three principles to evaluate any decision

From Google's "programming integrated over time":

1. **Time.** Will this still make sense when the codebase is ten times its current size? When a backend has been swapped twice? When the team has turned over?
2. **Scale.** Does this approach work when called once per minute? Once per second? At a million users?
3. **Tradeoffs.** Every architectural choice is a trade. Name what you're giving up. If you can't, you don't understand the choice yet.

## Deep modules, not many small ones

A module's *cost* is its interface (what other code must understand to use it). Its *benefit* is its functionality (what it actually does). Good modules maximize benefit per unit of interface — they're **deep**: simple to use, rich inside.

**Red flag:** a module whose interface is nearly as complex as its implementation. That's a shallow module — net negative, because the reader has to understand both sides.

This sets up a real tension with Clean Code's "many small functions" prescription. The reconciliation: prefer depth over fragmentation, but a function still needs to fit in your head. Split a function when it genuinely does unrelated things; don't split it just to make each piece shorter. A function with substance and a clear contract is better than three one-liners that exist only to be named.

## Hexagonal architecture (ports and adapters) — Verdure's structural pattern

Verdure's situation — a core orchestration brain that must work behind multiple swappable inference backends, model sources, storage layers, and UI surfaces — is exactly what hexagonal architecture exists for.

The pattern in Rust:

- **Domain (the inside of the hexagon)** — pure logic about hardware tiers, model classes, recommendations, intent matching. No I/O. No knowledge of llama.cpp, no knowledge of the filesystem, no knowledge of HTTP. Just types and rules.
- **Ports** — traits that define what the domain needs from the outside world (`trait InferenceBackend`, `trait ModelSource`, `trait HardwareDetector`).
- **Adapters** — concrete implementations of those traits (`LlamaCppAdapter`, `HuggingFaceSource`, `MacOsDetector`). Lives in `infrastructure/`.

The rule: the domain never imports an adapter. It only sees ports (traits). This is what lets `llama-cpp-2` be replaced with `mistral.rs` later without rewriting the orchestrator. See `../knowledge/07-runtime-backends.md` and `../knowledge/08-rust-ecosystem.md`.

Workspace layout:

```
crates/
  verdure-domain/          # pure types and logic, no I/O
  verdure-application/     # orchestration, the seven decisions
  verdure-infrastructure/  # FFI bindings, network, filesystem (adapters)
  verdure-cli/             # presentation (later)
  verdure-daemon/          # presentation (later)
```

Dependencies point inward only: `cli` and `daemon` depend on `application`; `application` depends on `domain`; `infrastructure` depends on `domain` (to implement its traits) but `domain` never depends on `infrastructure`.

## Domain-Driven Design — name the things first

Before writing orchestration logic, name the domain entities. Verdure's domain has rich vocabulary already documented in `../knowledge/`: HardwareTier, ModelClass, QuantizationLevel, Intent, Recommendation, Backend.

Model these as first-class Rust types, not bags of fields:

```rust
// not this
fn pick_model(ram_gb: u8, intent: &str) -> String { ... }

// this
fn recommend(device: Device, intent: Intent) -> Recommendation { ... }
```

Ubiquitous language: if the knowledge base calls it "Tier L2-high," the code says `Tier::L2High`. If it calls it "Gemma 3 4B at Q4_K_M," the code says `Model::Gemma3_4B { quant: Quantization::Q4KM }`. The knowledge base and the codebase share one vocabulary. You can read the code aloud and a non-engineer can follow.

## Design it twice

For any non-trivial interface, sketch two materially different designs before committing. Not two phrasings of the same idea — two genuinely different approaches. Pick the one that's deeper (more capability behind less surface). The first design that works is almost never the best one.

## GoF patterns, translated to Rust idiom

Classic patterns matter for shared vocabulary, but Rust expresses them differently:

- Strategy → trait with multiple implementations + enum dispatch (or `Box<dyn Trait>` when needed)
- Adapter → wrapper struct that implements a target trait over a foreign API (this is literally what `llama-cpp-2` is — an adapter over llama.cpp)
- Factory → trait with associated functions, or a registry that returns `Box<dyn Trait>`
- Builder → Rust's idiomatic builder pattern (chained methods returning `Self`)
- Observer → channels (`std::sync::mpsc` or `tokio::sync::broadcast`) — not shared mutable state with callbacks

Knowing the name lets you communicate the design move. Implementing it as the Java textbook does it is usually wrong in Rust.

## Document every architectural decision with a reason

Any choice that affects how multiple pieces of code fit together: write down why. A short ADR (Architecture Decision Record) in the codebase, or a note in `FEATURE_LOG.md`. "Chose X over Y because Z" — if you can't fill that in, you haven't decided, you've defaulted.

## Self-check before declaring an architectural change done

- Does this make Verdure simpler or more complex? If more complex, is the added capability worth it?
- Is the domain still pure (no infrastructure imports)?
- Could a backend be swapped without touching the domain?
- Is every dependency on the outside world behind a port (trait) the domain owns?
- Will this make sense in five years?
