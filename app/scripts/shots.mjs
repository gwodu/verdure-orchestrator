// Visual self-review for the porter UI (skills/design.md "The visual self-review
// loop"). Serves the built app via `npm run preview`, then captures the two
// screens at desktop window size into app/screenshots/ (git-ignored).
//
// Run from the website workspace so Playwright resolves:
//   node ../app/scripts/shots.mjs
// with the preview server already running on :4173.

import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";

const here = dirname(fileURLToPath(import.meta.url));
// Playwright lives in the website workspace; anchor resolution there so this
// script can run from anywhere without a duplicate install.
const requireFromWebsite = createRequire(
  join(here, "..", "..", "website", "package.json"),
);
const { chromium } = requireFromWebsite("playwright");
const outDir = join(here, "..", "screenshots");
mkdirSync(outDir, { recursive: true });

const BASE = process.env.SHOTS_URL || "http://localhost:4173";

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 720, height: 820 } });

// Screen 1 — the intent grid.
await page.goto(BASE, { waitUntil: "networkidle" });
await page.waitForSelector(".grid .card");
await page.screenshot({ path: join(outDir, "1-home.png") });

// Screen 2 — the in-app transcription panel (the pivot, knowledge/18).
const cards = page.locator(".grid .card");
const count = await cards.count();
for (let i = 0; i < count; i++) {
  const title = (await cards.nth(i).locator(".card-title").innerText()).trim();
  if (title.toLowerCase().includes("recordings")) {
    await cards.nth(i).click();
    break;
  }
}
await page.waitForSelector(".dropzone");
await page.screenshot({ path: join(outDir, "2-transcribe.png") });

// Screen 2b — live (microphone) transcription, mid-session (browser sim).
const liveTab = page.locator(".mode-toggle .tab", { hasText: "Record live" });
if (await liveTab.count()) {
  await liveTab.click();
  await page.locator(".live-area .record").click();
  // Let the browser simulation stream a couple of segments in.
  await page
    .waitForFunction(
      () => (document.querySelector(".transcript")?.value.length ?? 0) > 0,
      { timeout: 5000 },
    )
    .catch(() => {});
  await page.screenshot({ path: join(outDir, "2b-live.png") });
}

// Screen 3 — the global Storage screen.
await page.goto(BASE, { waitUntil: "networkidle" });
await page.waitForSelector(".footer .link");
await page.locator(".footer .link").click();
await page.waitForSelector(".model-list");
await page.screenshot({ path: join(outDir, "3-storage.png") });

await browser.close();
console.log("Wrote screenshots to", outDir);
