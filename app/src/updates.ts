// Update preferences and the consent that governs them (knowledge/23).
//
// Verdure's principle is that nothing leaves the device without the user asking
// for it (knowledge/06). An update check is a network request to our server: it
// says someone runs Verdure and roughly when. That is small, but it is exactly
// the class of quiet signal we promise not to collect — so it is opt-in, chosen
// once, plainly worded, and reversible from Storage at any time.
//
// This module owns the RULE. `bridge.ts` owns the mechanism and deliberately
// does not consult the stored preference, so there is exactly one place that
// decides whether an automatic check may happen.

import { checkForUpdate, installUpdate, updatesSupported, type UpdateInfo } from "./bridge";

/** How the user wants to hear about new versions. */
export type UpdateMode =
  /** Download and install new versions in the background. */
  | "auto"
  /** Check on launch and tell them; installing stays their choice. */
  | "notify"
  /** Never check on its own. The button in Storage still works. */
  | "manual";

const MODE_KEY = "verdure.updates.mode";

/** The stored choice, or `null` if we have never asked. */
export function updateMode(): UpdateMode | null {
  const v = localStorage.getItem(MODE_KEY);
  return v === "auto" || v === "notify" || v === "manual" ? v : null;
}

export function setUpdateMode(mode: UpdateMode): void {
  localStorage.setItem(MODE_KEY, mode);
}

/**
 * Whether to put the one-time question in front of the user. Only on desktop —
 * asking an iPad user how they'd like Verdure to update would be a question
 * about a thing that cannot happen, since iOS updates through the App Store.
 */
export function shouldAskAboutUpdates(): boolean {
  return updatesSupported() && updateMode() === null;
}

/**
 * The launch-time check. Returns an update only when the user's own choice
 * allows a check to happen at all — `manual` returns `null` without touching
 * the network, which is the whole point of offering it.
 *
 * Never throws: a failed check is not something to interrupt someone's launch
 * with. If the network is down or the endpoint is wrong, Verdure stays quiet
 * and the Storage button remains available to try deliberately.
 */
export async function checkOnLaunch(): Promise<UpdateInfo | null> {
  const mode = updateMode();
  if (mode !== "auto" && mode !== "notify") return null;
  try {
    return await checkForUpdate();
  } catch {
    return null;
  }
}

export { checkForUpdate, installUpdate, updatesSupported };
export type { UpdateInfo };
