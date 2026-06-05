// Recommendation engine — multimodal. PRESENTATION layer.
//
// The DECISION (which model, and whether it's feasible) lives in lib/matrix.ts
// (the single source of truth, mirrored by the future Rust ModelMatcher). This
// file wraps each Decision in the non-technical copy + install paths a person
// can act on. Three outcomes:
//   - recommended    → this runs well on your device; here's the tool.
//   - works-but-slow → it'll run, but be patient.
//   - not-local-yet  → honestly, your device can't do this on its own yet.
//
// Copy is verb-first and jargon-free (see verdure-orchestrator/skills/design.md,
// "Assume zero technical knowledge"): the headline says what the user will be
// able to *do*; the model name is the labeled answer, not buried.

import type { Tier } from './tiers';
import { taskLabel, type Classification, type Modality, type TextTask } from './modality';
import { decide, deviceClass, type DeviceClass, type Decision } from './matrix';

export type { Feasibility } from './matrix';
import type { Feasibility } from './matrix';

// An install path the user can actually follow — a friendly app wherever one
// exists, a command only where the audience is expected to be technical.
export type InstallOption = {
  name: string;
  detail: string;
  url: string;
  platforms?: string;
  command?: string;
};

export type Alternative = { name: string; reason: string };

// The underlying model/tool, shown as the labeled answer with a learn-more link.
export type ModelInfo = { name: string; infoUrl?: string; note?: string };

export type Recommendation = {
  modality: Modality;
  taskLabel: string;
  feasibility: Feasibility;
  headline: string; // verb-first, jargon-free, no model name
  body: string; // honest expectation in human terms
  model: ModelInfo | null; // null only when not-local-yet
  installOptions: InstallOption[];
  alternatives: Alternative[];
};

// --- Shared install paths ---------------------------------------------------
const COMFYUI: InstallOption = {
  name: 'ComfyUI',
  detail: 'A free app for advanced users. More setup, but it runs everything.',
  url: 'https://github.com/comfyanonymous/ComfyUI',
  platforms: 'Mac · Windows · Linux',
};

// ===========================================================================
// Entry point — get the decision from the matrix, then dress it for a person.
// ===========================================================================
export function recommend(tier: Tier, c: Classification): Recommendation {
  const label = taskLabel(c);
  const decision = decide(tier, c);
  const dc = deviceClass(tier);
  switch (c.modality) {
    case 'text':
      return recommendText(tier, c.task, label, decision);
    case 'audio':
      return recommendAudio(dc, c.task, label, decision);
    case 'image':
      return recommendImage(dc, c.task, label, decision);
    case 'video':
      return recommendVideo(dc, c.task, label, decision);
  }
}

// ===========================================================================
// Text
// ===========================================================================
const TEXT_INFO: Record<string, string> = {
  'Gemma 3 1B': 'https://ai.google.dev/gemma',
  'Gemma 3 4B': 'https://ai.google.dev/gemma',
  'Llama 3.2 3B': 'https://www.llama.com/',
  'Llama 3.1 8B': 'https://www.llama.com/',
  'Phi-4-mini': 'https://huggingface.co/microsoft/Phi-4-mini-instruct',
  'Qwen 2.5 1.5B': 'https://qwenlm.github.io/',
  'Qwen 2.5 3B': 'https://qwenlm.github.io/',
  'Mistral Small': 'https://mistral.ai/news/mistral-small/',
};

const OLLAMA_TAGS: Record<string, string> = {
  'Gemma 3 1B': 'gemma3:1b',
  'Gemma 3 4B': 'gemma3:4b',
  'Llama 3.2 3B': 'llama3.2:3b',
  'Llama 3.1 8B': 'llama3.1:8b',
  'Phi-4-mini': 'phi4-mini',
  'Qwen 2.5 1.5B': 'qwen2.5:1.5b',
  'Qwen 2.5 3B': 'qwen2.5:3b',
  'Mistral Small': 'mistral-small',
};

// The three text install paths, ranked easiest → most technical. The Verdure
// waitlist (the "we'll do all this for you" CTA) is rendered separately by the
// page, independent of modality.
function textInstallOptions(modelName: string): InstallOption[] {
  const tag = OLLAMA_TAGS[modelName] ?? modelName.toLowerCase().replace(/\s+/g, '-');
  return [
    {
      name: 'LM Studio',
      detail: 'A free app. Open it, search for the model, click install.',
      url: 'https://lmstudio.ai',
      platforms: 'Mac · Windows · Linux',
    },
    {
      name: 'Jan',
      detail: 'Another free app that works the same way.',
      url: 'https://jan.ai',
      platforms: 'Mac · Windows · Linux',
    },
    {
      name: 'Ollama',
      detail: 'For the comfortable: one command in the terminal.',
      url: 'https://ollama.com',
      platforms: 'Mac · Windows · Linux',
      command: `ollama run ${tag}`,
    },
  ];
}

type TextCopy = { headline: string; body: string; alternatives: Alternative[] };

function recommendText(tier: Tier, task: TextTask, label: string, decision: Decision): Recommendation {
  const model = decision.model!; // text always resolves to a model
  const copy = textCopy(tier, task);
  return {
    modality: 'text',
    taskLabel: label,
    feasibility: decision.feasibility,
    headline: copy.headline,
    body: copy.body,
    model: { name: model, infoUrl: TEXT_INFO[model], note: 'Free. Runs on your computer.' },
    installOptions: textInstallOptions(model),
    alternatives: copy.alternatives,
  };
}

// Copy only — the model name comes from the matrix. Same branch structure as
// the matrix's decideText so the wording stays matched to the pick.
function textCopy(tier: Tier, task: TextTask): TextCopy {
  switch (tier) {
    case 'L1':
      return {
        headline: 'Get a capable AI assistant on your laptop.',
        body: 'A small, fast helper for everyday questions, writing, and ideas. It fits comfortably and leaves room for your other apps.',
        alternatives: [
          { name: 'Qwen 2.5 1.5B', reason: 'A bit larger — better if you mainly work in other languages.' },
        ],
      };
    case 'L2-low':
      if (task === 'multilingual') {
        return {
          headline: 'Work across languages, right on your laptop.',
          body: 'Strong in 35+ languages and quick on your machine.',
          alternatives: [{ name: 'Llama 3.2 3B', reason: 'Better all-round if you mostly work in English.' }],
        };
      }
      return {
        headline: 'Get a strong all-round AI assistant on your laptop.',
        body: 'A capable general helper that fits comfortably on your machine.',
        alternatives: [{ name: 'Qwen 2.5 1.5B', reason: 'Lighter and faster — better for other languages.' }],
      };
    case 'L2-high':
      if (task === 'creative') {
        return {
          headline: 'Write and brainstorm with an AI partner on your laptop.',
          body: 'Punches above its weight on creative writing and ideas.',
          alternatives: [{ name: 'Llama 3.2 3B', reason: 'A little faster, still strong for general use.' }],
        };
      }
      if (task === 'code') {
        return {
          headline: 'Get coding help on your laptop.',
          body: 'Strong at reasoning through technical problems.',
          alternatives: [{ name: 'Llama 3.2 3B', reason: 'Better for general chat alongside code questions.' }],
        };
      }
      if (task === 'multilingual') {
        return {
          headline: 'Work across languages, right on your laptop.',
          body: 'Strong in 35+ languages and comfortable on your machine.',
          alternatives: [{ name: 'Gemma 3 4B', reason: 'Slower, but better if you also do creative writing.' }],
        };
      }
      return {
        headline: 'Get a fast, strong AI assistant on your laptop.',
        body: 'Quick and capable for chat and everyday questions.',
        alternatives: [
          { name: 'Phi-4-mini', reason: 'Better for technical questions.' },
          { name: 'Gemma 3 4B', reason: 'A little slower, better for longer creative writing.' },
        ],
      };
    case 'L3':
    case 'L3+':
      if (task === 'code') {
        return {
          headline: 'Get powerful coding help on your computer.',
          body: 'Strong reasoning for code and technical questions.',
          alternatives: [{ name: 'Mistral Small', reason: 'Comparable quality for general tasks.' }],
        };
      }
      return {
        headline: 'Get a powerful AI assistant on your computer.',
        body: 'Excellent all-round performance on capable machines.',
        alternatives: [{ name: 'Llama 3.1 8B', reason: 'A little larger — strong for technical work.' }],
      };
  }
}

// ===========================================================================
// Audio — the strongest local story: transcription & speech run on any laptop.
// ===========================================================================
function recommendAudio(dc: DeviceClass, task: string, label: string, decision: Decision): Recommendation {
  const { feasibility, model } = decision;
  switch (task) {
    case 'transcription':
      return {
        modality: 'audio',
        taskLabel: label,
        feasibility,
        headline: 'Turn recordings into text, right on your computer.',
        body: 'Works on any laptop — no internet and no graphics card needed. Your audio never leaves your machine.',
        model: {
          name: model!,
          infoUrl: 'https://github.com/openai/whisper',
          note: dc === 'cpu-laptop' ? 'Free. Runs on your laptop.' : 'Free. Fast on your computer.',
        },
        installOptions: [
          {
            name: 'MacWhisper',
            detail: 'A friendly Mac app. Drag in a file, get the text back.',
            url: 'https://goodsnooze.com/macwhisper/',
            platforms: 'Mac',
          },
          {
            name: 'Buzz',
            detail: 'A free app that turns audio and video into text.',
            url: 'https://github.com/chidiwilliams/buzz',
            platforms: 'Windows · Mac · Linux',
          },
        ],
        alternatives: [],
      };
    case 'speech':
      return {
        modality: 'audio',
        taskLabel: label,
        feasibility,
        headline: 'Turn your writing into a natural spoken voice.',
        body: 'Runs on any laptop, no graphics card needed. Great for narration, audiobooks, and read-alouds.',
        model: {
          name: model!,
          infoUrl: 'https://huggingface.co/hexgrad/Kokoro-82M',
          note: 'Free. Tiny and fast, even on older laptops.',
        },
        installOptions: [
          {
            name: 'Kokoro',
            detail: 'A free, natural-sounding voice you can run on your computer.',
            url: 'https://huggingface.co/hexgrad/Kokoro-82M',
            platforms: 'Mac · Windows · Linux',
          },
          {
            name: 'Piper',
            detail: 'Another free voice tool — very light, runs almost anywhere.',
            url: 'https://github.com/OHF-Voice/piper1-gpl',
            platforms: 'Mac · Windows · Linux',
          },
        ],
        alternatives: [],
      };
    case 'separation':
      return {
        modality: 'audio',
        taskLabel: label,
        feasibility,
        headline: 'Split a song into vocals and music, on your computer.',
        body:
          dc === 'cpu-laptop'
            ? 'This works on your laptop, but each song takes several minutes. A computer with a graphics card is much faster.'
            : 'Pull a track apart into vocals, drums, and more — quickly on your machine.',
        model: {
          name: model!,
          infoUrl: 'https://ultimatevocalremover.com/',
          note: 'Free. A simple app — no setup commands.',
        },
        installOptions: [
          {
            name: 'Ultimate Vocal Remover',
            detail: 'A free app. Drop in a song, get separate vocal and music files.',
            url: 'https://ultimatevocalremover.com/',
            platforms: 'Windows · Mac · Linux',
          },
        ],
        alternatives: [],
      };
    case 'music':
    default:
      if (feasibility === 'not-local-yet') {
        return notLocalYet(
          'audio',
          label,
          'Making music with AI needs a graphics card.',
          'Your laptop can record, transcribe, and narrate audio beautifully — but generating music from scratch needs a computer with a dedicated graphics card. On this device it isn’t practical yet.',
        );
      }
      return {
        modality: 'audio',
        taskLabel: label,
        feasibility,
        headline: 'Generate music and sound on your computer.',
        body:
          dc === 'apple-silicon'
            ? 'This runs on your Mac, but expect a wait per clip. It’s best for short sounds and samples.'
            : 'Create short pieces of music and sound effects on your machine.',
        model: {
          name: model!,
          infoUrl: 'https://huggingface.co/stabilityai/stable-audio-open-1.0',
          note: 'Free for most uses. Best for short clips, not full songs.',
        },
        installOptions: [COMFYUI],
        alternatives: [],
      };
  }
}

// ===========================================================================
// Image — needs graphics power. Upscaling is the exception that runs anywhere.
// ===========================================================================
function recommendImage(dc: DeviceClass, task: string, label: string, decision: Decision): Recommendation {
  const { feasibility, model } = decision;

  if (task === 'upscale') {
    return {
      modality: 'image',
      taskLabel: label,
      feasibility,
      headline: 'Sharpen and enlarge your photos, on your computer.',
      body: 'Runs on any laptop. Make small or blurry images bigger and clearer.',
      model: {
        name: model!,
        infoUrl: 'https://upscayl.org/',
        note: 'Free. A simple app — drop a photo in, get a sharper one out.',
      },
      installOptions: [
        {
          name: 'Upscayl',
          detail: 'A free app for making images larger and clearer.',
          url: 'https://upscayl.org/',
          platforms: 'Windows · Mac · Linux',
        },
      ],
      alternatives: [],
    };
  }

  // Generate / edit both need real graphics power.
  if (feasibility === 'not-local-yet') {
    return notLocalYet(
      'image',
      label,
      'Creating images needs a graphics card.',
      'Making images with AI needs a computer with a dedicated graphics card, or a newer Apple-chip Mac. On this laptop it would be far too slow to be useful. (Sharpening and enlarging existing photos, though, works fine here.)',
    );
  }

  const onMac = dc === 'apple-silicon';
  if (task === 'edit') {
    return {
      modality: 'image',
      taskLabel: label,
      feasibility,
      headline: 'Edit images by just describing the change.',
      body: onMac
        ? 'Runs on your Mac. Add, remove, or change parts of a picture in plain words. Bigger edits need a higher-memory Mac.'
        : 'Add, remove, or change parts of a picture just by describing it.',
      model: {
        name: model!,
        infoUrl: 'https://huggingface.co/Qwen',
        note: 'Free, including for commercial use.',
      },
      installOptions: onMac ? [drawThings()] : [fooocus(), COMFYUI],
      alternatives: [],
    };
  }
  // generate
  return {
    modality: 'image',
    taskLabel: label,
    feasibility,
    headline: 'Create images from a description, on your computer.',
    body: onMac
      ? 'Runs well on your Apple-chip Mac. Type what you want to see and get a picture back in seconds.'
      : 'Type what you want to see and get a picture back in seconds.',
    model: onMac
      ? { name: model!, infoUrl: 'https://drawthings.ai/', note: 'Free. Built into the Draw Things app.' }
      : { name: model!, infoUrl: 'https://huggingface.co/black-forest-labs/FLUX.1-schnell', note: 'Free, including for commercial use.' },
    installOptions: onMac ? [drawThings()] : [fooocus(), COMFYUI],
    alternatives: onMac
      ? []
      : [{ name: 'Stable Diffusion XL', reason: 'A well-established option with the easiest app (Fooocus).' }],
  };
}

function drawThings(): InstallOption {
  return {
    name: 'Draw Things',
    detail: 'A free Mac app. Type a description, get an image — all on your Mac.',
    url: 'https://drawthings.ai/',
    platforms: 'Mac',
  };
}

function fooocus(): InstallOption {
  return {
    name: 'Fooocus',
    detail: 'A free, simple app for creating images. Double-click and go.',
    url: 'https://github.com/lllyasviel/Fooocus',
    platforms: 'Windows · Linux',
  };
}

// ===========================================================================
// Video — the honest-decline modality. Generation needs serious graphics power.
// ===========================================================================
function recommendVideo(dc: DeviceClass, task: string, label: string, decision: Decision): Recommendation {
  const { feasibility, model } = decision;

  if (task === 'enhance') {
    return {
      modality: 'video',
      taskLabel: label,
      feasibility,
      headline: 'Make videos smoother and sharper, on your computer.',
      body:
        dc === 'cpu-laptop'
          ? 'Smoothing motion and sharpening video works on your laptop, but it takes time. A graphics card speeds it up a lot.'
          : 'Smooth out motion and increase the sharpness of your videos.',
      model: {
        name: model!,
        infoUrl: 'https://github.com/TNTwise/REAL-Video-Enhancer',
        note: 'Free. A simple app for upscaling and smoothing video.',
      },
      installOptions: [
        {
          name: 'REAL Video Enhancer',
          detail: 'A free app for sharpening and smoothing video.',
          url: 'https://github.com/TNTwise/REAL-Video-Enhancer',
          platforms: 'Windows · Linux · Mac',
        },
      ],
      alternatives: [],
    };
  }

  // generate
  if (feasibility === 'not-local-yet') {
    return notLocalYet(
      'video',
      label,
      'Creating video on your own device isn’t practical yet.',
      dc === 'apple-silicon'
        ? 'Making video with AI is the most demanding thing here — only the very highest-memory Macs can manage it, and even then slowly. For now, a cloud tool will serve you far better.'
        : 'Making video with AI needs a powerful, dedicated graphics card — far more than a laptop has. For now, a cloud tool will serve you far better. We’ll let you know when Verdure can help with this.',
    );
  }

  return {
    modality: 'video',
    taskLabel: label,
    feasibility,
    headline: 'Create short video clips on your computer.',
    body: 'Your graphics card can do this, but it’s slow and best left to comfortable users — expect minutes per few-second clip, and a fair bit of setup.',
    model: {
      name: model!,
      infoUrl: 'https://www.lightricks.com/ltxv',
      note: 'Free, including for commercial use.',
    },
    installOptions: [COMFYUI],
    alternatives: [{ name: 'Wan 2.2', reason: 'Higher quality, but heavier and slower.' }],
  };
}

// ===========================================================================
// Shared: the honest "not local yet" outcome.
// ===========================================================================
function notLocalYet(
  modality: Modality,
  label: string,
  headline: string,
  body: string,
): Recommendation {
  return {
    modality,
    taskLabel: label,
    feasibility: 'not-local-yet',
    headline,
    body,
    model: null,
    installOptions: [],
    alternatives: [],
  };
}
