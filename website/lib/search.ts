// The web-search relay's pure half (knowledge/24): validate the query, build
// the provider request, and reduce the provider's reply to the few compact
// snippets the phone's small model can actually read. The route handler in
// app/api/search/route.ts is the thin I/O shell around these.
//
// Everything here is about keeping two promises. To the user: only their
// approved words go upstream, and only short snippets come back (a few KB, so
// a search costs nothing noticeable on mobile data). To ourselves: the relay
// never logs a query — nothing in this file prints, and the route prints only
// upstream status codes.

/** One result, exactly as the app's `web.rs::Snippet` deserializes it. */
export type Snippet = {
  title: string;
  url: string;
  domain: string;
  date: string | null;
  text: string;
};

/** Matches the app's cap (web.rs MAX_QUERY_CHARS): anything longer is not a
 *  search, it is a pasted document. */
export const MAX_QUERY_CHARS = 200;
/** At most this many results, matching the app's MAX_SOURCES. */
export const MAX_RESULTS = 5;
/** Per-snippet and whole-reply character caps. ~3 chars per token: a 600-token
 *  budget (the app's largest) is ~1,800 chars across all snippets. */
const CHARS_PER_TOKEN = 3;
const MAX_SNIPPET_CHARS = 700;
/** Brave's LLM Context endpoint accepts 1024–32768 for its token cap; we ask
 *  for the floor and trim to the phone's real budget ourselves. */
const BRAVE_CONTEXT_MIN_TOKENS = 1024;

const BRAVE_CONTEXT_URL = 'https://api.search.brave.com/res/v1/llm/context';
const BRAVE_WEB_URL = 'https://api.search.brave.com/res/v1/web/search';

/** The query, cleaned and bounded, or `null` when the body isn't a search. */
export function validateQuery(q: unknown): string | null {
  if (typeof q !== 'string') return null;
  const cleaned = q.replace(/\s+/g, ' ').trim();
  if (!cleaned) return null;
  return Array.from(cleaned).slice(0, MAX_QUERY_CHARS).join('').trim();
}

/** The phone's snippet budget in tokens, bounded so a bad client can't ask for
 *  a novel. Anything unparseable gets the smallest budget, not an error. */
export function clampBudgetTokens(n: unknown): number {
  const v = typeof n === 'number' && Number.isFinite(n) ? Math.floor(n) : 0;
  return Math.min(600, Math.max(120, v));
}

/** Brave's LLM Context request: extracted, ranked text chunks per page. No
 *  location, no language hint, no user identifiers — the query and a cap. */
export function braveContextUrl(query: string): string {
  const u = new URL(BRAVE_CONTEXT_URL);
  u.searchParams.set('q', query);
  u.searchParams.set('maximum_number_of_tokens', String(BRAVE_CONTEXT_MIN_TOKENS));
  return u.toString();
}

/** Brave's plain web search: the fallback when LLM Context isn't available on
 *  the plan or returns an error. Descriptions are shorter but well-formed. */
export function braveWebUrl(query: string): string {
  const u = new URL(BRAVE_WEB_URL);
  u.searchParams.set('q', query);
  u.searchParams.set('count', String(MAX_RESULTS));
  u.searchParams.set('text_decorations', 'false');
  return u.toString();
}

/** The headers every upstream call sends — the key and nothing about the user. */
export function braveHeaders(apiKey: string): Record<string, string> {
  return {
    Accept: 'application/json',
    'Accept-Encoding': 'gzip',
    'X-Subscription-Token': apiKey,
  };
}

export function domainOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

/** Brave returns descriptions with light HTML (`<strong>`); the model wants text. */
export function stripTags(s: string): string {
  return s
    .replace(/<[^>]*>/g, '')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;|&#x27;/g, "'")
    .replace(/\s+/g, ' ')
    .trim();
}

type Json = Record<string, unknown>;
const obj = (v: unknown): Json | null => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Json) : null);
const str = (v: unknown): string => (typeof v === 'string' ? v : '');

/**
 * Reduce an LLM Context reply. The documented shape is `grounding.generic[]`
 * with `url`, `title` and `snippets[]` (text chunks); this reads that, and
 * tolerates chunks given as `{text}` objects and a flat `results[]` — the
 * schema was assembled from Brave's skill docs, not a live call, and the first
 * real reply decides (knowledge/24, [OQ-90]). Unknown shapes yield `[]`, and
 * the route then falls back to plain web search.
 */
export function snippetsFromContext(json: unknown): Snippet[] {
  const root = obj(json);
  if (!root) return [];
  const grounding = obj(root.grounding);
  const items = (Array.isArray(grounding?.generic) ? grounding!.generic : Array.isArray(root.results) ? root.results : []) as unknown[];
  const out: Snippet[] = [];
  for (const raw of items) {
    const it = obj(raw);
    if (!it) continue;
    const url = str(it.url);
    const chunks = Array.isArray(it.snippets) ? it.snippets : Array.isArray(it.chunks) ? it.chunks : [];
    const text = stripTags(
      chunks
        .map((c) => (typeof c === 'string' ? c : str(obj(c)?.text)))
        .filter(Boolean)
        .join(' '),
    );
    if (!url || !text) continue;
    out.push({
      title: stripTags(str(it.title)) || domainOf(url),
      url,
      domain: domainOf(url),
      date: str(it.date) || str(it.page_age) || str(it.age) || null,
      text,
    });
  }
  return out;
}

/** Reduce a plain web-search reply (`web.results[]` with `description`). */
export function snippetsFromWeb(json: unknown): Snippet[] {
  const root = obj(json);
  const web = obj(root?.web);
  const items = (Array.isArray(web?.results) ? web!.results : []) as unknown[];
  const out: Snippet[] = [];
  for (const raw of items) {
    const it = obj(raw);
    if (!it) continue;
    const url = str(it.url);
    const text = stripTags([str(it.description), ...(Array.isArray(it.extra_snippets) ? it.extra_snippets.map(str) : [])].join(' '));
    if (!url || !text) continue;
    out.push({
      title: stripTags(str(it.title)) || domainOf(url),
      url,
      domain: domainOf(url),
      date: str(it.page_age) || str(it.age) || null,
      text,
    });
  }
  return out;
}

/**
 * Cut the reply to what the phone asked for: at most `MAX_RESULTS` snippets,
 * each at most `MAX_SNIPPET_CHARS`, and together within the token budget. The
 * straddling snippet is trimmed, not dropped — a short top result beats a
 * whole fifth one — and nothing empty survives.
 */
export function trimToBudget(snippets: Snippet[], budgetTokens: number): Snippet[] {
  let remaining = clampBudgetTokens(budgetTokens) * CHARS_PER_TOKEN;
  const out: Snippet[] = [];
  for (const s of snippets.slice(0, MAX_RESULTS)) {
    if (remaining < 80) break;
    const text = Array.from(s.text).slice(0, Math.min(MAX_SNIPPET_CHARS, remaining)).join('').trim();
    if (!text) continue;
    remaining -= text.length;
    out.push({ ...s, title: Array.from(s.title).slice(0, 120).join(''), text });
  }
  return out;
}
