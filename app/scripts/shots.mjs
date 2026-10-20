// Visual self-review for the app UI (skills/design.md "The visual self-review
// loop"). Captures today's screens from the built frontend in a browser preview
// (bridge.ts serves plausible data when there is no Tauri backend) into
// app/screenshots/ (git-ignored):
//
//   1-picker.png   first run: "Choose your model", with the recommended choice
//   2-chat.png     chat, once a model has been chosen
//   3-storage.png  Storage & your computer
//
// Usage, from app/:
//   npm run build && npx vite preview --port 4173 --strictPort &
//   node scripts/shots.mjs
//
// (Rewritten 2026-10-20: the previous version drove the pre-pivot intent grid
// and transcription screens, which no longer exist, so the loop was broken.)

import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";

const here = dirname(fileURLToPath(import.meta.url));
// Playwright lives in the website workspace; anchor resolution there so this
// script can run from anywhere without a duplicate install.
const requireFromWebsite = createRequire(join(here, "..", "..", "website", "package.json"));
const { chromium } = requireFromWebsite("playwright");
const outDir = join(here, "..", "screenshots");
mkdirSync(outDir, { recursive: true });

const BASE = process.env.SHOTS_URL || "http://localhost:4173";
const MODEL_CHOICE_KEY = "verdure.chat.modelId"; // main.ts
const settle = (page) => page.waitForTimeout(900); // entry animations

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 720, height: 900 } });

// 1 — first run: no model chosen yet, so the picker is the first screen.
await page.goto(BASE, { waitUntil: "networkidle" });
await page.evaluate(() => localStorage.clear());
await page.reload({ waitUntil: "networkidle" });
await page.waitForSelector(".model-picker .pick");
await settle(page);
await page.screenshot({ path: join(outDir, "1-picker.png") });

// 2 — chat, as a returning user who has chosen a model.
await page.evaluate(
  ([key]) => localStorage.setItem(key, "qwen3-4b-instruct-2507-byteshape"),
  [MODEL_CHOICE_KEY],
);
await page.reload({ waitUntil: "networkidle" });
await page.waitForSelector(".model-chip");
await settle(page);
await page.screenshot({ path: join(outDir, "2-chat.png") });

// 3 — Storage, from the header.
await page.getByRole("button", { name: "Storage & your computer" }).click();
await page.waitForSelector(".model-list");
await settle(page);
await page.screenshot({ path: join(outDir, "3-storage.png") });

await browser.close();
console.log("Wrote screenshots to", outDir);
