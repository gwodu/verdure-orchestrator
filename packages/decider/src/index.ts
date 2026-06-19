// @verdure/decider — the matching brain, shared verbatim by every surface.
//
// This is the single source of truth for Verdure's decision: given what the
// user wants to do (a Classification) and what device they have (a Tier),
// WHICH local tool/model do we recommend, and is it feasible here? The website
// (the public funnel) and the desktop porter (the MVP app) both import this
// package, so the decision can never differ between surfaces — exactly the
// guarantee knowledge/14-architecture.md ("The Phase 0 ↔ Phase 1 boundary")
// and knowledge/17-build-plan.md require. The future Rust ModelMatcher must
// reproduce the same fixtures (see matrix.test.ts).
//
// Layers, inner → outer:
//   tiers      — device vocabulary + the plain-language device picker
//   modality   — the (modality, task) taxonomy + the intent cards
//   matrix     — the pure DECISION (decide): which model, how feasible
//   intent     — keyword classifier: free text → a Classification
//   recommend  — PRESENTATION: wraps a Decision in non-technical copy + installs

export * from './tiers';
export * from './modality';
export * from './matrix';
export * from './intent';
export * from './recommend';
export * from './performance';
