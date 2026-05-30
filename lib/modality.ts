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
