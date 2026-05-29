---
# brutui-fe0z
title: Implement startup flow, collection picker, and setup messaging
status: completed
type: task
priority: high
tags:
- ui
- startup
created_at: 2026-05-29T17:28:16.856780Z
updated_at: 2026-05-29T17:28:16.856780Z
parent: brutui-carh
blocked_by:
- brutui-h39n
- brutui-iblr
---

## Context

Startup must be fast and predictable: explicit path opens directly, cwd discovery is frictionless, one collection opens automatically, many collections show a searchable picker, and no discovery shows helpful setup guidance.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 1-13 and startup collection selection implementation decisions.

## Scope

- Wire CLI launch options into discovery precedence.
- Auto-open exactly one discovered collection.
- Render/search a lightweight collection picker when multiple collections are discovered.
- Show actionable setup messaging when no collection/config is found or `bru` cannot be resolved.
- Ensure config is optional for inside-collection launch.

## Validation

- Tests cover explicit/cwd/config precedence at startup, one collection auto-open, many collection picker state/search, no collection messaging, and missing `bru` messaging.

## Implementation Notes

- Replaced the startup placeholder in `src/app.rs` with a real bootstrap flow that loads config, runs PRD-aligned discovery precedence, auto-opens a single collection, shows setup guidance when nothing is found, and surfaces missing-`bru` resolution as an actionable startup message.
- Added a simple runtime split between startup state and loaded-session state so the real TUI now launches immediately into either a collection picker/setup screen or the collection browser/run UI.
- Extended `src/ui.rs` startup handling to support searchable collection picking with type-to-filter, arrow/j/k navigation, Enter-to-open, and startup-specific rendering instead of the previous placeholder text.
- Added focused startup tests in `src/app.rs` for explicit-path precedence, cwd discovery without config, configured auto-open, multi-collection picker startup, no-collection messaging, and missing-`bru` messaging.
- Hardened `tests/run_ui_flow.rs` to assert stdout/stderr presence without depending on cross-thread event ordering.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `AppBootstrap::prepare_runtime()` and `load_collection_runtime()` are now the seams for future startup refinements; keep discovery/setup decisions there instead of reintroducing startup logic in `main`.
- Startup picker selection currently resolves `bru` only when transitioning into a concrete collection; if future UX wants preflight diagnostics before selection, that behavior should be adjusted in bootstrap rather than UI state.
- The loaded-session loop already reuses `AppController`, so upcoming details/environment-picker work can extend the same event/render path without reshaping startup again.
