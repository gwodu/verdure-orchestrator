// Verdure — the UI: one interface over local models (knowledge/18). The app
// opens into chat (dictation is the mic button in it); the model picker and
// Storage are the only other screens. Pure logic lives in format.ts, picker.ts
// and updates.ts so it can be tested without booting the app. Copy is
// jargon-free (skills/design.md, skills/copywriting.md).

import {
  startLiveTranscription,
  stopLiveTranscription,
  chatSend,
  type ChatTurn,
  chatModelOptions,
  listModels,
  removeModel,
  systemResources,
  inTauri,
  freeUpSpace,
  openStorageSettings,
  modelsDir,
  revealModelsDir,
  type ChatModelOption,
  openThirdPartyNotices,
} from "./bridge";

import {
  escapeHtml,
  fmtBytes,
  fmtSize,
  friendlyError,
  looksInterrupted,
  micPermissionHelp,
  parseProgress,
  renderMarkdown,
} from "./format";
import { recommendedChoice } from "./picker";

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
// The chat model the user picked (Small/Balanced/Powerful), by model id.
const MODEL_CHOICE_KEY = "verdure.chat.modelId";
// Plain-language tier names + one-line blurbs for the picker, keyed by model id
// (design.md: the headline is the benefit; the model name is a demoted detail).
// Order here is the display order (smallest → most capable).
const MODEL_TIERS: { id: string; tier: string; blurb: string }[] = [
  { id: "qwen2.5-0.5b-instruct", tier: "Small", blurb: "Quick and light — runs on any device. Great for short questions." },
  { id: "qwen3.5-2b-instruct", tier: "Balanced", blurb: "A capable all-rounder, and it remembers much more of the conversation." },
  { id: "qwen3-4b-instruct-2507-byteshape", tier: "Powerful", blurb: "The most capable — richer answers, needs a little more room." },
  // iPad experiment ([OQ-84]): the backend offers it, as an EXTRA choice, only on
  // iOS devices with 16 GB of memory. Only offered options are drawn, so no other
  // device sees it. Labelled as a test, not sold as a product (copywriting.md).
  { id: "qwen3.8-27b-byteshape", tier: "Experimental", blurb: "A much larger model we're still testing. A very big download, and it may be slow or fail to start." },
];
const tierName = (id: string): string =>
  MODEL_TIERS.find((t) => t.id === id)?.tier ?? "Chat model";
const chosenModelId = (): string => localStorage.getItem(MODEL_CHOICE_KEY) ?? "";

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

// --- Choose your model: first-launch pick + change-anytime -------------------
// The user picks Small / Balanced / Powerful, with each one's download size and
// whether it fits this device (design.md: honest space, no jargon on the surface —
// the model name is a demoted detail). `onboarding` = the first-run flow (no way
// back until a choice is made); otherwise it's reached from chat, with a back.
async function renderModelPicker(onboarding: boolean): Promise<void> {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());
  if (!onboarding) root.appendChild(backButton(renderChat));

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Choose your model"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      onboarding
        ? "Verdure runs a chat model right on your device — nothing leaves it. Pick one to start; you can change it anytime. It downloads the first time you chat."
        : "Switch the model Verdure uses to chat. The new one downloads the first time you use it.",
    ),
  );

  // Live free-space readout (the transparency the founder liked from Storage),
  // right where you're deciding what to download.
  const freeEl = el("p", "picker-free", "");
  panel.appendChild(freeEl);
  const list = el("div", "model-picker");
  panel.appendChild(list);
  root.appendChild(panel);

  // A stored id can name a tier that no longer exists (a model retired between
  // versions — the [OQ-71] diagnostics were the first case). Fall back to the
  // first tier so the picker always shows a real selection, matching what the
  // backend already does with a stale choice (`chat_model_choice` auto-picks).
  const stored = chosenModelId();
  let selected = MODEL_TIERS.some((t) => t.id === stored) ? stored : MODEL_TIERS[0].id;
  let options = await chatModelOptions().catch(() => [] as ChatModelOption[]);
  // The backend's menu differs per device, so a selection it doesn't offer would
  // be drawn as nothing at all. Fall back to its first offer.
  if (options.length > 0 && !options.some((o) => o.id === selected)) selected = options[0].id;
  // First run: pre-select the most capable standard model that fits, and say
  // so, so the first screen is one obvious tap (design.md: zero configuration).
  const STANDARD_IDS = MODEL_TIERS.filter((t) => t.tier !== "Experimental").map((t) => t.id);
  const recommended = recommendedChoice(options, STANDARD_IDS);
  if (!stored && recommended) selected = recommended;

  // Which OS this is decides the honest way to help when a model doesn't fit the
  // disk ([OQ-67]): Android can clear cached junk on request, the Mac has its own
  // storage screen, the iPhone only has instructions. Learned async; re-render
  // once known so the right help appears.
  let os = "";
  // Feedback from the last "make room" attempt — survives re-renders.
  let spaceNote = "";
  const fillFree = async () => {
    try {
      const r = await systemResources();
      os = r.os;
      freeEl.textContent = `${fmtSize(r.freeDiskMb)} free space · ${fmtSize(r.availableRamMb)} memory free`;
    } catch {
      freeEl.textContent = "";
    }
  };

  const render = () => {
    list.innerHTML = "";
    // Show in tier order (small → powerful), pairing tier copy with device facts.
    for (const t of MODEL_TIERS) {
      const opt = options.find((o) => o.id === t.id);
      // The backend's menu is per-platform (the iPad's Powerful slot differs), so
      // once it has answered, draw only what it offers. Before that (or in a
      // browser preview) there are no options and every tier shows.
      if (options.length > 0 && !opt) continue;
      const card = el("div", "pick" + (selected === t.id ? " selected" : ""));

      const main = document.createElement("button");
      main.className = "pick-main";
      main.type = "button";
      const fitsMem = !opt || opt.fitsMemory; // unknown (preview) → assume fine
      const fitsDisk = !opt || opt.fitsDisk;
      const sizeText = opt ? (opt.installed ? "Downloaded" : `${fmtSize(opt.downloadMb)} download`) : "";
      let warn = "";
      // Memory is a LIVE condition, not a blocker: downloading depends on disk,
      // running depends on how much memory is free at that moment. Selecting is
      // never prevented, and the backend explains it properly at load time. So
      // this is a heads-up, phrased as something the user can act on — the old
      // wording ("Not enough free memory") read as a refusal, directly under a
      // line saying "7.0 GB free space", which looked like a contradiction
      // unless you already knew "memory" meant RAM.
      if (!fitsMem)
        warn = `You'll need to close some apps to use this one — it wants about ${fmtSize(opt!.runtimeMb)} of free memory.`;
      else if (!fitsDisk) {
        warn = `Not enough free space to download (needs ${fmtSize(opt!.downloadMb)}). Remove a model to make room.`;
        // iPhone/iPad: no API and no deep link — honest instructions are the help.
        if (os === "ios")
          warn +=
            " Or free space in Settings → General → iPhone Storage — offloading apps you don't use is the quickest win.";
      }
      main.innerHTML = `
        <span class="pick-radio" aria-hidden="true"></span>
        <span class="pick-body">
          <span class="pick-head">
            <span class="pick-tier">${t.tier}</span>
            <span class="pick-size">${escapeHtml(sizeText)}</span>
          </span>
          ${t.id === recommended ? `<span class="pick-rec">Recommended for this device</span>` : ""}
          <span class="pick-blurb">${escapeHtml(t.blurb)}</span>
          ${warn ? `<span class="pick-warn">${escapeHtml(warn)}</span>` : ""}
        </span>`;
      main.addEventListener("click", () => {
        selected = t.id;
        render();
      });
      card.appendChild(main);

      // Platform-specific "make room" help, only where honest help exists
      // ([OQ-67]). Android: the OS clears other apps' cached junk on request —
      // one tap, nothing important touched. macOS: open the system's own
      // storage screen. (iOS help is in the warn text above.)
      if (opt && !opt.installed && !fitsDisk && (os === "android" || os === "macos")) {
        const help = el("div", "pick-help");
        const btn = document.createElement("button");
        btn.className = "link";
        btn.type = "button";
        if (os === "android") {
          btn.textContent = "Clear cached junk to make room";
          btn.addEventListener("click", async () => {
            btn.disabled = true;
            btn.textContent = "Making room…";
            spaceNote = "";
            try {
              // Ask for the download plus a little headroom, so a successful
              // clear doesn't leave the disk a byte short mid-download.
              await freeUpSpace((opt.downloadMb + 64) * 1_048_576);
              options = await chatModelOptions().catch(() => options);
            } catch (err) {
              spaceNote = friendlyError(err);
            }
            await fillFree();
            render(); // fresh fit flags — if it fits now, the warning simply goes away
          });
        } else {
          btn.textContent = "See what's filling your disk (opens Mac settings)";
          btn.addEventListener("click", () => {
            void openStorageSettings().catch((err) => {
              spaceNote = friendlyError(err);
              render();
            });
          });
        }
        help.appendChild(btn);
        if (spaceNote) help.appendChild(el("span", "pick-warn", spaceNote));
        card.appendChild(help);
      }

      // Remove — only for downloaded models, so you can free space to fit another
      // right here, no separate trip to Storage.
      if (opt?.installed) {
        const rm = document.createElement("button");
        rm.className = "pick-remove";
        rm.type = "button";
        rm.textContent = "Remove";
        rm.addEventListener("click", async (e) => {
          e.stopPropagation();
          rm.disabled = true;
          rm.textContent = "Removing…";
          try {
            await removeModel(t.id);
            options = await chatModelOptions().catch(() => options);
            void fillFree(); // space freed — refresh the readout + fit flags
          } finally {
            render();
          }
        });
        card.appendChild(rm);
      }
      list.appendChild(card);
    }
    const go = document.createElement("button");
    go.className = "primary";
    go.type = "button";
    go.textContent = onboarding ? "Start chatting" : "Use this model";
    go.addEventListener("click", () => {
      const prev = chosenModelId();
      localStorage.setItem(MODEL_CHOICE_KEY, selected);
      // Switching away from a downloaded model leaves it as redundant weight on
      // a tight disk — offer its removal at the exact moment it stops being
      // needed ([OQ-67]). Optional; never automatic (coming back = re-download).
      const old =
        prev && prev !== selected
          ? options.find((o) => o.id === prev && o.installed)
          : undefined;
      if (old) renderFreeOldModel(old);
      else renderChat();
    });
    list.appendChild(go);
  };
  render();
  // The free-space readout — and which OS this is — arrive async; re-render so
  // the platform's "make room" help shows up where relevant.
  void fillFree().then(render);
}

// After a model switch: one optional tap to reclaim the old model's space.
// Honest about the trade — it re-downloads if the user ever switches back.
function renderFreeOldModel(old: ChatModelOption): void {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Free up space?"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      `The ${tierName(old.id)} model you were using is still on your device, taking ${fmtSize(old.downloadMb)}. ` +
        "Remove it? If you ever switch back, it simply downloads again.",
    ),
  );

  const status = el("p", "status");
  const removeLabel = `Remove it — free ${fmtSize(old.downloadMb)}`;
  const remove = document.createElement("button");
  remove.className = "primary";
  remove.type = "button";
  remove.textContent = removeLabel;
  remove.addEventListener("click", async () => {
    remove.disabled = true;
    remove.textContent = "Removing…";
    try {
      await removeModel(old.id);
      renderChat();
    } catch (err) {
      remove.disabled = false;
      remove.textContent = removeLabel;
      status.textContent = friendlyError(err);
    }
  });

  const keep = document.createElement("button");
  keep.className = "link";
  keep.type = "button";
  keep.textContent = "Keep both";
  keep.addEventListener("click", renderChat);

  panel.appendChild(remove);
  panel.appendChild(keep);
  panel.appendChild(status);
  root.appendChild(panel);
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
  modelBtn.title = "Change the chat model";
  modelBtn.setAttribute("aria-label", "Change the chat model");
  modelBtn.textContent = tierName(chosenModelId());
  modelBtn.addEventListener("click", () => void renderModelPicker(false));
  root.appendChild(
    brandBar([modelBtn, iconButton(ICON_STORAGE, "Storage & your computer", renderStorage)]),
  );

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Ask me anything"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Type to chat, or tap the microphone to turn speech into text. Everything runs on your device — private, no account. The first message downloads the chat model you picked.",
    ),
  );

  const thread = el("div", "chat-thread");
  panel.appendChild(thread);

  const status = el("p", "status");
  panel.appendChild(status);

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

  // The chosen model's size + install state, for the one-time "installing" notice.
  let chosen: ChatModelOption | undefined;
  void chatModelOptions()
    .then((opts) => (chosen = opts.find((o) => o.id === chosenModelId())))
    .catch(() => {});

  // The conversation so far — this IS the chat's memory (knowledge/22). Only
  // completed exchanges go in: a question whose answer failed would otherwise
  // teach the model that questions go unanswered. The backend keeps whichever
  // recent turns fit the window it sized for this device, so we pass all of it.
  const history: ChatTurn[] = [];

  const addMsg = (who: "user" | "ai", text: string) => {
    // Only the model's replies get Markdown rendering. What the USER typed is
    // shown exactly as typed — if they write **bold**, they meant asterisks.
    const m =
      who === "ai"
        ? (() => {
            const node = el("div", "msg ai");
            node.innerHTML = renderMarkdown(text); // escaped inside renderMarkdown
            return node;
          })()
        : el("div", "msg user", text);
    thread.appendChild(m);
    thread.scrollTop = thread.scrollHeight;
    return m;
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

  // Run one chat turn for `text`. Split out from `submit` so the "Resume" button
  // can re-attempt (which resumes the download) without re-posting the question.
  const runChat = async (text: string) => {
    busy = true;
    send.disabled = true;
    const downloading = !!(chosen && !chosen.installed);
    let card: ReturnType<typeof makeInstallCard> | null = null;
    if (downloading) {
      card = makeInstallCard(`Setting up your ${tierName(chosen!.id)} model — one-time ${fmtSize(chosen!.downloadMb)} download.`);
    }
    const onProgress = (msg: string) => {
      const p = parseProgress(msg);
      if (p && card) card.update(p.done, p.total);
      else status.textContent = msg;
    };
    status.textContent = "Thinking…";
    try {
      // Hold a screen wake lock while a (possibly large) model downloads.
      const decision = downloading
        ? await withScreenAwake(() => chatSend(text, onProgress, chosenModelId(), history))
        : await chatSend(text, onProgress, chosenModelId(), history);
      status.textContent = "";
      card?.el.remove();
      if (chosen) chosen.installed = true; // downloaded now — don't renotify
      // Chat is a plain conversation now — just show the reply.
      const reply = decision.reply || "(no reply)";
      addMsg("ai", reply);
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
          () => void runChat(text),
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

  const submit = async () => {
    const text = input.value.trim();
    if (!text || busy) return;
    input.value = "";
    examples.remove(); // empty-state examples make way for the conversation
    addMsg("user", text);
    await runChat(text);
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

// --- boot -------------------------------------------------------------------
async function boot(): Promise<void> {
  mountBuildStamp();
  // First launch (no model chosen yet) → the "Choose your model" screen; otherwise
  // straight into chat.
  if (chosenModelId()) renderChat();
  else void renderModelPicker(true);
  void runUpdateFlow();
}

/**
 * Ask about updates once, then act on the answer. Runs AFTER the first paint and
 * after device detection, so it never delays the app appearing — an update is
 * never more urgent than the thing the user opened Verdure to do.
 *
 * Ordering note: the question is deliberately held back until a model has been
 * chosen. On a true first launch the model picker is already on screen, and
 * stacking a second decision under it would make Verdure feel like a settings
 * wizard. They get asked the next time they land in chat.
 */
async function runUpdateFlow(): Promise<void> {
  if (!updatesSupported()) return; // iOS/Android update through their stores
  const host = document.querySelector<HTMLElement>(".panel");
  if (shouldAskAboutUpdates()) {
    if (!chosenModelId() || !host) return; // ask once they're past onboarding
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
