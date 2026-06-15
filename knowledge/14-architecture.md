# 14 — Architecture

**Status:** 🟡 Provisional (the seams and layering are settled; specific trait signatures will firm up as v1 code lands)

This file answers: **how is Verdure structured so that any single part can be replaced without disturbing the rest?**

It is the structural companion to the matching brain (`02-hardware-tiers.md` + `03-model-matrix.md` + `12-multimodal-models.md` + `13-use-case-demand.md`). Those files decide *what to recommend*; this file decides *how the system is built so the decision can be executed on any device, now or later*.

It **composes, but does not duplicate**, the decision-content files: backend specifics live in `07-runtime-backends.md`; the Rust crates that implement them live in `08-rust-ecosystem.md`; distribution mechanics live in `10-distribution.md`; per-platform packaging lives in `11-platform-deployment.md`. This file owns *structure* (the seams and how they compose); those files own *choices* (which engine/crate/source/packaging, and why). See "Relationship to 07/08/10/11" below — that split is deliberate and the rule for avoiding redundancy.

## Governing principle: depend on seams, not implementations

Verdure is built as a small pure **core** surrounded by replaceable **adapters**, wired together by a thin per-platform **shell**. The core contains all the orchestration logic and knows nothing about which inference engine, which operating system, which model source, or which IPC mechanism it is running against. Everything platform- or vendor-specific sits behind a trait (a "seam"). The core depends only on the trait.

This is the entire resilience strategy. A new device class is a new adapter. A new inference engine is a new adapter. A failed vendor bet is one adapter deleted. Nothing in the core changes. No backend, no source, and no platform can lock the orchestration layer in, because the orchestration layer only ever talks to interfaces it owns.

The discipline that keeps this from becoming abstraction-for-its-own-sake: **a seam must name a concrete second implementation we actually intend to build.** If there is no real future implementation, there is no seam — just a concrete type. Every trait below earns its place by pointing at a device or engine on the real roadmap.

## The three layers

```
┌─────────────────────────────────────────────────────────┐
│  SHELL  (per target: desktop binary, Android app, etc.)  │
│  Wires concrete adapters into the core for one platform. │
└───────────────────────────┬─────────────────────────────┘
                            │ constructs + injects
┌───────────────────────────▼─────────────────────────────┐
│  CORE  (pure Rust — no FFI, no OS calls, no I/O)         │
│  Tier resolution · model matching · install/run state    │
│  machine · lifecycle/eviction policy · orchestration.    │
│  Depends ONLY on the trait definitions below.            │
└───────────────────────────┬─────────────────────────────┘
                            │ calls through traits
┌───────────────────────────▼─────────────────────────────┐
│  ADAPTERS  (the replaceable parts — one impl per trait    │
│  per platform/engine/source)                              │
│  Backend · DeviceProbe · Executor · ModelSource ·         │
│  ModelStore · Transport · IntentClassifier                │
└───────────────────────────────────────────────────────────┘
```

The core compiles for every target Verdure will ever support, including ones we cannot run inference on yet, because it has no platform dependencies. If the core ever fails to compile for a target, that is the signal that platform detail has leaked across the seam — fix the leak, do not special-case the core.

**Implication for `src/`:** the current flat scaffold (`hardware/`, `install/`, `models/`, `inference/`) reorganizes into this layering — a pure `core` (no FFI/IO), an `adapters` module per seam, and a thin `shell` per target. The module names above are the seams, not the current folders.

## The seam inventory

Each row is a replaceable part. "v1 ships" is what actually exists in the first laptop binary; the trait is defined from day one even where only one implementation exists, so the future implementations drop in without touching the core.

| Seam (trait) | Responsibility | v1 ships | Defined-but-deferred implementations | Why it is a seam |
|---|---|---|---|---|
| `Backend` | Load a model, run generation, report capabilities, unload | `LlamaCppBackend` (via `llama-cpp-2`) | `MistralRsBackend` (pure Rust, drops the C++ FFI), `ExecuTorchBackend` (Android NPU), `MlxBackend` (Apple Silicon — supersedes the MLC candidate), `AppleSystemBackend` (resident Apple system model, zero download) | The engine is the most likely thing to change per device; see `07-runtime-backends.md` |
| `DeviceProbe` | Read real hardware and return a normalized `DeviceProfile` | `MacosProbe`, `WindowsProbe`, `LinuxProbe` | `AndroidProbe`, `IosProbe` | New device classes are added here and nowhere else |
| `Executor` | Decide *where* inference runs (its own process vs. in-process) | `SubprocessExecutor` (supervisor/worker) | `InProcessExecutor` (phones — no subprocess freedom) | The process model itself differs laptop vs. phone |
| `ModelSource` | Resolve a model ID to bytes, with resume + verify | `HuggingFaceSource` + `MirrorSource` | `ObjectStoreSource` (R2/S3 CDN), `LocalImportSource` | Where models come from will change at scale; see `10-distribution.md` |
| `ModelStore` | Persist downloaded models, content-addressed | `FsStore` | `SharedStore` (Android shared dir serving multiple apps) | Storage layout differs where models are shared across apps |
| `Transport` | Expose the inference API to callers | `UnixSocketTransport`, `LoopbackHttpTransport` | `NamedPipeTransport` (Windows), `BinderTransport` (Android IPC) | How other apps reach Verdure is OS-specific; see `11-platform-deployment.md` |
| `IntentClassifier` | Turn free-text intent into a structured `(modality, task)` | (today the website classifies; the binary receives a decided choice or asks via cards) | `RemoteLlmClassifier` (Haiku), `OnDeviceClassifier` (SmolLM 2) | Classification moves from cloud to on-device over phases; see `03`, `05`, `13`, and the boundary section below |

## Notes on the load-bearing seams

**`Backend` — design the trait to not smell like llama.cpp.** This is the seam that makes device extensibility real, so its shape matters more than any other. The trait must be expressed in engine-neutral terms — load a model handle from a store path, accept a generation request (messages, sampling parameters, context budget), return a stream of tokens, report `Capabilities` (max context, quantization formats understood, whether it can offload to GPU/NPU), and unload cleanly. It must *not* expose llama.cpp concepts like raw `gguf` structs or backend-specific tensor flags. If `ExecuTorchBackend` cannot satisfy the trait without contortion, the trait was written around llama.cpp and needs to be re-neutralized. Treat "could a hypothetical second engine implement this cleanly?" as the acceptance test for every method added. **This is also the FFI boundary** — per `skills/testing.md`, all `unsafe` is wrapped inside the adapter and sanitizers run against it in CI; the pure core has no FFI and is tested fast against fake adapters.

**`Executor` — the supervisor/worker split is a seam, not a constant.** On laptops, `SubprocessExecutor` runs inference in worker processes (the same signed binary launched in worker mode), so an engine crash kills a worker and the supervisor restarts it rather than taking down the daemon that other apps depend on. On phones, where long-lived subprocesses are not freely available, `InProcessExecutor` runs the backend in-process via FFI and accepts the loss of crash isolation in exchange for fitting inside the OS sandbox and the memory budget. The core asks the `Executor` to "run this request against this backend" and never knows which strategy answered. This is what lets one orchestration codebase serve both a desktop daemon and a phone app.

**Heterogeneous workers.** The original formulation assumed the worker is the same signed binary in worker mode. That assumption is now relaxed: a worker may be a different binary speaking the same worker protocol. Concrete driver: a Swift sidecar worker linking MLX Swift or Apple's Foundation Models framework, where the best engine on a platform is not cleanly reachable from Rust (see `07-runtime-backends.md`, `[OQ-41]`). The supervisor's contract with workers — spawn, health-check, route requests, restart on crash — is unchanged; only the executable behind it varies. This is a trait-level generalization, not a core change.

**`ModelSource` + `ModelStore` — the resilience-critical pair.** `ModelSource` owns the hard part for emerging-market networks: ranged HTTP with resume (never restart from zero), SHA256 verification on completion, mirror failover from primary to CDN, and honest progress reporting. `ModelStore` is content-addressed by hash so the same GGUF is never fetched twice and can be shared across apps. Keeping them as separate seams means the download-resilience logic is reused unchanged whether bytes come from Hugging Face, a regional mirror, or a user's local import. See `10-distribution.md` for the open distribution decisions these adapters will eventually resolve.

**`Transport` — localhost-only is a property of every implementation.** Per `06-privacy-principles.md` and the permission posture in `11-platform-deployment.md`, no `Transport` implementation ever binds a public interface. Desktop prefers a file-permissioned Unix domain socket (no port, no inbound-firewall prompt), falling back to loopback HTTP where a socket is awkward. Windows uses a named pipe. Android uses Binder/IPC. The core hands a request/response interface to the transport and is indifferent to the wire.

## The Phase 0 ↔ Phase 1 boundary (the single-source-of-truth hinge)

The website (Phase 0) and the binary (Phase 1) are not two products — they are a **front door and the full product**. The website shows what Verdure can do and *optionally* captures intent + device; the binary downloads, verifies, installs, configures, and runs. They overlap on exactly one thing — **the decision** — and that overlap is the only real risk, so it is governed explicitly.

**Three input paths, one pipeline.** The binary's core gets its inputs three ways, all converging on the same install→run pipeline:
1. **Handoff** — user came through the website → the answers (intent + device → model choice) ride into the binary on install. Decision already made.
2. **Ask** — user installed the binary directly → it asks the *same questions the website asks* (the intent cards + device cards), then decides.
3. **Probe** — the binary can also read real hardware via `DeviceProbe`, which the website cannot.

**What's shared vs. what isn't:**

| | Owner | Must stay in sync? |
|---|---|---|
| The **decision** — intent → `(modality, task)`, device → tier, → model + feasibility | Shared logic (website + binary both compute it) | **Yes — the one real risk** |
| The **execution** — download, verify, install, configure, run, lifecycle | Binary only | No (website doesn't have it) |
| The **inputs** — how intent/device are obtained | Website: ask. Binary: handoff / ask / probe | No (different by design) |

The website and binary overlap *only* on the decision — and even there the binary can have **better inputs** (probe vs. the coarse device cards), so it may land on a finer tier. That is not drift; it is the binary legitimately being more precise. The decision *function* is shared; the *inputs* differ by surface.

**The sync mechanism (decided; implementation phased).** The knowledge base (`02`/`03`/`12`/`13`) is the human-authored source of truth. Neither implementation is canonical. To keep the two from drifting:
- **The matching matrix is a single data artifact + shared conformance fixtures.** Today it lives in `website/lib/matrix.ts` (`decide()` — pure data, no copy) with the canonical `(device, intent) → decision` fixtures in `website/lib/matrix.test.ts`. The Rust `ModelMatcher` must reproduce those fixtures exactly; if the two ever disagree on a fixture, CI screams.
- **Near-term (done):** the matrix is already extracted from presentation in the website, so it is one declarative module + fixtures rather than logic tangled into the recommendation copy.
- **Deferred (gated on the binary existing):** promoting the matrix to a *language-neutral* shared file both surfaces load. Per the counter-discipline above, that cross-language artifact earns its keep only when the second consumer (the Rust core) is real — until then there is one consumer and nothing to drift against. What *is* genuinely open is the data/logic line (the lookups are data; intent classification and feasibility gating are partly logic that the fixtures, not the data file, must pin) and the format.

**`DeviceProbe` vs. "ask, don't sniff" (`06`).** These look contradictory and are not. `06` forbids a *website* fingerprinting the user (Phase 0 asks via cards for exactly this reason). `DeviceProbe` is a *trusted, user-installed, local-only* binary reading its own host to serve the user better, sending nothing anywhere. Phase 0 *asks*; Phase 1 *probes*; both honor local-by-default and no-telemetry. The architecture upholds `06`, it does not bend it.

## The one-click path, as a state machine

The "one click" the user experiences is the core walking a single request through the seams. The two entry points (handoff vs. ask/probe) converge at the decision, then run the seven decisions from `01-thesis.md` as module handoffs:

```
ENTRY A: Install(ModelChoice)        ← website handoff (decision already made)
ENTRY B: Request(intent[, device])   ← direct: classify + ask/probe locally
            │
            ▼  (Entry B only)
  → IntentClassifier   → (modality, task)         (when given free text)
  → DeviceProbe        → DeviceProfile            (decision 1: fingerprint; or use the asked device)
  → TierResolver       → Tier                     (uses 02)
  → ModelMatcher       → ModelChoice              (decisions 2–3: model + quant; uses 03/04/12/13)
            │
            ▼  (both entries converge here on a ModelChoice)
  → ModelStore.has?    ── hit ──────────────┐
        │ miss                              │
  → ModelSource.fetch  → verified bytes      │  (decision 5: source + download)
  → ModelStore.put     → stored handle ──────┤
                                            ▼
  → Executor.run(Backend, handle, request)  → token stream   (decisions 4 + 7: route + infer)
  → Transport streams tokens to the caller
```

**Resident models — a domain evolution, recorded honestly.** The state machine as originally drawn assumed every `ModelChoice` resolves to bytes via `ModelSource` → `ModelStore`. Apple's resident system model breaks that assumption: some models are pre-installed by the OS, with no artifact to fetch, verify, or store. `ModelChoice` therefore gains a resident variant, and the install path gains a skip-fetch branch (matcher returns resident → go directly to `Executor.run`). This is a change to core logic — but it is the legitimate kind: the *domain* evolved (operating systems now ship models), rather than a platform detail leaking through a seam. The distinction matters: seam leaks get pushed back behind traits; domain evolutions get modeled in the core. See `03-model-matrix.md` (zero-download recommendation) and `07-runtime-backends.md` (`AppleSystemBackend`).

Lifecycle (decision 6 — storage budgets, swapping, eviction) is a core concern that wraps this path: before `ModelSource.fetch`, the core checks the storage budget against `ModelStore`; under memory pressure it asks the `Executor` to unload according to an eviction policy. The policy is core logic, not an adapter — it is the same on every device even though the things it manages are not.

## Concurrency & resource arbitration (the daemon is multi-caller)

The pitch is "other apps call Verdure via local IPC" — so the core is not single-request. Multiple apps may request inference concurrently, against possibly-different models, inside one finite memory budget. The core (not any adapter) owns the arbitration: how many models stay resident, how requests queue when a model is loading or memory is tight, fairness across callers, and per-session context. This interacts directly with lifecycle/eviction (decision 6) and with the `Executor` (a `SubprocessExecutor` can isolate concurrent workers; an `InProcessExecutor` cannot). The single-request state machine above is one lane; the arbitration model is the traffic control around it, and it is core logic so it behaves the same on every device. Tracked as `[OQ-39]`.

## The Tool layer (above the seams) — deferred, data-driven, port-don't-build

A recommendation isn't finished until it's *running*. Above the seams sits a future **Tool** layer that turns the decider's output into a one-click, working local tool — the local answer to "There's An AI For That" (`01-thesis.md`, "the decider that finishes the job"). The catalog of portable tools and their licenses is `15-port-catalog.md`.

**Verdure ports existing tools; it builds none.** A Tool is delivered one of two ways:
1. **Standalone-app port** — install + launch an existing local app, configured for the device (transcription → Buzz; stem-separation → UVR; upscaling → Upscayl; image → ComfyUI/DiffusionBee; coding → Continue).
2. **Assistant-config on a hosted shell** — for chat-style jobs (writing, doc-Q&A, LLM translation), Verdure ports **one** permissive open-source shell — recommended **AnythingLLM** (MIT; its REST API creates a workspace + persona + RAG fully unattended) or **Jan** (Apache) — and pushes a per-use-case **config** to it. The surface is the ported shell's; Verdure never builds one.

So a **Tool is a data manifest**, not a codebase:

> **Tool = which existing app/shell to port + engine selector (→ the decider + `Backend`) + persona/system prompt + optional RAG + the feasibility verdict — bound to one `(modality, task)`.**

Disciplines (preserve the neutral-decider moat):

- **Never author or brand a tool.** Shipping "Verdure Transcribe" would forfeit the right to neutrally recommend a better app. We configure and launch; we can always launch a competitor's tool. **The test: if Verdure is writing application/UI logic, stop — port a tool or push a config instead.**
- **Bundle only permissive licenses.** *Porting* (installing on the user's machine) is fine for any license; *bundling/forking* a shell requires permissive (AnythingLLM MIT; Jan / llamafile Apache). Avoid bundling AGPL / branding-locked / proprietary shells (Open WebUI, Cherry Studio, GPT4All-app, LM Studio, Msty). See `09` / `15`.
- **No abstraction without a second implementation.** Build the first tool concretely; let the second force the minimal Tool manifest.
- **Non-agentic first.** v1 tools are single-step or retrieval-only. Autonomous agents are deferred — trust-gated (~17%) and unreliable on small local models.

**Mac MVP framing.** The first tools are the manifest's first implementations — all *ported*, all on a portable engine (Ollama / llamafile, which run on the founder's 2019 Intel Mac *today*; `AppleSystemBackend` / `MlxBackend` slot in later on Apple Silicon): (A) **writing assistant** and (B) **"talk to my document"** as AnythingLLM/Jan configs; transcription (Buzz) as the first standalone-app port. The MVP's first deliverable is the porter installing + configuring one of these in one click. Refines `[OQ-45]`; Intel-Mac shell verification is `[OQ-46]`.

## How to extend (the actual procedure)

Adding a new device we previously could not serve should be a small, bounded change:

1. Write a new `DeviceProbe` that returns a `DeviceProfile` for the new platform. Add tier archetypes to `02-hardware-tiers.md` if the device falls outside existing tiers.
2. If the existing engine cannot run there, write a new `Backend` (and pick the right `Executor` for the platform's process constraints).
3. Write the platform's `Transport` and, if models are shared across apps, its `ModelStore`.
4. Write a shell that constructs these adapters and injects them into the unchanged core.

If adding a device requires editing the core, the architecture has failed and the leak must be found and pushed back behind a seam. The core changing is the alarm bell.

The counter-discipline, restated because it is easy to violate: do not add a seam speculatively. `ModelStore` is a trait because Android genuinely needs a shared store — a real, named future implementation. A trait with one implementation and no concrete second implementation on the roadmap is premature abstraction and should be collapsed back to a concrete type until the second implementation actually arrives.

## What v1 ships versus what is only seamed

v1 (the Phase 1 laptop binary) ships exactly one implementation of each laptop-relevant seam: `LlamaCppBackend`, the three desktop `DeviceProbe`s, `SubprocessExecutor`, `HuggingFaceSource` + `MirrorSource`, `FsStore`, and the desktop `Transport`s. The phone-side traits (`AndroidProbe`, `IosProbe`, `InProcessExecutor`, `BinderTransport`, `SharedStore`) and the alternative backends are **defined as traits but not implemented**. That is the whole point: the responsibility to those devices is encoded in the interfaces today, and discharged with code when we are ready, without re-architecting.

## Relationship to 07/08/10/11 (the no-redundancy contract)

This file and `07/08/10/11` change at different rates and own different things, which is why they stay separate:

- **`14` (here) owns structure** — the seams, their contracts, how they compose. Stable; rarely changes.
- **`07/08/10/11` own decision content** — which engine, which crate, which source, which packaging, with the reasoning, tradeoffs, and their own open questions. Volatile; changes whenever a vendor fact does.

`14` may *list* the v1/deferred implementations in the seam table (an index), but the moment it explains *why* an implementation is chosen, that reasoning has leaked from `07/10` and must move back. Each of `07/08/10/11` opens with a one-line pointer declaring which seam it is the content behind, so future edits land in the right file.

## Open Questions

- [OQ-35] `Executor` on Android — confirm whether `InProcessExecutor` (FFI, no isolation) is the only viable strategy, or whether a foreground-service worker can give partial crash isolation within the Android sandbox. Blocks the Phase 2 process model. (Related to `[OQ-25]` in `11-platform-deployment.md`.)
- [OQ-36] `Transport` for cross-app IPC — what do consuming apps actually need from the API surface (streaming, cancellation, concurrent sessions), and **what is the per-app authorization model** (any local process via socket file-perms, or explicit user consent per calling app)? Does either shape leak across Unix socket / named pipe / Binder into the trait? **[GAP — needs research]** before freezing the `Transport` trait.
- [OQ-37] Core purity litmus test — should the core be required to compile to `wasm32` as a CI gate? Not a product target, but a cheap automated proof that no platform detail has leaked into the core. Decide whether the gate is worth the build-time cost.
- [OQ-38] Eviction policy under memory pressure — LRU, intent-priority, or pinned-plus-LRU? This is core logic and needs a default before the lifecycle manager is built. **[GAP — needs decision]**
- [OQ-39] Concurrency & resource arbitration for the daemon — how many models resident at once, request queueing/fairness across concurrent callers, per-session context, and how that interacts with eviction and the `Executor` choice. Core logic; needs a model before the daemon is built.
- [OQ-40] Model catalog / registry — who resolves a model ID to `(hash, metadata, file)`, and how does that catalog **update and version** across an always-latest website and pinned, installed apps? May warrant its own seam or a named core concern. (Ties to `[OQ-21]`/`[OQ-22]` in `10-distribution.md`.)
