# 17 — Build Plan: the MVP and the road from here

**Status:** 🟡 Provisional — the current build target. **New session here to build? Start with this file.**

This states, in one place, **what we build first (the MVP), where it lives, and what it becomes** — so a fresh session can continue the build. It reconciles the architecture (`14`), the tool catalog (`15`), and the mission (`01`, `16`).

## What Verdure is (one line)
The user's **local AI conductor**: a neutral *decider* + *porter* that recommends the right local tools, installs/sets them up, and (later) connects them — all on the user's own device, owned by them. **Agency, not extraction.** Verdure ports existing tools; it never builds its own. Mission over moat; **no data harvesting**.

## The atomic unit
**One OSS tool installed & running, from the user's intent, in one click** (Verdure's "ride"). The MVP proves exactly this, for one use case. Verdure is a **utility that empowers users** — that's the goal, not a stepping stone to a data/network moat.

## The MVP (v1.0) — what we build now
A small **Tauri desktop app** (Rust backend + web UI) that:
1. **Asks intent** — reuses the existing **intent cards + decider** from `website/lib` (cards in `website/lib/modality.ts`; the decision in `website/lib/matrix.ts` → `decide()`). The hard part — the decider — is already built and tested.
2. **Decides** the right **OSS tool** for the use case + device (the catalog, `15`).
3. **Ports it in one click** — the Rust backend installs the tool (and a portable engine if needed) from the tool's official source, configures it, and launches it.
4. **It runs locally** — private, free, on the user's machine.

**Engine = portable: Ollama (MIT) or llamafile (Apache).** They run on the founder's **2019 Intel Mac today** (no GPU / no Apple Silicon needed). Ollama is "llama.cpp with batteries" + a local REST API.

**First slice (smallest thing that proves the loop): transcription via Buzz.** MIT, Whisper-based, runs on Intel CPU, the simplest standalone-app port. Prove: *intent ("turn a recording into text") → Buzz installed & running, one click.*

## Where it lives
- **New `app/` directory** — the Tauri desktop porter (Rust backend = the porter; web frontend = the reused decider/cards).
- **Reuses `website/lib`** — `matrix.ts` (the decision, single source of truth) + `modality.ts` (the cards). Keep the matrix as the one source both the website and the app consume (`[OQ-49]` confirms shared-package vs copy).
- **`website/`** stays the public funnel / Phase 0 decider (verdureai.com).
- **`src/`** (the pure-Rust core + seams daemon, `14`) is the **long-term** architecture, **not** the MVP — the MVP's Rust backend is a thin porter that grows toward it.

## Build steps for the first slice
1. ✅ Scaffold the Tauri app in `app/` (Rust + web — Vite + vanilla TS frontend, Tauri v2 backend).
2. ✅ Frontend: intent cards + `recommend()` from the shared decider → user picks intent → app shows the recommended tool. Device is **detected, not asked** (`app/src-tauri/src/device.rs`), removing the website's device-picker screen.
3. ✅ Catalog: a manifest entry for **Buzz** as **data** (`app/src-tauri/src/catalog.rs`) — install recipe = Homebrew cask `buzz`; launch = `open -a Buzz`.
4. ✅ Rust backend (the porter, `app/src-tauri/src/porter.rs`): locate `brew` (Finder-PATH-safe), check-if-installed → install → launch, streaming progress to the UI. Written + unit-tested.
5. 🟡 Verify the loop end-to-end on the Intel Mac — **blocked on Xcode Command Line Tools** (no `cc`/linker → Tauri/Rust can't compile). Everything else verified (decider 64 tests, website 9 + build, frontend typecheck/build/screenshots). See FEATURE_LOG.md "The Buzz slice".

**Build status (2026-06-16):** the whole app exists and the loop is proven in a browser preview (the Tauri calls degrade to a simulation when there's no backend — `app/src/bridge.ts`). The only thing between here and a real Buzz-installs-on-the-Mac run is installing Xcode CLT (`xcode-select --install`), then `cd app && npm run tauri dev`. Disk is tight (~3 GB free after clearing the npm cache) — the Tauri build + CLT may need a cleanup first.

## Decided while building (was open)
- **Stack & location** (`app/`, Tauri v2): Rust backend + a small Vite + vanilla-TS frontend (no React — two screens, keep deps light).
- **Decider sharing = shared package, not copy.** Extracted `website/lib` → `packages/decider/` (`@verdure/decider`); website + porter both import it via npm workspaces. Ships TS source (consumed by Next `transpilePackages`, Vite, Vitest). Resolves `[OQ-49]`.
- **Catalog is data.** A `Tool` = `InstallMethod` + `LaunchMethod` enums; command construction is pure (testable), execution is isolated. Adding a port is a data entry, per `15`.

## What it becomes (roadmap)
- **v1.1 — connect tools (composition).** User-directed, deterministic 2-tool chains (e.g. Buzz → AnythingLLM), typed I/O ports + the 3-field pipe contract, hard-capped at 2 nodes. See `14` (composition layer).
- **Grow the catalog (`15`).** More ported OSS tools = data entries (the seed supply, from the open-source ecosystem). Includes the **assistant-config class**: port **AnythingLLM** (MIT) once and push per-use-case configs (incl. optional personas) via its REST API.
- **Later — the Apple adapter.** `MlxBackend` / `AppleSystemBackend` (resident zero-download model) as a `Backend` adapter, once on **Apple Silicon** (the Intel Mac can't run MLX — needs a used M-series or cloud). Slots into the same Tool layer.
- **Later — the full Rust daemon** (`14`): pure core + adapter seams + multi-caller `Transport`. The MVP porter evolves into this.
- **Phase 2 — phones** (`01`, `11`): Android first; iOS = a thin decider over Apple's stack.
- **Explicitly NOT:** data harvesting (`16`/`06`); autonomous agents (until trust + reliability); heavy image/video generation on weak hardware; building bespoke tools or I/O surfaces (`14` bright line).

## Principles to hold while building (`CLAUDE.md` + `skills/`)
- **Port, don't build.** Never author/brand a tool or build an I/O surface a tool lacks. If Verdure is writing application/UI logic, stop — port a tool or push a config.
- **Mission over moat. No data harvesting.** Agency, privacy, user benefit first (`06`, `16`).
- **Assume zero technical knowledge** (`skills/design.md`); plain, verb-first copy (`skills/copywriting.md`).
- **Tests ship with the feature** (`skills/testing.md`). The decider already has the cross-impl contract (`website/lib/matrix.test.ts`).
- **No abstraction without a second implementation** (`14`).

## Where to start (new session)
Read `KNOWLEDGE.md` → `CLAUDE.md` → `01-thesis.md` → `14-architecture.md` (Tool layer + composition) → `15-port-catalog.md` → `16-growth-asset.md` → **this file**. Then scaffold `app/` and build the Buzz slice.

## Small known follow-ups (not blockers)
- `02`/`03` still present Apple-Silicon zero-download prominently — remember the founder's Mac is Intel, so that path is deferred behind the Apple `Backend` adapter.
- `09`/`12` mention a possible "Verdure redistributes models" future — the porter installs from official sources instead; redistribution is a deferred maybe.

## Open questions
- ~~[OQ-49]~~ **Resolved 2026-06-16:** stack = Tauri v2 in `app/` (Vite + vanilla TS frontend); decider shared via a **workspace package** `@verdure/decider` (`packages/decider/`), imported by both website and porter. See "Decided while building" above.
- [OQ-50] (new) End-to-end verification on the Intel Mac is pending Xcode Command Line Tools + a possible disk cleanup. Once CLT is in, run `cd app && npm run tauri dev`, port Buzz, and confirm install+launch; then log the result in TEST_REPORT.md.
