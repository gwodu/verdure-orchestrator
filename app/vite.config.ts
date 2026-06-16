import { defineConfig } from "vite";

// Vite config tuned for Tauri: a fixed dev port Tauri can point its webview at,
// and a build target matching the platforms Tauri ships to. Running `vite` on
// its own (without Tauri) also works — the UI degrades to a browser preview so
// the design review loop doesn't need the Rust backend compiled. See
// app/src/bridge.ts.
export default defineConfig({
  clearScreen: false,
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
