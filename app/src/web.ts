// Web search: the rules around the one thing Verdure sends off the device
// (knowledge/24). This module owns the RULES — when a search is offered, whether
// the network may be used, what the user is told. `bridge.ts` owns the
// mechanism and never consults these, so exactly one place decides.
//
// The promise these rules serve: only the search words the user approved leave
// the device, and only when they said the connection may be used.

/** Which connections a search may use, once web search is on. */
export type WebMode =
  /** Wi-Fi only (the default). */
  | "wifi"
  /** Any connection, mobile data included. */
  | "any";

/** What the OS says about reaching the relay (web.rs `NetState`). */
export type NetState = "offline" | "wifi" | "cellular" | "unknown";

/** The decision for one approved search. */
export type SearchGate =
  /** Go: the connection is one the user allowed. */
  | "search"
  /** On mobile data (or the platform can't tell) with Wi-Fi-only set: ask. */
  | "ask-mobile-data"
  /** No connection: answer from memory and say the web wasn't checked. */
  | "offline";

const ENABLED_KEY = "verdure.web.enabled";
const MODE_KEY = "verdure.web.mode";
const DISCLOSURE_SEEN_KEY = "verdure.web.disclosureSeen";

/**
 * The switch itself — the one consent (knowledge/24). Off until the user turns
 * it on: web search is the one feature that sends anything off the device, and
 * `06` says such things are opt-in. While it is on, searches run without a
 * per-search tap; what was searched is always shown under the answer.
 */
export function webEnabled(): boolean {
  return localStorage.getItem(ENABLED_KEY) === "1";
}

export function setWebEnabled(on: boolean): void {
  localStorage.setItem(ENABLED_KEY, on ? "1" : "0");
}

/**
 * The connection rule. Defaults to Wi-Fi only — the connection that costs
 * nothing on a data plan. Changed from Storage, or by the one-time ask when a
 * search would run on mobile data.
 */
export function webMode(): WebMode {
  const v = localStorage.getItem(MODE_KEY);
  return v === "wifi" || v === "any" ? v : "wifi";
}

export function setWebMode(mode: WebMode): void {
  localStorage.setItem(MODE_KEY, mode);
}

/**
 * The data rule for one search. `unknown` is treated like cellular: when the
 * platform cannot say (Android today), asking is the honest default — silently
 * spending someone's data on a guess is what the Wi-Fi-only setting exists to
 * stop. Callers check `webEnabled()` first; this decides only the connection.
 */
export function searchGate(mode: WebMode, net: NetState): SearchGate {
  if (net === "offline") return "offline";
  if (net === "wifi") return "search";
  return mode === "any" ? "search" : "ask-mobile-data";
}

/**
 * Does this message look like it needs fresh information? Deterministic and
 * local — nothing is sent to decide this. With web search on, a match searches
 * BEFORE asking the model (the fast path); anything else is the model's call
 * through its `SEARCH:` line. Whole-word matches, so "nowhere" doesn't trip
 * "now" (the intent classifier's lesson, knowledge/13).
 */
export function looksTimeSensitive(text: string): boolean {
  return TIME_SENSITIVE.test(text);
}

const TIME_SENSITIVE = new RegExp(
  "\\b(" +
    [
      "today", "tonight", "yesterday", "tomorrow", "this (week|month|year|weekend|morning|evening)",
      "right now", "currently", "current", "latest", "newest", "recent(ly)?", "nowadays",
      "news", "headlines?", "breaking",
      "prices?", "costs?", "how much (is|are|does)", "rates?", "exchange", "dollar", "naira", "cedi", "shilling", "rand",
      "scores?", "results?", "fixtures?", "who won", "standings", "table",
      "weather", "forecast", "temperature", "rain(ing)?",
      "open(ing)? hours", "is .{1,30} open", "when (is|does|do|will)", "what time",
      "released?", "release date", "announced?", "launch(ed)?", "update[sd]?", "version",
      "election", "president", "minister", "governor",
      "stock", "shares", "bitcoin", "crypto",
      "202[4-9]", "203\\d",
    ].join("|") +
    ")\\b",
  "i",
);

/** Whether the user has been shown what leaves the device at least once. */
export function disclosureSeen(): boolean {
  return localStorage.getItem(DISCLOSURE_SEEN_KEY) === "1";
}

export function markDisclosureSeen(): void {
  localStorage.setItem(DISCLOSURE_SEEN_KEY, "1");
}

/**
 * What leaves the device, in one honest paragraph (knowledge/24). Shown when
 * web search is first switched on, and behind a small "What leaves your
 * device?" toggle in Storage and under every searched answer. Every sentence
 * is a checkable claim; change the code, change the sentence.
 */
export const WEB_DISCLOSURE =
  "While web search is on, only short search words for your question leave your device — never the conversation. " +
  "You can see them under each answer. Verdure's relay forwards them to Brave and keeps no record of them. " +
  "Our hosting provider briefly records that your device contacted verdureai.com, but not what you asked. " +
  "Brave may keep the search words for up to 90 days.";
