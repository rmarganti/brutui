---
# brutui-hrnz
title: Pre-feature maintainability hardening
status: completed
type: epic
priority: high
tags:
- architecture
- maintainability
- pre-feature
created_at: 2026-05-30T17:22:58.009946Z
updated_at: 2026-05-30T17:59:34.732250Z
---

## Context

Codebase analysis found that Brutui v1 is functional and well tested, but several organization and state-structure choices will make future features harder to test, iterate on, and understand if left as-is. Current hotspots include large mixed-responsibility modules (`src/ui.rs`, `src/state.rs`, `src/app.rs`, `src/runner.rs`), core state depending on UI types, render-time mutation of state, public mutable state fields, concrete process/global side effects in the app controller, and a flat collection model that will fight features like filtering/collapsing/preserving tree state.

This epic captures pre-feature maintainability hardening. The goal is not to add user-visible features; it is to preserve current behavior while improving seams, invariants, and verification loops so future feature work is safer.

## Work

Complete the child ishes in dependency order. Prefer behavior-preserving refactors with small, reviewable diffs and tests proving no regressions.

## Verification

The epic is complete when all child ishes are completed and the repo passes:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`

## Implementation Notes

- Completed the maintainability hardening sequence across app, state, UI, collection, and runner seams without changing the external v1 feature set.
- Key outcomes: UI split into focused modules, render path made pure with explicit measurements, app side effects placed behind small ports, state mutation encapsulated behind selectors/transitions, collection lookup moved to an indexed read model, and runner internals split into command/process/report-path/classification responsibilities.
- The final runner refactor (`brutui-8k2u`) closed the last remaining hotspot from the original analysis, leaving the codebase in a better position for future feature work like richer tree behavior and more sophisticated run execution.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- Future feature ishes should build on the narrowed seams introduced here instead of reintroducing cross-module state mutation or concrete global side effects into controller/rendering code.
