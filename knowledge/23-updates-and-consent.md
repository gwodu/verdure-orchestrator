# 23 — Updates, and the consent that governs them

*Role: the **decision + rationale** for how Verdure updates itself, and why the update check is the one network request that needed a user's permission. Written 2026-08-28, when the desktop updater was built.*

## The problem

macOS has no App Store handling updates for us. iOS and Android do — their stores own it, which is why the updater is **desktop-only** and every code path is gated on that.

Left alone, a downloaded Mac app is frozen at whatever version someone installed. That is unacceptable for an app with a **`[OQ-58]`-class crash** still open: the fix could exist for months while a tester sits on the build that vanishes.

## Why this needed a decision at all

The mechanism is ordinary — check a JSON file, compare versions, download, verify, swap, restart. What is not ordinary is that **Verdure promises not to talk to the internet about you** (`06`: local, private, no telemetry, owned by the user).

An update check is a network request to our server. It carries no personal data, but it reveals:

- that this person runs Verdure,
- roughly when they use it (each launch is a ping),
- their IP, and therefore approximate location.

That is precisely the shape of the quiet signal we say we don't collect. Shipping a silent launch-time check would have been a small, invisible breach of the product's central claim — the kind nobody notices until someone reads the network log and stops trusting everything else we said.

**So the check is opt-in.** Not because the data is sensitive, but because *we told people we don't do this*, and a promise with an unstated exception is not a promise.

## The decision

Ask **once**, plainly, and make it reversible. Three options:

| Mode | Behaviour |
|---|---|
| `notify` *(recommended)* | Check on launch; show what's available; installing stays the user's call. |
| `auto` | Check and install in the background. What they asked for, so it happens silently. |
| `manual` | **Never check on its own.** The button in Storage still works. |

Design choices that follow from the principle, not from convention:

- **The question names the actual thing.** *"Checking asks our server whether one exists — the only time Verdure reaches the internet on its own."* Not "Check for updates?", which hides that checking **is** the request being consented to.
- **`manual` is a real option, styled like the others.** A greyed-out or buried decline is a dark pattern; it makes consent theatre. Only `notify` carries visual weight, as a recommendation.
- **Reversible forever**, from Storage. A choice you cannot revisit is not consent, it is a one-time trap.
- **Unanswered ≠ yes.** Until the question is answered, no check happens. A corrupted or hand-edited stored value reads as "never asked" and re-asks — failing toward the private side.
- **A failed check is silent.** No error dialog. Someone whose only problem is being offline should not be greeted by a complaint.
- **The question waits for onboarding to finish.** On a true first launch the model picker is already on screen; stacking a second decision under it turns the first run into a settings wizard.

## Where the rule lives

Deliberately split, so the two cannot drift:

- **`app/src/updates.ts`** owns the **rule** — the stored preference, and `checkOnLaunch()`, which returns `null` without touching the network unless the mode allows it.
- **`app/src/bridge.ts`** owns the **mechanism** and does *not* consult the preference. `checkForUpdate()` always checks.

That asymmetry is intentional. The Storage button must work in **every** mode including `manual` — pressing it is itself the consent for that one request. If `bridge.ts` also enforced the preference, the button would silently do nothing for the users who most deliberately chose their setting.

`app/src/updates.test.ts` asserts the negative case, which is the one that matters: with `manual`, or with the question unanswered, the network function is never called.

## Operational notes

- **Signing key.** Updates are signed with a minisign keypair (`npx tauri signer generate`), separate from the Apple Developer ID. The private half is a GitHub secret and must never enter the repo or a transcript — the same boundary `build-testflight.yml` documents for the Apple keys. **Losing it means existing installs can never be auto-updated again**; they would have to reinstall by hand.
- **Endpoint.** `latest.json` on a public GitHub Release, so the file is free to host and free to serve. The URL is baked into the binary at build time.
- **This must ship before the first public download link.** The public key and endpoint are compiled in, so anyone who installs an earlier build can never auto-update. Retrofitting means asking every existing user to reinstall manually.
- **Updates are small.** Models live in Application Support, not the app bundle, so an update is ~30 MB rather than 1.4 GB. Nobody re-downloads a model.

## Open

- `[OQ-78]` — the update check reveals an IP. If that ever matters more than it does at a handful of testers, the honest fixes are a long random check interval (not per-launch) and/or serving `latest.json` from a CDN that doesn't retain logs. Not solved, and not pretended to be.
