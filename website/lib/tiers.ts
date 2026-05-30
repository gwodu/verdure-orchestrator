// Hardware tier definitions and the user-facing device buckets.
// Source of truth: verdure-orchestrator/knowledge/02-hardware-tiers.md
// When the bucket list here changes, mirror the change in 02-hardware-tiers.md.

export type Tier = 'L1' | 'L2-low' | 'L2-high' | 'L3' | 'L3+';

export type DeviceBucket = {
  id: Tier;
  label: string;
  description: string;
  ramRange: string;
  example: string;
};

export const DEVICE_BUCKETS: DeviceBucket[] = [
  {
    id: 'L1',
    label: 'Older laptop',
    description: '5+ years old, basic specs',
    ramRange: '~4 GB RAM',
    example: 'e.g. older Dell Inspiron, HP Pavilion, low-spec Chromebook',
  },
  {
    id: 'L2-low',
    label: 'Standard laptop',
    description: '3–5 years old',
    ramRange: '6–8 GB RAM',
    example: 'e.g. mid-tier Windows laptop, base-spec ThinkPad',
  },
  {
    id: 'L2-high',
    label: 'Recent laptop or Intel MacBook',
    description: '8 GB and recent',
    ramRange: '8 GB RAM',
    example: 'e.g. 2019 MacBook Pro, modern Windows laptop',
  },
  {
    id: 'L3',
    label: 'MacBook with Apple Silicon (M1+)',
    description: 'M1, M2, M3, or M4',
    ramRange: '16 GB+ unified',
    example: 'e.g. MacBook Air M2, MacBook Pro M3',
  },
  {
    id: 'L3+',
    label: 'Workstation or Gaming PC',
    description: 'Desktop with dedicated GPU',
    ramRange: '16–32 GB + GPU',
    example: 'e.g. gaming PC, workstation tower',
  },
];

export function findBucket(tier: Tier): DeviceBucket | undefined {
  return DEVICE_BUCKETS.find((b) => b.id === tier);
}

export function isTier(value: string): value is Tier {
  return (DEVICE_BUCKETS as { id: string }[]).some((b) => b.id === value);
}
