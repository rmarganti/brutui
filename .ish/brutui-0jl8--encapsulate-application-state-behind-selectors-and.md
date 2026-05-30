---
# brutui-0jl8
title: Encapsulate application state behind selectors and transitions
status: todo
type: task
priority: normal
tags:
- state
- invariants
- architecture
created_at: 2026-05-30T17:23:54.658802Z
updated_at: 2026-05-30T17:24:20.633084Z
parent: brutui-hrnz
blocking:
- brutui-u3c9
blocked_by:
- brutui-uamc
- brutui-0f8d
- brutui-4gfn
---

## Context

Most application state structs expose public mutable fields (`AppState`, `SessionState`, `CollectionPickerState`, etc.). The code already has many transition methods, but public fields allow tests and future code to bypass invariants around selected node, selected environment, active runs, selected result index, modal state, and scroll state.

## Dependencies

- Blocked by `brutui-uamc` so state no longer depends on UI types.
- Blocked by `brutui-0f8d` so UI callers are easier to update in focused modules.
- Blocked by `brutui-4gfn` so app/controller seams are explicit before tightening state mutation access.

## Work

- Make state fields private where practical, starting with `SessionState` internals most prone to invariant drift.
- Add read-only selectors/accessors for rendering and controller code.
- Keep all mutations through explicit transition methods on `AppState`, `SessionState`, or smaller sub-state types.
- Add invariant-oriented tests for selected environment/result bounds, selected node validity after navigation, run lifecycle, and modal transitions.
- Avoid a disruptive all-at-once rewrite if needed; the outcome should meaningfully reduce direct public mutation while preserving ergonomics.

## Verification

- Existing state, UI, app, and run-flow tests pass after updating call sites.
- New/updated tests prove invalid direct state combinations are prevented or handled by transition methods.
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`
