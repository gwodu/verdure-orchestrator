import { describe, it, expect } from 'vitest';
import { INTENT_CARDS, taskLabel } from './modality';
import { classifyIntent } from './intent';

describe('INTENT_CARDS', () => {
  it('each card has a unique id', () => {
    const ids = INTENT_CARDS.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("each card's example phrase classifies to its declared (modality, task)", () => {
    // The contract that lets a card skip the classifier: picking it feeds
    // `example` into the flow, and the classifier must land on `classification`.
    // If this fails, the card or the classifier keywords drifted.
    for (const card of INTENT_CARDS) {
      expect(classifyIntent(card.example), `card "${card.id}"`).toEqual(card.classification);
    }
  });

  it('every card has a human label and non-empty copy', () => {
    for (const card of INTENT_CARDS) {
      expect(card.title.length).toBeGreaterThan(0);
      expect(card.description.length).toBeGreaterThan(0);
      expect(taskLabel(card.classification).length).toBeGreaterThan(0);
    }
  });
});
