import { defineConfig } from "vitest/config";

// The frontend had no unit-test harness until 2026-08-28; `npm run build` (tsc)
// was the only automated check, which cannot assert behaviour. Added for the
// update-consent rule (knowledge/23) — a privacy promise that is easy to break
// silently and has no visible symptom when broken.
//
// jsdom because the rule is stored in `localStorage`, and testing it against a
// hand-rolled stub would be testing the stub.
export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
});
