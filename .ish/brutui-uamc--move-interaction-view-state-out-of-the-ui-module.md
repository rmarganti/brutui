---
# brutui-uamc
title: Move interaction view state out of the UI module
status: completed
type: task
priority: high
tags:
- state
- architecture
- ui
created_at: 2026-05-30T17:23:15.650952Z
updated_at: 2026-05-30T17:28:11.309030Z
parent: brutui-hrnz
blocking:
- brutui-0f8d
- brutui-0jl8
blocked_by:
- brutui-ev42
---

## Context

`src/state.rs` currently imports `FocusPane` from `crate::ui::FocusPane`, so core application/session state depends on the UI rendering module. This dependency direction makes state harder to test in isolation and will get worse as `ui.rs` is split into rendering/input/presentation modules.

## Dependencies

- Blocked by `brutui-ev42` so the quality gate is clean before behavior-preserving refactors begin.

## Work

- Move `FocusPane` out of `src/ui.rs` into `src/state.rs` or a small neutral module such as `src/view_state.rs`.
- Update imports across app, UI, and tests so the dependency direction is UI -> state/shared, not state -> UI.
- Preserve existing focus behavior and keybindings exactly.
- Keep the change narrow; do not split the entire UI module in this ish.

## Verification

- Unit tests covering focus cycling still pass.
- UI key mapping/render smoke tests still pass.
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`



## Implementation Notes

- Moved `FocusPane` from `src/ui.rs` into `src/state.rs` so session/application state no longer depends on the UI module for interaction view state.
- Updated `src/ui.rs` and state/UI tests to import `FocusPane` from `crate::state`, preserving the existing focus enum shape and all current keybinding behavior.
- Kept the refactor intentionally narrow so follow-on UI splitting work can now proceed with dependency direction `ui -> state` instead of `state -> ui`.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`
