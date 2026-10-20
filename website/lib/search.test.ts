import { describe, it, expect } from 'vitest';
import {
  MAX_QUERY_CHARS,
  MAX_RESULTS,
  braveContextUrl,
  braveHeaders,
  braveWebUrl,
  clampBudgetTokens,
  domainOf,
  snippetsFromContext,
  snippetsFromWeb,
  stripTags,
  trimToBudget,
  validateQuery,
  type Snippet,
} from './search';

describe('validateQuery', () => {
  it('accepts a normal question and collapses whitespace', () => {
    expect(validateQuery('  naira   dollar rate\ntoday ')).toBe('naira dollar rate today');
  });

  it('rejects non-strings and empty strings', () => {
    for (const bad of [undefined, null, 42, {}, [], '', '   \n']) expect(validateQuery(bad)).toBeNull();
  });

  it('caps the length by characters, not bytes', () => {
    const q = validateQuery('é'.repeat(500))!;
    expect(Array.from(q).length).toBe(MAX_QUERY_CHARS);
  });
});

describe('clampBudgetTokens', () => {
  it('bounds the budget and tolerates garbage', () => {
    expect(clampBudgetTokens(341)).toBe(341);
    expect(clampBudgetTokens(50)).toBe(120);
    expect(clampBudgetTokens(10_000)).toBe(600);
    expect(clampBudgetTokens('lots')).toBe(120);
    expect(clampBudgetTokens(NaN)).toBe(120);
  });
});

describe('upstream requests carry the query and nothing about the user', () => {
  it('builds the LLM Context URL with the floor token cap', () => {
    const u = new URL(braveContextUrl('fuel price Accra'));
    expect(u.origin + u.pathname).toBe('https://api.search.brave.com/res/v1/llm/context');
    expect(u.searchParams.get('q')).toBe('fuel price Accra');
    expect(u.searchParams.get('maximum_number_of_tokens')).toBe('1024');
    // No location, language or identifying parameter rides along.
    expect([...u.searchParams.keys()].sort()).toEqual(['maximum_number_of_tokens', 'q']);
  });

  it('builds the web-search fallback URL', () => {
    const u = new URL(braveWebUrl('x'));
    expect(u.pathname).toBe('/res/v1/web/search');
    expect(u.searchParams.get('count')).toBe(String(MAX_RESULTS));
  });

  it('sends the key and no cookies or client headers', () => {
    const h = braveHeaders('k');
    expect(h['X-Subscription-Token']).toBe('k');
    expect(Object.keys(h).map((k) => k.toLowerCase())).not.toContain('cookie');
    expect(Object.keys(h).map((k) => k.toLowerCase())).not.toContain('x-forwarded-for');
  });
});

describe('text helpers', () => {
  it('strips the light HTML Brave decorates descriptions with', () => {
    expect(stripTags('The <strong>naira</strong> fell &amp; rose  again')).toBe('The naira fell & rose again');
  });

  it('reads a bare domain', () => {
    expect(domainOf('https://www.cbn.gov.ng/rates/')).toBe('cbn.gov.ng');
    expect(domainOf('not a url')).toBe('');
  });
});

describe('snippetsFromContext', () => {
  it('reads grounding.generic[] with string snippets', () => {
    const got = snippetsFromContext({
      grounding: {
        generic: [
          { url: 'https://www.cbn.gov.ng/rates', title: 'Rates', snippets: ['Closed at 1,480.', 'Opened at 1,470.'], date: '2026-10-20' },
          { url: 'https://x.example/empty', title: 'Nothing', snippets: [] },
        ],
      },
    });
    expect(got).toEqual<Snippet[]>([
      { title: 'Rates', url: 'https://www.cbn.gov.ng/rates', domain: 'cbn.gov.ng', date: '2026-10-20', text: 'Closed at 1,480. Opened at 1,470.' },
    ]);
  });

  it('tolerates {text} chunks and a flat results[]', () => {
    const got = snippetsFromContext({ results: [{ url: 'https://a.com/p', snippets: [{ text: 'Hello' }] }] });
    expect(got[0]).toMatchObject({ domain: 'a.com', title: 'a.com', text: 'Hello', date: null });
  });

  it('yields nothing for an unknown shape, so the route can fall back', () => {
    expect(snippetsFromContext({ type: 'error' })).toEqual([]);
    expect(snippetsFromContext('nope')).toEqual([]);
    expect(snippetsFromContext(null)).toEqual([]);
  });
});

describe('snippetsFromWeb', () => {
  it('reads web.results[] descriptions and ages', () => {
    const got = snippetsFromWeb({
      web: { results: [{ url: 'https://b.com/x', title: 'B <b>bold</b>', description: 'A <strong>desc</strong>', page_age: '2026-10-19T00:00:00' }] },
    });
    expect(got).toEqual<Snippet[]>([{ title: 'B bold', url: 'https://b.com/x', domain: 'b.com', date: '2026-10-19T00:00:00', text: 'A desc' }]);
  });
});

describe('trimToBudget', () => {
  const s = (domain: string, len: number): Snippet => ({ title: 'T', url: `https://${domain}/`, domain, date: null, text: 'x'.repeat(len) });

  it('keeps at most MAX_RESULTS, in order', () => {
    const got = trimToBudget(Array.from({ length: 9 }, (_, i) => s(`s${i}.com`, 20)), 600);
    expect(got.map((x) => x.domain)).toEqual(['s0.com', 's1.com', 's2.com', 's3.com', 's4.com']);
  });

  it('caps each snippet and the whole reply, trimming the straddler', () => {
    const got = trimToBudget([s('a.com', 5000), s('b.com', 5000)], 300); // 900 chars
    expect(got.length).toBe(2);
    expect(got[0].text.length).toBe(700);
    expect(got[1].text.length).toBe(200);
  });

  it('stays small enough for mobile data: a maximal reply is a few KB', () => {
    const got = trimToBudget(Array.from({ length: 5 }, (_, i) => s(`s${i}.com`, 5000)), 600);
    const bytes = new TextEncoder().encode(JSON.stringify({ results: got })).length;
    expect(bytes).toBeLessThan(4096);
  });
});
