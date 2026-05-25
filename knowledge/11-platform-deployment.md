# 11 — Platform Deployment

**Status:** 🔴 Open — high-level shape known, specifics need resolution

This file documents the practical realities of shipping Verdure on each platform. Phase 0 (the website) sidesteps these. Phase 1 and beyond hit them directly.

## macOS

- Notarization required for distribution outside the App Store
- Hardened runtime entitlements required for notarization
- Code signing certificate via Apple Developer Program ($99/year as of last check)
- Gatekeeper bypass needed for first-run on unsigned binaries, or properly notarized binary distribution

**[GAP — needs research]** Current Apple Developer Program pricing in 2026, any recent notarization policy changes. Tracked as `[OQ-23]`.

## Windows

- Code signing certificate (~$200–500/year from a CA) — exact pricing fluctuates
- Microsoft SmartScreen builds reputation slowly; early users may see warnings on a new certificate
- Distribution: MSI installer or portable EXE
- Windows Defender may flag unknown binaries until reputation accrues

**[GAP — needs research]** Current Windows code signing certificate pricing and CA recommendations in 2026. SmartScreen reputation-building timelines. Tracked as `[OQ-24]`.

## Linux

- Strategy for v1: AppImage (universal) plus optional distro packages later
- No signing infrastructure required for v1
- Possible future: Flatpak for sandboxed deployment

## Android

- Foreground service for the orchestration daemon
- Storage permissions for the shared model directory
- Background execution policies have tightened across recent Android versions
- Google Play review: system-level access requests may trigger additional scrutiny

**[GAP — needs research]** Current Android 14+ background service policies and Play Store review requirements for system-level access in 2026. Tracked as `[OQ-25]`.

## iOS

- Background execution heavily restricted (likely too restricted for a true Verdure daemon model)
- App sandbox more rigid than Android — no equivalent to Android's shared-storage system service pattern
- TestFlight viable for early distribution
- True system-service model likely **not feasible** — Verdure on iOS may default to a single-app experience

**[GAP — needs research]** Confirm iOS feasibility for the daemon model vs. single-app model. This is a strategic blocker for Phase 2 on iOS. Tracked as `[OQ-26]`.

## Open questions

- [OQ-23] Current Apple Developer Program pricing, notarization policy changes in 2026.
- [OQ-24] Current Windows code signing certificate pricing, CA recommendations, SmartScreen reputation timeline.
- [OQ-25] Android 14+ background service policies and Play Store requirements for system-level apps in 2026.
- [OQ-26] iOS feasibility for the daemon model — definitive answer needed before committing to Phase 2 on iOS.
