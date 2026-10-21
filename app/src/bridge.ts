// The bridge to the Rust backend, with a browser fallback.
//
// In the packaged app these call the Tauri commands in lib.rs. When the same
// frontend is served by plain Vite in a browser (the design review loop), there
// is no backend, so each call returns plausible data for the UI to render.

export type ProgressFn = (msg: string) => void;

/** True only inside the real Tauri shell. */
export const inTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** The model's reply (`action: "answer"`), or — with web search on and no
 *  results yet — its request for one (`action: "search"`, `reply` = the words). */
export type ChatDecision = { action: "answer" | "search" | string; reply: string };

/** A chat model as the UI shows it (model.rs `ModelSummary`). */
export type ModelSummary = {
  id: string;
  name: string;
  downloadMb: number;
  runtimeMb: number;
  installed: boolean;
};

/** Why the best model isn't running, with the number the user can act on. */
export type Shortfall = { kind: "memory" | "storage"; neededMb: number };

/**
 * Verdure's model choice for this device (knowledge/21): what it picked, the
 * best it would pick with more room, and what stands between them.
 */
export type ChatModelPlan = {
  picked: ModelSummary | null;
  best: ModelSummary;
  shortfall: Shortfall | null;
  experiment: ModelSummary | null;
};

/** What Verdure will run here, and why. Fresh each call — memory moves. */
export async function chatModelPlan(): Promise<ChatModelPlan> {
  if (!inTauri) {
    // Browser preview: a tight machine, so the shortfall note is reviewable.
    const best = { id: "qwen3-4b-instruct-2507-byteshape", name: "Chat model (Qwen3 4B)", downloadMb: 1336, runtimeMb: 2300, installed: false };
    return {
      picked: { id: "qwen3.5-2b-instruct", name: "Chat model (Qwen3.5 2B)", downloadMb: 1221, runtimeMb: 1800, installed: true },
      best,
      shortfall: { kind: "memory", neededMb: 700 },
      experiment: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<ChatModelPlan>("chat_model_plan");
}

/** One completed exchange, as the backend wants it for conversation memory. */
export type ChatTurn = { user: string; assistant: string };

/** One web search result (web.rs `Snippet`), as the prompt and the UI use it. */
export type Snippet = {
  title: string;
  url: string;
  domain: string;
  date?: string | null;
  text: string;
};

/**
 * Why an approved search produced no sources (web.rs `UnavailableReason`).
 * The model is told the same reason the badge shows, so they can't disagree.
 */
export type UnavailableReason = "offline" | "failed" | "empty";

/**
 * This turn's web context (knowledge/24): the approved query and what came
 * back, or `unavailable` when the user wanted the check and it produced
 * nothing — the model is told why, so it says so.
 */
export type WebContext = {
  query: string;
  sources: Snippet[];
  unavailable: UnavailableReason | null;
};

/**
 * Send a chat message to the local model, streaming a status line. `modelId` is
 * the model the user picked (Small/Balanced/Powerful); the backend loads it if it
 * fits, or explains why it can't — never a silent downgrade. Empty id → auto-pick.
 *
 * `history` is the conversation so far — this is what gives the chat memory
 * (knowledge/22). The backend keeps whichever recent turns fit the window it
 * sized for this device, so the frontend can pass everything it has. `web` is
 * for this answer only and never enters the history.
 */
export async function chatSend(
  message: string,
  onProgress: ProgressFn,
  modelId = "",
  history: ChatTurn[] = [],
  web: WebContext | null = null,
  webEnabled = false,
): Promise<ChatDecision> {
  if (!inTauri) {
    onProgress("Thinking…");
    await new Promise((r) => setTimeout(r, 800));
    // Browser preview of the model asking for a search (knowledge/24).
    if (webEnabled && !web && /who|members|party|election/i.test(message)) {
      return { action: "search", reply: "Nigeria opposition coalition members 2027" };
    }
    if (!webEnabled && /search|online|web/i.test(message)) {
      return { action: "answer", reply: "(Browser preview.) Web search is switched off, so I can't look that up." };
    }
    // Browser preview: mimic the tool decision so the flow is reviewable.
    if (/dictat|transcri|record|voice note|audio|meeting/i.test(message)) {
      return {
        action: "set_up_transcription",
        reply: "I can set you up to turn recordings into text.",
      };
    }
    if (web?.sources.length) {
      return {
        action: "answer",
        reply: `(Browser preview.) From the results: ${web.sources[0].text} [1] The second source agrees [2].`,
      };
    }
    if (web?.unavailable) {
      return { action: "answer", reply: `(Browser preview.) I couldn't check the web (${web.unavailable}). From memory, …` };
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
    return await invoke<ChatDecision>("chat", {
      message,
      modelId,
      history,
      webQuery: web?.query ?? null,
      webSources: web?.sources ?? null,
      webUnavailable: web?.unavailable ?? null,
      webEnabled,
    });
  } finally {
    unlisten();
  }
}

// --- Web search (knowledge/24) ------------------------------------------------
//
// Mechanism only. Whether a search may run — the user's Wi-Fi/mobile-data
// choice, the offer, the approval of the words — is decided in web.ts and
// main.ts. Nothing here consults that preference, so one place owns the rule.

/** Whether the relay is reachable, and over Wi-Fi or mobile data. */
export async function networkState(): Promise<"offline" | "wifi" | "cellular" | "unknown"> {
  if (!inTauri) return "wifi";
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke("network_state");
}

/** The scrubbed words that WOULD leave the device for `message` — for the user
 *  to read and approve before anything is sent. */
export async function prepareWebQuery(message: string): Promise<string> {
  if (!inTauri) return message.replace(/\S+@\S+/g, "").replace(/\s+/g, " ").trim();
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<string>("prepare_web_query", { message });
}

/** Open a web-search source in the system browser (the opener plugin; URL
 *  schemes are limited by the capability file). */
export async function openUrl(url: string): Promise<void> {
  if (!inTauri) {
    window.open(url, "_blank", "noopener");
    return;
  }
  const { openUrl: open } = await import("@tauri-apps/plugin-opener");
  await open(url);
}

/** Run a search through Verdure's relay. */
export async function webSearch(query: string, modelId = ""): Promise<Snippet[]> {
  if (!inTauri) {
    await new Promise((r) => setTimeout(r, 600));
    return [
      { title: "Exchange rates — Central Bank", url: "https://cbn.gov.ng/rates", domain: "cbn.gov.ng", date: "2026-10-20", text: "The official rate closed at 1,480 naira to the dollar on Monday." },
      { title: "Markets today", url: "https://example.news/markets", domain: "example.news", date: "2026-10-20", text: "The naira held steady in the parallel market at around 1,520." },
    ];
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return await invoke<Snippet[]>("web_search", { query, modelId });
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
