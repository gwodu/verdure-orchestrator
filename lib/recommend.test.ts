import { describe, it, expect } from 'vitest';
import { recommend } from './recommend';
import type { Tier } from './tiers';
import type { Intent } from './intent';

describe('recommend', () => {
  it('L1 always returns Gemma 3 1B regardless of intent', () => {
    const intents: Intent[] = ['general', 'creative', 'code', 'multilingual'];
    for (const i of intents) {
      const r = recommend('L1', i);
      expect(r.modelName).toBe('Gemma 3 1B');
      expect(r.footprintGB).toBeLessThan(2);
    }
  });

  it('L2-low shifts to Qwen 2.5 1.5B for multilingual', () => {
    expect(recommend('L2-low', 'multilingual').modelName).toBe('Qwen 2.5 1.5B');
    expect(recommend('L2-low', 'general').modelName).toBe('Llama 3.2 3B');
  });

  it('L2-high intent shifts match the matrix in 03-model-matrix.md', () => {
    expect(recommend('L2-high', 'creative').modelName).toBe('Gemma 3 4B');
    expect(recommend('L2-high', 'code').modelName).toBe('Phi-4-mini');
    expect(recommend('L2-high', 'multilingual').modelName).toBe('Qwen 2.5 3B');
    expect(recommend('L2-high', 'general').modelName).toBe('Llama 3.2 3B');
  });

  it('L3 and L3+ pick 7B+ models', () => {
    for (const t of ['L3', 'L3+'] as Tier[]) {
      expect(recommend(t, 'general').modelName).toBe('Mistral Small');
      expect(recommend(t, 'code').modelName).toBe('Llama 3.1 8B');
    }
  });

  it('every recommendation defaults to Q4_K_M quantization', () => {
    const tiers: Tier[] = ['L1', 'L2-low', 'L2-high', 'L3', 'L3+'];
    const intents: Intent[] = ['general', 'creative', 'code', 'multilingual'];
    for (const t of tiers) {
      for (const i of intents) {
        expect(recommend(t, i).quantization).toBe('Q4_K_M');
      }
    }
  });

  it('every recommendation has a footprint that fits the tier (RAM envelope)', () => {
    // The model footprint must leave headroom for the OS + other apps.
    // 4 GB tier → < 2 GB footprint. 6–8 GB tier → < 4 GB. 16+ GB → < 8 GB.
    const cases: { tier: Tier; maxGB: number }[] = [
      { tier: 'L1', maxGB: 2 },
      { tier: 'L2-low', maxGB: 4 },
      { tier: 'L2-high', maxGB: 4 },
      { tier: 'L3', maxGB: 8 },
      { tier: 'L3+', maxGB: 8 },
    ];
    const intents: Intent[] = ['general', 'creative', 'code', 'multilingual'];
    for (const { tier, maxGB } of cases) {
      for (const i of intents) {
        expect(recommend(tier, i).footprintGB).toBeLessThan(maxGB);
      }
    }
  });

  it('every recommendation surfaces at least one alternative', () => {
    const r = recommend('L2-high', 'creative');
    expect(r.alternatives.length).toBeGreaterThan(0);
    expect(r.alternatives[0].name).toBeTruthy();
    expect(r.alternatives[0].reason).toBeTruthy();
  });
});
