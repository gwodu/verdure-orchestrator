// Injected by Vite's `define` at build time (see app/vite.config.ts). Holds the
// tag this build came from — `release-14` — or `dev <sha>` for an untagged local
// build, with a trailing `+` when the tree had uncommitted changes.
declare const __BUILD_STAMP__: string;

// Minimal ambient declaration for the one Node API the Vite config uses to read
// git. Deliberately not `@types/node`: this is the only Node call in the whole
// frontend, and a ~2 MB dependency to type one function is a poor trade on a
// disk-tight machine. Widen this if the config ever genuinely needs more.
declare module "node:child_process" {
  export function execSync(
    command: string,
    options?: { stdio?: (string | number)[]; encoding?: string },
  ): { toString(): string };
}
