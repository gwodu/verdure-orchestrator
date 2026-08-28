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

/** One chat model as the picker needs it: size + whether it fits this device. */
export type ChatModelOption = {
  id: string;
  name: string;
  downloadMb: number;
  runtimeMb: number;
  installed: boolean;
  fitsMemory: boolean;
  fitsDisk: boolean;
};

/** The chat models the user chooses from, with per-device fit flags. */
export async function chatModelOptions(): Promise<ChatModelOption[]> {
  if (!inTauri) {
    // Browser preview: plausible options so the picker is reviewable.
    return [
      { id: "qwen2.5-0.5b-instruct", name: "Chat model (Qwen2.5 0.5B)", downloadMb: 400, runtimeMb: 1100, installed: false, fitsMemory: true, fitsDisk: true },
      { id: "qwen2.5-1.5b-instruct", name: "Chat model (Qwen2.5 1.5B)", downloadMb: 1100, runtimeMb: 2400, installed: false, fitsMemory: true, fitsDisk: true },
      { id: "qwen3-4b-instruct-2507-byteshape", name: "Chat model (Qwen3 4B)", downloadMb: 1400, runtimeMb: 2300, installed: false, fitsMemory: true, fitsDisk: true },
    ];
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<ChatModelOption[]>("chat_model_options");
}

/** One completed exchange, as the backend wants it for conversation memory. */
export type ChatTurn = { user: string; assistant: string };

/**
 * Send a chat message to the local model, streaming a status line. `modelId` is
 * the model the user picked (Small/Balanced/Powerful); the backend loads it if it
 * fits, or explains why it can't — never a silent downgrade. Empty id → auto-pick.
 *
 * `history` is the conversation so far — this is what gives the chat memory
 * (knowledge/22). The backend keeps whichever recent turns fit the window it
 * sized for this device, so the frontend can pass everything it has.
 */
export async function chatSend(
  message: string,
  onProgress: ProgressFn,
  modelId = "",
  history: ChatTurn[] = [],
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
    // Echo the memory back so the preview shows history is actually flowing.
    return {
      action: "answer",
      reply: history.length
        ? `(Browser preview — chat runs only in the desktop app. Remembering ${history.length} earlier message${history.length === 1 ? "" : "s"}.)`
        : "(Browser preview — chat runs only in the desktop app.)",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<string>("chat:progress", (e) =>
    onProgress(e.payload),
  );
  try {
    return await invoke<ChatDecision>("chat", { message, modelId, history });
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

/** A live snapshot of free resources — the machine-context for orchestration. */
export type SystemResources = {
  arch: string;
  cores: number;
  totalRamMb: number;
  availableRamMb: number;
  freeDiskMb: number;
  lowPower: boolean;
  /** "macos" | "ios" | "android" — picks the right way to help free space. */
  os: string;
};

/** Read the machine's live free resources (available RAM, free disk, low power). */
export async function systemResources(): Promise<SystemResources> {
  if (!inTauri) {
    return {
      arch: "intel",
      cores: 4,
      totalRamMb: 8192,
      availableRamMb: 2048,
      freeDiskMb: 2560,
      lowPower: true,
      os: "macos",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<SystemResources>("system_resources");
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

/** Callbacks for a live (microphone) transcription session (see live.rs). */
export type LiveHandlers = {
  onStatus?: (msg: string) => void;
  onSegment: (text: string) => void;
  onError?: (msg: string) => void;
  onStopped?: () => void;
};

/**
 * Start live transcription: register the event listeners, then ask the backend
 * to begin capture. Resolves to a cleanup function that removes the listeners.
 * Finished segments arrive via `onSegment`; the session ends via `onStopped`
 * (after `stopLiveTranscription`) or `onError` (if capture can't start).
 */
export async function startLiveTranscription(h: LiveHandlers): Promise<() => void> {
  if (!inTauri) return simulateLive(h);

  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const unlisteners = await Promise.all([
    listen<string>("live:status", (e) => h.onStatus?.(e.payload)),
    listen<string>("live:segment", (e) => h.onSegment(e.payload)),
    listen<string>("live:error", (e) => h.onError?.(e.payload)),
    listen("live:stopped", () => h.onStopped?.()),
  ]);
  const cleanup = () => unlisteners.forEach((u) => u());
  try {
    await invoke("start_live_transcription");
  } catch (err) {
    cleanup();
    throw err;
  }
  return cleanup;
}

/** Stop the live session and release the microphone. The backend then emits the
 * final segment (if any) and `live:stopped`. */
export async function stopLiveTranscription(): Promise<void> {
  if (!inTauri) {
    stopSimulatedLive();
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("stop_live_transcription");
}

// Browser preview: walk a plausible live session with timers, so the UI can be
// reviewed without a microphone or the Rust backend. Nothing is recorded.
let simStop: (() => void) | null = null;
function simulateLive(h: LiveHandlers): () => void {
  const lines = [
    "This is a browser preview of live transcription.",
    "In the desktop app your speech appears here as you talk.",
    "Each pause commits a sentence — all on your computer.",
  ];
  h.onStatus?.("Listening… speak now. (Browser preview)");
  let i = 0;
  const timer = setInterval(() => {
    if (i < lines.length) h.onSegment(lines[i++]);
    else stopSimulatedLive();
  }, 1200);
  const cleanup = () => {
    clearInterval(timer);
    simStop = null;
  };
  simStop = () => {
    cleanup();
    h.onStopped?.();
  };
  return cleanup;
}
function stopSimulatedLive(): void {
  simStop?.();
}

export type ModelEntry = {
  id: string;
  name: string;
  modality: string;
  installed: boolean;
  sizeBytes: number;
  /** Bytes in an interrupted download (.part) — reclaimable space. */
  partialBytes: number;
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
 * Ask the OS to make room for a download (Android only: the system clears other
 * apps' clearable cache — we never touch anyone's files). Rejects with an honest
 * message when it can't free enough.
 */
export async function freeUpSpace(bytes: number): Promise<void> {
  if (!inTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("free_up_space", { bytes });
}

/** Open the system's storage-management screen (macOS only). */
export async function openStorageSettings(): Promise<void> {
  if (!inTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_storage_settings");
}

/** Where downloaded models live on this device, for display (macOS: survives uninstall). */
export async function modelsDir(): Promise<string> {
  if (!inTauri) return "~/Library/Application Support/ai.verdure.porter";
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<string>("models_dir");
}

/** Open the models folder in Finder (macOS only). */
export async function revealModelsDir(): Promise<void> {
  if (!inTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("reveal_models_dir");
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

// --- Updates (desktop only) -------------------------------------------------
//
// Verdure checks for updates ONLY when the user has allowed it (knowledge/23).
// A check is a network request that reveals someone runs Verdure and roughly
// when they use it, which is exactly the kind of quiet signal knowledge/06 says
// we don't collect — so it is opt-in, asked once, and always reversible.
//
// The capability is desktop-only. iOS and Android update through their stores,
// so on those platforms every function here reports "no update" rather than
// pretending to check.

export type UpdateInfo = {
  version: string;
  notes: string;
};

/** True where self-update exists at all — desktop, in the real shell. */
export const updatesSupported = (): boolean =>
  inTauri && !/iPhone|iPad|Android/i.test(navigator.userAgent);

/**
 * Ask whether a newer Verdure exists. Returns `null` for "you're up to date",
 * and throws only on a genuine failure the caller should surface (offline, bad
 * endpoint) — being up to date is not an error.
 *
 * The caller is responsible for having consent. This function does not check
 * the stored preference, deliberately: the "check now" button is an explicit
 * act of consent on its own, and burying that rule here would make the two
 * paths disagree.
 */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!updatesSupported()) return null;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return null;
  return { version: update.version, notes: update.body ?? "" };
}

/**
 * Download and install the pending update, then restart into it. `onProgress`
 * receives a 0–1 fraction where the total size is known, and -1 where it isn't
 * (the endpoint may omit content-length) so the UI can show motion rather than
 * a bar frozen at zero.
 */
export async function installUpdate(
  onProgress: (fraction: number) => void,
): Promise<void> {
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return;
  let total = 0;
  let got = 0;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? 0;
      onProgress(total ? 0 : -1);
    } else if (event.event === "Progress") {
      got += event.data.chunkLength;
      onProgress(total ? got / total : -1);
    } else if (event.event === "Finished") {
      onProgress(1);
    }
  });
  // The bundle on disk is new; this process is still the old one until it restarts.
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}

/**
 * Open the third-party notices bundled inside the app ([OQ-79]).
 *
 * Verdure is built on ~500 open-source components whose licences require their
 * copyright notices to travel with the binary. The file ships inside the .app;
 * this makes it reachable.
 */
export async function openThirdPartyNotices(): Promise<void> {
  if (!inTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_third_party_notices");
}
