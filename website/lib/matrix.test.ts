import { describe, it, expect } from 'vitest';
import { decide, type Decision } from './matrix';
import type { Tier } from './tiers';
import type { Classification } from './modality';

// Conformance fixtures: the canonical (device, intent) → decision contract.
// This is the single source of truth for "which model, and is it feasible."
// When the Phase 1 Rust binary's ModelMatcher lands, it must reproduce exactly
// these outcomes (see knowledge/14-architecture.md). Treat this list as the
// shared spec, not just a TS test — keep it in lockstep with knowledge/03 + 12.
type Fixture = { tier: Tier; c: Classification; expect: Decision };

const t = (modality: Classification['modality'], task: string) =>
  ({ modality, task } as Classification);

export const FIXTURES: Fixture[] = [
  // Text — tier (+ intent within tier) picks the model; always feasible.
  { tier: 'L1', c: t('text', 'general'), expect: { feasibility: 'recommended', model: 'Gemma 3 1B' } },
  { tier: 'L1', c: t('text', 'multilingual'), expect: { feasibility: 'recommended', model: 'Gemma 3 1B' } },
  { tier: 'L2-low', c: t('text', 'general'), expect: { feasibility: 'recommended', model: 'Llama 3.2 3B' } },
  { tier: 'L2-low', c: t('text', 'multilingual'), expect: { feasibility: 'recommended', model: 'Qwen 2.5 1.5B' } },
  { tier: 'L2-high', c: t('text', 'creative'), expect: { feasibility: 'recommended', model: 'Gemma 3 4B' } },
  { tier: 'L2-high', c: t('text', 'code'), expect: { feasibility: 'recommended', model: 'Phi-4-mini' } },
  { tier: 'L2-high', c: t('text', 'multilingual'), expect: { feasibility: 'recommended', model: 'Qwen 2.5 3B' } },
  { tier: 'L2-high', c: t('text', 'general'), expect: { feasibility: 'recommended', model: 'Llama 3.2 3B' } },
  { tier: 'L3', c: t('text', 'general'), expect: { feasibility: 'recommended', model: 'Mistral Small' } },
  { tier: 'L3', c: t('text', 'code'), expect: { feasibility: 'recommended', model: 'Llama 3.1 8B' } },
  { tier: 'L3+', c: t('text', 'code'), expect: { feasibility: 'recommended', model: 'Llama 3.1 8B' } },

  // Audio.
  { tier: 'L2-high', c: t('audio', 'transcription'), expect: { feasibility: 'recommended', model: 'Whisper' } },
  { tier: 'L1', c: t('audio', 'speech'), expect: { feasibility: 'recommended', model: 'Kokoro' } },
  { tier: 'L2-high', c: t('audio', 'separation'), expect: { feasibility: 'works-but-slow', model: 'Ultimate Vocal Remover' } },
  { tier: 'L3+', c: t('audio', 'separation'), expect: { feasibility: 'recommended', model: 'Ultimate Vocal Remover' } },
  { tier: 'L2-high', c: t('audio', 'music'), expect: { feasibility: 'not-local-yet', model: null } },
  { tier: 'L3', c: t('audio', 'music'), expect: { feasibility: 'works-but-slow', model: 'Stable Audio Open' } },
  { tier: 'L3+', c: t('audio', 'music'), expect: { feasibility: 'recommended', model: 'Stable Audio Open' } },

  // Image.
  { tier: 'L1', c: t('image', 'upscale'), expect: { feasibility: 'recommended', model: 'Upscayl' } },
  { tier: 'L2-high', c: t('image', 'generate'), expect: { feasibility: 'not-local-yet', model: null } },
  { tier: 'L3', c: t('image', 'generate'), expect: { feasibility: 'recommended', model: 'Stable Diffusion' } },
  { tier: 'L3+', c: t('image', 'generate'), expect: { feasibility: 'recommended', model: 'FLUX.1 schnell' } },
  { tier: 'L2-high', c: t('image', 'edit'), expect: { feasibility: 'not-local-yet', model: null } },
  { tier: 'L3', c: t('image', 'edit'), expect: { feasibility: 'recommended', model: 'Qwen-Image-Edit' } },

  // Video — the honest-decline modality.
  { tier: 'L2-high', c: t('video', 'generate'), expect: { feasibility: 'not-local-yet', model: null } },
  { tier: 'L3', c: t('video', 'generate'), expect: { feasibility: 'not-local-yet', model: null } },
  { tier: 'L3+', c: t('video', 'generate'), expect: { feasibility: 'works-but-slow', model: 'LTX-2' } },
  { tier: 'L2-high', c: t('video', 'enhance'), expect: { feasibility: 'works-but-slow', model: 'REAL Video Enhancer' } },
  { tier: 'L3+', c: t('video', 'enhance'), expect: { feasibility: 'recommended', model: 'REAL Video Enhancer' } },
];

describe('decide — conformance fixtures (the cross-implementation contract)', () => {
  for (const f of FIXTURES) {
    it(`${f.tier} · ${f.c.modality}/${f.c.task} → ${f.expect.model ?? 'not-local-yet'}`, () => {
      expect(decide(f.tier, f.c)).toEqual(f.expect);
    });
  }
});

describe('decide — invariants over the whole matrix', () => {
  const ALL_TIERS: Tier[] = ['L1', 'L2-low', 'L2-high', 'L3', 'L3+'];
  const ALL: Classification[] = [
    ...(['general', 'creative', 'code', 'multilingual'] as const).map((task) => t('text', task)),
    ...(['transcription', 'speech', 'separation', 'music'] as const).map((task) => t('audio', task)),
    ...(['generate', 'edit', 'upscale'] as const).map((task) => t('image', task)),
    ...(['generate', 'enhance'] as const).map((task) => t('video', task)),
  ];

  it('model is null if and only if not-local-yet', () => {
    for (const tier of ALL_TIERS) {
      for (const c of ALL) {
        const d = decide(tier, c);
        expect(d.model === null).toBe(d.feasibility === 'not-local-yet');
      }
    }
  });
});
