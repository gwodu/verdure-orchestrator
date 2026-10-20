// The matching matrix — the canonical decision.
//
// Given a device tier and a (modality, task), this answers the only question
// that must never differ between surfaces: WHICH model do we recommend, and is
// it feasible on this class of device? It is DATA + a pure decision function,
// with NO presentation — the same decision the Phase 1 Rust "ModelMatcher" must
// reproduce.
//
// This is the website's implementation of the matching brain
// (knowledge/02 + 03 + 12 + 13). It is the single place the website decides a
// model; lib/recommend.ts wraps each Decision in non-technical copy + install
// paths. The cross-implementation contract is locked by lib/matrix.test.ts —
// when the Rust binary lands it must pass the same fixtures. See
// knowledge/14-architecture.md, "The Phase 0 ↔ Phase 1 boundary".

import type { Tier } from './tiers';
import type { Classification, AudioTask, ImageTask, TextTask, VideoTask } from './modality';

export type Feasibility = 'recommended' | 'works-but-slow' | 'not-local-yet';

// The decision, stripped of all copy: what (if anything) to run, and how well.
// `model` is null exactly when feasibility is 'not-local-yet'.
export type Decision = { feasibility: Feasibility; model: string | null };

// For non-text work, feasibility is driven by graphics power, not RAM, so the
// five tiers collapse into three capability classes.
export type DeviceClass = 'cpu-laptop' | 'apple-silicon' | 'gpu';

export function deviceClass(tier: Tier): DeviceClass {
  if (tier === 'L3+') return 'gpu';
  if (tier === 'L3') return 'apple-silicon';
  return 'cpu-laptop';
}

const recommended = (model: string): Decision => ({ feasibility: 'recommended', model });
const slow = (model: string): Decision => ({ feasibility: 'works-but-slow', model });
const notLocal: Decision = { feasibility: 'not-local-yet', model: null };

/** The canonical decision. Pure; no copy, no install paths. */
export function decide(tier: Tier, c: Classification): Decision {
  switch (c.modality) {
    case 'text':
      return recommended(decideText(tier, c.task));
    case 'audio':
      return decideAudio(deviceClass(tier), c.task);
    case 'image':
      return decideImage(deviceClass(tier), c.task);
    case 'video':
      return decideVideo(deviceClass(tier), c.task);
  }
}

// Text always runs locally; the tier (and intent within it) picks the model.
function decideText(tier: Tier, task: TextTask): string {
  switch (tier) {
    case 'L1':
      return 'Gemma 3 1B';
    case 'L2-low':
      return task === 'multilingual' ? 'Qwen 2.5 1.5B' : 'Llama 3.2 3B';
    case 'L2-high':
      if (task === 'creative') return 'Gemma 3 4B';
      if (task === 'code') return 'Phi-4-mini';
      if (task === 'multilingual') return 'Qwen 2.5 3B';
      return 'Llama 3.2 3B';
    case 'L3':
    case 'L3+':
      return task === 'code' ? 'Llama 3.1 8B' : 'Qwen3 8B';
  }
}

function decideAudio(dc: DeviceClass, task: AudioTask): Decision {
  switch (task) {
    case 'transcription':
      return recommended('Whisper');
    case 'speech':
      return recommended('Kokoro');
    case 'separation':
      return dc === 'cpu-laptop' ? slow('Ultimate Vocal Remover') : recommended('Ultimate Vocal Remover');
    case 'music':
      if (dc === 'cpu-laptop') return notLocal;
      return dc === 'apple-silicon' ? slow('Stable Audio Open') : recommended('Stable Audio Open');
  }
}

function decideImage(dc: DeviceClass, task: ImageTask): Decision {
  if (task === 'upscale') return recommended('Upscayl');
  if (dc === 'cpu-laptop') return notLocal; // generate + edit both need graphics power
  if (task === 'edit') return recommended('Qwen-Image-Edit');
  // GPU: SDXL, because the friendly app (Fooocus) runs SDXL only; FLUX.1
  // schnell is the ComfyUI alternative (knowledge/12, corrected 2026-10-20).
  return recommended(dc === 'apple-silicon' ? 'Stable Diffusion' : 'Stable Diffusion XL');
}

function decideVideo(dc: DeviceClass, task: VideoTask): Decision {
  if (task === 'enhance') {
    return dc === 'cpu-laptop' ? slow('REAL Video Enhancer') : recommended('REAL Video Enhancer');
  }
  return dc === 'gpu' ? slow('LTX-2') : notLocal; // generate
}
