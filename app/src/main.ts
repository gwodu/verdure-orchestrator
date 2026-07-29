// Verdure desktop porter — the UI.
//
// Two screens, the whole "ride" (knowledge/17 "the atomic unit"):
//   1. Pick what you want to do   (the reused intent cards from @verdure/decider)
//   2. See the recommendation, set it up in one click
//
// The decision is NOT made here — it comes from `recommend()` in the shared
// decider, the single source of truth the website uses too. This file only
// presents it and triggers the porter. Copy is verb-first and jargon-free
// (skills/design.md): the headline is the action, the tool is the labeled answer.

import {
  INTENT_CARDS,
  recommend,
  type IntentCard,
  type Tier,
} from "@verdure/decider";
import {
  detectDevice,
  portTool,
  startLiveTranscription,
  stopLiveTranscription,
  chatSend,
  chatModelOptions,
  listModels,
  removeModel,
  systemResources,
  inTauri,
  type PortOutcome,
  type ChatModelOption,
} from "./bridge";

const WHISPER_MODEL_ID = "whisper-base-en";
const fmtMb = (bytes: number) => `${Math.max(1, Math.round(bytes / 1_048_576))} MB`;
import "./styles.css";

// Modalities now wrapped *inside* Verdure (knowledge/18 — run the model here, no
// separate app). Transcription is the first. Others still fall back to the
// legacy recommendation/porter view until they're wrapped too.
const WRAPPED = new Set(["audio:transcription"]);
// The chat model the user picked (Small/Balanced/Powerful), by model id.
const MODEL_CHOICE_KEY = "verdure.chat.modelId";
// Plain-language tier names + one-line blurbs for the picker, keyed by model id
// (design.md: the headline is the benefit; the model name is a demoted detail).
// Order here is the display order (smallest → most capable).
const MODEL_TIERS: { id: string; tier: string; blurb: string }[] = [
  { id: "qwen2.5-0.5b-instruct", tier: "Small", blurb: "Quick and light — runs on any device. Great for short questions." },
  { id: "qwen2.5-1.5b-instruct", tier: "Balanced", blurb: "A capable all-rounder for everyday chat." },
  { id: "qwen3-4b-instruct-2507-byteshape", tier: "Powerful", blurb: "The most capable — richer answers, needs a little more room." },
];
const tierName = (id: string): string =>
  MODEL_TIERS.find((t) => t.id === id)?.tier ?? "Chat model";
const chosenModelId = (): string => localStorage.getItem(MODEL_CHOICE_KEY) ?? "";
// Example questions shown in the empty chat — playful + informational, to invite
// a first message. Tapping one fills the input (the user still hits Send, since a
// first message triggers the one-time model download).
const CHAT_EXAMPLES = [
  { emoji: "💧", text: "What's the chemical composition of water?" },
  { emoji: "🥧", text: "How do you bake a pie?" },
  { emoji: "🌌", text: "How do the northern lights work?" },
  { emoji: "🐝", text: "Tell me a surprising fact about honeybees" },
];

// The MVP's wired ports: which catalog tool backs which classification, and the
// name to SHOW the user. Only transcription→Buzz is one-click in v1.0
// (knowledge/17 "first slice"); every other card still gets a real
// recommendation, just not a button yet.
//
// The shown name is the TOOL we actually install (Buzz) — not the underlying
// engine the decider names (Whisper). The user must be told what lands on their
// machine; the engine is shown as supporting context. (`id` mirrors the Rust
// catalog id in catalog.rs.)
type PortableTool = { id: string; name: string };
const PORTABLE: Record<string, PortableTool> = {
  "audio:transcription": { id: "buzz", name: "Buzz" },
};

const root = document.querySelector<HTMLDivElement>("#app")!;
let tier: Tier = "L2-high"; // replaced by detection on boot

// Active window drag-drop listener (transcription panel). Cleaned up on nav.
let dropUnlisten: (() => void) | null = null;
// Active live-transcription listeners, if a mic session is running. Every screen
// calls clearDrop() on entry, so leaving the transcribe panel always releases the
// microphone — no recording survives navigation.
let liveCleanup: (() => void) | null = null;
function clearDrop(): void {
  if (dropUnlisten) {
    dropUnlisten();
    dropUnlisten = null;
  }
  if (liveCleanup) {
    liveCleanup();
    liveCleanup = null;
    void stopLiveTranscription();
  }
}

function classKey(c: IntentCard["classification"]): string {
  return `${c.modality}:${c.task}`;
}

// --- Legacy front door: the task grid ---------------------------------------
// No longer the entry screen (the app opens straight into chat as of the 2026-07
// redesign). Retained — with the intent-grid / recommendation / porter flow it
// leads to — as the only path to un-wrapped modalities (image/video), pending the
// legacy-porter keep-or-remove decision ([OQ-60]/gap #2). Not currently reachable
// from the UI; wire a link back to it if that flow is kept.
function renderHome(): void {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());

  const intro = el("header", "intro");
  intro.appendChild(el("h1", "title", "What would you like to do?"));
  intro.appendChild(
    el(
      "p",
      "subtitle",
      "Pick a task. Verdure finds a free tool that runs right on your computer — private, no account, no cloud.",
    ),
  );
  root.appendChild(intro);

  const grid = el("div", "grid");
  for (const card of INTENT_CARDS) {
    grid.appendChild(intentCard(card));
  }
  root.appendChild(grid);

  // Footer link to the global Storage screen (manage downloaded models / space).
  const footer = el("div", "footer");
  const storageLink = document.createElement("button");
  storageLink.className = "link";
  storageLink.type = "button";
  storageLink.textContent = "Manage storage";
  storageLink.addEventListener("click", renderStorage);
  footer.appendChild(storageLink);
  root.appendChild(footer);
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

  // Live "Your computer" readout — the machine-context Verdure uses to size what
  // it runs (knowledge/20/21). Honest, plain numbers; no jargon.
  const sysBox = el("div", "sysbox");
  panel.appendChild(sysBox);
  const fillSys = async () => {
    const r = await systemResources();
    const gb = (mb: number) => `${(mb / 1024).toFixed(1)} GB`;
    sysBox.innerHTML = "";
    sysBox.appendChild(el("span", "sysbox-title", "Your computer"));
    const grid = el("div", "sysbox-grid");
    const row = (label: string, value: string) => {
      const r = el("div", "sysbox-row");
      r.appendChild(el("span", "sysbox-label", label));
      r.appendChild(el("span", "sysbox-value", value));
      grid.appendChild(r);
    };
    row("Memory free now", `${gb(r.availableRamMb)} of ${gb(r.totalRamMb)}`);
    row("Disk free", gb(r.freeDiskMb));
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
    listEl.innerHTML = "";

    if (!inTauri) {
      listEl.appendChild(el("p", "subtitle", "Storage is managed in the desktop app."));
      return;
    }
    if (downloaded.length === 0) {
      listEl.appendChild(
        el("p", "model-empty", "Nothing downloaded yet — nothing taking up space."),
      );
      return;
    }

    const total = downloaded.reduce((sum, m) => sum + m.sizeBytes, 0);
    panel.querySelector(".model-total")?.remove();

    for (const m of downloaded) {
      const row = el("div", "model-row");
      const info = el("div", "model-info");
      info.appendChild(el("span", "model-name", m.name));
      info.appendChild(el("span", "model-meta", `${m.modality} · ${fmtMb(m.sizeBytes)}`));
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
      listEl.appendChild(row);
    }

    const totalEl = el("p", "model-total", `Total on this computer: ${fmtMb(total)}`);
    panel.appendChild(totalEl);
  };

  void refresh();
}

function intentCard(card: IntentCard): HTMLElement {
  const btn = document.createElement("button");
  btn.className = "card";
  btn.type = "button";
  btn.innerHTML = `
    <span class="card-emoji" aria-hidden="true">${card.emoji}</span>
    <span class="card-title">${escapeHtml(card.title)}</span>
    <span class="card-desc">${escapeHtml(card.description)}</span>
  `;
  btn.addEventListener("click", () => {
    const key = classKey(card.classification);
    if (key === "text:general" || WRAPPED.has(key)) renderChat();
    else renderRecommendation(card);
  });
  return btn;
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

  let selected = chosenModelId() || MODEL_TIERS[0].id;
  let options = await chatModelOptions().catch(() => [] as ChatModelOption[]);

  const gb = (mb: number) => (mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`);
  const fillFree = async () => {
    try {
      const r = await systemResources();
      freeEl.textContent = `${gb(r.freeDiskMb)} free space · ${gb(r.availableRamMb)} memory free`;
    } catch {
      freeEl.textContent = "";
    }
  };
  void fillFree();

  const render = () => {
    list.innerHTML = "";
    // Show in tier order (small → powerful), pairing tier copy with device facts.
    for (const t of MODEL_TIERS) {
      const opt = options.find((o) => o.id === t.id);
      const card = el("div", "pick" + (selected === t.id ? " selected" : ""));

      const main = document.createElement("button");
      main.className = "pick-main";
      main.type = "button";
      const fitsMem = !opt || opt.fitsMemory; // unknown (preview) → assume fine
      const fitsDisk = !opt || opt.fitsDisk;
      const sizeText = opt ? (opt.installed ? "Downloaded" : `${gb(opt.downloadMb)} download`) : "";
      let warn = "";
      if (!fitsMem) warn = "Not enough free memory to run this one right now.";
      else if (!fitsDisk)
        warn = `Not enough free space to download (needs ${gb(opt!.downloadMb)}). Remove a model to make room.`;
      main.innerHTML = `
        <span class="pick-radio" aria-hidden="true"></span>
        <span class="pick-body">
          <span class="pick-head">
            <span class="pick-tier">${t.tier}</span>
            <span class="pick-size">${escapeHtml(sizeText)}</span>
          </span>
          <span class="pick-blurb">${escapeHtml(t.blurb)}</span>
          ${warn ? `<span class="pick-warn">${escapeHtml(warn)}</span>` : ""}
        </span>`;
      main.addEventListener("click", () => {
        selected = t.id;
        render();
      });
      card.appendChild(main);

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
      localStorage.setItem(MODEL_CHOICE_KEY, selected);
      renderChat();
    });
    list.appendChild(go);
  };
  render();
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
      "Type to chat, or tap the microphone to turn speech into text. Everything runs on your device — private, no account. The first message downloads a chat model (about 0.4–1.4 GB).",
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
  let recording = false;
  const setRecording = (on: boolean) => {
    recording = on;
    mic.classList.toggle("recording", on);
    mic.title = on ? "Stop dictation" : "Dictate — turn speech into text";
  };
  // Begin dictation. `needsInstall` shows the one-time speech-model download
  // notice; it clears as soon as the first words come back.
  const startDictation = async (needsInstall: boolean) => {
    setRecording(true);
    status.textContent = needsInstall ? "Setting up speech-to-text…" : "Listening… speak, then tap the mic again.";
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
    try {
      liveCleanup = await startLiveTranscription({
        onStatus: (m) => (status.textContent = m),
        onSegment: (seg) => {
          note?.remove();
          note = null; // first words → the download finished
          // Drop Whisper's non-speech markers ([BLANK_AUDIO], [ Silence ], …).
          const t = seg.replace(/\[[A-Za-z_ ]+\]/g, "").trim();
          if (t) input.value = input.value ? `${input.value} ${t}` : t; // into the box
        },
        onError: (m) => {
          note?.remove();
          status.textContent = m;
          setRecording(false);
        },
        onStopped: () => {
          note?.remove();
          setRecording(false);
          status.textContent = "";
          if (liveCleanup) {
            liveCleanup();
            liveCleanup = null;
          }
          input.focus();
        },
      });
    } catch (err) {
      note?.remove();
      status.textContent = String(err);
      setRecording(false);
    }
  };
  // Tap the mic: stop if recording; otherwise, if the speech model isn't installed
  // yet, ask permission first (with its size — the same honest prompt as the chat
  // model), then dictate. The user opts into the extra download.
  const toggleMic = async () => {
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

  const addMsg = (who: "user" | "ai", text: string) => {
    const m = el("div", `msg ${who}`, text);
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
  const submit = async () => {
    const text = input.value.trim();
    if (!text || busy) return;
    busy = true;
    send.disabled = true;
    input.value = "";
    examples.remove(); // empty-state examples make way for the conversation
    addMsg("user", text);
    // One-time install notice: if the chosen model isn't downloaded yet, say so
    // prominently (named tier + size) instead of a silent wait.
    let installEl: HTMLElement | null = null;
    if (chosen && !chosen.installed) {
      const mb = chosen.downloadMb;
      const size = mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`;
      installEl = el(
        "div",
        "install-note",
        `Setting up your ${tierName(chosen.id)} model — a one-time ${size} download. This can take a minute…`,
      );
      thread.appendChild(installEl);
      thread.scrollTop = thread.scrollHeight;
    }
    status.textContent = "Thinking…";
    try {
      const decision = await chatSend(text, (msg) => (status.textContent = msg), chosenModelId());
      status.textContent = "";
      if (chosen) chosen.installed = true; // downloaded now — don't renotify
      // Chat is a plain conversation now — just show the reply. (Transcription is
      // the mic button; the old JSON tool-call routed here and misfired on the tiny
      // model — observed on-device 2026-07-27.)
      addMsg("ai", decision.reply || "(no reply)");
    } catch (err) {
      status.textContent = String(err);
    } finally {
      installEl?.remove();
      busy = false;
      send.disabled = false;
      input.focus();
    }
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


// --- Screen 2b: the recommendation + one-click setup (legacy, un-wrapped) ----
function renderRecommendation(card: IntentCard): void {
  clearDrop();
  const rec = recommend(tier, card.classification);
  const portable = PORTABLE[classKey(card.classification)];

  root.innerHTML = "";
  root.appendChild(brandBar());

  const back = document.createElement("button");
  back.className = "back";
  back.type = "button";
  back.textContent = "← Back";
  back.addEventListener("click", renderHome);
  root.appendChild(back);

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", rec.headline));
  panel.appendChild(el("p", "subtitle", rec.body));

  if (rec.feasibility === "not-local-yet" || !rec.model) {
    // Honest decline — no tool to set up on this device yet (skills/design.md).
    panel.appendChild(badge("Not ready for your computer yet", "muted"));
    root.appendChild(panel);
    return;
  }

  // The answer the user came for — prominent, not buried. We name the TOOL we
  // actually install (Buzz); the decider's model (Whisper) is the engine, shown
  // as context so the user knows both what lands and what powers it.
  const answerName = portable ? portable.name : rec.model.name;
  const engineNote =
    portable && portable.name !== rec.model.name
      ? `Free · Runs on your computer · Powered by ${rec.model.name}`
      : rec.model.note;

  const answer = el("div", "answer");
  answer.appendChild(el("span", "answer-label", "We’ll set up"));
  answer.appendChild(el("span", "answer-name", answerName));
  if (engineNote) answer.appendChild(el("span", "answer-note", engineNote));
  panel.appendChild(answer);

  if (portable) {
    panel.appendChild(setupArea(portable));
  } else {
    // Recommended, but not yet a one-click port — be honest about it.
    panel.appendChild(
      el(
        "p",
        "soon",
        `One-click setup for ${rec.model.name} is coming soon. For now, this is the tool we recommend for ${rec.taskLabel}.`,
      ),
    );
  }

  root.appendChild(panel);
}

function setupArea(tool: PortableTool): HTMLElement {
  const wrap = el("div", "setup");

  const button = document.createElement("button");
  button.className = "primary";
  button.type = "button";
  button.textContent = `Set it up`;

  const status = el("p", "status");

  button.addEventListener("click", async () => {
    button.disabled = true;
    wrap.classList.add("working");
    status.textContent = "Starting…";
    try {
      const outcome: PortOutcome = await portTool(tool.id, (msg) => {
        status.textContent = msg;
      });
      renderDone(tool, outcome);
    } catch (err) {
      wrap.classList.remove("working");
      wrap.classList.add("failed");
      status.textContent = String(err);
      button.disabled = false;
      button.textContent = "Try again";
    }
  });

  wrap.appendChild(button);
  wrap.appendChild(status);
  return wrap;
}

function renderDone(tool: PortableTool, outcome: PortOutcome): void {
  const panel = root.querySelector(".panel");
  if (!panel) return;
  const done = el("div", "done");
  done.appendChild(el("span", "done-check", "✓"));
  done.appendChild(el("p", "done-msg", outcome.message));
  done.appendChild(
    el("p", "done-sub", `You can find ${tool.name} in your Applications from now on.`),
  );
  // Replace the setup area with the success state.
  const setup = panel.querySelector(".setup");
  if (setup) setup.replaceWith(done);
  else panel.appendChild(done);
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

function badge(text: string, kind: "muted" | "ok"): HTMLElement {
  return el("span", `badge ${kind}`, text);
}

function el(tag: string, className: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (ch) => {
    switch (ch) {
      case "&": return "&amp;";
      case "<": return "&lt;";
      case ">": return "&gt;";
      case '"': return "&quot;";
      default: return "&#39;";
    }
  });
}

// --- boot -------------------------------------------------------------------
async function boot(): Promise<void> {
  // First launch (no model chosen yet) → the "Choose your model" screen; otherwise
  // straight into chat. Detection refines the tier in the background either way.
  if (chosenModelId()) renderChat();
  else void renderModelPicker(true);
  try {
    tier = await detectDevice();
  } catch {
    // keep the safe default; detection is an optimization, never a blocker
  }
}

void boot();
