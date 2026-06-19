// performance.ts — predict how well a local model will run on a device, FROM
// SPECS, without making the user benchmark. The honest-performance promise
// (skills/design.md) generalized: estimate before download, then correct with a
// real on-device measurement when we have one.
//
// This is deliberately coarse and HONEST about it: it returns a category and a
// rough "× real-time" range, never false precision. The constants are anchored
// to published whisper.cpp CPU benchmarks and are PROVISIONAL — every real
// measurement we collect (opt-in, no telemetry — knowledge/06) is a better
// anchor. Scoped to Whisper CPU transcription for now (the first wrapped
// modality, knowledge/18); other model families get their own cost tables later.
// See knowledge/open-questions.md [OQ-03] (device anchors) and [OQ-51] (model per device).

export type ArchClass = "apple-silicon" | "intel" | "unknown";

/** What we can detect about a machine without asking a technical question. */
export interface DeviceSpec {
  arch: ArchClass;
  /** Logical CPU cores. */
  cores: number;
  /** Total RAM in GB. 0 = unknown (skip the memory gate). */
  ramGb: number;
}

export type WhisperSize = "tiny" | "base" | "small" | "medium" | "large";

export type SpeedCategory = "fast" | "comfortable" | "slow" | "very-slow" | "wont-fit";

export interface SpeedEstimate {
  category: SpeedCategory;
  /** Rough "× faster than real-time" range for display, e.g. [2, 4]. null when wont-fit. */
  realtimeFactor: [number, number] | null;
  /** Human, jargon-free one-liner. */
  note: string;
  /** true = predicted from specs; false = derived from a real measurement. */
  estimated: boolean;
}

// Relative CPU cost vs `base` (~1.0), from whisper.cpp parameter counts / bench.
const MODEL_COST: Record<WhisperSize, number> = {
  tiny: 0.45,
  base: 1.0,
  small: 2.6,
  medium: 7.0,
  large: 13.0,
};

// Approx RAM (GB) to run each model comfortably (weights + runtime overhead).
const MODEL_RAM_GB: Record<WhisperSize, number> = {
  tiny: 1,
  base: 1.5,
  small: 2.5,
  medium: 5,
  large: 8,
};

// A rough compute score for whisper.cpp CPU inference. Apple Silicon cores are
// far faster per-core for ggml (NEON/AMX + memory bandwidth) than an older Intel
// core; thread scaling is sub-linear past ~8. Provisional.
export function deviceScore(spec: DeviceSpec): number {
  const perCore = spec.arch === "apple-silicon" ? 3.2 : spec.arch === "intel" ? 1.0 : 0.8;
  const cores = Math.max(1, spec.cores || 1);
  const effectiveCores = Math.min(cores, 8) * 0.85 + Math.max(0, cores - 8) * 0.3;
  return perCore * Math.max(1, effectiveCores);
}

// Anchor: a 4-core Intel laptop running `base` at ~3× real-time (RTF ≈ 0.33),
// grounded in published whisper.cpp CPU benchmarks. PROVISIONAL — calibrated by
// real measurements over time.
const REF_SCORE = deviceScore({ arch: "intel", cores: 4, ramGb: 8 });
const REF_RTF_BASE = 0.33;

/** Predicted real-time factor (processing ÷ audio duration). Lower = faster. */
function predictRtf(spec: DeviceSpec, size: WhisperSize): number {
  return REF_RTF_BASE * MODEL_COST[size] * (REF_SCORE / deviceScore(spec));
}

function roundFactor(x: number): number {
  return x >= 2 ? Math.round(x) : Math.round(x * 10) / 10;
}

function categorize(rtf: number): SpeedCategory {
  if (rtf <= 0.25) return "fast"; // ≥ ~4× real-time
  if (rtf <= 0.6) return "comfortable";
  if (rtf <= 1.2) return "slow";
  return "very-slow";
}

const CATEGORY_NOTE: Record<SpeedCategory, string> = {
  fast: "Should transcribe quickly on this computer.",
  comfortable: "Should run comfortably on this computer.",
  slow: "Will work, but expect to wait a while on this computer.",
  "very-slow": "Will be slow on this computer — a smaller model may feel better.",
  "wont-fit": "This computer doesn't have enough memory for this model.",
};

/**
 * Estimate transcription speed for a Whisper model on a device. Pass
 * `measuredRtf` (from a real on-device run) to return the measured truth instead
 * of a prediction.
 */
export function estimateTranscriptionSpeed(
  spec: DeviceSpec,
  size: WhisperSize,
  measuredRtf?: number,
): SpeedEstimate {
  const ramNeed = MODEL_RAM_GB[size];
  if (spec.ramGb > 0 && spec.ramGb < ramNeed) {
    return {
      category: "wont-fit",
      realtimeFactor: null,
      note: `${CATEGORY_NOTE["wont-fit"]} (needs about ${ramNeed} GB.)`,
      estimated: measuredRtf == null,
    };
  }
  const rtf = measuredRtf ?? predictRtf(spec, size);
  const category = categorize(rtf);
  const mid = 1 / rtf;
  return {
    category,
    realtimeFactor: [roundFactor(mid * 0.7), roundFactor(mid * 1.4)],
    note: CATEGORY_NOTE[category],
    estimated: measuredRtf == null,
  };
}

/**
 * Of the candidate models, the largest one that still runs at least
 * "comfortable" on this device — the basis for picking a default per device
 * ([OQ-51]). Falls back to the smallest if none are comfortable.
 */
export function bestModelFor(
  spec: DeviceSpec,
  candidates: WhisperSize[],
): WhisperSize {
  const ok = candidates.filter((s) => {
    const e = estimateTranscriptionSpeed(spec, s);
    return e.category === "fast" || e.category === "comfortable";
  });
  if (ok.length > 0) {
    return ok.reduce((a, b) => (MODEL_COST[b] > MODEL_COST[a] ? b : a));
  }
  return candidates.reduce((a, b) => (MODEL_COST[b] < MODEL_COST[a] ? b : a));
}
