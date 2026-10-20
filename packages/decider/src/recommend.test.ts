import { describe, it, expect } from 'vitest';
import { recommend, type Recommendation } from './recommend';
import type { Tier } from './tiers';
import type { Classification } from './modality';

const ALL_TIERS: Tier[] = ['L1', 'L2-low', 'L2-high', 'L3', 'L3+'];
const CPU_TIERS: Tier[] = ['L1', 'L2-low', 'L2-high'];

const text = (task: Classification['task']) => ({ modality: 'text', task } as Classification);

describe('recommend — text (model picks preserved from the original matrix)', () => {
  it('L1 returns Gemma 3 1B regardless of task', () => {
    for (const task of ['general', 'creative', 'code', 'multilingual'] as const) {
      expect(recommend('L1', text(task)).model?.name).toBe('Gemma 3 1B');
    }
  });

  it('L2-low shifts to Qwen 2.5 1.5B for multilingual', () => {
    expect(recommend('L2-low', text('multilingual')).model?.name).toBe('Qwen 2.5 1.5B');
    expect(recommend('L2-low', text('general')).model?.name).toBe('Llama 3.2 3B');
  });

  it('L2-high intent shifts match 03-model-matrix.md', () => {
    expect(recommend('L2-high', text('creative')).model?.name).toBe('Gemma 3 4B');
    expect(recommend('L2-high', text('code')).model?.name).toBe('Phi-4-mini');
    expect(recommend('L2-high', text('multilingual')).model?.name).toBe('Qwen 2.5 3B');
    expect(recommend('L2-high', text('general')).model?.name).toBe('Llama 3.2 3B');
  });

  it('L3 / L3+ pick 7B+ models', () => {
    for (const t of ['L3', 'L3+'] as Tier[]) {
      expect(recommend(t, text('general')).model?.name).toBe('Mistral Small');
      expect(recommend(t, text('code')).model?.name).toBe('Llama 3.1 8B');
    }
  });

  it('text always offers the three install paths incl. an Ollama command', () => {
    const r = recommend('L2-high', text('creative'));
    const names = r.installOptions.map((o) => o.name);
    expect(names).toEqual(['LM Studio', 'Jan', 'Ollama']);
    expect(r.installOptions.find((o) => o.name === 'Ollama')?.command).toContain('ollama run');
  });
});

describe('recommend — audio', () => {
  it('transcription is recommended on every tier (CPU-feasible)', () => {
    for (const t of ALL_TIERS) {
      const r = recommend(t, { modality: 'audio', task: 'transcription' });
      expect(r.feasibility).toBe('recommended');
      expect(r.model?.name).toBe('Whisper');
    }
  });

  it('text-to-speech is recommended on every tier', () => {
    for (const t of ALL_TIERS) {
      expect(recommend(t, { modality: 'audio', task: 'speech' }).feasibility).toBe('recommended');
    }
  });

  it('stem separation works-but-slow on laptops, recommended with graphics power', () => {
    for (const t of CPU_TIERS) {
      expect(recommend(t, { modality: 'audio', task: 'separation' }).feasibility).toBe('works-but-slow');
    }
    expect(recommend('L3+', { modality: 'audio', task: 'separation' }).feasibility).toBe('recommended');
  });

  it('music generation is honestly declined on laptops, available on a GPU', () => {
    for (const t of CPU_TIERS) {
      const r = recommend(t, { modality: 'audio', task: 'music' });
      expect(r.feasibility).toBe('not-local-yet');
      expect(r.model).toBeNull();
    }
    expect(recommend('L3+', { modality: 'audio', task: 'music' }).feasibility).toBe('recommended');
  });
});

describe('recommend — image', () => {
  it('upscaling is recommended on every tier (CPU-friendly)', () => {
    for (const t of ALL_TIERS) {
      expect(recommend(t, { modality: 'image', task: 'upscale' }).feasibility).toBe('recommended');
    }
  });

  it('generation is declined on plain laptops, recommended on Mac / GPU', () => {
    for (const t of CPU_TIERS) {
      expect(recommend(t, { modality: 'image', task: 'generate' }).feasibility).toBe('not-local-yet');
    }
    expect(recommend('L3', { modality: 'image', task: 'generate' }).feasibility).toBe('recommended');
    expect(recommend('L3+', { modality: 'image', task: 'generate' }).feasibility).toBe('recommended');
  });

  it('editing is declined on plain laptops, recommended on Mac / GPU', () => {
    expect(recommend('L2-high', { modality: 'image', task: 'edit' }).feasibility).toBe('not-local-yet');
    expect(recommend('L3', { modality: 'image', task: 'edit' }).feasibility).toBe('recommended');
  });
});

describe('recommend — video', () => {
  it('generation is declined on everything except a dedicated GPU', () => {
    for (const t of [...CPU_TIERS, 'L3'] as Tier[]) {
      const r = recommend(t, { modality: 'video', task: 'generate' });
      expect(r.feasibility).toBe('not-local-yet');
      expect(r.model).toBeNull();
    }
    // Even on a GPU it's only "works-but-slow", never a clean recommend.
    expect(recommend('L3+', { modality: 'video', task: 'generate' }).feasibility).toBe('works-but-slow');
  });

  it('enhancement works-but-slow on laptops, recommended with graphics power', () => {
    expect(recommend('L2-high', { modality: 'video', task: 'enhance' }).feasibility).toBe('works-but-slow');
    expect(recommend('L3+', { modality: 'video', task: 'enhance' }).feasibility).toBe('recommended');
  });
});

describe('recommend — invariants across the whole matrix', () => {
  const everyClassification: Classification[] = [
    ...(['general', 'creative', 'code', 'multilingual'] as const).map(text),
    ...(['transcription', 'speech', 'separation', 'music'] as const).map(
      (task) => ({ modality: 'audio', task } as Classification),
    ),
    ...(['generate', 'edit', 'upscale'] as const).map(
      (task) => ({ modality: 'image', task } as Classification),
    ),
    ...(['generate', 'enhance'] as const).map((task) => ({ modality: 'video', task } as Classification)),
  ];

  const all: Recommendation[] = ALL_TIERS.flatMap((t) => everyClassification.map((c) => recommend(t, c)));

  it('always has a jargon-free headline and an honest body', () => {
    for (const r of all) {
      expect(r.headline.length).toBeGreaterThan(0);
      expect(r.body.length).toBeGreaterThan(0);
      expect(r.taskLabel.length).toBeGreaterThan(0);
    }
  });

  it('not-local-yet means no model and nothing to install; otherwise a model is present', () => {
    for (const r of all) {
      if (r.feasibility === 'not-local-yet') {
        expect(r.model).toBeNull();
        expect(r.installOptions).toHaveLength(0);
      } else {
        expect(r.model?.name).toBeTruthy();
      }
    }
  });

  // knowledge/12 (corrected 2026-10-20): the friendly install path must be able
  // to run the model it is offered for. Fooocus runs SDXL only.
  it('never offers Fooocus for a model it cannot run', () => {
    for (const tier of ['L1', 'L2-low', 'L2-high', 'L3', 'L3+'] as const) {
      for (const task of ['generate', 'edit', 'upscale'] as const) {
        const rec = recommend(tier, { modality: 'image', task });
        if (rec.installOptions?.some((o) => o.name === 'Fooocus')) {
          expect(rec.model?.name, `${tier} image ${task}`).toMatch(/Stable Diffusion/);
        }
      }
    }
  });
});
