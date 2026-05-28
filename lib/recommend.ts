// Deterministic recommendation engine.
//
// Source of truth: verdure-orchestrator/knowledge/03-model-matrix.md (matrix
// + intent-shifts) and 04-quantization.md (Q4_K_M default).
// Throughput numbers are PROVISIONAL extrapolations and must be labeled
// as estimates in the UI until [OQ-03] / [OQ-05] are resolved with real
// device benchmarks (see verdure-orchestrator/knowledge/open-questions.md).

import type { Tier } from './tiers';
import type { Intent } from './intent';

export type Alternative = { name: string; reason: string };

export type Recommendation = {
  modelName: string;
  paramCount: string;        // e.g. "1B", "3B", "3.8B"
  quantization: string;      // e.g. "Q4_K_M"
  footprintGB: number;       // memory while running
  tokensPerSecEstimate: string; // e.g. "12–18" — always presented as an estimate
  why: string;
  alternatives: Alternative[];
};

export function recommend(tier: Tier, intent: Intent): Recommendation {
  switch (tier) {
    case 'L1':
      // 4 GB floor. Only 1B-class fits comfortably; intent does not shift
      // the model at this tier (the model class is too small to specialize).
      return {
        modelName: 'Gemma 3 1B',
        paramCount: '1B',
        quantization: 'Q4_K_M',
        footprintGB: 1.0,
        tokensPerSecEstimate: '12–18',
        why: 'The smallest capable general-purpose model. Fits comfortably in 4 GB and leaves room for the rest of your apps.',
        alternatives: [
          {
            name: 'Qwen 2.5 1.5B',
            reason: 'A bit larger; pick this if you mainly need stronger multilingual support and have closer to 5 GB free.',
          },
        ],
      };

    case 'L2-low':
      // 6–8 GB. 1.5B–3B class. Intent shifts between Llama 3.2 3B (general)
      // and Qwen 2.5 1.5B (multilingual).
      if (intent === 'multilingual') {
        return {
          modelName: 'Qwen 2.5 1.5B',
          paramCount: '1.5B',
          quantization: 'Q4_K_M',
          footprintGB: 1.1,
          tokensPerSecEstimate: '16–22',
          why: 'Strong across 35+ languages and very fast on 6–8 GB machines.',
          alternatives: [
            { name: 'Llama 3.2 3B', reason: 'Better general performance if you mostly work in English.' },
          ],
        };
      }
      return {
        modelName: 'Llama 3.2 3B',
        paramCount: '3B',
        quantization: 'Q4_K_M',
        footprintGB: 2.2,
        tokensPerSecEstimate: '12–18',
        why: 'Strong general-purpose model that fits comfortably in 6–8 GB.',
        alternatives: [
          { name: 'Qwen 2.5 1.5B', reason: 'Lighter and faster; better if you need multilingual support.' },
        ],
      };

    case 'L2-high':
      // 8 GB. Fully documented intent-shifts in 03-model-matrix.md.
      if (intent === 'creative') {
        return {
          modelName: 'Gemma 3 4B',
          paramCount: '4B',
          quantization: 'Q4_K_M',
          footprintGB: 2.9,
          tokensPerSecEstimate: '15–20',
          why: 'Punches above its weight on creative writing and brainstorming.',
          alternatives: [
            { name: 'Llama 3.2 3B', reason: 'Slightly faster; still strong on general tasks.' },
          ],
        };
      }
      if (intent === 'code') {
        return {
          modelName: 'Phi-4-mini',
          paramCount: '3.8B',
          quantization: 'Q4_K_M',
          footprintGB: 2.6,
          tokensPerSecEstimate: '18–25',
          why: 'Reasoning-strong on technical tasks; MIT-licensed.',
          alternatives: [
            { name: 'Llama 3.2 3B', reason: 'Better for general chat alongside code questions.' },
          ],
        };
      }
      if (intent === 'multilingual') {
        return {
          modelName: 'Qwen 2.5 3B',
          paramCount: '3B',
          quantization: 'Q4_K_M',
          footprintGB: 2.2,
          tokensPerSecEstimate: '18–25',
          why: 'Strong across 35+ languages and runs comfortably on 8 GB.',
          alternatives: [
            { name: 'Gemma 3 4B', reason: 'Slower but better for creative tasks if you switch contexts.' },
          ],
        };
      }
      return {
        modelName: 'Llama 3.2 3B',
        paramCount: '3B',
        quantization: 'Q4_K_M',
        footprintGB: 2.2,
        tokensPerSecEstimate: '20–28',
        why: 'Fast and strong for chat and quick questions on 8 GB.',
        alternatives: [
          { name: 'Phi-4-mini', reason: 'Better for technical questions.' },
          { name: 'Gemma 3 4B', reason: 'A little slower; better for longer creative output.' },
        ],
      };

    case 'L3':
    case 'L3+':
      // 16 GB+. 7B+ class. The matrix in 03-model-matrix.md does not
      // exhaustively document intent-shifts at this tier; the split below
      // is a reasonable extrapolation kept narrow on purpose. Revisit
      // once real-world L3 benchmarks land.
      if (intent === 'code') {
        return {
          modelName: 'Llama 3.1 8B',
          paramCount: '8B',
          quantization: 'Q4_K_M',
          footprintGB: 5.0,
          tokensPerSecEstimate: '25–40',
          why: 'Strong reasoning at the 8B class for code and technical questions.',
          alternatives: [
            { name: 'Mistral Small', reason: 'Apache 2.0; comparable quality for general tasks.' },
          ],
        };
      }
      return {
        modelName: 'Mistral Small',
        paramCount: '7B',
        quantization: 'Q4_K_M',
        footprintGB: 4.5,
        tokensPerSecEstimate: '25–40',
        why: 'Excellent general-purpose performance on Apple Silicon and modern hardware.',
        alternatives: [
          { name: 'Llama 3.1 8B', reason: 'A little larger; strong reasoning for technical work.' },
        ],
      };
  }
}
