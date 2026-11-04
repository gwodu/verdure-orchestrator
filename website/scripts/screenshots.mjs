// Visual self-review loop (see ../../skills/design.md, "The visual self-review loop").
//
// Captures the key screens at mobile width into ./screenshots/ (git-ignored) so
// a change can be reviewed against its design intent — including states that
// need session data (the recommendation pages) and the reduced-motion variant.
//
// One-time setup:  npx playwright install chromium
// Usage:           npm run dev      (in one terminal)
//                  npm run shots    (in another)
//                  BASE=https://verdureai.com npm run shots   (against prod)

import { chromium } from 'playwright';
import { mkdir } from 'node:fs/promises';

const BASE = process.env.BASE || 'http://localhost:3000';
const OUT = new URL('../screenshots/', import.meta.url).pathname;
const VIEWPORT = { width: 390, height: 844 }; // a common phone — our primary audience

// Each shot maps to a real user state. `session` pre-seeds the flow's
// sessionStorage so the recommendation pages render without clicking through.
const SHOTS = [
  { name: 'home', url: '/', full: true },
  { name: 'home-reduced-motion', url: '/', full: true, reducedMotion: 'reduce' },
  { name: 'sdk', url: '/sdk', full: true },
  { name: 'sdk-hello', url: '/sdk#hello' },
  { name: 'try', url: '/try', full: true },
  { name: 'intent', url: '/start' },
  { name: 'device', url: '/start/device', session: { 'verdure.flow.intent': 'transcribe my recordings' } },
  { name: 'rec-audio', url: '/start/recommendation', session: { 'verdure.flow.intent': 'transcribe my recordings', 'verdure.flow.device': 'laptop' } },
  { name: 'rec-video-not-local', url: '/start/recommendation', session: { 'verdure.flow.intent': 'make a video of my dog', 'verdure.flow.device': 'laptop' } },
  { name: 'rec-image-gpu', url: '/start/recommendation', session: { 'verdure.flow.intent': 'create a logo for my shop', 'verdure.flow.device': 'gaming' } },
];

await mkdir(OUT, { recursive: true });
const browser = await chromium.launch();
let ok = 0;

for (const s of SHOTS) {
  const ctx = await browser.newContext({
    viewport: VIEWPORT,
    deviceScaleFactor: 2,
    ...(s.reducedMotion ? { reducedMotion: s.reducedMotion } : {}),
  });
  if (s.session) {
    await ctx.addInitScript((data) => {
      for (const [k, v] of Object.entries(data)) window.sessionStorage.setItem(k, v);
    }, s.session);
  }
  const page = await ctx.newPage();
  try {
    await page.goto(BASE + s.url, { waitUntil: 'networkidle', timeout: 15000 });
  } catch {
    console.error(`✗ ${s.name}: could not load ${BASE}${s.url} — is the dev server running? (npm run dev)`);
    await ctx.close();
    continue;
  }
  if (s.scrollTo) await page.evaluate((y) => window.scrollTo(0, y), s.scrollTo);
  await page.waitForTimeout(800);
  await page.screenshot({ path: `${OUT}${s.name}.png`, fullPage: !!s.full });
  await ctx.close();
  console.log('✓', s.name);
  ok++;
}

await browser.close();
console.log(`\n${ok}/${SHOTS.length} screenshots written to ${OUT}`);
