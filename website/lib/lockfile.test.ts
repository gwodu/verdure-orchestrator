// Guard for [OQ-81]: Vercel builds on Linux, and the lockfile is written on a
// Mac. npm drops other platforms' optional native packages from a lockfile it
// writes while `node_modules` exists, after which `npm ci` on Linux installs no
// Linux binary for Tailwind's oxide or Next's SWC and the build dies with
// "Cannot find module '@tailwindcss/oxide-linux-x64-gnu'". That broke every
// production deploy for twelve days before anyone read the log. This test
// fails on the Mac, before the push, instead.
//
// If it fails: do NOT run `npm install` to fix it (that is how the entries
// vanish). Regenerate the lockfile in a tree without node_modules, or re-add
// the platform entries — see knowledge/11, "Website: the Linux lockfile rule".

import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const lock = JSON.parse(readFileSync(join(__dirname, '..', '..', 'package-lock.json'), 'utf8')) as {
  packages: Record<string, { version?: string; optionalDependencies?: Record<string, string> }>;
};
const keys = Object.keys(lock.packages);
const has = (name: string) => keys.some((k) => k.endsWith(`node_modules/${name}`));

describe('the lockfile carries the Linux x64 native packages Vercel needs', () => {
  it.each(['@tailwindcss/oxide-linux-x64-gnu', '@next/swc-linux-x64-gnu', 'lightningcss-linux-x64-gnu'])(
    'records %s',
    (name) => {
      expect(has(name), `${name} missing — see the note at the top of this file`).toBe(true);
    },
  );

  it('positive control: the Mac packages are there too (the check can see entries at all)', () => {
    expect(has('@tailwindcss/oxide-darwin-x64')).toBe(true);
  });

  it('every declared optional platform package of oxide and next is recorded', () => {
    for (const [key, entry] of Object.entries(lock.packages)) {
      if (!/node_modules\/(@tailwindcss\/oxide|next)$/.test(key)) continue;
      for (const dep of Object.keys(entry.optionalDependencies ?? {})) {
        expect(has(dep), `${key} declares optional ${dep}, which the lockfile lacks`).toBe(true);
      }
    }
  });
});
