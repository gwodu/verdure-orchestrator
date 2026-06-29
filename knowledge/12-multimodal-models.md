# 12 — Multimodal Model Matrix (audio · image · video)

**Status:** 🟡 Provisional — model facts grounded in web research on 2026-05-29; feasibility tiers and throughput are estimates pending real-device testing (`[OQ-03]`, `[OQ-28]`).

This file extends Verdure's matching brain beyond text. `03-model-matrix.md` answers "given a tier, which **text** model?"; this file answers the same question for **audio, image, and video** tasks. It is the source of truth for the website's `packages/decider/src/recommend.ts` non-text branches and `packages/decider/src/intent.ts` non-text classification.

For why Verdure recommends across modalities at all, see `01-thesis.md` ("What Verdure recommends"). For device tiers, see `02-hardware-tiers.md`. For licensing posture, see `09-licensing.md`.

## The core insight: feasibility is graphics-bound, not RAM-bound

Text models are sized by system RAM. Audio/image/video are different: what gates them is **graphics power** (a dedicated GPU's VRAM, or an Apple-Silicon Mac's unified memory), not system RAM. So the five device buckets in `02-hardware-tiers.md` collapse into three **capability classes** for non-text work:

| Capability class | Maps from tier | What it means |
|---|---|---|
| **cpu-laptop** | L1, L2-low, L2-high | No real graphics card. CPU-only. |
| **apple-silicon** | L3 | M-series Mac; capable via unified memory. |
| **gpu** | L3+ | Dedicated graphics card (gaming PC / workstation). |

This collapse is a deliberate simplification for v1. It is imperfect — it cannot tell a 16 GB M-chip Mac from a 64 GB one, nor an 8 GB GPU from a 24 GB one, and those differences decide whether the heaviest tasks (image editing, video) actually run. Tracked as `[OQ-29]`.

## The three honest outcomes

Every non-text recommendation resolves to one of three feasibility verdicts. The third is as important as the others — Verdure's value is honesty, and telling a user "your device can't do this yet" beats handing them a model that will crawl.

- **recommended** — runs well on this device.
- **works-but-slow** — runs, but be patient (and a better machine would help).
- **not-local-yet** — honestly, this device can't do it on its own; point elsewhere (cloud) and capture interest for when Verdure can help.

## The feasibility matrix

✅ recommended · ⚠️ works-but-slow · ❌ not-local-yet

| Modality / task | cpu-laptop | apple-silicon | gpu | Recommended model | Easy install path |
|---|---|---|---|---|---|
| **Audio — transcription** | ✅ | ✅ | ✅ | Whisper (small on laptops, large-v3-turbo above) | MacWhisper (Mac), Buzz (cross-platform) |
| **Audio — speech / TTS** | ✅ | ✅ | ✅ | Kokoro-82M | Kokoro / Piper |
| **Audio — stem separation** | ⚠️ | ✅ | ✅ | Demucs | Ultimate Vocal Remover (GUI) |
| **Audio — music generation** | ❌ | ⚠️ | ✅ | Stable Audio Open | ComfyUI (technical) |
| **Image — upscaling** | ✅ | ✅ | ✅ | Real-ESRGAN | Upscayl (GUI) |
| **Image — generation** | ❌ | ✅ | ✅ | Draw Things' SD/SDXL (Mac); FLUX.1 schnell (GPU) | Draw Things (Mac), Fooocus (Win/Linux) |
| **Image — editing** | ❌ | ✅ | ✅ | Qwen-Image-Edit | Draw Things (Mac), ComfyUI |
| **Video — generation** | ❌ | ❌ | ⚠️ | LTX-2 (alt: Wan 2.2) | ComfyUI (technical) |
| **Video — enhancement** | ⚠️ | ✅ | ✅ | Real-ESRGAN + RIFE | REAL Video Enhancer |

**Audio is the strongest local story** — transcription and speech run on any laptop, CPU-only, with clean MIT/Apache licenses. **Video generation is the weakest** — it is honestly declined for everyone except dedicated-GPU users, and even there it is only "works-but-slow." This asymmetry is real and reflected in the product copy.

## Model details and licenses

All licenses below are friction-free to *recommend* (Phase 0 only refers users; see `09-licensing.md`). Flags marked ⚠️ matter for Phase 1 if Verdure ever redistributes, or for users who care about commercial use of outputs.

### Audio
- **Whisper** (OpenAI) — speech-to-text. Sizes tiny→large-v3; **large-v3-turbo** is the best default above laptops (near-large accuracy, ~5× faster, transcription-only). **MIT** — ideal. CPU-feasible at small sizes on a 4 GB laptop. Runs via whisper.cpp / faster-whisper; friendly apps: **MacWhisper** (Mac), **Buzz** / **Vibe** (cross-platform).
- **Kokoro-82M** — text-to-speech. **Apache 2.0** — ideal. Tiny (82M), CPU-real-time even on weak hardware. **Piper** is the lighter alternative (huge voice library; code moved MIT→GPL fork — fine to recommend; per-voice licenses vary).
- **Demucs** (Meta) — stem separation. **MIT** — ideal. CPU works but slow (~10–15 min/track); GPU drops it to <90 s. Recommend the **Ultimate Vocal Remover (UVR)** GUI, not raw Demucs.
- **Stable Audio Open** (Stability) — short music / SFX generation. **Stability Community License** — commercial OK below a revenue threshold ⚠️ verify. ~12 GB VRAM → gpu-class. **Avoid MusicGen for our purposes: its weights/outputs are CC-BY-NC (non-commercial).** ⚠️

### Image
- **Real-ESRGAN** — upscaling/restoration. **BSD-3** — ideal. The one image task that runs comfortably on almost anything (ncnn-vulkan binary; **Upscayl** GUI). Recommend widely.
- **FLUX.1 schnell** (Black Forest Labs) — text-to-image. **Apache 2.0** — ideal, best free+commercial quality. ~8 GB VRAM quantized. (FLUX.1 *dev* is non-commercial weights though outputs are allowed ⚠️ — prefer schnell to avoid confusion.)
- **Stable Diffusion / SDXL** — text-to-image. **CreativeML OpenRAIL-M**, no revenue cap. Lowest hardware bar; the default behind the **Draw Things** Mac app and **Fooocus** (Win/Linux).
- **Qwen-Image-Edit** (Alibaba) — instruction-based editing/inpainting. **Apache 2.0** — ideal; cleanest-license editor.
- Apps: **Draw Things** (Mac, top non-technical pick), **Fooocus** (simple, SDXL), **Upscayl** (upscaling), **ComfyUI** (powerful, technical). Avoid **DiffusionBee** — abandoned since Aug 2024.

### Video
- **LTX-2** (Lightricks, Jan 2026) — text/image-to-video (+ audio). **Apache 2.0** — ideal; best low-VRAM story (8 GB quantized, slow). Practical floor for usable video is **~12 GB VRAM + 32 GB RAM**.
- **Wan 2.2** (Alibaba) — higher quality, heavier. **Apache 2.0** — ideal.
- ⚠️ **HunyuanVideo** (Tencent) — strong, but a restrictive license (excludes EU/UK/KR, scale limits). Not friction-free; avoid as a default recommendation.
- **"AI video editing" is not a single local model** — it's frame interpolation (**RIFE**) + upscaling (**Real-ESRGAN**), wrapped in tools like **REAL Video Enhancer** / Flowframes, plus pro software (DaVinci Resolve). Verdure recommends the enhancement tools as the genuinely-local path; actual editing is out of scope as a "model."

## Relationship to the install-path philosophy

Phase 0 refers users to existing tools rather than hosting models (`05-website-spec.md`, `10-distribution.md`). The twist for multimodal: **install paths are modality-specific**. Ollama / LM Studio / Jan run *text* models only. Audio routes to MacWhisper / Buzz / UVR; image to Draw Things / Fooocus / Upscayl; video to REAL Video Enhancer / ComfyUI. Wherever a friendly GUI exists, Verdure leads with it; raw commands appear only where the audience is necessarily technical (ComfyUI for image/video generation).

## Open questions

- [OQ-28] Real on-device throughput for non-text models (Whisper sizes, SD/SDXL, LTX-2) across the three capability classes — currently estimated. Parallels `[OQ-05]` for text.
- [OQ-29] The five-tier → three-class collapse cannot capture VRAM / unified-memory *amount*, which decides whether the heaviest tasks (image editing, video) actually run. Does the device picker need a graphics-memory signal, asked in non-technical terms?
- [OQ-30] Resolved (2026-05-29) — recommend music & video generation, keeping the honest "hard / for comfortable users / needs ComfyUI" framing. The setup friction is on-strategy: it demonstrates Verdure's value. Don't over-soften it.
- [OQ-31] Validate the modality keyword classifier against real user phrasings (parallels `[OQ-06]`/`[OQ-07]`); the LLM classifier must handle modality, not just text intent.
