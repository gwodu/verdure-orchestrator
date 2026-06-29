# 15 — The Port Catalog (the local-tool database)

**Status:** 🟡 Largely superseded by the 2026-06-18 pivot (`18`) — read through that lens (`[OQ-52]`).

> **Read through the pivot.** This file is the supply side of the *old* delivery model: a catalog of **separate third-party apps to install**. The 2026-06-18 pivot (`18`) **demoted standalone-app porting** in favor of running models **in-app** (whisper.cpp, llama.cpp inside the Verdure shell), so this catalog no longer describes the primary path. What **survives**: the research method (curate free, locally-runnable, well-licensed OSS), the licensing gate (`09`), and the "seed supply from the OSS ecosystem" growth idea. The install-and-launch-external-apps mechanism is **legacy** — kept only as a transitional fallback for capabilities not yet wrapped in-app (the porter code, `app/src-tauri/catalog.rs`/`porter.rs`).

This is the **supply side of the porter**: the curated registry of free, open-source, *locally-runnable* AI tools Verdure can **port** (install + configure) onto a user's device. It pairs with `13-use-case-demand.md` (what people want), is gated by `02`/`03`/`12` (what the device can run) and `09` (licensing), and is the "what we can actually deliver" half of *the decider that finishes the job* (`01`, `14`).

This catalog is also the **seed supply** for growth — Verdure's "20,000 public-domain books" (the existing open-source ecosystem we bootstrap from). See `16-growth-asset.md` (the ASSET framework, stage "S — seed the supply side").

## Why not seed from "There's An AI For That"
TAAFT and the AI-tool directories are **~95%+ cloud SaaS** — they filter by price, not license, and an open engine (Whisper, Stable Diffusion) appears only as a hosted wrapper, not the portable project. So directories are useful **only as the demand taxonomy** (which jobs people search for), never as a source of portable tools. Seed the catalog from the open-source-local ecosystem instead: `janhq/awesome-local-ai`, **Pinokio**'s app catalog, **AlternativeTo** (Open-Source + Self-Hosted filters), LocalAlternative.io, r/LocalLLaMA tooling lists.

## Two kinds of port
1. **Standalone-app port** — install + launch an existing local app, configured for the device. (transcription, stem-separation, upscaling, background-removal, image gen, coding.)
2. **Assistant-config on a hosted shell** — for chat-style jobs (writing, doc-Q&A, LLM translation), Verdure ports **one** permissive open-source shell and pushes a per-use-case **config** (model + persona + optional RAG) to it. A "writing assistant" is a config payload, not an app. See the shell decision below.

## Seed catalog (best portable picks by use case)
License: 🟢 permissive (bundle-safe) · 🟡 copyleft GPL/AGPL (fine to *install*, constrained to *bundle/fork*) · 🔴 proprietary (port/launch only, never fork). "Intel?" = runs on the 2019 Intel Mac target.

| Use case | Best portable pick(s) | License | Intel Mac? | Port class |
|---|---|---|---|---|
| Transcription | **Buzz** / Vibe (+ whisper.cpp) | 🟢 MIT | ✅ CPU | standalone |
| Stem separation | **Ultimate Vocal Remover** | 🟢 MIT (Intel build) | ✅ | standalone |
| Document Q&A / RAG | **AnythingLLM** | 🟢 MIT | ✅ | shell-config |
| Chat & writing | **AnythingLLM** / Jan (assistant config) | 🟢 MIT / Apache | ✅ / verify Jan | shell-config |
| Coding | **Continue** (+ local model) / Tabby | 🟢 Apache | ✅ CPU-speed | standalone (IDE ext) |
| Background removal | **rembg** | 🟢 MIT | ✅ CPU | standalone |
| Image upscaling | **Upscayl** (Real-ESRGAN) | 🟡 AGPL | ⚠️ needs Vulkan GPU | standalone |
| Text-to-speech | **Kokoro** / Piper | 🟢 Apache / 🟡 mixed | ✅ CPU | standalone |
| Translation | **Argos Translate** (or any local LLM) | 🟢 MIT | ✅ CPU | standalone / shell |
| Image generation | **DiffusionBee** (Intel) / ComfyUI / Fooocus | 🟡 GPL | ⚠️ slow on Intel | standalone |
| Image editing | Krita + AI Diffusion | 🟡 GPL | ⚠️ CPU-bound | standalone |
| Video enhancement | — none good on OSS + Intel Mac | — | ❌ | gap (see below) |

Permissive engines/shells safe to *bundle*: Ollama (MIT), **llamafile** (Apache — runs on old/Intel CPUs), AnythingLLM (MIT), Jan (Apache).

## The shell decision (answers "build our own Jan-like UI?")
**Port and configure an existing permissive shell — do NOT build a base UI.** Recommended: **AnythingLLM (MIT)** as the assistant/RAG runtime — its full **REST API** lets the porter create a workspace, set persona/system prompt, upload + auto-embed documents, and wire agents/MCP **fully unattended**. That API *is* the "1-click port an assistant" mechanism. **Jan (Apache 2.0)** is secondary (file-based assistant configs + OpenAI-compatible server; verify Intel-Mac on current builds — `[OQ-46]`). **llamafile (Apache)** is the bare-engine fallback.

Building our own chat shell is rejected: it contradicts "port, don't build," and a chat surface is a maintenance trap (streaming, model management, RAG ingest, MCP). MIT/Apache rebranding rights let us skin AnythingLLM/Jan as "Verdure" if we want a unified look — without building or forfeiting neutrality. *(Recommendation — confirm before committing.)*

## Bundling rule (read with `09-licensing.md`)
*Porting* (installing a tool on the user's own machine) is fine for **any** license. *Bundling, forking, or rebranding* a tool into Verdure requires a permissive license. **Avoid bundling:** Open WebUI (custom branding-lock license, post-2025), Cherry Studio (AGPL), GPT4All *desktop app* (GPL copyleft), and proprietary shells (LM Studio, Draw Things, Msty, MacWhisper) — port/launch them, never fork.

## Honest gaps (no good fully-local option yet → the `12` "not-local-yet" verdict)
- **Video enhancement** on Mac (Video2X has no Mac build; alternatives need a GPU the 2019 Intel lacks).
- **Expressive voice cloning** that's both easy *and* commercially licensed (XTTS-v2 has CPML non-commercial constraints).
- **Frontier-quality reasoning/coding fully offline** — the *tooling* is solved; local model quality at home is the gap, worse on modest/Intel hardware.
- **Turnkey meeting assistants** (auto-join calls + diarization + CRM) and **NotebookLM-style audio overviews** end-to-end offline.

## Pipeability (for chaining — see `14`, composition layer)
A tool can be chained only if it exposes I/O programmatically (file/CLI output, a watched folder, or a localhost REST API). GUI-only tools can't be wired as-shipped — and Verdure must not build them an interface (port a headless equivalent instead).
- **Pipeable today:** whisper.cpp & Buzz (Buzz's **watch folder** is the best non-technical trigger), Ollama, AnythingLLM (REST consumer), Jan, llamafile, rembg, Piper, Upscayl-CLI, ComfyUI (REST, but JSON-format input is technical).
- **GUI-only as shipped (not chainable):** Ultimate Vocal Remover official app — port the headless `audio-separator` (pip) as a separate tool if its chain is needed.
A `pipeable: yes/no` flag + the typed I/O ports belong in each tool's manifest.

## Open questions
- [OQ-46] Verify **AnythingLLM / Jan on the Intel-Mac target**, and prove the unattended REST/file config as the port mechanism for assistant-style tools (the core of the shell decision). (From `15-port-catalog.md`.)
