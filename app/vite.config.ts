import { defineConfig } from "vite";
import { execSync } from "node:child_process";

// The build stamp shown in the app's corner. Answers one question the founder
// kept having to guess at: "which build am I actually looking at?" — the thing
// that let a stale, translocated copy get tested against on 2026-08-22.
//
// Prefers the tag when HEAD is tagged (`release-14`), because that is the name a
// build is discussed by; falls back to the short SHA for untagged working
// builds, prefixed `dev` so a local build is never mistaken for a shipped one.
// A dirty tree gets a `+` — a build with uncommitted changes is not reproducible
// from any commit, and that is worth one character to know.
function buildStamp(): string {
  const git = (cmd: string): string => {
    try {
      return execSync(`git ${cmd}`, { stdio: ["ignore", "pipe", "ignore"] })
        .toString()
        .trim();
    } catch {
      return ""; // not a git checkout (a tarball build) — degrade, never throw
    }
  };
  const tag = git("tag --points-at HEAD");
  const sha = git("rev-parse --short HEAD");
  const dirty = git("status --porcelain") ? "+" : "";
  if (!sha) return "unknown build";
  // Multiple tags can point at one commit; the last line is the newest.
  const name = tag ? tag.split("\n").filter(Boolean).pop()! : `dev ${sha}`;
  return `${name}${dirty}`;
}

// Vite config tuned for Tauri: a fixed dev port Tauri can point its webview at,
// and a build target matching the platforms Tauri ships to. Running `vite` on
// its own (without Tauri) also works — the UI degrades to a browser preview so
// the design review loop doesn't need the Rust backend compiled. See
// app/src/bridge.ts.
export default defineConfig({
  clearScreen: false,
  // Baked in at build time — the running app has no git and no shell.
  define: {
    __BUILD_STAMP__: JSON.stringify(buildStamp()),
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2021",
    outDir: "dist",
    emptyOutDir: true,
  },
});
