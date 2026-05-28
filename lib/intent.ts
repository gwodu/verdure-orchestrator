// Intent classifier: keyword-based fallback for Phase 0 v1.
// The natural-language intent string the user types is always preserved
// raw; this function only assigns a coarse category used to shift the
// model recommendation within a hardware tier.
//
// Source of truth for intent categories: verdure-orchestrator/knowledge/03-model-matrix.md
// ("Intent shifts the recommendation within a tier"). When the LLM-based
// classifier ([OQ-07]) is wired in, replace this function but keep the same
// Intent type so callers don't change.

export type Intent = 'creative' | 'code' | 'multilingual' | 'general';

const KEYWORDS: Record<Exclude<Intent, 'general'>, readonly string[]> = {
  creative: [
    'write', 'writing', 'wrote', 'writer',
    'story', 'stories', 'novel', 'novels', 'fiction', 'fantasy',
    'poem', 'poetry', 'lyrics', 'song',
    'creative', 'brainstorm', 'blog', 'essay', 'screenplay',
  ],
  code: [
    'code', 'coding', 'coder', 'codebase',
    'debug', 'debugging', 'bug', 'bugs', 'fix',
    'programming', 'program', 'script', 'developer', 'engineer',
    'rust', 'python', 'javascript', 'typescript', 'java', 'c++', 'go ',
    'sql', 'html', 'css', 'react', 'function',
  ],
  multilingual: [
    'spanish', 'french', 'german', 'mandarin', 'chinese', 'japanese',
    'korean', 'arabic', 'hindi', 'portuguese', 'italian', 'dutch',
    'translate', 'translation', 'translating',
    'language', 'languages', 'multilingual', 'bilingual',
  ],
};

const PRIORITY: Exclude<Intent, 'general'>[] = ['code', 'creative', 'multilingual'];

export function classifyIntent(text: string): Intent {
  const lower = text.toLowerCase();
  for (const intent of PRIORITY) {
    if (KEYWORDS[intent].some((kw) => lower.includes(kw))) {
      return intent;
    }
  }
  return 'general';
}
