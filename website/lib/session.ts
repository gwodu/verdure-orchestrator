// Lightweight sessionStorage helpers for the four-screen recommendation flow.
// The user's typed intent and chosen tier are carried across screens client-side
// — no server state required for v1, and nothing leaks across browser sessions.
//
// All accessors are SSR-safe: they return null on the server (where sessionStorage
// doesn't exist) and clients call them from useEffect anyway.

import { isTier, type Tier } from './tiers';

const INTENT_KEY = 'verdure.flow.intent';
const TIER_KEY = 'verdure.flow.tier';

function isBrowser(): boolean {
  return typeof window !== 'undefined' && typeof window.sessionStorage !== 'undefined';
}

export function saveIntent(text: string): void {
  if (!isBrowser()) return;
  window.sessionStorage.setItem(INTENT_KEY, text);
}

export function getIntent(): string | null {
  if (!isBrowser()) return null;
  return window.sessionStorage.getItem(INTENT_KEY);
}

export function saveTier(tier: Tier): void {
  if (!isBrowser()) return;
  window.sessionStorage.setItem(TIER_KEY, tier);
}

export function getTier(): Tier | null {
  if (!isBrowser()) return null;
  const raw = window.sessionStorage.getItem(TIER_KEY);
  return raw && isTier(raw) ? raw : null;
}

export function clearFlow(): void {
  if (!isBrowser()) return;
  window.sessionStorage.removeItem(INTENT_KEY);
  window.sessionStorage.removeItem(TIER_KEY);
}
