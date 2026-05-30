// Hardware tiers and the plain-language device picker that maps onto them.
//
// The tier specs (RAM, graphics, device archetypes) are the INTERNAL source of
// truth and live in verdure-orchestrator/knowledge/02-hardware-tiers.md. The
// user never sees a spec — no RAM, no "GPU", no "Apple Silicon". They pick the
// card that sounds like their computer, and each card maps to one tier we
// reason about. See skills/design.md, "Assume zero technical knowledge".

export type Tier = 'L1' | 'L2-low' | 'L2-high' | 'L3' | 'L3+';

const TIERS: readonly Tier[] = ['L1', 'L2-low', 'L2-high', 'L3', 'L3+'];

// A user-facing device card: described entirely in lived experience, each
// mapping to one internal tier. The card also quietly carries the graphics
// signal the multimodal engine needs — a gaming computer implies a real GPU,
// a newer Mac implies Apple Silicon — without ever asking a technical
// question. See knowledge/12-multimodal-models.md for why that signal matters.
export type DeviceCard = {
  id: string;
  emoji: string;
  title: string;
  description: string;
  tier: Tier;
};

export const DEVICE_CARDS: DeviceCard[] = [
  {
    id: 'newer-mac',
    emoji: '🍎',
    title: 'A newer Mac',
    description: 'A MacBook or iMac from about 2021 or later.',
    tier: 'L3',
  },
  {
    id: 'older-mac',
    emoji: '🍎',
    title: 'An older Mac',
    description: 'A Mac from before about 2021.',
    tier: 'L2-high',
  },
  {
    id: 'laptop',
    emoji: '💻',
    title: 'A regular laptop',
    description: 'A Windows laptop for everyday things — work, email, browsing.',
    tier: 'L2-low',
  },
  {
    id: 'gaming',
    emoji: '🎮',
    title: 'A gaming computer',
    description: 'Built for games or video editing — big and fast, sometimes with colorful lights.',
    tier: 'L3+',
  },
  {
    id: 'older-basic',
    emoji: '🐢',
    title: 'An older or basic computer',
    description: 'Several years old, or it feels slow with everyday tasks.',
    tier: 'L1',
  },
  {
    id: 'unsure',
    emoji: '🤔',
    title: 'I’m not sure',
    description: 'No problem — we’ll start with a safe match you can change on the next screen.',
    tier: 'L2-low',
  },
];

export function findCard(id: string): DeviceCard | undefined {
  return DEVICE_CARDS.find((c) => c.id === id);
}

export function isTier(value: string): value is Tier {
  return (TIERS as readonly string[]).includes(value);
}
