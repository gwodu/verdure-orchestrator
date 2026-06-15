# 11 — Platform Deployment

**Status:** 🟡 Provisional — iOS (the largest unknown) resolved at WWDC 2026; remaining gaps are research-grade (signing/store specifics)

This file documents the practical realities of shipping Verdure on each platform. Phase 0 (the website) sidesteps these. Phase 1 and beyond hit them directly.

> **Decision content behind the `Transport` / shell layer** (`14-architecture.md`) — signing, packaging, and per-OS IPC. Structure (the `Transport` trait, the shell's job) lives there; the per-platform packaging realities live here.

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

**[OQ-26] is resolved: the daemon model is not feasible on iOS — and no longer needs to be.** Background execution restrictions and the app sandbox rule out a Verdure system service. But as of iOS 27 (WWDC 2026), the OS itself ships the shared inference service Verdure would have had to build: a resident ~3B system model behind Apple's Foundation Models framework, accessible to every app via the public `LanguageModel` protocol.

The iOS strategy therefore pivots from "daemon" to **thin decider over Apple's stack**:
- Verdure on iOS is an app and/or embeddable library, not a service.
- For in-app open-weight models, MLX Swift works but is app-scoped: requires the Increased Memory Limit entitlement, lives under hard per-app memory ceilings (a 4 GB iPhone is limited to roughly 1B-parameter models), and models are killed with the app. Pending `[OQ-42]` on whether `MLXLanguageModel` itself is available on iOS.
- The distribution-channel opportunity: ship a **`VerdureLanguageModel` Swift package** conforming to Apple's `LanguageModel` protocol. Swift developers would get intent-first, device-aware model selection through Apple's own API surface — Verdure deciding between the system model, an MLX model, or a future cloud option, behind the same protocol Anthropic and Google are using to enter the ecosystem. Tracked as `[OQ-43]`.

TestFlight remains viable for early distribution. All of the above reflects iOS 27 developer betas (June 2026); re-verify at public release in the fall.

## Open questions

- [OQ-23] Current Apple Developer Program pricing, notarization policy changes in 2026.
- [OQ-24] Current Windows code signing certificate pricing, CA recommendations, SmartScreen reputation timeline.
- [OQ-25] Android 14+ background service policies and Play Store requirements for system-level apps in 2026.
- [OQ-26] Resolved (WWDC 2026) — daemon infeasible on iOS, confirmed; strategy pivoted to a thin decider over Apple's Foundation Models stack (resident system model does the shared-service job).
- [OQ-43] `VerdureLanguageModel` Swift package — do we ship it, and in which phase? Strategic distribution decision, not just engineering.
