---
# brutui-hrnz
title: Pre-feature maintainability hardening
status: todo
type: epic
priority: high
tags:
- architecture
- maintainability
- pre-feature
created_at: 2026-05-30T17:22:58.009946Z
updated_at: 2026-05-30T17:22:58.009946Z
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
