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
  transcribeAudio,
  onAudioDrop,
  inTauri,
  type PortOutcome,
} from "./bridge";
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
function clearDrop(): void {
  if (dropUnlisten) {
    dropUnlisten();
    dropUnlisten = null;
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
    if (WRAPPED.has(classKey(card.classification))) renderTranscribe();
    else renderRecommendation(card);
  });
  return btn;
}

// --- Screen 2a: transcription, wrapped IN-APP (the pivot, knowledge/18) ------
function renderTranscribe(): void {
  clearDrop();
  root.innerHTML = "";
  root.appendChild(brandBar());
  root.appendChild(backButton(renderHome));

  const panel = el("section", "panel");
  panel.appendChild(el("h1", "title", "Turn a recording into text"));
  panel.appendChild(
    el(
      "p",
      "subtitle",
      "Drop an audio file below. It's transcribed right on your computer — private, free, no account. The first time, Verdure downloads a small speech model (~140 MB).",
    ),
  );

  const zone = el("div", "dropzone");
  zone.innerHTML = `<span class="dropzone-emoji" aria-hidden="true">🎙️</span>
    <span class="dropzone-text">Drop an audio file here</span>
    <span class="dropzone-hint">mp3, m4a, wav, and more</span>`;
  panel.appendChild(zone);

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

  root.appendChild(panel);

  let busy = false;
  const run = async (path: string) => {
    if (busy) return;
    busy = true;
    zone.classList.add("working");
    result.style.display = "none";
    actions.style.display = "none";
    status.textContent = "Starting…";
    try {
      const text = await transcribeAudio(path, (msg) => (status.textContent = msg));
      status.textContent = text.trim() ? "Done — transcript below." : "Done — but no speech was found.";
      result.value = text;
      result.style.display = "block";
      actions.style.display = text.trim() ? "flex" : "none";
    } catch (err) {
      status.textContent = String(err);
      zone.classList.add("failed");
    } finally {
      busy = false;
      zone.classList.remove("working");
    }
  };

  if (inTauri) {
    onAudioDrop(
      (paths) => {
        const audio = paths.find((p) => AUDIO_EXTS.includes(p.split(".").pop()?.toLowerCase() ?? ""));
        if (audio) void run(audio);
        else status.textContent = "That doesn't look like an audio file — try an mp3, m4a, or wav.";
      },
      (hovering) => zone.classList.toggle("hover", hovering),
    ).then((un) => (dropUnlisten = un));
  } else {
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
