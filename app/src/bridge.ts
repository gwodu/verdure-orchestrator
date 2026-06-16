// The bridge to the Rust porter — with a browser fallback.
//
// In the packaged app these call the Tauri backend (device.rs, porter.rs). When
// the same frontend is served by plain Vite in a browser (the design review
// loop, before the Rust side is compiled), there is no backend — so we detect
// that and degrade to a sensible default tier and a simulated port. The
// simulation NEVER installs anything; it only exercises the UI states.

import type { Tier } from "@verdure/decider";

export type ProgressFn = (msg: string) => void;

export type PortOutcome = {
  installedNow: boolean;
  launched: boolean;
  message: string;
};

/** True only inside the real Tauri shell. */
export const inTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Ask the backend which hardware tier this machine is (no question to the user). */
export async function detectDevice(): Promise<Tier> {
  if (!inTauri) return "L2-high"; // browser preview: a safe, capable default
  const { invoke } = await import("@tauri-apps/api/core");
  return (await invoke<string>("detect_device")) as Tier;
}

/** Install (if needed) and launch a tool, streaming progress lines to the UI. */
export async function portTool(
  toolId: string,
  onProgress: ProgressFn,
): Promise<PortOutcome> {
  if (!inTauri) return simulatePort(onProgress);

  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<string>("porter:progress", (e) =>
    onProgress(e.payload),
  );
  try {
    return await invoke<PortOutcome>("port_tool", { toolId });
  } finally {
    unlisten();
  }
}

// Browser-only: walk the same progress stages the real porter emits, so the UI
// can be reviewed end-to-end without a backend. Marked clearly in the message.
async function simulatePort(onProgress: ProgressFn): Promise<PortOutcome> {
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
  onProgress("Checking if Buzz is already set up…");
  await wait(700);
  onProgress("Setting up Buzz — downloading and installing. This can take a minute…");
  await wait(1200);
  onProgress("Opening Buzz…");
  await wait(600);
  return {
    installedNow: true,
    launched: true,
    message:
      "Buzz is set up and open. (Browser preview — nothing was actually installed.)",
  };
}
