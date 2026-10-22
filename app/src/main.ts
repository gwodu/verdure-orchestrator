// Verdure — the UI: one interface over local models (knowledge/18). The app
// opens into chat (dictation is the mic button in it); "Your model" and Storage
// are the only other screens. Pure logic lives in format.ts, web.ts and
// updates.ts so it can be tested without booting the app. Copy is jargon-free
// (skills/design.md, skills/copywriting.md).

import {
  startLiveTranscription,
  stopLiveTranscription,
  chatSend,
  type ChatTurn,
  chatModelPlan,
  listModels,
  removeModel,
  systemResources,
  inTauri,
  freeUpSpace,
  openStorageSettings,
  modelsDir,
  revealModelsDir,
  openThirdPartyNotices,
  networkState,
  openUrl,
  prepareWebQuery,
  webSearch,
  memoryOverview,
  memoryRecall,
  memoryThread,
  memoryForget,
  memoryImportWhatsapp,
  memorySetApps,
  openNotificationAccess,
  type ChatModelPlan,
  type MemoryOverview,
  type Recall,
  type WebContext,
} from "./bridge";

import {
  citedSources,
  fmtBytes,
  fmtSize,
  friendlyError,
  looksInterrupted,
  micPermissionHelp,
  parseProgress,
  renderMarkdown,
  shortfallMessage,
} from "./format";
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
  type WebMode,
} from "./web";

const WHISPER_MODEL_ID = "whisper-base-en";
import {
  checkForUpdate,
  checkOnLaunch,
  installUpdate,
  setUpdateMode,
  shouldAskAboutUpdates,
  updateMode,
  updatesSupported,
  type UpdateMode,
} from "./updates";
import "./styles.css";
// Verdure picks the chat model (knowledge/21, 2026-10-21 — the user no longer
// chooses a tier). The plan says what it picked and what more room would buy;
// it is fetched fresh whenever a screen needs it, because free memory moves.
let currentPlan: ChatModelPlan | null = null;
async function refreshPlan(): Promise<ChatModelPlan | null> {
  try {
    currentPlan = await chatModelPlan();
  } catch {
    currentPlan = null;
  }
  return currentPlan;
}
// The one opt-in: the 27B experiment, on devices that offer it ([OQ-84]).
const EXPERIMENT_KEY = "verdure.chat.experiment";
const experimentOn = (): boolean => localStorage.getItem(EXPERIMENT_KEY) === "1";
// The id handed to the backend: empty = Verdure picks; the experiment's id only
// when it is on AND offered here (anything else is treated as empty anyway).
const modelIdForChat = (): string =>
  experimentOn() && currentPlan?.experiment ? currentPlan.experiment.id : "";
// Plain-language tier names + one-line blurbs, keyed by model id (design.md: the
// headline is the benefit; the model name is a demoted detail).
const MODEL_TIERS: { id: string; tier: string; blurb: string }[] = [
  { id: "qwen2.5-0.5b-instruct", tier: "Small", blurb: "Quick and light — runs on any device. Best for short questions." },
  { id: "qwen2.5-1.5b-instruct", tier: "Compact", blurb: "A little more capable than Small." },
  { id: "qwen3.5-2b-instruct", tier: "Balanced", blurb: "A capable all-rounder that remembers much of the conversation." },
  { id: "qwen3-4b-instruct-2507-byteshape", tier: "Powerful", blurb: "The most capable — richer, better-grounded answers." },
  { id: "qwen3.8-27b-byteshape", tier: "Experimental", blurb: "A much larger model we're still testing. A very big download, and it may be slow or fail to start." },
];
const tierName = (id: string): string =>
  MODEL_TIERS.find((t) => t.id === id)?.tier ?? "Chat model";

// Keep the screen awake during a long download so an idle screen doesn't suspend
// the app and kill the network mid-download (the big-model failure on Android).
// Best-effort: the Wake Lock API is a no-op where unsupported, and resumable
// downloads (model.rs) cover any interruption that slips through. Run `work` with
// the lock held, always releasing it — even on error.
type WakeLockLike = { release: () => Promise<void> };
async function withScreenAwake<T>(work: () => Promise<T>): Promise<T> {
  let lock: WakeLockLike | null = null;
  try {
    const nav = navigator as unknown as {
      wakeLock?: { request: (t: "screen") => Promise<WakeLockLike> };
    };
    lock = (await nav.wakeLock?.request("screen")) ?? null;
  } catch {
    // Unsupported or denied — proceed without it.
  }
  try {
    return await work();
  } finally {
    try {
      await lock?.release();
    } catch {
      /* already released (e.g. screen turned off) */
    }
  }
}
// Example questions shown in the empty chat — playful + informational, to invite
// a first message. Tapping one fills the input (the user still hits Send, since a
// first message triggers the one-time model download).
const CHAT_EXAMPLES = [
  { emoji: "💧", text: "What's the chemical composition of water?" },
  { emoji: "🥧", text: "How do you bake a pie?" },
  { emoji: "🌌", text: "How do the northern lights work?" },
  { emoji: "🐝", text: "Tell me a surprising fact about honeybees" },
];

const root = document.querySelector<HTMLDivElement>("#app")!;

// Active live-transcription listeners, if a mic session is running. Every screen
// calls clearDrop() on entry, so leaving the transcribe panel always releases the
// microphone — no recording survives navigation.
let liveCleanup: (() => void) | null = null;
function clearDrop(): void {
  if (liveCleanup) {
    liveCleanup();
    liveCleanup = null;
    void stopLiveTranscription();
  }
}

// --- Storage screen: every downloaded model, with size + Remove --------------
function renderStorage(): void {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());
  root.appendChild(backButton(renderChat));

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Storage"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Models you've downloaded live on your computer so they work offline. Remove any to free up space — Verdure re-downloads it next time you need it.",
    ),
  );

  // Updates. Lives here because Storage is already the "Verdure and your
  // computer" screen, and because a consent choice the user cannot revisit isn't
  // really consent (knowledge/23) — the mode they picked on first launch is
  // changeable from this row, forever.
  if (updatesSupported()) panel.appendChild(updatesSection());
  // Web search: the other thing Verdure can send off the device (knowledge/24).
  // Same reasoning as updates — a choice that cannot be revisited isn't a choice.
  panel.appendChild(webSection());

  // Live "Your computer" readout — the machine-context Verdure uses to size what
  // it runs (knowledge/20/21). Honest, plain numbers; no jargon.
  const sysBox = el("div", "sysbox");
  panel.appendChild(sysBox);
  const fillSys = async () => {
    const r = await systemResources();
    sysBox.innerHTML = "";
    sysBox.appendChild(el("span", "sysbox-title", "Your computer"));
    const grid = el("div", "sysbox-grid");
    const row = (label: string, value: string) => {
      const r = el("div", "sysbox-row");
      r.appendChild(el("span", "sysbox-label", label));
      r.appendChild(el("span", "sysbox-value", value));
      grid.appendChild(r);
    };
    row("Memory free now", `${fmtSize(r.availableRamMb)} of ${fmtSize(r.totalRamMb)}`);
    row("Disk free", fmtSize(r.freeDiskMb));
    // arch is "apple-silicon" | "intel" | "arm" (Android) — name it honestly.
    const archName =
      r.arch === "apple-silicon" ? "Apple Silicon" : r.arch === "intel" ? "Intel" : "ARM";
    row("Processor", `${r.cores} cores · ${archName}`);
    if (r.lowPower) row("Low Power Mode", "On — runs slower to save battery");
    sysBox.appendChild(grid);
  };
  void fillSys();

  const listEl = el("div", "model-list");
  panel.appendChild(listEl);
  root.appendChild(panel);

  const refresh = async () => {
    const models = await listModels();
    const downloaded = models.filter((m) => m.installed);
    // Interrupted downloads take real space too — show them so the user can
    // reclaim it, instead of a hidden .part silently eating the disk ([OQ-67]).
    const partials = models.filter((m) => !m.installed && m.partialBytes > 0);
    listEl.innerHTML = "";

    if (!inTauri) {
      listEl.appendChild(el("p", "subtitle", "Storage is managed in the desktop app."));
      return;
    }
    if (downloaded.length === 0 && partials.length === 0) {
      listEl.appendChild(
        el("p", "model-empty", "Nothing downloaded yet — nothing taking up space."),
      );
      return;
    }

    const total =
      downloaded.reduce((sum, m) => sum + m.sizeBytes, 0) +
      partials.reduce((sum, m) => sum + m.partialBytes, 0);
    panel.querySelector(".model-total")?.remove();

    const modelRow = (m: { id: string; name: string }, meta: string) => {
      const row = el("div", "model-row");
      const info = el("div", "model-info");
      // Chat models by the picker's tier name, not the internal one (jargon).
      const tier = MODEL_TIERS.find((t) => t.id === m.id)?.tier;
      info.appendChild(el("span", "model-name", tier ? `${tier} chat model` : m.name));
      info.appendChild(el("span", "model-meta", meta));
      row.appendChild(info);

      const remove = document.createElement("button");
      remove.className = "link-danger";
      remove.type = "button";
      remove.textContent = "Remove";
      remove.addEventListener("click", async () => {
        remove.disabled = true;
        remove.textContent = "Removing…";
        try {
          await removeModel(m.id);
        } finally {
          await refresh();
        }
      });
      row.appendChild(remove);
      return row;
    };

    for (const m of downloaded)
      listEl.appendChild(modelRow(m, `${m.modality} · ${fmtBytes(m.sizeBytes)}`));
    // A paused download resumes where it left off if kept — say so, so removing
    // it is an informed choice, not a surprise restart later.
    for (const m of partials)
      listEl.appendChild(
        modelRow(m, `Unfinished download · ${fmtBytes(m.partialBytes)} so far — resumes if kept`),
      );

    const totalEl = el("p", "model-total", `Total on this computer: ${fmtBytes(total)}`);
    panel.appendChild(totalEl);
  };

  // Where the files actually are. The user owns them, so they get to know.
  // It matters most on macOS: deleting Verdure leaves this folder behind (macOS
  // gives apps no uninstall hook), so without this the models become invisible
  // orphaned gigabytes. iOS/Android delete the whole container on uninstall.
  const whereEl = el("div", "where");
  panel.appendChild(whereEl);
  void (async () => {
    if (!inTauri) return;
    let dir = "";
    let os = "";
    try {
      dir = await modelsDir();
      os = (await systemResources()).os;
    } catch {
      return; // can't locate it — say nothing rather than something wrong
    }
    whereEl.appendChild(el("span", "where-label", "Stored on this device at"));
    whereEl.appendChild(el("code", "where-path", dir));
    if (os === "macos") {
      whereEl.appendChild(
        el(
          "span",
          "where-note",
          "If you delete Verdure, this folder stays — remove it yourself to reclaim the space.",
        ),
      );
      const open = document.createElement("button");
      open.className = "link";
      open.type = "button";
      open.textContent = "Open folder";
      open.addEventListener("click", () => void revealModelsDir().catch(() => {}));
      whereEl.appendChild(open);
    } else {
      whereEl.appendChild(
        el("span", "where-note", "Uninstalling Verdure removes these files too."),
      );
    }
  })();

  void refresh();
}

// --- Your model: what Verdure picked, and what more room would buy -----------
// There is no choice to make here (knowledge/21): Verdure runs the most capable
// model that fits this device right now. What the user gets instead is the
// truth — which model, how big, and when closing apps or freeing space would
// get them a better one — with a "Check again" that re-reads the device.
async function renderYourModel(): Promise<void> {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());
  root.appendChild(backButton(renderChat));

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Your model"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Verdure runs the most capable model that fits your device, and tells you when more room would get you a better one.",
    ),
  );
  const box = el("div", "yourmodel");
  panel.appendChild(box);
  const status = el("p", "status");
  panel.appendChild(status);
  root.appendChild(panel);
  // Which OS decides the honest "make room" help below; learned async.
  let os = "";
  void systemResources().then((r) => (os = r.os)).catch(() => {});

  const paint = (plan: ChatModelPlan | null) => {
    box.innerHTML = "";
    if (!plan) {
      box.appendChild(el("p", "subtitle", "Couldn't read your device just now. Try again in a moment."));
      return;
    }
    const picked = experimentOn() && plan.experiment ? plan.experiment : plan.picked;
    const head = el("div", "yourmodel-head");
    head.appendChild(el("span", "yourmodel-tier", picked ? tierName(picked.id) : "No model fits right now"));
    if (picked) {
      head.appendChild(
        el("span", "yourmodel-meta", `${picked.name} · ${picked.installed ? "downloaded" : `${fmtSize(picked.downloadMb)} download`}`),
      );
      const blurb = MODEL_TIERS.find((t) => t.id === picked.id)?.blurb;
      if (blurb) head.appendChild(el("span", "yourmodel-blurb", blurb));
    }
    box.appendChild(head);

    // The honest part: what stands between this device and the best model.
    const note = shortfallMessage(
      plan.picked ? tierName(plan.picked.id) : null,
      tierName(plan.best.id),
      plan.shortfall,
    );
    if (note) {
      box.appendChild(el("p", "model-note", note));
      const row = el("div", "web-row");
      const check = document.createElement("button");
      check.type = "button";
      check.className = "update-choice preferred";
      check.textContent = "Check again";
      check.addEventListener("click", async () => {
        check.disabled = true;
        check.textContent = "Checking…";
        paint(await refreshPlan());
      });
      row.appendChild(check);
      // Platform-specific "make room" help for a STORAGE shortfall, only where
      // honest help exists ([OQ-67]): Android can clear other apps' cached junk
      // on request; the Mac has its own storage screen; iOS gets instructions.
      if (plan.shortfall?.kind === "storage") {
        const help = document.createElement("button");
        help.type = "button";
        help.className = "update-choice";
        if (os === "android") {
          help.textContent = "Clear cached junk to make room";
          help.addEventListener("click", async () => {
            help.disabled = true;
            help.textContent = "Making room…";
            try {
              await freeUpSpace((plan.best.downloadMb + 64) * 1_048_576);
            } catch (err) {
              status.textContent = friendlyError(err);
            }
            paint(await refreshPlan());
          });
          row.appendChild(help);
        } else if (os === "macos") {
          help.textContent = "See what's filling your disk";
          help.addEventListener("click", () => void openStorageSettings().catch((err) => (status.textContent = friendlyError(err))));
          row.appendChild(help);
        } else if (os === "ios") {
          box.appendChild(el("p", "where-note", "Free space in Settings › General › iPhone Storage — offloading apps you don't use is the quickest win."));
        }
      }
      box.appendChild(row);
    } else if (!experimentOn()) {
      box.appendChild(el("p", "yourmodel-ok", "This is the most capable model Verdure has for everyday use."));
    }

    // The experiment, where the device offers it: a labelled switch, never the
    // default (copywriting.md — say what it is, not what sounds good).
    if (plan.experiment) {
      const ex = document.createElement("button");
      ex.type = "button";
      ex.className = "updates-mode" + (experimentOn() ? " selected" : "");
      ex.setAttribute("aria-pressed", String(experimentOn()));
      ex.textContent = `Try the experimental model (${plan.experiment.name}, ${fmtSize(plan.experiment.downloadMb)} download — may be slow or fail to start)`;
      ex.addEventListener("click", () => {
        localStorage.setItem(EXPERIMENT_KEY, experimentOn() ? "0" : "1");
        paint(plan);
      });
      box.appendChild(ex);
    }
  };
  paint(currentPlan);
  paint(await refreshPlan());
}

// --- Chat: the conversational front door (knowledge/18) ---------------------
function renderChat(): void {
  clearDrop();
  root.innerHTML = "";
  // Chat is the home screen (opens straight here — no task grid). Header actions:
  // the current model (tap to change) and a quiet gear to storage.
  const modelBtn = document.createElement("button");
  modelBtn.className = "model-chip";
  modelBtn.type = "button";
  modelBtn.title = "Your model — what Verdure picked for this device";
  modelBtn.setAttribute("aria-label", "Your model — what Verdure picked for this device");
  modelBtn.textContent = currentPlan?.picked ? tierName(currentPlan.picked.id) : "Your model";
  modelBtn.addEventListener("click", () => void renderYourModel());
  // The web switch (knowledge/24): the one consent, in view. On = the model is
  // told it can check the web and searches run without a per-search tap; off =
  // it is told it can't. What was searched always shows under the answer.
  const webBtn = iconButton(ICON_GLOBE, "Web search", () => toggleWeb());
  const paintWeb = () => {
    const on = webEnabled();
    webBtn.classList.toggle("on", on);
    webBtn.setAttribute("aria-pressed", String(on));
    webBtn.title = on ? "Web search is on — tap to switch off" : "Web search is off — tap to switch on";
  };
  paintWeb();
  root.appendChild(
    brandBar([
      modelBtn,
      webBtn,
      iconButton(ICON_MEMORY, "What Verdure knows", () => void renderMemory()),
      iconButton(ICON_STORAGE, "Storage & your computer", renderStorage),
    ]),
  );

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Ask me anything"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Type to chat, or tap the microphone to turn speech into text. Everything runs on your device — private, no account. The first message downloads the chat model Verdure picked for it.",
    ),
  );

  // When a smaller model is running than the device could with more room, say
  // so here — the answer quality the user is about to see has a reason, and the
  // reason has a number (design.md: honest numbers in human terms).
  const modelNote = el("p", "model-note");
  modelNote.hidden = true;
  panel.appendChild(modelNote);
  void refreshPlan().then((plan) => {
    if (!plan) return;
    modelBtn.textContent = plan.picked ? tierName(plan.picked.id) : "Your model";
    const note = shortfallMessage(plan.picked ? tierName(plan.picked.id) : null, tierName(plan.best.id), plan.shortfall);
    modelNote.textContent = note;
    modelNote.hidden = !note;
  });

  const thread = el("div", "chat-thread");
  panel.appendChild(thread);

  const status = el("p", "status");
  panel.appendChild(status);

  // Switching web search on for the first time shows what leaves the device,
  // once, where the user is looking; after that it lives in Storage.
  const toggleWeb = () => {
    const on = !webEnabled();
    setWebEnabled(on);
    paintWeb();
    if (on && !disclosureSeen()) {
      markDisclosureSeen();
      const card = el("div", "msg ai web-offer");
      card.appendChild(el("p", "", "Web search is on. What leaves your device:"));
      card.appendChild(el("p", "disclose-text", WEB_DISCLOSURE));
      const ok = document.createElement("button");
      ok.type = "button";
      ok.className = "update-choice preferred";
      ok.textContent = "Got it";
      ok.addEventListener("click", () => card.remove());
      card.appendChild(ok);
      thread.appendChild(card);
      thread.scrollTop = thread.scrollHeight;
    } else {
      status.textContent = on ? "Web search is on." : "Web search is off.";
    }
  };

  const rowEl = el("div", "chat-input-row");
  const input = document.createElement("textarea");
  input.className = "chat-input";
  input.rows = 2;
  input.placeholder = "Type a message…";
  // Microphone → dictation straight into the box (the Whisper flow). Tap to start,
  // speak (words stream in as you pause), tap again to stop; then edit + Send.
  const mic = document.createElement("button");
  mic.className = "mic-btn";
  mic.type = "button";
  mic.title = "Dictate — turn speech into text";
  mic.setAttribute("aria-label", "Dictate — turn speech into text");
  mic.innerHTML = ICON_MIC;
  // Mic states: idle → preparing (neutral, while the model downloads / mic opens)
  // → recording (red, pulsing, actually listening). The button only goes red once
  // audio capture is really live, so "installing" never looks like "recording".
  let recording = false;
  let preparing = false;
  const setRecording = (on: boolean) => {
    recording = on;
    preparing = false;
    mic.classList.toggle("recording", on);
    mic.classList.remove("preparing");
    mic.title = on ? "Stop dictation" : "Dictate — turn speech into text";
  };
  const setPreparing = () => {
    preparing = true;
    recording = false;
    mic.classList.remove("recording");
    mic.classList.add("preparing");
    mic.title = "Getting the microphone ready…";
  };
  // Begin dictation. `needsInstall` shows the one-time speech-model download
  // notice; it clears as soon as the first words come back.
  const startDictation = async (needsInstall: boolean) => {
    // Belt and braces: never stack sessions. `liveCleanup` is only assigned once
    // the start promise resolves, so an error arriving mid-start could otherwise
    // slip past the per-path cleanup below. Releasing here means a second set of
    // listeners can never accumulate, whatever went wrong last time.
    if (liveCleanup) {
      liveCleanup();
      liveCleanup = null;
    }
    setPreparing(); // neutral — NOT red — until capture is actually live
    status.textContent = needsInstall ? "Setting up speech-to-text…" : "Getting the microphone ready…";
    let note: HTMLElement | null = needsInstall
      ? el(
          "div",
          "install-note",
          "Installing the speech model — a one-time ~140 MB download. It runs on your device and can take a minute…",
        )
      : null;
    if (note) {
      thread.appendChild(note);
      thread.scrollTop = thread.scrollHeight;
    }
    // Watchdog: if capture never reaches "listening" (e.g. microphone permission
    // wasn't granted), don't hang silently — surface a clear, actionable message.
    let live = false;
    // Every path out of a session MUST release the listeners. Leaving them
    // attached meant the next session added a second set, and each `live:segment`
    // was then handled twice — the "transcribes the same thing twice" beta report.
    // It was sticky: one mic error poisoned every later session until navigation.
    const releaseListeners = () => {
      if (liveCleanup) {
        liveCleanup();
        liveCleanup = null;
      }
    };
    const watchdog = window.setTimeout(() => {
      if (live || !preparing) return;
      note?.remove();
      void stopLiveTranscription();
      releaseListeners();
      setRecording(false);
      status.textContent = micPermissionHelp("");
      // Name THIS device's settings path once we know which OS this is.
      void systemResources()
        .then((r) => (status.textContent = micPermissionHelp(r.os)))
        .catch(() => {});
    }, 30000);
    try {
      liveCleanup = await withScreenAwake(() =>
        startLiveTranscription({
          onStatus: (m) => {
            status.textContent = m;
            // live.rs emits "Listening… speak now." once the stream is really open.
            if (/listening/i.test(m) && !live) {
              live = true;
              window.clearTimeout(watchdog);
              setRecording(true); // now — and only now — turn red
            }
          },
          onSegment: (seg) => {
            note?.remove();
            note = null; // first words → the download finished
            // Drop Whisper's non-speech markers ([BLANK_AUDIO], [ Silence ], …).
            const t = seg.replace(/\[[A-Za-z_ ]+\]/g, "").trim();
            if (t) input.value = input.value ? `${input.value} ${t}` : t; // into the box
          },
          onError: (m) => {
            live = true;
            window.clearTimeout(watchdog);
            note?.remove();
            status.textContent = m;
            releaseListeners(); // an error ends the session too — don't leak
            setRecording(false);
          },
          onStopped: () => {
            window.clearTimeout(watchdog);
            note?.remove();
            setRecording(false);
            status.textContent = "";
            releaseListeners();
            input.focus();
          },
        }),
      );
    } catch (err) {
      window.clearTimeout(watchdog);
      note?.remove();
      status.textContent = friendlyError(err);
      setRecording(false);
    }
  };
  // Tap the mic: stop if recording; otherwise, if the speech model isn't installed
  // yet, ask permission first (with its size — the same honest prompt as the chat
  // model), then dictate. The user opts into the extra download.
  const toggleMic = async () => {
    if (preparing) return; // mid-setup — ignore extra taps
    if (recording) {
      setRecording(false);
      await stopLiveTranscription(); // emits the final segment, then live:stopped
      return;
    }
    const installed = (await listModels().catch(() => [])).some(
      (m) => m.id === WHISPER_MODEL_ID && m.installed,
    );
    if (!installed) {
      addActionMsg(
        "To talk to Verdure by voice, I'll install a small speech model (Whisper) — a one-time ~140 MB download that runs entirely on your device.",
        "Install & start talking",
        () => void startDictation(true),
      );
      return;
    }
    void startDictation(false);
  };
  mic.addEventListener("click", () => void toggleMic());
  const send = document.createElement("button");
  send.className = "primary chat-send";
  send.type = "button";
  send.textContent = "Send";
  rowEl.appendChild(input);
  rowEl.appendChild(mic);
  rowEl.appendChild(send);
  panel.appendChild(rowEl);

  root.appendChild(panel);

  // Empty-state example questions. Tapping one loads it into the box so the user
  // can send when ready. Removed as soon as the first message goes out.
  const examples = el("div", "suggestions");
  for (const ex of CHAT_EXAMPLES) {
    const chip = document.createElement("button");
    chip.className = "suggest-chip";
    chip.type = "button";
    chip.innerHTML = `<span class="suggest-emoji" aria-hidden="true">${ex.emoji}</span>`;
    chip.appendChild(document.createTextNode(ex.text));
    chip.addEventListener("click", () => {
      input.value = ex.text;
      input.focus();
    });
    examples.appendChild(chip);
  }
  thread.appendChild(examples);


  // The conversation so far — this IS the chat's memory (knowledge/22). Only
  // completed exchanges go in: a question whose answer failed would otherwise
  // teach the model that questions go unanswered. The backend keeps whichever
  // recent turns fit the window it sized for this device, so we pass all of it.
  const history: ChatTurn[] = [];

  const addMsg = (who: "user" | "ai", text: string, web: WebContext | null = null) => {
    // Only the model's replies get Markdown rendering. What the USER typed is
    // shown exactly as typed — if they write **bold**, they meant asterisks.
    const m =
      who === "ai"
        ? (() => {
            const node = el("div", "msg ai");
            node.innerHTML = renderMarkdown(text, web?.sources.length ?? 0); // escaped inside renderMarkdown
            if (web) {
              // Wikipedia-style: the little number opens the source it names.
              node.querySelectorAll<HTMLElement>("sup.cite").forEach((sup) => {
                const src = web.sources[Number(sup.textContent) - 1];
                if (!src) return;
                sup.setAttribute("role", "link");
                sup.title = src.domain;
                sup.addEventListener("click", () => void openSource(src.url));
              });
              node.appendChild(sourcesFooter(text, web));
            }
            return node;
          })()
        : el("div", "msg user", text);
    thread.appendChild(m);
    thread.scrollTop = thread.scrollHeight;
    return m;
  };

  // Under a web-checked answer: the words that left the device, and the
  // sources, each a tap from its page (knowledge/24). The ones the answer cited
  // are highlighted; the rest are listed without a verdict — on `testflight-15`
  // the 2B plainly used results it never numbered, and a badge that called that
  // "from memory" was wrong more often than right.
  const sourcesFooter = (reply: string, web: WebContext): HTMLElement => {
    const box = el("div", "sources");
    if (web.unavailable) {
      // The same reason the model was given — never a guess about the cause.
      const why = {
        offline: "Couldn't check the web — no connection. Answered from memory.",
        failed: "Couldn't check the web — the search service didn't answer. Answered from memory.",
        empty: "Checked the web — nothing useful came back. Answered from memory.",
      }[web.unavailable];
      box.appendChild(el("span", "badge", why));
      return box;
    }
    box.appendChild(
      el("span", "sources-query", web.memoryLabel ? `From your phone — ${web.memoryLabel}` : `Searched the web for “${web.query}”`),
    );
    const used = citedSources(reply, web.sources.length);
    const list = el("ol", "sources-list");
    web.sources.forEach((s, i) => {
      const li = el("li", used.includes(i + 1) ? "cited" : "");
      li.setAttribute("role", "link");
      li.title = s.url;
      li.appendChild(el("span", "source-domain", s.domain));
      // A record's day in the user's words ("21 Oct"), not the store's ISO date.
      const day = s.date && web.memoryLabel ? whenShort(Date.parse(`${s.date}T12:00:00Z`)) : "";
      li.appendChild(el("span", "source-title", day ? `${s.title} · ${day}` : s.title));
      li.addEventListener("click", () => void openSource(s.url));
      list.appendChild(li);
    });
    box.appendChild(list);
    return box;
  };

  // An AI message that offers to run a capability — the human confirms before
  // Verdure acts (knowledge/18/20: tools the model invokes, human in the loop).
  const addActionMsg = (text: string, label: string, run: () => void) => {
    const m = addMsg("ai", text);
    const btn = document.createElement("button");
    btn.className = "primary chat-action";
    btn.type = "button";
    btn.textContent = label;
    btn.addEventListener("click", run);
    m.appendChild(btn);
    thread.scrollTop = thread.scrollHeight;
  };

  let busy = false;

  // A download-progress card: a filling bar + "N of M MB". `update` is driven by
  // the backend's "PROGRESS done total" lines; a fresh one is built per attempt.
  const makeInstallCard = (headline: string) => {
    const card = el("div", "install-note");
    card.appendChild(el("div", "install-headline", headline));
    const bar = el("div", "dlbar");
    const fill = el("div", "dlbar-fill");
    bar.appendChild(fill);
    card.appendChild(bar);
    const label = el("div", "dlbar-label", "Starting…");
    card.appendChild(label);
    thread.appendChild(card);
    thread.scrollTop = thread.scrollHeight;
    return {
      el: card,
      update(doneMb: number, totalMb: number) {
        const pct = totalMb > 0 ? Math.min(100, Math.round((doneMb / totalMb) * 100)) : 0;
        fill.style.width = `${pct}%`;
        label.textContent = `${doneMb} of ${totalMb} MB · ${pct}%`;
      },
    };
  };

  // On mobile data with Wi-Fi only set (or when the device can't tell): ask,
  // once, to allow it — the answer is a setting, changeable from Storage. No
  // byte counts here; a search is a few kilobytes and the number is noise.
  const askMobileData = (text: string, web: WebContext) => {
    const card = el("div", "msg ai web-offer");
    card.appendChild(
      el("p", "", "You're not on Wi-Fi, and web search is set to Wi-Fi only. Allow it on mobile data too? A search uses very little."),
    );
    const row = el("div", "web-row");
    const allow = document.createElement("button");
    allow.type = "button";
    allow.className = "update-choice preferred";
    allow.textContent = "Allow on mobile data";
    allow.addEventListener("click", () => {
      setWebMode("any");
      card.remove();
      void runChat(text, web);
    });
    const not = document.createElement("button");
    not.type = "button";
    not.className = "update-choice";
    not.textContent = "Not now";
    not.addEventListener("click", () => {
      card.remove();
      void runChat(text);
    });
    row.appendChild(allow);
    row.appendChild(not);
    card.appendChild(row);
    thread.appendChild(card);
    thread.scrollTop = thread.scrollHeight;
  };

  // Run a not-yet-run search through the data rule (knowledge/24). Returns the
  // context to answer with, or ASKED when the mobile-data question was put to
  // the user (its buttons re-enter `runChat`).
  const ASKED = Symbol("asked");
  const resolveWeb = async (text: string, web: WebContext | null): Promise<WebContext | null | typeof ASKED> => {
    if (!web || web.sources.length > 0 || web.unavailable) return web;
    const gate = searchGate(webMode(), await networkState().catch(() => "unknown" as const));
    if (gate === "ask-mobile-data") {
      askMobileData(text, web);
      return ASKED;
    }
    if (gate === "offline") return { ...web, unavailable: "offline" };
    status.textContent = "Checking the web…";
    try {
      const sources = await webSearch(web.query, modelIdForChat());
      // Nothing came back — say so, rather than framing an empty source list.
      return sources.length ? { ...web, sources } : { ...web, unavailable: "empty" };
    } catch (err) {
      // Couldn't reach the relay, or it answered badly: the model is told
      // exactly that (not "offline"), and the user sees the same reason.
      status.textContent = friendlyError(err);
      return { ...web, unavailable: "failed" };
    }
  };

  // Run one chat turn for `text`. Split out from `submit` so the "Resume" button
  // can re-attempt (which resumes the download) without re-posting the question.
  // `web` is a search decided before asking the model (the time-word fast path).
  const runChat = async (text: string, web: WebContext | null = null) => {
    const resolved = await resolveWeb(text, web);
    if (resolved === ASKED) return;
    web = resolved;
    busy = true;
    send.disabled = true;
    const picked = currentPlan?.picked ?? null;
    const downloading = !!(picked && !picked.installed);
    let card: ReturnType<typeof makeInstallCard> | null = null;
    if (downloading) {
      card = makeInstallCard(`Setting up your ${tierName(picked!.id)} model — one-time ${fmtSize(picked!.downloadMb)} download.`);
    }
    const onProgress = (msg: string) => {
      const p = parseProgress(msg);
      if (p && card) card.update(p.done, p.total);
      else status.textContent = msg;
    };
    status.textContent = "Thinking…";
    try {
      const ask = () => chatSend(text, onProgress, modelIdForChat(), history, web, webEnabled());
      // Hold a screen wake lock while a (possibly large) model downloads.
      let decision = downloading ? await withScreenAwake(ask) : await ask();
      card?.el.remove();
      if (picked) picked.installed = true; // downloaded now — don't renotify
      // The model asked to check the web (its one-line SEARCH: protocol, llm.rs):
      // run the words through the data rule, then ask again with what came back.
      if (decision.action === "search" && webEnabled() && !web) {
        const asked = await resolveWeb(text, { query: decision.reply, sources: [], unavailable: null });
        if (asked === ASKED) return;
        web = asked;
        status.textContent = "Thinking…";
        decision = await ask();
      }
      status.textContent = "";
      const reply = decision.reply || "(no reply)";
      addMsg("ai", reply, web);
      // Only a completed exchange becomes memory (see `history`'s note).
      if (decision.reply) history.push({ user: text, assistant: decision.reply });
    } catch (err) {
      card?.el.remove();
      status.textContent = "";
      // A download that was interrupted (screen off / app left) keeps its progress
      // on disk — offer a friendly one-tap resume instead of a raw OS error.
      const msg = friendlyError(err);
      if (downloading && looksInterrupted(msg)) {
        addActionMsg(
          "Download paused — your progress is saved. Tap to pick up where it left off. (If it keeps pausing, check your connection or allow background data for Verdure in Settings.)",
          "Resume download →",
          () => void runChat(text, web),
        );
      } else {
        addMsg("ai", msg);
      }
    } finally {
      busy = false;
      send.disabled = false;
      input.focus();
    }
  };

  // Two people share the name the user typed: ask which, never guess
  // (knowledge/25). The chosen full name replaces the word and the question
  // runs again.
  const askWhich = (text: string, r: Recall) => {
    const card = el("div", "msg ai web-offer");
    card.appendChild(el("p", "", `Which ${capitalise(r.label)} do you mean?`));
    const row = el("div", "web-row");
    for (const p of r.ambiguous) {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "update-choice";
      b.textContent = p.displayName;
      b.addEventListener("click", () => {
        card.remove();
        void ask(text.replace(new RegExp(r.label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "i"), p.displayName));
      });
      row.appendChild(b);
    }
    card.appendChild(row);
    thread.appendChild(card);
    thread.scrollTop = thread.scrollHeight;
  };

  // Answer `text`: first from what the phone has seen (knowledge/25 — rules,
  // no model, for a pure lookup; a few exact records for the model otherwise),
  // then the web if it applies, then the model alone.
  const ask = async (text: string) => {
    const r = await memoryRecall(text).catch((): Recall | null => null);
    if (r?.kind === "ambiguous") {
      askWhich(text, r);
      return;
    }
    if (r?.kind === "direct" && r.direct) {
      // A lookup: shown as-is, with the records under it. No model ran.
      addMsg("ai", r.direct, { query: r.label, sources: r.sources, unavailable: null, memoryLabel: r.label });
      history.push({ user: text, assistant: r.direct });
      return;
    }
    if (r?.kind === "sources" && r.sources.length) {
      await runChat(text, { query: r.label, sources: r.sources, unavailable: null, memoryLabel: r.label });
      return;
    }
    // With web search on, a question that plainly needs fresh facts is searched
    // before the model is asked (its own words, scrubbed); everything else is
    // the model's call through its SEARCH: line (knowledge/24).
    if (webEnabled() && looksTimeSensitive(text)) {
      await runChat(text, { query: await prepareWebQuery(text), sources: [], unavailable: null });
      return;
    }
    await runChat(text);
  };

  const submit = async () => {
    const text = input.value.trim();
    if (!text || busy) return;
    input.value = "";
    examples.remove(); // empty-state examples make way for the conversation
    addMsg("user", text);
    await ask(text);
  };

  send.addEventListener("click", () => void submit());
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void submit();
    }
  });
  input.focus();
}


// --- What Verdure knows (knowledge/25, 27) ------------------------------------
// The transparency page a powerful permission requires: what is held, from
// which apps, who and which conversations — and a way to forget any of it.
// Nothing here leaves the device, and the page says so in one sentence.

/** The app's everyday name from its package (mirrors context.rs `app_label`). */
function appName(pkg: string): string {
  return (
    {
      "com.whatsapp": "WhatsApp",
      "com.whatsapp.w4b": "WhatsApp Business",
      "com.instagram.android": "Instagram",
      "org.telegram.messenger": "Telegram",
      "com.facebook.orca": "Messenger",
      "com.snapchat.android": "Snapchat",
      "com.google.android.apps.messaging": "Messages",
      sms: "Messages",
      calendar: "Calendar",
      "com.google.android.calendar": "Calendar",
      photos: "Photos",
    }[pkg] ?? pkg
  );
}

/** "21 Oct" / "21 Oct 2025" — the day, in the user's locale. */
function whenShort(ms: number): string {
  const d = new Date(ms);
  const sameYear = d.getFullYear() === new Date().getFullYear();
  return d.toLocaleDateString(undefined, { day: "numeric", month: "short", ...(sameYear ? {} : { year: "numeric" }) });
}

function capitalise(s: string): string {
  return s ? s[0].toUpperCase() + s.slice(1) : s;
}

/** A tapped source: Verdure's own links open in-app (a conversation, in a
 *  sheet over the chat); anything else is a web page for the browser. */
async function openSource(url: string): Promise<void> {
  const m = /^verdure:\/\/thread\/(\d+)$/.exec(url);
  if (m) {
    await openThreadSheet(Number(m[1]));
    return;
  }
  if (url.startsWith("verdure://")) return; // an event or a single record: nothing more to show yet
  await openUrl(url).catch(() => {});
}

/** A conversation, oldest first, over whatever screen is showing. */
async function openThreadSheet(threadId: number): Promise<void> {
  const sheet = el("div", "sheet");
  const body = el("div", "sheet-body");
  const head = el("div", "sheet-head");
  const title = el("span", "sheet-title", "Conversation");
  head.appendChild(title);
  const close = document.createElement("button");
  close.type = "button";
  close.className = "update-choice";
  close.textContent = "Close";
  close.addEventListener("click", () => sheet.remove());
  head.appendChild(close);
  body.appendChild(head);
  const list = el("div", "sheet-thread");
  body.appendChild(list);
  sheet.appendChild(body);
  sheet.addEventListener("click", (e) => {
    if (e.target === sheet) sheet.remove();
  });
  document.body.appendChild(sheet);
  try {
    const items = await memoryThread(threadId);
    if (items.length) title.textContent = `${items[0].threadTitle} · ${appName(items[0].app)}`;
    for (const o of items) {
      const m = el("div", o.direction === "out" ? "msg user" : "msg ai", o.text);
      m.appendChild(el("span", "msg-when", `${o.direction === "out" ? "You" : o.sender} · ${whenShort(o.atMs)}`));
      list.appendChild(m);
    }
    if (!items.length) list.appendChild(el("p", "subtitle", "Nothing here any more."));
    body.scrollTop = body.scrollHeight;
  } catch (err) {
    list.appendChild(el("p", "subtitle", friendlyError(err)));
  }
}

async function renderMemory(): Promise<void> {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());
  root.appendChild(backButton(renderChat));

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "What Verdure knows"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Messages and events Verdure has seen on this phone, so it can answer questions like “what did Tunde say about the rent?”. Everything stays on this device — nothing is sent anywhere — and you can forget any of it here.",
    ),
  );
  const status = el("p", "status");
  const box = el("div", "memory");
  panel.appendChild(box);
  panel.appendChild(status);
  root.appendChild(panel);

  const paint = (o: MemoryOverview) => {
    box.innerHTML = "";

    // 1. The feed: Android's notification access, in the OS's own hands.
    const feed = el("div", "updates-box");
    feed.appendChild(el("h2", "section-title", "Read my messages"));
    if (o.listener === "unsupported") {
      feed.appendChild(
        el("p", "memory-status", "Reading messages as they arrive is an Android feature. On this device you can share a WhatsApp chat export below instead."),
      );
    } else {
      const line = el("p", "memory-status");
      line.innerHTML =
        o.listener === "on"
          ? "<strong>On.</strong> New messages from the apps below are remembered as they arrive."
          : "<strong>Off.</strong> Allow Verdure “notification access” in your phone’s settings, and messages from the apps you choose are remembered as they arrive — on this phone only.";
      feed.appendChild(line);
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "update-choice" + (o.listener === "on" ? "" : " preferred");
      btn.textContent = o.listener === "on" ? "Change in phone settings" : "Turn on in phone settings";
      btn.addEventListener("click", () => void openNotificationAccess().catch((err) => (status.textContent = friendlyError(err))));
      feed.appendChild(btn);
      // Per-app opt-in: the user chooses which apps are read.
      feed.appendChild(el("p", "memory-status", "Apps to remember messages from:"));
      const checks = el("div", "check-list");
      const chosen = new Set(o.apps);
      for (const [pkg, label] of o.knownApps) {
        const c = document.createElement("button");
        c.type = "button";
        c.className = "check-row" + (chosen.has(pkg) ? " selected" : "");
        c.setAttribute("aria-pressed", String(chosen.has(pkg)));
        c.textContent = label;
        c.addEventListener("click", async () => {
          if (chosen.has(pkg)) chosen.delete(pkg);
          else chosen.add(pkg);
          c.classList.toggle("selected", chosen.has(pkg));
          c.setAttribute("aria-pressed", String(chosen.has(pkg)));
          await memorySetApps([...chosen]).catch((err) => (status.textContent = friendlyError(err)));
        });
        checks.appendChild(c);
      }
      feed.appendChild(checks);
    }
    box.appendChild(feed);

    // 2. What is held, per app — the honest number.
    const held = el("div", "updates-box");
    held.appendChild(el("h2", "section-title", "What's remembered"));
    if (!o.counts.length) {
      held.appendChild(el("p", "model-empty", "Nothing yet."));
    } else {
      for (const c of o.counts) {
        held.appendChild(
          el("p", "memory-status", `${appName(c.app)}: ${c.items} message${c.items === 1 ? "" : "s"}${c.threads ? ` in ${c.threads} conversation${c.threads === 1 ? "" : "s"}` : ""}`),
        );
      }
    }
    box.appendChild(held);

    const forgetRow = (label: string, meta: string, onOpen: (() => void) | null, onForget: () => Promise<void>) => {
      const row = el("div", "model-row memory-row");
      const info = el("div", "model-info");
      info.appendChild(el("span", "model-name", label));
      info.appendChild(el("span", "model-meta", meta));
      if (onOpen) info.addEventListener("click", onOpen);
      row.appendChild(info);
      const forget = document.createElement("button");
      forget.className = "link-danger";
      forget.type = "button";
      forget.textContent = "Forget";
      forget.addEventListener("click", async (e) => {
        e.stopPropagation();
        forget.disabled = true;
        forget.textContent = "Forgetting…";
        try {
          await onForget();
        } catch (err) {
          status.textContent = friendlyError(err);
        }
        paint(await memoryOverview());
      });
      row.appendChild(forget);
      return row;
    };

    // 3. Conversations: tap to read, Forget to drop.
    if (o.threads.length) {
      const conv = el("div", "updates-box");
      conv.appendChild(el("h2", "section-title", "Conversations"));
      const list = el("div", "model-list");
      for (const t of o.threads) {
        list.appendChild(
          forgetRow(
            `${t.title}${t.isGroup ? " (group)" : ""} · ${appName(t.app)}`,
            `${whenShort(t.lastAtMs)} · ${t.lastText}`,
            () => void openThreadSheet(t.id),
            () => memoryForget("thread", t.id),
          ),
        );
      }
      conv.appendChild(list);
      box.appendChild(conv);
    }

    // 4. People.
    if (o.people.length) {
      const ppl = el("div", "updates-box");
      ppl.appendChild(el("h2", "section-title", "People"));
      const list = el("div", "model-list");
      for (const p of o.people) {
        list.appendChild(forgetRow(p.displayName, `${p.items} message${p.items === 1 ? "" : "s"}`, null, () => memoryForget("person", p.id)));
      }
      ppl.appendChild(list);
      box.appendChild(ppl);
    }

    // 5. Coming up.
    if (o.events.length) {
      const ev = el("div", "updates-box");
      ev.appendChild(el("h2", "section-title", "Coming up"));
      const list = el("div", "model-list");
      for (const e of o.events) {
        list.appendChild(forgetRow(e.title, `${whenShort(e.startMs)}${e.place ? ` · ${e.place}` : ""}`, null, () => memoryForget("event", e.id)));
      }
      ev.appendChild(list);
      box.appendChild(ev);
    }

    // 6. Bring in a chat's past: WhatsApp's "Export chat" file.
    const imp = el("div", "updates-box");
    imp.appendChild(el("h2", "section-title", "Add a chat's history"));
    imp.appendChild(
      el(
        "p",
        "memory-status",
        "In WhatsApp, open a chat › ⋮ › More › Export chat › Without media, and save the file here. Verdure reads it on this phone; the same file twice adds nothing.",
      ),
    );
    const form = el("div", "memory-import");
    const title = document.createElement("input");
    title.type = "text";
    title.placeholder = "Who the chat is with (as WhatsApp names it)";
    const me = document.createElement("input");
    me.type = "text";
    me.placeholder = "Your own name, as it appears in the export";
    const file = document.createElement("input");
    file.type = "file";
    file.accept = ".txt,text/plain";
    const go = document.createElement("button");
    go.type = "button";
    go.className = "update-choice preferred";
    go.textContent = "Add this chat";
    go.addEventListener("click", async () => {
      const f = file.files?.[0];
      if (!f || !title.value.trim() || !me.value.trim()) {
        status.textContent = "Choose the export file and fill in both names first.";
        return;
      }
      go.disabled = true;
      go.textContent = "Adding…";
      try {
        const n = await memoryImportWhatsapp(await f.text(), title.value.trim(), me.value.trim());
        status.textContent = n ? `Added ${n} message${n === 1 ? "" : "s"} from ${title.value.trim()}.` : "Nothing new in that file.";
        paint(await memoryOverview());
      } catch (err) {
        status.textContent = friendlyError(err);
      } finally {
        go.disabled = false;
        go.textContent = "Add this chat";
      }
    });
    form.appendChild(title);
    form.appendChild(me);
    form.appendChild(file);
    form.appendChild(go);
    imp.appendChild(form);
    box.appendChild(imp);

    // 7. The exit.
    if (o.counts.length) {
      const all = document.createElement("button");
      all.type = "button";
      all.className = "link-danger";
      all.textContent = "Forget everything";
      all.addEventListener("click", async () => {
        all.disabled = true;
        try {
          await memoryForget("all");
        } catch (err) {
          status.textContent = friendlyError(err);
        }
        paint(await memoryOverview());
      });
      box.appendChild(all);
    }
  };

  try {
    const o = await memoryOverview();
    paint(o);
    if (o.drained.inserted) status.textContent = `${o.drained.inserted} new message${o.drained.inserted === 1 ? "" : "s"} since last time.`;
  } catch (err) {
    box.appendChild(el("p", "subtitle", friendlyError(err)));
  }
}

// --- small DOM helpers ------------------------------------------------------
function backButton(onClick: () => void): HTMLElement {
  const back = document.createElement("button");
  back.className = "back";
  back.type = "button";
  back.textContent = "← Back";
  back.addEventListener("click", onClick);
  return back;
}


function brandBar(actions?: HTMLElement[]): HTMLElement {
  const bar = el("div", "brand");
  bar.appendChild(el("span", "brand-dot", ""));
  bar.appendChild(el("span", "brand-name", "Verdure"));
  if (actions && actions.length) {
    const box = el("div", "brand-actions");
    for (const a of actions) box.appendChild(a);
    bar.appendChild(box);
  }
  return bar;
}

// Inline SVGs (no external assets — the app's CSP blocks remote fetches). They
// inherit color via `currentColor`, so CSS controls their tint.
// Lucide (github.com/lucide-icons/lucide, ISC) — clean, evenly-weighted paths.
const ICON_MIC =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/></svg>';
// Globe — the web search switch in the chat bar.
const ICON_GLOBE =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/></svg>';
// Book-open — "what Verdure knows": the memory page.
const ICON_MEMORY =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 7v14"/><path d="M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z"/></svg>';
// Hard-drive — reads as "storage / what's on your computer" more clearly than a gear.
const ICON_STORAGE =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" x2="2" y1="12" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11Z"/><line x1="6" x2="6.01" y1="16" y2="16"/><line x1="10" x2="10.01" y1="16" y2="16"/></svg>';

// A small, quiet header action (gear, etc.). aria-label so it's not a mystery
// glyph to a screen reader (skills/design.md — signifiers).
function iconButton(svg: string, label: string, onClick: () => void): HTMLButtonElement {
  const b = document.createElement("button");
  b.className = "icon-btn";
  b.type = "button";
  b.title = label;
  b.setAttribute("aria-label", label);
  b.innerHTML = svg;
  b.addEventListener("click", onClick);
  return b;
}

function el(tag: string, className: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}


// Which build is this? Stamped into the corner so the answer is never a guess —
// on 2026-08-22 a stale copy of the app was tested for several minutes because
// two builds were indistinguishable on screen. Attached to <body>, not to #app,
// because every screen re-render clears #app; this must outlive all of them.
// Deliberately tiny and dim: useful when you look for it, invisible when you
// don't (design.md — the interface shows the user's work, not the software's).
function mountBuildStamp(): void {
  const stamp = el("div", "build-stamp", __BUILD_STAMP__);
  // Not announced to screen readers: it is a developer breadcrumb, not content.
  stamp.setAttribute("aria-hidden", "true");
  stamp.title = `Verdure build ${__BUILD_STAMP__}`;
  document.body.appendChild(stamp);
}

// --- Updates ----------------------------------------------------------------

// The one-time question (knowledge/23). Shown as a quiet bar rather than a modal:
// the user came here to use Verdure, and a blocking dialog on first launch — on
// top of the model picker — would be two walls before the product. Choosing is
// one tap, and every option is reversible from Storage.
//
// Copy rule (copywriting.md): say what actually happens, not what sounds safest.
// "Check for updates?" hides that checking IS the network request; the wording
// below names it, because that is the thing being consented to.
function updateConsentBar(onDone: () => void): HTMLElement {
  const bar = el("div", "update-consent");
  bar.appendChild(
    el(
      "p",
      "update-consent-q",
      "How should Verdure handle new versions? Checking asks our server whether one exists — the only time Verdure reaches the internet on its own. Everything else stays on your device.",
    ),
  );
  const row = el("div", "update-consent-row");
  const choose = (mode: UpdateMode, label: string, title: string): HTMLElement => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "update-choice" + (mode === "notify" ? " preferred" : "");
    b.textContent = label;
    b.title = title;
    b.addEventListener("click", () => {
      setUpdateMode(mode);
      bar.remove();
      onDone();
    });
    return b;
  };
  row.appendChild(choose("notify", "Tell me", "Check on launch and let you decide whether to install"));
  row.appendChild(choose("auto", "Update automatically", "Download and install new versions in the background"));
  row.appendChild(choose("manual", "Don't check", "Verdure never reaches out; check yourself from Storage"));
  bar.appendChild(row);
  return bar;
}

// A found update, offered rather than imposed. In "auto" mode we install without
// asking (that is what they chose); in "notify" mode this bar is the choice.
function updateAvailableBar(version: string): HTMLElement {
  const bar = el("div", "update-available");
  const msg = el("p", "update-available-msg", `Verdure ${version} is available.`);
  bar.appendChild(msg);
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "update-choice preferred";
  btn.textContent = "Install and restart";
  btn.addEventListener("click", () => {
    btn.disabled = true;
    void installUpdate((f) => {
      msg.textContent =
        f < 0 ? "Downloading…" : f >= 1 ? "Installing…" : `Downloading… ${Math.round(f * 100)}%`;
    }).catch((e) => {
      console.error("update install failed", e);
      msg.textContent = "That update couldn't be installed. The version you have keeps working — please try again later.";
      btn.disabled = false;
    });
  });
  bar.appendChild(btn);
  return bar;
}

/**
 * The Updates row in Storage: check now, and change the automatic-check setting.
 * "Check for updates" works in every mode INCLUDING "Don't check" — pressing it
 * is itself the consent for that one request, which is why it is always offered.
 */
function updatesSection(): HTMLElement {
  const box = el("div", "updates-box");
  box.appendChild(el("h2", "section-title", "Updates"));

  const status = el("p", "updates-status", `You're on ${__BUILD_STAMP__}.`);
  box.appendChild(status);

  const check = document.createElement("button");
  check.type = "button";
  // Same pill as the consent bar's choices — the update UI should read as one
  // thing wherever it appears, and this is not a primary action competing with
  // the model controls below it.
  check.className = "update-choice";
  check.textContent = "Check for updates";
  check.addEventListener("click", () => {
    check.disabled = true;
    status.textContent = "Checking…";
    void checkForUpdate()
      .then((found) => {
        if (!found) {
          status.textContent = "You're up to date.";
          return;
        }
        status.textContent = `Verdure ${found.version} is available.`;
        box.appendChild(updateAvailableBar(found.version));
      })
      .catch(() => {
        // Say what failed in the user's terms. Most likely cause by far is being
        // offline, and guessing that is more useful than echoing a stack trace.
        status.textContent = "Couldn't reach the update server. Check your connection and try again.";
      })
      .finally(() => {
        check.disabled = false;
      });
  });
  box.appendChild(check);

  // The reversible preference. Radio-style so the current state is visible at a
  // glance rather than hidden behind a toggle whose meaning has to be inferred.
  const modes: { mode: UpdateMode; label: string }[] = [
    { mode: "notify", label: "Tell me when there's a new version" },
    { mode: "auto", label: "Install new versions automatically" },
    { mode: "manual", label: "Never check on its own" },
  ];
  const list = el("div", "updates-modes");
  const paint = () => {
    list.innerHTML = "";
    const current = updateMode() ?? "notify";
    for (const m of modes) {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "updates-mode" + (m.mode === current ? " selected" : "");
      b.textContent = m.label;
      b.setAttribute("aria-pressed", String(m.mode === current));
      b.addEventListener("click", () => {
        setUpdateMode(m.mode);
        paint();
      });
      list.appendChild(b);
    }
  };
  paint();
  box.appendChild(list);

  // Attribution for the open-source software Verdure is built on ([OQ-79]).
  // Placed here rather than behind an "About" screen we don't have: this is the
  // app-and-your-computer screen, and the notices are about what the app is made
  // of. Quiet link styling — it is a credit, not a call to action.
  const credits = document.createElement("button");
  credits.type = "button";
  credits.className = "link";
  credits.textContent = "Open-source licences";
  credits.title = "The open-source software Verdure is built on";
  credits.addEventListener("click", () => void openThirdPartyNotices());
  box.appendChild(credits);
  return box;
}

// --- Web search (knowledge/24) ------------------------------------------------

/**
 * The Web search row in Storage: the connection rule, and the disclosure of
 * what leaves the device — the same small toggle the chat shows, so the words
 * are identical wherever the user meets them.
 */
function webSection(): HTMLElement {
  const box = el("div", "updates-box");
  box.appendChild(el("h2", "section-title", "Web search"));
  box.appendChild(
    el(
      "p",
      "updates-status",
      "Switch web search on or off with the globe in the chat bar. While it's on, Verdure can check the web for up-to-date answers, using:",
    ),
  );
  const modes: { mode: WebMode; label: string }[] = [
    { mode: "wifi", label: "Wi-Fi only" },
    { mode: "any", label: "Wi-Fi and mobile data" },
  ];
  const list = el("div", "updates-modes");
  const paint = () => {
    list.innerHTML = "";
    const current = webMode();
    for (const m of modes) {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "updates-mode" + (m.mode === current ? " selected" : "");
      b.textContent = m.label;
      b.setAttribute("aria-pressed", String(m.mode === current));
      b.addEventListener("click", () => {
        setWebMode(m.mode);
        paint();
      });
      list.appendChild(b);
    }
  };
  paint();
  box.appendChild(list);
  const d = document.createElement("details");
  d.className = "disclose";
  const s = document.createElement("summary");
  s.textContent = "What leaves your device?";
  d.appendChild(s);
  d.appendChild(el("p", "disclose-text", WEB_DISCLOSURE));
  box.appendChild(d);
  return box;
}

// --- boot -------------------------------------------------------------------
async function boot(): Promise<void> {
  mountBuildStamp();
  // Straight into chat: there is nothing to choose first (knowledge/21).
  renderChat();
  void runUpdateFlow();
}

/**
 * Ask about updates once, then act on the answer. Runs AFTER the first paint and
 * after device detection, so it never delays the app appearing — an update is
 * never more urgent than the thing the user opened Verdure to do.
 *
 * It is the only question on a first launch — there is no model to choose
 * first — and it is a quiet bar, not a wall.
 */
async function runUpdateFlow(): Promise<void> {
  if (!updatesSupported()) return; // iOS/Android update through their stores
  const host = document.querySelector<HTMLElement>(".panel");
  if (shouldAskAboutUpdates()) {
    if (!host) return;
    host.prepend(updateConsentBar(() => void runUpdateFlow()));
    return;
  }
  const found = await checkOnLaunch();
  if (!found) return;
  if (updateMode() === "auto") {
    // They asked for this to be silent, so it is — no bar, no interruption.
    await installUpdate(() => {}).catch(() => {
      // A failed background update is not worth a dialog; the Storage button and
      // the next launch both remain available.
    });
    return;
  }
  document.querySelector<HTMLElement>(".panel")?.prepend(updateAvailableBar(found.version));
}

void boot();
