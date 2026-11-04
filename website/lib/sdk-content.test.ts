import { describe, expect, it } from 'vitest';
import { API, SAMPLES, SPEECH, TIERS } from './sdk-content';

// The names the API table lists, in both spellings the bindings use.
const methodNames = API.map((r) => r.method.replace(/\(.*$/, ''));
const snake = (s: string) => s.replace(/[A-Z]/g, (c) => '_' + c.toLowerCase());
const known = new Set([...methodNames, ...methodNames.map(snake)]);

describe('the hello-world samples only call methods the SDK has', () => {
  it.each(SAMPLES.map((s) => [s.label, s] as const))('%s', (_label, sample) => {
    // Every `sdk.<name>(` call in the sample.
    const calls = [...sample.code.matchAll(/\bsdk\.([a-zA-Z_]+)\(/g)].map((m) => m[1]);
    expect(calls.length).toBeGreaterThan(5);
    for (const c of calls) expect(known, `${c} is not in the API table`).toContain(c);
  });

  it('every sample walks the whole flow: start, choose, download, load, transcribe, extract', () => {
    for (const s of SAMPLES) {
      for (const step of ['start', 'recommend', 'download', 'load', 'transcribe', 'extract']) {
        expect(s.code.toLowerCase(), `${s.label} lacks ${step}`).toContain(step);
      }
    }
  });
});

describe('the numbers on the page are the measured ones', () => {
  it('names the two shipped scribe bundles with their measured phone speeds', () => {
    expect(SPEECH.map((r) => r.bundle)).toEqual(['scribe-floor', 'scribe-small']);
    expect(SPEECH[1].pixel).toBe('3.2×');
  });
  it('lists the six tiers in knowledge/02 order', () => {
    expect(TIERS.map((t) => t.tier)).toEqual(['P1', 'P2', 'P3', 'L1', 'L2', 'L3']);
  });
});
