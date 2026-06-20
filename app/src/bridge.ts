// The bridge to the Rust porter — with a browser fallback.
//
// In the packaged app these call the Tauri backend (device.rs, porter.rs). When
// the same frontend is served by plain Vite in a browser (the design review
// loop, before the Rust side is compiled), there is no backend — so we detect
// that and degrade to a sensible default tier and a simulated port. The
// simulation NEVER installs anything; it only exercises the UI states.

import type { Tier, DeviceSpec } from "@verdure/decider";

export type Transcription = {
  text: string;
  audioSeconds: number;
  processingSeconds: number;
};

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

/** The model's decision: answer, or invoke a tool (grammar-locked). */
export type ChatDecision = { action: string; reply: string };

/** Send a chat message to the local model, streaming a status line. */
export async function chatSend(
  message: string,
  onProgress: ProgressFn,
): Promise<ChatDecision> {
  if (!inTauri) {
    onProgress("Thinking…");
    await new Promise((r) => setTimeout(r, 800));
    // Browser preview: mimic the tool decision so the flow is reviewable.
    if (/dictat|transcri|record|voice note|audio|meeting/i.test(message)) {
      return {
        action: "set_up_transcription",
        reply: "I can set you up to turn recordings into text.",
      };
    }
    return {
      action: "answer",
      reply: "(Browser preview — chat runs only in the desktop app.)",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<string>("chat:progress", (e) =>
    onProgress(e.payload),
  );
  try {
    return await invoke<ChatDecision>("chat", { message });
  } finally {
    unlisten();
  }
}

/** The machine's specs, for the performance predictor. */
export async function deviceSpec(): Promise<DeviceSpec> {
  if (!inTauri) return { arch: "intel", cores: 4, ramGb: 8 };
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<DeviceSpec>("device_spec");
}

/** Transcribe an audio file on-device, streaming progress lines to the UI. */
export async function transcribeAudio(
  path: string,
  onProgress: ProgressFn,
): Promise<Transcription> {
  if (!inTauri) return simulateTranscribe(onProgress);

  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<string>("transcribe:progress", (e) =>
    onProgress(e.payload),
  );
  try {
    return await invoke<Transcription>("transcribe_audio", { path });
  } finally {
    unlisten();
  }
}

export type ModelEntry = {
  id: string;
  name: string;
  modality: string;
  installed: boolean;
  sizeBytes: number;
};

/** Every known model, with whether it's downloaded and its size. */
export async function listModels(): Promise<ModelEntry[]> {
  if (!inTauri) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<ModelEntry[]>("list_models");
}

/** Completely remove a downloaded model by id, reclaiming the space. */
export async function removeModel(id: string): Promise<void> {
  if (!inTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("remove_model", { id });
}

/**
 * Listen for files dropped onto the window. Returns an unlisten function.
 * In the browser preview there's no native drag-drop with real paths, so this
 * is a no-op there (the panel shows a desktop-only note instead).
 */
export async function onAudioDrop(
  onFiles: (paths: string[]) => void,
  onHover?: (hovering: boolean) => void,
): Promise<() => void> {
  if (!inTauri) return () => {};
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
    const p = event.payload;
    if (p.type === "enter" || p.type === "over") onHover?.(true);
    else if (p.type === "leave") onHover?.(false);
    else if (p.type === "drop") {
      onHover?.(false);
      onFiles(p.paths);
    }
  });
  return unlisten;
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

async function simulateTranscribe(onProgress: ProgressFn): Promise<Transcription> {
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
  onProgress("Reading the audio…");
  await wait(700);
  onProgress("Transcribing — this runs on your computer…");
  await wait(1200);
  return {
    text: "(Browser preview — transcription runs only in the desktop app.) This is where your transcript would appear.",
    audioSeconds: 60,
    processingSeconds: 15,
  };
}
