# Verdure — desktop porter (MVP)

The MVP from `knowledge/17-build-plan.md`: pick a task → Verdure recommends a
free, local, open-source tool → **one click** installs and launches it. First
wired tool: **Buzz** (transcription). Tauri v2 (Rust backend) + Vite/TS frontend.

## What's where

```
app/
  index.html            entry
  src/
    main.ts             the two-screen UI (intent grid → recommendation → set up)
    bridge.ts           Tauri calls, with a browser-preview fallback (simulated port)
    styles.css          calm, floor-user-first styling
  scripts/
    shots.mjs           Playwright screenshot loop (design self-review)
    make-icon.mjs       generates the source icon
  src-tauri/
    src/
      lib.rs            Tauri app + commands: detect_device, port_tool
      catalog.rs        the port catalog AS DATA (Buzz) — add tools here
      device.rs         detect the hardware tier (no question asked)
      porter.rs         locate brew → check → install → launch, with progress
    tauri.conf.json     window + bundle config
    capabilities/       webview permissions
```

The recommendation logic is **not** in here — it's imported from
`@verdure/decider` (`packages/decider/`), the same brain the website uses.

## Run it

Prerequisite (one-time): **Xcode Command Line Tools** — Tauri's Rust build needs
a C linker.

```sh
xcode-select --install          # if `cc` doesn't work yet
```

Then, from the repo root:

```sh
npm install                      # workspaces: decider + website + app
cd app && npm run tauri dev      # launches the desktop app
```

Pick **"Turn recordings into text"** → **Set it up**. The porter runs
`brew install --cask buzz`, then `open -a Buzz`. Buzz then runs entirely on your
machine — private, free, no account.

## Preview the UI without compiling Rust

The frontend runs in a plain browser; Tauri calls degrade to a simulation
(nothing is actually installed):

```sh
cd app && npm run build && npm run preview     # http://localhost:4173
# screenshots (Playwright, from repo root):
node app/scripts/shots.mjs                     # → app/screenshots/
```

## Test

```sh
npm test --workspace @verdure/decider          # the decision (64 tests)
cd app && npm run build                         # frontend typecheck + bundle
cd app/src-tauri && cargo test                  # porter unit tests (needs CLT)
```

## Add a ported tool

Add a `Tool` entry to `app/src-tauri/src/catalog.rs` (id, name, `InstallMethod`,
`LaunchMethod`) and map its classification → `{ id, name }` in the `PORTABLE`
table in `app/src/main.ts`. No new control flow — the porter interprets the
recipe. See `knowledge/15-port-catalog.md`.
