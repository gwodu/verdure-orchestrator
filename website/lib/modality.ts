// The modality taxonomy — the vocabulary shared by the classifier (intent.ts)
// and the recommendation engine (recommend.ts).
//
// Verdure started text-LLM-only; this file is where it becomes multimodal.
// A user's request resolves to a (modality, task) pair. The modality decides
// which family of models and install paths apply; the task picks within it.
//
// Source of truth for what's recommendable per modality/tier:
// verdure-orchestrator/knowledge/03-model-matrix.md (the multimodal matrix).

export type Modality = 'text' | 'audio' | 'image' | 'video';

// Tasks are scoped per modality. Keeping them as a discriminated union (rather
// than one flat enum) lets callers switch exhaustively and keeps an audio task
// from ever being handed to the image path.
export type TextTask = 'creative' | 'code' | 'multilingual' | 'general';
export type AudioTask = 'transcription' | 'speech' | 'separation' | 'music';
export type ImageTask = 'generate' | 'edit' | 'upscale';
export type VideoTask = 'generate' | 'enhance';

export type Classification =
  | { modality: 'text'; task: TextTask }
  | { modality: 'audio'; task: AudioTask }
  | { modality: 'image'; task: ImageTask }
  | { modality: 'video'; task: VideoTask };

// A plain-language, verb-first label for a classification — what the user is
// trying to *do*, in words a non-technical person uses. Used in headlines.
// No jargon (see verdure-orchestrator/skills/design.md, "Assume zero technical
// knowledge"): lead with the action, never the technology.
export function taskLabel(c: Classification): string {
  switch (c.modality) {
    case 'text':
      return {
        creative: 'writing and ideas',
        code: 'coding help',
        multilingual: 'languages and translation',
        general: 'everyday questions',
      }[c.task];
    case 'audio':
      return {
        transcription: 'turning speech into text',
        speech: 'turning writing into a spoken voice',
        separation: 'splitting a song into vocals and music',
        music: 'making music',
      }[c.task];
    case 'image':
      return {
        generate: 'creating images',
        edit: 'editing images',
        upscale: 'sharpening and enlarging images',
      }[c.task];
    case 'video':
      return {
        generate: 'creating video',
        enhance: 'improving video',
      }[c.task];
  }
}

// A user-facing intent card for Screen 1 — what the user wants to do, in plain
// words, mirroring the device cards. Picking one feeds its `example` phrase into
// the flow; the keyword classifier (lib/intent.ts) maps that phrase to
// `classification`, locked by a unit test so cards and classifier can't drift.
// The set and ordering are grounded in the demand research — see
// verdure-orchestrator/knowledge/13-use-case-demand.md. (Notably: video
// generation is deliberately NOT a card — lowest demand, least local-feasible —
// but stays reachable via the free-text fallback.)
export type IntentCard = {
  id: string;
  emoji: string;
  title: string;
  description: string;
  example: string;
  classification: Classification;
};

export const INTENT_CARDS: IntentCard[] = [
  {
    id: 'everyday',
    emoji: '💬',
    title: 'Everyday questions & chat',
    description: 'Ask anything, get advice, think things through.',
    example: 'answer everyday questions and help me think things through',
    classification: { modality: 'text', task: 'general' },
  },
  {
    id: 'writing',
    emoji: '📝',
    title: 'Writing & ideas',
    description: 'Stories, blog posts, emails, brainstorming.',
    example: 'write stories and brainstorm ideas',
    classification: { modality: 'text', task: 'creative' },
  },
  {
    id: 'coding',
    emoji: '💻',
    title: 'Coding & tech help',
    description: 'Write, fix, and understand code.',
    example: 'help me write and debug code',
    classification: { modality: 'text', task: 'code' },
  },
  {
    id: 'transcribe',
    emoji: '🎙️',
    title: 'Turn recordings into text',
    description: 'Transcribe meetings, voice notes, and interviews.',
    example: 'transcribe my meetings and voice recordings',
    classification: { modality: 'audio', task: 'transcription' },
  },
  {
    id: 'images',
    emoji: '🎨',
    title: 'Create images',
    description: 'Make pictures, art, and logos from a description.',
    example: 'create images from a description',
    classification: { modality: 'image', task: 'generate' },
  },
  {
    id: 'languages',
    emoji: '🌍',
    title: 'Languages & translation',
    description: 'Translate text, and practice another language.',
    example: 'translate and practice another language',
    classification: { modality: 'text', task: 'multilingual' },
  },
];

export function findIntentCard(id: string): IntentCard | undefined {
  return INTENT_CARDS.find((c) => c.id === id);
}
