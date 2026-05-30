// Intent classifier: keyword-based fallback for Phase 0 v1.
//
// The raw natural-language string the user types is always preserved by the
// caller; this function assigns a coarse (modality, task) classification used
// to pick the right family of models and install paths.
//
// Multimodal as of 2026: a request resolves to text / audio / image / video.
// Source of truth for the taxonomy: lib/modality.ts and
// verdure-orchestrator/knowledge/03-model-matrix.md.
//
// This is keyword-only and deliberately conservative — it would rather fall
// back to a sensible default than misfire. The LLM-based classifier ([OQ-07])
// will replace the body of classifyIntent while keeping the same return type,
// so callers don't change.

import type { Classification } from './modality';

// Rules are evaluated top to bottom; first match wins. Order matters: the most
// specific, least ambiguous signals come first, and the broad text fallbacks
// come last. Keyword lists are kept tight to avoid false positives — a missed
// match degrades to a reasonable default, but a wrong match misleads.
type Rule = { when: readonly string[]; then: Classification };

const RULES: readonly Rule[] = [
  // --- Audio: specific tasks before the generic "audio" catch ---
  {
    when: [
      'transcribe', 'transcription', 'transcript',
      'subtitle', 'subtitles', 'caption', 'captions',
      'dictation', 'dictate', 'speech to text', 'audio to text',
      'voice memo', 'voice note', 'voice notes', 'meeting notes',
    ],
    then: { modality: 'audio', task: 'transcription' },
  },
  {
    when: [
      'text to speech', 'tts', 'voiceover', 'voice over', 'voice-over',
      'read aloud', 'narrate', 'narration', 'audiobook', 'spoken voice',
      'clone voice', 'voice clone', 'synthetic voice',
    ],
    then: { modality: 'audio', task: 'speech' },
  },
  {
    when: [
      'stem', 'stems', 'isolate vocals', 'separate vocals', 'remove vocals',
      'karaoke', 'acapella', 'a cappella', 'instrumental track', 'split the song',
    ],
    then: { modality: 'audio', task: 'separation' },
  },
  {
    when: [
      'compose music', 'make music', 'making music', 'generate music',
      'background music', 'soundtrack', 'a beat', 'beats', 'melody',
      'instrumental', 'sound effect', 'sound effects',
    ],
    then: { modality: 'audio', task: 'music' },
  },

  // --- Image: specific tasks (upscale/edit) before generic generation ---
  {
    when: [
      'upscale', 'enlarge', 'increase resolution', 'higher resolution',
      'sharpen', 'sharper', 'clearer', 'restore photo', 'restore image',
      'enhance photo', 'enhance image', 'enhance picture',
    ],
    then: { modality: 'image', task: 'upscale' },
  },
  {
    when: [
      'edit photo', 'edit image', 'edit picture', 'photo edit', 'retouch',
      'inpaint', 'remove background', 'remove object', 'remove the background',
      'photoshop',
    ],
    then: { modality: 'image', task: 'edit' },
  },

  // --- Video: enhancement before generic "video" (which defaults to create) ---
  {
    when: [
      'upscale video', 'enhance video', 'interpolate', 'frame rate', 'fps',
      'slow motion', 'slow-mo', 'slow mo', 'restore video', 'smoother video',
    ],
    then: { modality: 'video', task: 'enhance' },
  },
  {
    when: [
      'video', 'animation', 'animate', 'text to video', 'short film',
      'movie clip', 'video clip',
    ],
    then: { modality: 'video', task: 'generate' },
  },

  // --- Image generation (after edit/upscale so "edit a photo" isn't caught) ---
  {
    when: [
      'image', 'picture', 'pictures', 'photo', 'photos', 'illustration',
      'illustrate', 'drawing', 'draw me', 'artwork', 'logo', 'wallpaper',
      'concept art', 'generate an image', 'text to image',
    ],
    then: { modality: 'image', task: 'generate' },
  },

  // --- Generic audio fallback: a recording with no specific verb is almost
  //     always "I want the words out of this" — transcription is the most
  //     useful and most device-feasible default. ---
  {
    when: ['audio', 'recording', 'record ', 'podcast', 'interview', 'lecture'],
    then: { modality: 'audio', task: 'transcription' },
  },

  // --- Text: existing priority order (code, then creative, then multilingual) ---
  {
    when: [
      'code', 'coding', 'coder', 'codebase',
      'debug', 'debugging', 'bug', 'bugs', 'fix',
      'programming', 'program', 'script', 'developer', 'engineer',
      'rust', 'python', 'javascript', 'typescript', 'java', 'c++', 'go ',
      'sql', 'html', 'css', 'react', 'function',
    ],
    then: { modality: 'text', task: 'code' },
  },
  {
    when: [
      'write', 'writing', 'wrote', 'writer',
      'story', 'stories', 'novel', 'novels', 'fiction', 'fantasy',
      'poem', 'poetry', 'lyrics', 'song', 'songwriting',
      'creative', 'brainstorm', 'blog', 'essay', 'screenplay',
    ],
    then: { modality: 'text', task: 'creative' },
  },
  {
    when: [
      'spanish', 'french', 'german', 'mandarin', 'chinese', 'japanese',
      'korean', 'arabic', 'hindi', 'portuguese', 'italian', 'dutch',
      'translate', 'translation', 'translating',
      'language', 'languages', 'multilingual', 'bilingual',
    ],
    then: { modality: 'text', task: 'multilingual' },
  },
];

const DEFAULT: Classification = { modality: 'text', task: 'general' };

export function classifyIntent(text: string): Classification {
  const lower = text.toLowerCase();
  for (const rule of RULES) {
    if (rule.when.some((kw) => lower.includes(kw))) {
      return rule.then;
    }
  }
  return DEFAULT;
}
