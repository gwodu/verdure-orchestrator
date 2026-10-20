// The web-search relay (knowledge/24).
//
// The phone can't hold the search provider's key — anyone can lift a key out
// of an app — so the approved query comes here, this forwards it to Brave with
// our key, and the snippets go back. That is the whole job.
//
// The promise this file keeps (the app's disclosure text says exactly this):
//   "Verdure's relay forwards them to Brave and keeps no record of them."
// So: the query is read from the POST body (never the URL, which the host
// records), nothing here prints it, nothing stores it, and the reply is
// marked no-store. The host (Vercel) still records that a request hit this
// path from some IP; that is named in the disclosure, not hidden.
//
// Setup: BRAVE_SEARCH_API_KEY in the project's environment variables. Without
// it this answers 503 with a plain sentence, and the app says the web couldn't
// be checked — no crash, no silent failure.

import { NextResponse, type NextRequest } from 'next/server';
import {
  braveContextUrl,
  braveHeaders,
  braveWebUrl,
  clampBudgetTokens,
  snippetsFromContext,
  snippetsFromWeb,
  trimToBudget,
  validateQuery,
} from '@/lib/search';

export const dynamic = 'force-dynamic';

const NO_STORE = { 'Cache-Control': 'no-store' };
const UPSTREAM_TIMEOUT_MS = 8000;

export async function POST(request: NextRequest) {
  const apiKey = process.env.BRAVE_SEARCH_API_KEY;
  if (!apiKey) {
    return NextResponse.json({ error: "Web search isn't set up on the server yet." }, { status: 503, headers: NO_STORE });
  }

  let payload: { q?: unknown; maxTokens?: unknown };
  try {
    payload = await request.json();
  } catch {
    return NextResponse.json({ error: 'Body must be JSON' }, { status: 400, headers: NO_STORE });
  }
  const query = validateQuery(payload.q);
  if (!query) {
    return NextResponse.json({ error: 'Nothing to search for.' }, { status: 400, headers: NO_STORE });
  }
  const budget = clampBudgetTokens(payload.maxTokens);
  const headers = braveHeaders(apiKey);

  // LLM Context first (server-extracted chunks, the better grounding); plain
  // web search if that isn't available on the plan or answers with an error.
  let results = await fetchSnippets(braveContextUrl(query), headers, snippetsFromContext);
  if (results === null || results.length === 0) {
    results = await fetchSnippets(braveWebUrl(query), headers, snippetsFromWeb);
  }
  if (results === null) {
    return NextResponse.json({ error: "The search provider couldn't be reached right now." }, { status: 502, headers: NO_STORE });
  }
  return NextResponse.json({ query, results: trimToBudget(results, budget) }, { headers: NO_STORE });
}

/** One upstream call. `null` means the provider failed (status or network);
 *  an empty array means it answered with nothing usable. Logs the status code
 *  only — never the query. */
async function fetchSnippets(
  url: string,
  headers: Record<string, string>,
  reduce: (json: unknown) => ReturnType<typeof snippetsFromWeb>,
) {
  try {
    const res = await fetch(url, { headers, signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS), cache: 'no-store' });
    if (!res.ok) {
      console.error('search upstream status', res.status);
      return null;
    }
    return reduce(await res.json());
  } catch (err) {
    console.error('search upstream failed', err instanceof Error ? err.name : 'error');
    return null;
  }
}

export function GET() {
  // The query must never travel in a URL (hosts log paths). POST only.
  return NextResponse.json({ error: 'Use POST with a JSON body.' }, { status: 405, headers: NO_STORE });
}
