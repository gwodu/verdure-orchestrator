# 18 — The pivot: one interface over local models (not a launcher of apps)

**Status:** 🟢 The current direction (decided 2026-06-18). **Supersedes the standalone-app-porter framing of `17` as the *primary* surface.** New session here to build? Read this first, then `17` for what carries over.

## Mission (one line)
Verdure is the **cheapest, safest, and easiest way to use AI in daily life** — the user's **local AI conductor**: one simple interface that runs the right local model for whatever they want to do, on their own device, owned by them. Cheapest (free, local — no subscriptions/API bills), safest (on-device, no data leaves), easiest (one interface, no new apps or workflows to learn).

## The decision
Verdure is **one interface that leverages models** — not a launcher that installs and hands the user off to other apps. The user states intent; Verdure runs the appropriate **local model** behind a single, consistent Verdure UI and shows the result. The user never learns Buzz, Jan, ComfyUI, etc. We **conduct models, not apps.**

This realizes the north-star metaphor literally: a *conductor* conducts the players (models), it doesn't send the audience to a different concert hall per song.

## The doctrine shift (explicit reversal — flagged per CLAUDE.md)
`14`/`15` said: *"port, don't build — never build an I/O surface a tool lacks; port the whole app."* That produced an app-handoff experience (install Buzz → live in Buzz). We are reversing the bright line:

> **OLD:** never build a surface; port the whole app.
> **NEW:** never build a *model or an inference engine* — use existing **headless** OSS engines; build **exactly one** unified surface (the Verdure app) over them.

We still never write models or reinvent inference. We *do* now own one UI. `14`, `15`, `16`, `01`, `17` must be reconciled to this (see Open questions).

## Why this is *more* on-mission, not less
Every model family already has a **headless engine** under the app — the app is just a UI bolted on top:

| Task | Model family | Headless engine (what we drive) | The app we'd otherwise port |
|---|---|---|---|
| Chat / writing / coding / translation | LLM (Llama, Gemma, Qwen…) | **Ollama** (MIT, local REST) | Jan / AnythingLLM |
| Transcription | ASR (**Whisper**, *not* an LLM) | **whisper.cpp** (CPU, tiny models) | Buzz |
| Speak text aloud | TTS (Kokoro/Piper) | **Piper** (CLI) | — |
| Make images | diffusion (SD/FLUX) | a diffusion backend / ComfyUI API | Fooocus / Draw Things |

So "leverage models" = orchestrate **several model families** behind one UI by driving their headless engines directly. (Note: transcription uses Whisper, a speech-to-text model — a *different family* from the chat LLM. "One interface" must span families, not assume everything is an LLM.)

## Architecture
- **The Verdure shell** — the Tauri app (`app/`) evolves into the single interface. Per-modality UI panels, all consistent.
- **Headless engines** behind it — Ollama, whisper.cpp, etc. Verdure installs/manages them, never shows them.
- **The decider stays the brain** — `packages/decider` (`matrix.ts`) picks the right model for the task + device; Verdure owns model **lifecycle**: download, load, **swap easily, spin up a task-specific small model**.
- **Device detection** (`device.rs`) and the **CI build-and-ship pipeline** carry over unchanged.

## The atomic unit (redefined)
Was: *"one OSS app installed & running."* Now: **one everyday task done with a local model, inside Verdure, in one place** — no second app, no new workflow.

## The v1 bar (set by the founder, 2026-06-18)
Wrapped surfaces start **dead-simple** and shallow — e.g. transcription = *"drop audio → get text,"* nothing like Buzz's full feature set. Consistent-and-simple beats deep-and-fragmented for the floor user (`skills/design.md`). Depth per modality comes later, only where it earns its place. We consciously trade per-task depth for one coherent experience — and accept we maintain these surfaces.

## What carries over vs. what's demoted
- **Carries over:** the decider + model matrix (now central — the model-picker), `device.rs`, the Tauri shell, the CI cross-compile/build-and-ship pipeline, the install machinery (repurposed to fetch *engines + models*, not apps).
- **Demoted:** standalone-**app** porting. The Buzz *app* port becomes a stepping-stone — transcription will run on **whisper.cpp inside the shell** (killing the handoff + the Homebrew/CLT/512 MB pain). The porter proved install mechanics, the decider, and the ship pipeline; that value stands.

## First slice (proposed — to confirm)
Wrap **one** modality end-to-end in the shell, dead-simple, to prove "a local model running behind one Verdure UI":
- **Candidate A — Transcription via whisper.cpp.** Tiny models (~75–150 MB), CPU-only, no heavy runtime — best fit for "cheapest/easiest" + tight disk; directly replaces the Buzz handoff; proves the *hard* non-chat case early.
- **Candidate B — Chat/LLM via Ollama.** Highest demand (`13`); the natural home of model-swapping; but LLMs are 1–4 GB (disk) and need the Ollama runtime.

## The conversational front door (decided 2026-06-19)
The interface becomes **chat-first**: the user talks to Verdure ("I need to dictate some notes") and a small **local chat LLM orchestrates** — it picks the capability to set up, Verdure confirms, then runs it. This makes the *conductor* literal and is the strongest "easiest" play. The chat model is also the everyday-questions modality. Founder chose **LLM-driven tool-calling** (over deterministic routing).

Small local models misfire at agentic tool-calling, so we engineer for reliability (and respect the "no autonomous agents until reliable" line — `17`):
- **Grammar-constrained decoding (GBNF)** so tool calls are always well-formed JSON — removes the malformed-call failure mode entirely. The model only chooses *which* tool, never *whether it parses*.
- **A tool-calling-capable small model** — Qwen 2.5 / Llama 3.2 **3B Instruct**, not a generic 1B; the predictor (`19`) picks the size per device, stepping down if 3B is too slow.
- **A tiny, closed tool set** (e.g. `set_up_transcription`, `answer_in_chat`).
- **Deterministic cross-check + confirm** — `intent.ts` runs alongside as a confidence signal; Verdure confirms before any irreversible action (a model download). Human in the loop.
- **Discoverability** — a few suggested starters (the old intent cards become example prompts), so it's never a blank box.

**Runtime:** llama.cpp compiled into the app (a Rust crate, the way `whisper-rs` is), small **GGUF model downloaded on first use** (not bundled — disk), removable via the Storage screen. The x86_64 cross-compile deployment-target fix (`.cargo/config.toml`) already applies. This is the **LLM modality** the pivot needed; it also becomes how every other capability (incl. dictation) gets invoked.

**Sequencing:** the front door comes before live dictation — dictation gets reached *through* it ("set up dictation").

## Open questions
- [OQ-52] Reconcile `01`, `14`, `15`, `16`, `17` to this pivot (bright line, atomic unit, Tool layer → "engine layer", port-classes). Big edit; do before deep build.
- [OQ-53] Confirm the first wrapped slice (A whisper.cpp vs B Ollama) and the shell's panel/navigation model for spanning modalities.
- [OQ-51] Resolve "latest" engine/model versions dynamically vs pinned (carried from the porter work).
