// The rules around web search (knowledge/24). Like updates.test.ts, these test
// PROMISES: a search never runs on a connection the user didn't allow, nothing
// is sent to decide whether to offer one, and the default spends no mobile data.

import { describe, it, expect, beforeEach } from "vitest";
import {
  WEB_DISCLOSURE,
  disclosureSeen,
  looksTimeSensitive,
  markDisclosureSeen,
  searchGate,
  setWebEnabled,
  setWebMode,
  webEnabled,
  webMode,
} from "./web";

beforeEach(() => localStorage.clear());

describe("the switch", () => {
  it("is off until the user turns it on — the one network feature is opt-in", () => {
    expect(webEnabled()).toBe(false);
    setWebEnabled(true);
    expect(webEnabled()).toBe(true);
    setWebEnabled(false);
    expect(webEnabled()).toBe(false);
  });
});

describe("web mode", () => {
  it("defaults to Wi-Fi only, and remembers a change", () => {
    expect(webMode()).toBe("wifi");
    setWebMode("any");
    expect(webMode()).toBe("any");
  });

  it("treats a corrupt stored value as the default", () => {
    localStorage.setItem("verdure.web.mode", "always-yes");
    expect(webMode()).toBe("wifi");
  });
});

describe("the data rule", () => {
  it("searches on Wi-Fi, asks on mobile data, says so when offline", () => {
    expect(searchGate("wifi", "wifi")).toBe("search");
    expect(searchGate("wifi", "cellular")).toBe("ask-mobile-data");
    expect(searchGate("wifi", "offline")).toBe("offline");
  });

  it("asks when the platform cannot tell what the connection is", () => {
    // Android today: an unknown connection must not be spent on a guess.
    expect(searchGate("wifi", "unknown")).toBe("ask-mobile-data");
  });

  it("searches on any connection once the user allowed mobile data", () => {
    expect(searchGate("any", "cellular")).toBe("search");
    expect(searchGate("any", "unknown")).toBe("search");
    expect(searchGate("any", "offline")).toBe("offline");
  });
});

describe("offering a search", () => {
  it("fast-paths questions that plainly need fresh information", () => {
    for (const q of [
      "What's the naira to dollar rate today?",
      "Who won the AFCON final?",
      "latest news in Lagos",
      "Is the weather in Nairobi going to be rainy tomorrow?",
      "When is the next iPhone released?",
      "fuel price in Accra",
      "What happened in the 2026 election?",
    ]) {
      expect(looksTimeSensitive(q), q).toBe(true);
    }
  });

  it("stays quiet for timeless questions", () => {
    for (const q of [
      "What's the chemical composition of water?",
      "How do you bake a pie?",
      "How do the northern lights work?",
      "Tell me a surprising fact about honeybees",
      "Write a short poem about the sea",
      "There is nowhere I'd rather be",
      "Explain photosynthesis",
    ]) {
      expect(looksTimeSensitive(q), q).toBe(false);
    }
  });
});

describe("the disclosure", () => {
  it("is shown until it has been seen once", () => {
    expect(disclosureSeen()).toBe(false);
    markDisclosureSeen();
    expect(disclosureSeen()).toBe(true);
  });

  it("names every party that sees anything, and nothing it can't promise", () => {
    expect(WEB_DISCLOSURE).toContain("only short search words");
    expect(WEB_DISCLOSURE).toContain("never the conversation");
    expect(WEB_DISCLOSURE).toContain("Brave");
    expect(WEB_DISCLOSURE).toContain("verdureai.com");
    expect(WEB_DISCLOSURE).toContain("90 days");
    // No absolute claim the hosting provider makes false (knowledge/24).
    expect(WEB_DISCLOSURE).not.toMatch(/nothing is (ever )?logged/i);
  });
});
