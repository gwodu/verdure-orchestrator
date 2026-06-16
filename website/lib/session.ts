// Lightweight sessionStorage helpers for the four-screen recommendation flow.
// The user's typed intent and chosen device are carried across screens
// client-side — no server state required for v1, and nothing leaks across
// browser sessions.
//
// The device is stored as the chosen DeviceCard id (a plain-language choice);
// the internal Tier is derived from it via getTier. The card is the single
// source of truth for the selection — see lib/tiers.ts.
//
// All accessors are SSR-safe: they return null on the server (where
// sessionStorage doesn't exist) and clients call them from useEffect anyway.

import { findCard, type Tier } from '@verdure/decider';

const INTENT_KEY = 'verdure.flow.intent';
const DEVICE_KEY = 'verdure.flow.device'; // the chosen DeviceCard id

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

export function saveDevice(cardId: string): void {
  if (!isBrowser()) return;
  window.sessionStorage.setItem(DEVICE_KEY, cardId);
}

export function getDevice(): string | null {
  if (!isBrowser()) return null;
  return window.sessionStorage.getItem(DEVICE_KEY);
}

// The internal tier for the chosen device, derived from the card. Returns null
// if no device was chosen or the stored id is unknown.
export function getTier(): Tier | null {
  const id = getDevice();
  return id ? findCard(id)?.tier ?? null : null;
}

export function clearFlow(): void {
  if (!isBrowser()) return;
  window.sessionStorage.removeItem(INTENT_KEY);
  window.sessionStorage.removeItem(DEVICE_KEY);
}
