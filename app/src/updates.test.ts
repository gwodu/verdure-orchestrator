// The consent rule around update checks (knowledge/23).
//
// These test a PROMISE, not a feature: Verdure does not reach the network on its
// own unless the user allowed it. That promise is easy to break by accident —
// one early `check()` added to a launch path and it is silently gone, with no
// visible symptom on any device. So the assertion that matters most here is a
// negative one: with the wrong preference, the network function is never called.

import { describe, it, expect, vi, beforeEach } from "vitest";

const checkForUpdate = vi.fn();
vi.mock("./bridge", () => ({
  checkForUpdate: (...a: unknown[]) => checkForUpdate(...a),
  installUpdate: vi.fn(),
  updatesSupported: () => true,
}));

import { checkOnLaunch, setUpdateMode, shouldAskAboutUpdates, updateMode } from "./updates";

beforeEach(() => {
  localStorage.clear();
  checkForUpdate.mockReset();
  checkForUpdate.mockResolvedValue({ version: "0.2.0", notes: "" });
});

describe("update consent", () => {
  it("asks before it ever checks", async () => {
    expect(updateMode()).toBeNull();
    expect(shouldAskAboutUpdates()).toBe(true);
    // The crucial one: an un-answered question must not be treated as a yes.
    await expect(checkOnLaunch()).resolves.toBeNull();
    expect(checkForUpdate).not.toHaveBeenCalled();
  });

  it("never touches the network when the user said don't check", async () => {
    setUpdateMode("manual");
    await expect(checkOnLaunch()).resolves.toBeNull();
    expect(checkForUpdate).not.toHaveBeenCalled();
    // And having chosen, they are not asked again.
    expect(shouldAskAboutUpdates()).toBe(false);
  });

  it("checks on launch for both modes that opted in", async () => {
    for (const mode of ["notify", "auto"] as const) {
      checkForUpdate.mockClear();
      setUpdateMode(mode);
      await expect(checkOnLaunch()).resolves.toEqual({ version: "0.2.0", notes: "" });
      expect(checkForUpdate).toHaveBeenCalledTimes(1);
    }
  });

  it("stays quiet when a check fails, rather than interrupting launch", async () => {
    setUpdateMode("notify");
    checkForUpdate.mockRejectedValue(new Error("offline"));
    // Must resolve, not throw: an unhandled rejection here would surface as a
    // broken launch for someone whose only crime is having no wifi.
    await expect(checkOnLaunch()).resolves.toBeNull();
  });

  it("keeps the choice reversible and ignores a corrupted stored value", () => {
    setUpdateMode("auto");
    expect(updateMode()).toBe("auto");
    setUpdateMode("manual");
    expect(updateMode()).toBe("manual");
    // A hand-edited or half-written localStorage value must read as "never
    // asked" — the safe direction, since it asks again rather than assuming yes.
    localStorage.setItem("verdure.updates.mode", "yes-please");
    expect(updateMode()).toBeNull();
    expect(shouldAskAboutUpdates()).toBe(true);
  });
});
