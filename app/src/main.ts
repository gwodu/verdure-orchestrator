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
  estimateTranscriptionSpeed,
  classifyIntent,
  type IntentCard,
  type Tier,
  type SpeedEstimate,
} from "@verdure/decider";
import {
  detectDevice,
  portTool,
  transcribeAudio,
  onAudioDrop,
  startLiveTranscription,
  stopLiveTranscription,
  chatSend,
  listModels,
  removeModel,
  deviceSpec,
  systemResources,
  inTauri,
  type PortOutcome,
} from "./bridge";

// The pre-run prediction line (from device specs).
function predictionLine(e: SpeedEstimate): string {
  if (e.category === "wont-fit") return `Estimate: ${e.note}`;
  const [lo, hi] = e.realtimeFactor!;
  const range = lo === hi ? `${lo}×` : `${lo}–${hi}×`;
  return `Estimate: ${e.note} (about ${range} faster than real-time).`;
}

// Audio length as m:ss (or Ns under a minute).
function fmtDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.round(sec % 60);
  return m > 0 ? `${m}:${String(s).padStart(2, "0")}` : `${s}s`;
}

// The post-run measured line, with the raw timing (our real calibration anchor).
function measuredLine(audioSeconds: number, processingSeconds: number): string {
  const factor = audioSeconds / processingSeconds;
  const shown = factor >= 2 ? Math.round(factor) : Math.round(factor * 10) / 10;
  return `Measured on your computer: ${fmtDuration(audioSeconds)} of audio in ${Math.round(
    processingSeconds,
  )}s — about ${shown}× faster than real-time.`;
}

const WHISPER_MODEL_ID = "whisper-base-en";
const fmtMb = (bytes: number) => `${Math.max(1, Math.round(bytes / 1_048_576))} MB`;
import "./styles.css";

// Modalities now wrapped *inside* Verdure (knowledge/18 — run the model here, no
// separate app). Transcription is the first. Others still fall back to the
// legacy recommendation/porter view until they're wrapped too.
const WRAPPED = new Set(["audio:transcription"]);
const AUDIO_EXTS = ["mp3", "m4a", "wav", "flac", "aac", "mp4", "ogg", "aiff", "caf"];

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

// --- Screen 1: pick an intent ----------------------------------------------
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
  root.appendChild(backButton(renderHome));

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
    row("Processor", `${r.cores} cores · ${r.arch === "apple-silicon" ? "Apple Silicon" : "Intel"}`);
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
    if (key === "text:general") renderChat();
    else if (WRAPPED.has(key)) renderTranscribe();
    else renderRecommendation(card);
  });
  return btn;
}

// --- Chat: the conversational front door (knowledge/18) ---------------------
function renderChat(): void {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());
  root.appendChild(backButton(renderHome));

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Chat"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Ask anything — it runs entirely on your computer, private and free. The first time, Verdure downloads a small chat model sized to your computer (about 0.4–1 GB).",
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
  const send = document.createElement("button");
  send.className = "primary chat-send";
  send.type = "button";
  send.textContent = "Send";
  rowEl.appendChild(input);
  rowEl.appendChild(send);
  panel.appendChild(rowEl);

  root.appendChild(panel);

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
    addMsg("user", text);
    status.textContent = "Thinking…";
    try {
      const decision = await chatSend(text, (msg) => (status.textContent = msg));
      status.textContent = "";
      // Cross-check the model's pick with our deterministic classifier.
      const cls = classifyIntent(text);
      const wantsTranscription =
        decision.action === "set_up_transcription" ||
        (cls.modality === "audio" && cls.task === "transcription");
      if (wantsTranscription) {
        addActionMsg(
          decision.reply || "I can set you up to turn recordings into text.",
          "Set it up →",
          renderTranscribe,
        );
      } else {
        addMsg("ai", decision.reply || "(no reply)");
      }
    } catch (err) {
      status.textContent = String(err);
    } finally {
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

// --- Screen 2a: transcription, wrapped IN-APP (the pivot, knowledge/18) ------
// Two ways to the same on-device result: drop a file, or record live from the
// mic. They share the transcript box, Copy, and the storage row; only the input
// (dropzone vs mic) and the status differ.
function renderTranscribe(): void {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());
  root.appendChild(backButton(renderHome));

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Turn speech into text"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Drop an audio file, or record live from your microphone. It's transcribed right on your computer — private, free, no account. The first time, Verdure downloads a small speech model (~140 MB).",
    ),
  );

  // Mode toggle: a file you already have, vs the mic right now.
  const modeToggle = el("div", "mode-toggle");
  const uploadTab = tabButton("Upload a file");
  const liveTab = tabButton("Record live");
  modeToggle.appendChild(uploadTab);
  modeToggle.appendChild(liveTab);
  panel.appendChild(modeToggle);

  // --- Upload mode: the dropzone + performance prediction --------------------
  const uploadArea = el("div", "upload-area");

  // Performance prediction (before any download) — honest expectation from the
  // machine's specs, calibrated to a real number after the first transcription.
  const perf = el("p", "perf");
  perf.style.display = "none";
  let perfReady = false;
  uploadArea.appendChild(perf);

  // Separate, persistent line for the real measured speed after a run (so the
  // estimate above stays visible — predicted vs actual).
  const measured = el("p", "perf measured");
  measured.style.display = "none";
  uploadArea.appendChild(measured);

  const zone = el("div", "dropzone");
  zone.innerHTML = `<span class="dropzone-emoji" aria-hidden="true">📄</span>
    <span class="dropzone-text">Drop an audio file here</span>
    <span class="dropzone-hint">mp3, m4a, wav, and more</span>`;
  uploadArea.appendChild(zone);
  panel.appendChild(uploadArea);

  // --- Live mode: the mic button + a recording indicator ---------------------
  const liveArea = el("div", "live-area");
  liveArea.style.display = "none";
  const recordBtn = document.createElement("button");
  recordBtn.className = "primary record";
  recordBtn.type = "button";
  recordBtn.textContent = "🎙️ Start recording";
  const recIndicator = el("span", "rec-indicator");
  recIndicator.style.display = "none";
  recIndicator.innerHTML = `<span class="rec-dot" aria-hidden="true"></span><span class="rec-time">0:00</span>`;
  liveArea.appendChild(recordBtn);
  liveArea.appendChild(recIndicator);
  panel.appendChild(liveArea);

  // --- Shared: status, transcript, copy, storage -----------------------------
  const status = el("p", "status");
  panel.appendChild(status);

  const result = document.createElement("textarea");
  result.className = "transcript";
  result.readOnly = true;
  result.placeholder = "Your transcript will appear here.";
  result.style.display = "none";
  panel.appendChild(result);

  const actions = el("div", "transcript-actions");
  actions.style.display = "none";
  const copyBtn = document.createElement("button");
  copyBtn.className = "secondary";
  copyBtn.type = "button";
  copyBtn.textContent = "Copy text";
  copyBtn.addEventListener("click", () => {
    result.select();
    void navigator.clipboard?.writeText(result.value).catch(() => {});
    copyBtn.textContent = "Copied ✓";
    setTimeout(() => (copyBtn.textContent = "Copy text"), 1500);
  });
  actions.appendChild(copyBtn);
  panel.appendChild(actions);

  // Storage row — shows the downloaded model's size with a one-click Remove, so
  // space can be reclaimed anytime (knowledge/18, the floor user's tight disk).
  const storage = el("div", "storage");
  storage.style.display = "none";
  panel.appendChild(storage);

  const refreshStorage = async () => {
    const models = await listModels();
    const whisper = models.find((m) => m.id === WHISPER_MODEL_ID);
    if (!whisper?.installed) {
      storage.style.display = "none";
      return;
    }
    storage.style.display = "flex";
    storage.innerHTML = `<span class="storage-label">Speech model on this computer · ${fmtMb(whisper.sizeBytes)}</span>`;
    const remove = document.createElement("button");
    remove.className = "link-danger";
    remove.type = "button";
    remove.textContent = "Remove";
    remove.addEventListener("click", async () => {
      remove.disabled = true;
      remove.textContent = "Removing…";
      try {
        await removeModel(whisper.id);
      } finally {
        await refreshStorage();
      }
    });
    storage.appendChild(remove);
  };

  root.appendChild(panel);
  void refreshStorage();

  // Predict the speed from this machine's specs (before any download).
  const showPrediction = async () => {
    const spec = await deviceSpec();
    const est = estimateTranscriptionSpeed(spec, "base");
    perf.textContent = predictionLine(est);
    perfReady = true;
    if (uploadArea.style.display !== "none") perf.style.display = "block";
  };
  void showPrediction();

  // --- Upload transcription (drop a file) ------------------------------------
  let busy = false;
  const run = async (path: string) => {
    if (busy) return;
    setMode("upload"); // a dropped file always belongs to the upload flow
    busy = true;
    zone.classList.add("working");
    result.style.display = "none";
    actions.style.display = "none";
    status.textContent = "Starting…";
    try {
      const r = await transcribeAudio(path, (msg) => (status.textContent = msg));
      const hasText = r.text.trim().length > 0;
      status.textContent = hasText ? "Done — transcript below." : "Done — but no speech was found.";
      result.value = r.text;
      result.style.display = "block";
      actions.style.display = hasText ? "flex" : "none";
      // Show the real measured speed alongside the estimate (predicted vs actual).
      if (r.audioSeconds > 0 && r.processingSeconds > 0) {
        measured.style.display = "block";
        measured.textContent = measuredLine(r.audioSeconds, r.processingSeconds);
      }
    } catch (err) {
      status.textContent = String(err);
      zone.classList.add("failed");
    } finally {
      busy = false;
      zone.classList.remove("working");
      void refreshStorage(); // the model now exists — show its size + Remove
    }
  };

  // --- Live transcription (record from the mic) ------------------------------
  let recording = false;
  let seconds = 0;
  let timer: number | undefined;
  const drawTimer = () => {
    const t = recIndicator.querySelector(".rec-time");
    if (t) t.textContent = fmtDuration(seconds);
  };

  // Reset the UI once a session has fully ended (driven by live:stopped/error).
  const finalize = () => {
    recording = false;
    recordBtn.disabled = false;
    recordBtn.textContent = "🎙️ Start recording";
    recordBtn.classList.remove("recording");
    recIndicator.style.display = "none";
    if (timer) {
      clearInterval(timer);
      timer = undefined;
    }
    if (liveCleanup) {
      liveCleanup();
      liveCleanup = null;
    }
    void refreshStorage();
  };

  const startRecording = async () => {
    if (recording) return;
    recording = true;
    recordBtn.textContent = "■ Stop recording";
    recordBtn.classList.add("recording");
    result.value = "";
    result.style.display = "block";
    actions.style.display = "none";
    status.textContent = "Starting…";
    recIndicator.style.display = "inline-flex";
    seconds = 0;
    drawTimer();
    timer = window.setInterval(() => {
      seconds += 1;
      drawTimer();
    }, 1000);
    try {
      liveCleanup = await startLiveTranscription({
        onStatus: (msg) => (status.textContent = msg),
        onSegment: (text) => {
          const t = text.trim();
          if (!t) return;
          result.value = result.value ? `${result.value} ${t}` : t;
          result.scrollTop = result.scrollHeight;
          actions.style.display = "flex";
        },
        onError: (msg) => {
          status.textContent = msg;
          finalize();
        },
        onStopped: () => {
          status.textContent = result.value.trim()
            ? "Stopped — transcript above."
            : "Stopped — no speech captured.";
          finalize();
        },
      });
    } catch (err) {
      status.textContent = String(err);
      finalize();
    }
  };

  recordBtn.addEventListener("click", () => {
    if (recording) {
      // Ask the backend to finish; the final segment + live:stopped drive finalize.
      status.textContent = "Finishing up…";
      recordBtn.disabled = true;
      void stopLiveTranscription();
    } else {
      void startRecording();
    }
  });

  // --- Mode switching --------------------------------------------------------
  function setMode(mode: "upload" | "live"): void {
    // Leaving live mode while recording must release the mic.
    if (mode !== "live" && recording) void stopLiveTranscription();
    uploadTab.classList.toggle("active", mode === "upload");
    liveTab.classList.toggle("active", mode === "live");
    uploadArea.style.display = mode === "upload" ? "block" : "none";
    liveArea.style.display = mode === "live" ? "flex" : "none";
    perf.style.display = mode === "upload" && perfReady ? "block" : "none";
    measured.style.display = "none";
    result.value = "";
    result.style.display = "none";
    actions.style.display = "none";
    status.textContent =
      mode === "live" && !inTauri ? "Live recording runs in the desktop app." : "";
  }
  uploadTab.addEventListener("click", () => setMode("upload"));
  liveTab.addEventListener("click", () => setMode("live"));
  setMode("upload");

  if (inTauri) {
    onAudioDrop(
      (paths) => {
        const audio = paths.find((p) => AUDIO_EXTS.includes(p.split(".").pop()?.toLowerCase() ?? ""));
        if (audio) void run(audio);
        else status.textContent = "That doesn't look like an audio file — try an mp3, m4a, or wav.";
      },
      (hovering) => zone.classList.toggle("hover", hovering),
    ).then((un) => (dropUnlisten = un));
  } else if (uploadArea.style.display !== "none") {
    status.textContent = "Drag-and-drop transcription runs in the desktop app.";
  }
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

function tabButton(label: string): HTMLButtonElement {
  const b = document.createElement("button");
  b.className = "tab";
  b.type = "button";
  b.textContent = label;
  return b;
}

function brandBar(): HTMLElement {
  const bar = el("div", "brand");
  bar.appendChild(el("span", "brand-dot", ""));
  bar.appendChild(el("span", "brand-name", "Verdure"));
  return bar;
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
  renderHome(); // paint immediately; detection refines the tier in the background
  try {
    tier = await detectDevice();
  } catch {
    // keep the safe default; detection is an optimization, never a blocker
  }
}

void boot();
