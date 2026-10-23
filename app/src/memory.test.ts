import { beforeEach, describe, expect, it } from "vitest";
import { askedToRemember, heldLine, markAskedToRemember, shouldAskToRemember } from "./memory";

describe("asking to remember", () => {
  beforeEach(() => localStorage.clear());

  it("asks once, only where there is something to allow", () => {
    expect(shouldAskToRemember("off", false)).toBe(true);
    expect(shouldAskToRemember("on", false)).toBe(false);
    expect(shouldAskToRemember("unsupported", false)).toBe(false);
    expect(shouldAskToRemember("off", true)).toBe(false);
  });

  it("remembers that it asked", () => {
    expect(askedToRemember()).toBe(false);
    markAskedToRemember();
    expect(askedToRemember()).toBe(true);
    expect(shouldAskToRemember("off", askedToRemember())).toBe(false);
  });
});

describe("heldLine — what is held, in words", () => {
  it("messages with their conversations", () => {
    expect(heldLine({ items: 19, threads: 2, notices: 0 })).toBe("19 messages in 2 conversations");
    expect(heldLine({ items: 1, threads: 1, notices: 0 })).toBe("1 message in 1 conversation");
  });

  it("notifications alone, and mixed", () => {
    expect(heldLine({ items: 4, threads: 0, notices: 4 })).toBe("4 notifications");
    expect(heldLine({ items: 23, threads: 2, notices: 4 })).toBe("19 messages in 2 conversations · 4 notifications");
  });

  it("nothing", () => {
    expect(heldLine({ items: 0, threads: 0, notices: 0 })).toBe("nothing yet");
  });
});
