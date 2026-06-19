import { describe, it, expect } from "vitest";
import {
  estimateTranscriptionSpeed,
  bestModelFor,
  deviceScore,
  type DeviceSpec,
  type WhisperSize,
} from "./performance";

const intelLaptop: DeviceSpec = { arch: "intel", cores: 4, ramGb: 8 };
const appleSilicon: DeviceSpec = { arch: "apple-silicon", cores: 8, ramGb: 16 };
const weakOld: DeviceSpec = { arch: "intel", cores: 2, ramGb: 4 };

describe("deviceScore", () => {
  it("rates Apple Silicon faster than an Intel machine with the same cores", () => {
    const a = deviceScore({ arch: "apple-silicon", cores: 4, ramGb: 8 });
    const i = deviceScore({ arch: "intel", cores: 4, ramGb: 8 });
    expect(a).toBeGreaterThan(i);
  });

  it("rewards more cores, with diminishing returns", () => {
    const four = deviceScore({ arch: "intel", cores: 4, ramGb: 8 });
    const eight = deviceScore({ arch: "intel", cores: 8, ramGb: 8 });
    const sixteen = deviceScore({ arch: "intel", cores: 16, ramGb: 8 });
    expect(eight).toBeGreaterThan(four);
    // diminishing: the 8→16 gain is smaller than the 4→8 gain
    expect(sixteen - eight).toBeLessThan(eight - four);
  });
});

describe("estimateTranscriptionSpeed", () => {
  it("returns a faster category on Apple Silicon than on a weak old laptop (same model)", () => {
    const order: Record<string, number> = {
      fast: 4,
      comfortable: 3,
      slow: 2,
      "very-slow": 1,
      "wont-fit": 0,
    };
    const a = estimateTranscriptionSpeed(appleSilicon, "base");
    const w = estimateTranscriptionSpeed(weakOld, "base");
    expect(order[a.category]).toBeGreaterThan(order[w.category]);
  });

  it("never rates a bigger model faster than a smaller one on the same device", () => {
    const sizes: WhisperSize[] = ["tiny", "base", "small", "medium", "large"];
    const rank: Record<string, number> = {
      fast: 4,
      comfortable: 3,
      slow: 2,
      "very-slow": 1,
      "wont-fit": 0,
    };
    let prev = 5;
    for (const s of sizes) {
      const r = rank[estimateTranscriptionSpeed(intelLaptop, s).category];
      expect(r).toBeLessThanOrEqual(prev);
      prev = r;
    }
  });

  it("gates on memory: a large model won't fit on a 4 GB machine", () => {
    const e = estimateTranscriptionSpeed(weakOld, "large");
    expect(e.category).toBe("wont-fit");
    expect(e.realtimeFactor).toBeNull();
  });

  it("marks predictions as estimated, and real measurements as not", () => {
    expect(estimateTranscriptionSpeed(intelLaptop, "base").estimated).toBe(true);
    expect(estimateTranscriptionSpeed(intelLaptop, "base", 0.3).estimated).toBe(false);
  });

  it("a real measurement overrides the prediction's category", () => {
    // A measured RTF of 0.1 (10× real-time) must read as 'fast' regardless of specs.
    const e = estimateTranscriptionSpeed(weakOld, "base", 0.1);
    expect(e.category).toBe("fast");
    expect(e.estimated).toBe(false);
  });

  it("returns an ascending, positive real-time-factor range when feasible", () => {
    const e = estimateTranscriptionSpeed(intelLaptop, "base");
    expect(e.realtimeFactor).not.toBeNull();
    const [lo, hi] = e.realtimeFactor!;
    expect(lo).toBeGreaterThan(0);
    expect(hi).toBeGreaterThanOrEqual(lo);
  });
});

describe("bestModelFor", () => {
  const candidates: WhisperSize[] = ["tiny", "base", "small"];

  it("picks a larger model on Apple Silicon than on a weak laptop", () => {
    const strong = bestModelFor(appleSilicon, candidates);
    const weak = bestModelFor(weakOld, candidates);
    const cost: Record<WhisperSize, number> = {
      tiny: 0.45,
      base: 1,
      small: 2.6,
      medium: 7,
      large: 13,
    };
    expect(cost[strong]).toBeGreaterThanOrEqual(cost[weak]);
  });

  it("always returns one of the candidates", () => {
    expect(candidates).toContain(bestModelFor(weakOld, candidates));
  });
});
