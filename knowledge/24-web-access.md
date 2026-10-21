# 24 — Web access: one narrow, opt-in search, and what leaves the device

**Status:** 🟡 Provisional (architecture settled and live on the iPad since `testflight-15`; trigger model revised 2026-10-21; relay abuse protection, ZDR and the eval pending — `[OQ-90]`–`[OQ-93]`)

*Role: the **decision + rationale** for how Verdure reaches the web, the exact promise made to the user about it, and the map of where that promise is enforced in code. Written 2026-10-20 from the founder's research report (October 2026) and the pilot built the same day. The report's full option landscape is condensed here; where a number comes from a third-party tracker rather than the vendor, it says so.*

## The decision

Verdure does **not** "use the browser." It adds exactly one capability: **`web_search`** — the user's approved search words go through a thin Verdure relay to a commercial search API, and 3–5 short snippets come back for the local model to answer from. Nothing else: no page fetching, no following links, no downloads, no forms, no background searches, no conversation text beyond the approved query.

Three facts from the research force this shape, and each was checked against the codebase:

1. **A relay is mandatory.** A search API key cannot ship inside a consumer app (anyone can lift it from the binary), so the phone must talk to a server we run. The relay holds the key and forwards the query. It is ~100 lines.
2. **The trigger is a switch plus a one-line protocol, not grammar-constrained tool-calling.** *(Revised 2026-10-21 after the `testflight-15` pilot.)* The first build asked the user to approve every search; on the iPad that meant a question without a time-word never got an offer, and the model — never told the tool existed — insisted it couldn't search. Now the consent is a **web switch** in the chat bar (off until turned on, `06`), and while it is on the model is *told* it can check the web and may reply with one line, `SEARCH: <3–8 words>`; the app runs those words through the data rule and asks again with results. A local time-word rule still searches first for the obvious cases. No grammar sampler, no JSON (the things that crashed or leaked in `18`); grammar-constrained tool calls remain gated on process isolation (`[OQ-58]`).
3. **IP unlinkability is phase 2.** Only Oblivious HTTP (RFC 9458) through a third-party relay lets Verdure honestly say that *no single party* sees both who you are and what you asked. Until then the promise is narrower, and stated as such (below).

### Why not the cheaper-looking options

The research scored four designs against the founder's priorities (cost, no user data outside, accuracy, reliability, complexity, simplicity, legal risk). Scraping results pages from the device or driving a WebView is $0 on the invoice and worst on everything that matters: it sends the phone's IP and fingerprint straight to a search engine, breaks whenever the markup changes (DuckDuckGo's endpoints rate-limit even popular open-source projects), and is a terms-of-service breach at consumer scale with Verdure as the identifiable party. A self-hosted SearXNG relay concentrates every user's queries on a few datacenter IPs that upstream engines block within minutes. Offline packs (daily briefs, Wikipedia ZIMs) answer only the questions you anticipated — a complement for phase 2, not a substitute. The relay + API design costs cents per user and is the only one that is private, reliable and legal at once.

### Why Brave, and the named second provider

The free/cheap API landscape shrank in 2025–26 (Bing retired Aug 2025; Google's Custom Search closes Jan 2027). **Brave Search API** is an independent index at $5 per 1,000 requests (every plan carries $5 of monthly credit, ~1,000 free requests — enough for the pilot), with an **LLM Context** endpoint that returns server-extracted text chunks under a token cap. **Mojeek** (independent UK/EU index, ~£3/1k, 90-day retention with ZDR on request, no sub-processors) is the named second adapter. Perplexity's Fast Search ($1/1k) is a cheaper third candidate once its retention terms are checked. Exa trains on queries by default; Tavily may share them with "third-party index providers"; Serper/SerpApi resell scraped Google results — all excluded as primary.

Brave's retention: query logs up to **90 days** (billing, troubleshooting, abuse) unless on an Enterprise Zero-Data-Retention plan. Brave's DPA excludes query data, treating it as non-personal; Verdure does not rely on that, because a query can contain personal data — hence the on-device scrub and the editable preview.

## What leaves the device — the promise, verbatim

This is the canonical user-facing text. It appears, word for word, in the chat (`app/src/web.ts`, `WEB_DISCLOSURE`): as a card the first time the web switch is turned on, and behind a one-line "What leaves your device?" toggle in Storage after that.

> While web search is on, only short search words for your question leave your device — never the conversation. You can see them under each answer. Verdure's relay forwards them to Brave and keeps no record of them. Our hosting provider briefly records that your device contacted verdureai.com, but not what you asked. Brave may keep the search words for up to 90 days.

*(Until 2026-10-21 the first sentence read "Only the search words you approve leave your device", when every search needed a tap; the switch replaced the per-search approval, and the words the model chooses are shown under the answer instead.)*

Every sentence is a checkable claim, and this table is where each is enforced:

| Claim | Where it holds |
|---|---|
| Only short search words, never the conversation | The words are either the user's own message (time-word fast path) or the model's `SEARCH:` line — never the transcript. `web.rs::prepare_query` scrubs emails and 7+-digit number strings and caps at 200 chars, and `web_search` scrubs again at the boundary. The request body is `{q, maxTokens}` — no history, no name, no location, fixed `User-Agent: Verdure/1`, no cookies. The exact words appear under the answer ("Searched the web for …"). |
| The relay keeps no record | `website/app/api/search/route.ts` reads the query from the POST **body** (never the URL — hosts log paths), prints nothing but upstream status codes, stores nothing, answers `Cache-Control: no-store`. |
| The host briefly records the contact | Vercel's runtime logs record path, method, status, duration and anything the function prints (we print no query); its traffic data records client IP, country, user agent and TLS fingerprint per request. Retention is short (hours on Hobby, about a day on Pro — figures to confirm in the dashboard, `[OQ-91]`). This is the sentence that stops us claiming "nothing is logged". |
| Brave may keep the words 90 days | Brave's API privacy notice, Aug 2026. Brave sees the relay's IP, not the phone's. |

**Residual risk, stated plainly:** if the user types their name or a health condition into the search words, the provider receives it. In v1 the relay *could* technically see IP + query together; we commit not to log, and phase 2 (OHTTP) removes the ability.

## The data rule — Wi-Fi by default, ask for mobile data, honest when offline

Decided with the founder 2026-10-20:

- **Wi-Fi only by default.** A search is a few KB (the response is hard-capped at 20 KB on the phone and trimmed to ~2–4 KB by the relay), which at any African data price is under a hundredth of a cent — but a default should spend nothing on someone's plan.
- **On mobile data** (or when the platform can't tell — Android today, `[OQ-94]`), Verdure asks once, in the moment: *"You're not on Wi-Fi, and web search is set to Wi-Fi only. Allow it on mobile data too? A search uses very little."* The answer is a setting (Storage → Web search), not a per-search nag. **No byte counts in the copy** — "5 KB" is jargon to the floor user.
- **Offline**, the model is **told** the check couldn't happen (`web.rs::unavailable_note`, placed in the prompt), so it answers from memory and says it wasn't checked, instead of pretending or failing. The answer carries a visible badge saying the same.

The rule is `app/src/web.ts::searchGate(mode, net)` — pure, unit-tested — over `web.rs::network_state()` (SCNetworkReachability via the `system-configuration` crate on iOS/macOS; `IS_WWAN` is cellular).

## The flow

0. The web switch (globe, chat bar) is off until the user turns it on; the first time, the disclosure card appears with "Got it". The model's system prompt says either that it can check the web (and how) or that it can't (`llm.rs::system_prompt`).
1. The user sends a message. With the switch on, a local, deterministic rule (`looksTimeSensitive`: today/latest/price/score/weather/… as whole words) sends the obvious cases straight to search with the user's own scrubbed words. Nothing is sent to decide this.
2. Otherwise the model answers — or, if it judges it needs current information, replies with one line `SEARCH: <words>` (`llm.rs::search_request`, first line only, ≤40 tokens). The app treats that as a search request.
3. The data rule runs (above). If allowed: `web_search` → relay → Brave LLM Context (falling back to plain web search) → ≤5 snippets, each `{title, url, domain, date, text}`. The model is then asked again with the results; with results present it is told not to ask for another search.
4. The snippets are fitted to **this device's context window** (`snippet_budget_tokens` = a third of `n_ctx`, 120–600 tokens; the relay was asked for the same budget) and rendered as a sources block in the final user turn, **question last** (position effects — "Lost in the Middle"). The block instructs: answer from the results, put `[n]` after facts taken from them, say so if they don't answer it. Sources never enter the conversation history (`22`); they belong to one answer.
5. When an approved search yields no sources, the UI knows *why* (offline before any call / the relay failed or answered badly / it answered with nothing) and the model is told **that same reason** (`web.rs::UnavailableReason`) — on `testflight-14` the model was told "the device is offline" when the relay had answered a redirect page, and it repeated the false reason to the user; the badge and the note now share one cause.
6. The reply shows `[n]` markers — each a tap from its page (the opener plugin, http(s) only) — then a footer: *Searched the web for "…"* and the source list by site, each tappable, cited ones highlighted. There is **no** "results weren't used" verdict: on `testflight-15` the 2B plainly used results it never numbered, and a badge that called that "from memory" was wrong more often than right. The honest badges remain for offline / failed / empty, and the model is told the same reason.

Settings: the switch itself is the globe in the chat bar; Storage → **Web search** holds the connection rule: Wi-Fi only (default) / Wi-Fi and mobile data, plus the disclosure.

## The relay (pilot) and its successor

**Pilot:** a Next.js route handler on verdureai.com (`website/app/api/search/route.ts`), because the hostname, deployment and trust already exist (the updater uses it, `23`). The app addresses **`www.verdureai.com`**, not the apex: the apex answers every request with a 307 to `www.`, and a POST is not replayed across a redirect (ureq follows 307s for GET/HEAD only, as the HTTP spec asks). `testflight-14` found this on the iPad: the reply was a "Redirecting..." page. `BRAVE_SEARCH_API_KEY` lives in the project's environment variables; without it the route answers 503 with a plain sentence and the app says the web couldn't be checked. The route tries LLM Context first and falls back to Brave's plain web search if the plan lacks it or it errors, trims to the phone's budget, and logs status codes only.

**Successor (phase 2):** a small open-source Rust relay with access logs disabled by construction, then an OHTTP front (Fastly or Cloudflare relay → Verdure gateway) so the relay sees content without IP and the third party sees IP without content. Negotiate Brave Enterprise ZDR when volume justifies it. Ship the daily brief pack per market as the offline adapter, and consider an explicit, user-initiated "read this page" with local extraction and injection hardening. Until the trait has that second adapter, `web.rs` is deliberately a module of functions, not a `WebSearch` trait (no abstraction without a second implementation — `14`'s seam table gets a `WebSearch` row when it lands).

## Cost (list prices, Oct 2026; provisional)

Assuming 15 active days a month, 2–5 searches per active day, 40% of users enabling search and a 30% cache hit later: ~**$0.04–0.10 per monthly active user per month** on Brave — about $420–1,050/month at 10k MAU, $42k–105k at 1M before volume pricing. Per *enabled* user that is $0.10–0.26/month, a natural "Verdure Plus" line item or telco-subsidised cost. The pilot runs inside Brave's monthly credit. Relay hosting is tens of dollars a month at 10k MAU.

## Implementation map

- `app/src-tauri/src/web.rs` — scrub, budget, fit, sources block, offline note, relay call (ureq/rustls, 12 s timeout, 20 KB cap), network state. Tests cover every pure function and the flags→state rule.
- `app/src-tauri/src/llm.rs` — `WebAccess` + `system_prompt` (the model is told whether it has the web), `search_request` (the `SEARCH:` line), `reply_budget` (256 at the floor window, 512 when roomy — grounded answers were cut off at 256), `Grounding` + `compose_user_turn`: sources ahead of the question in the final turn, costed with it so they displace old history rather than overflow.
- `app/src-tauri/src/lib.rs` — commands `network_state`, `prepare_web_query`, `web_search`; `chat` takes `webQuery`/`webSources`/`webUnavailable`.
- `app/src/web.ts` — the rules (the switch, the connection mode, the gate, the fast-path heuristic, the disclosure text). `bridge.ts` — mechanism only. `main.ts` — the globe switch and its first-time card, the search loop (fast path or model request → data rule → ask again), the mobile-data ask, the tappable sources footer, the Storage section. `format.ts` — `[n]` → citation markers, `citedSources`.
- `website/lib/search.ts` + `app/api/search/route.ts` — the relay.

## Open Questions

- `[OQ-90]` Brave's LLM Context response schema is assembled from Brave's skill docs and a third-party description (`grounding.generic[].{url,title,snippets[]}`), not a live call; the mapper is tolerant and the route falls back to web search. Confirm on the first keyed call, and get Brave's written answer on whether a short-lived device cache counts as "storing results".
- `[OQ-91]` The relay is an open endpoint spending our credit: add abuse protection (Vercel WAF rate limiting or a per-IP bucket), confirm Vercel's log-retention figures in the dashboard, and plan the own-relay + OHTTP step that turns "keeps no record" from policy into construction.
- `[OQ-92]` Measure recall on Nigerian/Kenyan/Ghanaian/South African queries and the grounded-answer quality on P1 models with a fixed eval set (ties to `[OQ-74]`); decide whether a model-written query rewrite beats the user's own scrubbed words.
- `[OQ-93]` Cross-border transfer compliance (NDPA, Kenya DPA, Act 843, POPIA, GDPR): consent wording, a DPA/ZDR with the provider, registration thresholds. Local counsel.
- `[OQ-94]` Android reports `Unknown` network state (asks every time on cellular until "any" is chosen); macOS on a phone hotspot reads as Wi-Fi. (Sources became tappable 2026-10-21 via the opener plugin.)
- `[OQ-95]` OEM/telco pre-installs: how opt-in consent is captured, and whether a carrier can zero-rate the relay hostname.
