import { describe, expect, it } from "vitest";
import type { ChatModelOption } from "./bridge";
import { recommendedChoice } from "./picker";

const STANDARD = ["small", "balanced", "powerful"];
const opt = (id: string, fitsMemory = true, fitsDisk = true): ChatModelOption => ({
  id, name: id, downloadMb: 1, runtimeMb: 1, installed: false, fitsMemory, fitsDisk,
});

describe("recommendedChoice", () => {
  it("recommends the most capable standard model that fits", () => {
    expect(recommendedChoice([opt("small"), opt("balanced"), opt("powerful")], STANDARD)).toBe("powerful");
    expect(recommendedChoice([opt("small"), opt("balanced"), opt("powerful", false)], STANDARD)).toBe("balanced");
    expect(recommendedChoice([opt("small"), opt("balanced", true, false), opt("powerful", false)], STANDARD)).toBe("small");
  });

  it("never recommends the experiment, even when it fits", () => {
    expect(recommendedChoice([opt("small"), opt("balanced"), opt("powerful"), opt("experimental")], STANDARD)).toBe("powerful");
  });

  it("falls back to the smallest when nothing fits, so there is always one obvious tap", () => {
    expect(recommendedChoice([opt("small", false), opt("balanced", false)], STANDARD)).toBe("small");
  });

  it("returns null with no options (browser preview before the backend answers)", () => {
    expect(recommendedChoice([], STANDARD)).toBeNull();
  });
});
