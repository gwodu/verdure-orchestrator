# verdureai.com

The public site, Next.js on Vercel. Every push to `main` deploys it (knowledge/21, "How to build / ship").
What the site is for and why it is shaped this way: `knowledge/05-website-spec.md`, "The 2026-11-04 revamp".

## Pages

| Route | Job |
|---|---|
| `/` | A company deciding whether to put Verdure in its app knows in ten seconds what it does and is one click from running it. Then: what we measured, who it is for, the app, keep-me-posted, the founder. |
| `/sdk` | The white paper, navigable: what it is, how it works, the hello world (Kotlin, Rust, Python), the API, which model for which device, what we measured, privacy and licences, what is not done, how to get it. Facts are data in `lib/sdk-content.ts` and tested. |
| `/try` | Get the app onto the reader's own device: the macOS download, and a "send me a test build" form for iPhone and Android (testers are added by hand). |
| `/privacy` | The privacy policy. Required by the app stores; written as a true description of the software. |
| `/start/*` | The device recommender from Phase 0: pick what you want to do and your device, get an honest recommendation. Linked from `/try`, not from the nav. |
| `/api/waitlist` | Where every form on the site lands (Google Sheets; `SETUP_SHEETS.md`). `intentText` says which form. |
| `/api/search` | The relay for the app's opt-in web search (knowledge/24). |
| `/updates/*` | The desktop updater's endpoint, redirected to GitHub Releases (`next.config.ts`, knowledge/23). Never share it with a person. |

## Work on it

```sh
npm install
npm run dev          # http://localhost:3000
npm test             # vitest: the lockfile guard, the links, the SDK page's facts, search, sheets
npm run build        # what Vercel runs; typechecks everything
npm run shots        # phone-width screenshots of every page into ./screenshots/ (needs `npx playwright install chromium`)
```

Look at the screenshots after a change (skills/design.md, "The visual self-review loop"). The identity — deep navy,
aurora mint, frosted glass, system fonts — lives as tokens in `app/globals.css`; the older pages' Tailwind names
(`slate-*`, `emerald-*`) are re-pointed at those tokens there, so one palette covers the whole site.

Outward links (the macOS download, the contact address) live once, in `lib/links.ts`, with a test.

**Do not run `npm install` to "fix" the lockfile** — read `lib/lockfile.test.ts` first (knowledge/11, "the Linux lockfile rule").
