// Verdure's memory, the rules side (knowledge/25): when the app asks to be
// allowed to remember, and how what is held is said in plain words.
// `bridge.ts` owns the mechanism (the store, the listener's state) and never
// consults these, so exactly one place decides.

const ASKED_KEY = "verdure.memory.asked";

/** What the OS says about the feed (lib.rs `listener_state`). */
export type FeedState = "on" | "off" | "unsupported";

/** Whether the app has already asked, once, to be allowed to remember. */
export function askedToRemember(): boolean {
  return localStorage.getItem(ASKED_KEY) === "1";
}

export function markAskedToRemember(): void {
  localStorage.setItem(ASKED_KEY, "1");
}

/**
 * Ask in the chat, once, on a phone that can do it and hasn't allowed it
 * yet. Never on iOS or the desktop (nothing to allow), never again after
 * the user answered either way — the book icon is where it lives after that.
 */
export function shouldAskToRemember(feed: FeedState, askedBefore: boolean): boolean {
  return feed === "off" && !askedBefore;
}

/** What the user reads when asked (skills/copywriting.md: the benefit, then
 *  the honest mechanism, then exactly what the tap does). */
export const REMEMBER_ASK =
  "Verdure can remember what your phone shows you — messages, reminders, receipts — so you can ask about them later, like “what did Tunde say about the rent?”. It all stays on this phone; nothing is sent anywhere. Android asks you to allow this in Settings, under “Notification access”.";

/** The line under the ask once it has been declined: where it lives now. */
export const REMEMBER_LATER = "No problem. You can turn it on any time from the book icon at the top.";

/** What is held from one app, in words: "19 messages in 2 conversations · 4 notifications". */
export function heldLine(c: { items: number; threads: number; notices: number }): string {
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
  const messages = c.items - c.notices;
  const parts: string[] = [];
  if (messages > 0) {
    parts.push(plural(messages, "message") + (c.threads ? ` in ${plural(c.threads, "conversation")}` : ""));
  }
  if (c.notices > 0) parts.push(plural(c.notices, "notification"));
  return parts.join(" · ") || "nothing yet";
}
