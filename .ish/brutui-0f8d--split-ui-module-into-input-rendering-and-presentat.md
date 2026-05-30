---
# brutui-0f8d
title: Split UI module into input rendering and presentation text
status: completed
type: task
priority: high
tags:
- ui
- architecture
- refactor
created_at: 2026-05-30T17:23:24.480800Z
updated_at: 2026-05-30T17:33:21.159929Z
parent: brutui-hrnz
blocking:
- brutui-ovb3
- brutui-0jl8
- brutui-u3c9
blocked_by:
- brutui-uamc
---

## Context

`src/ui.rs` is over 1100 lines and mixes terminal setup/teardown, key handling, layout/rendering, modal rendering, response text formatting, and render tests. This makes small UI changes risky and makes non-rendering behavior harder to test.

## Dependencies

- Blocked by `brutui-uamc` because state/shared interaction types should be moved before splitting the UI module.

## Work

Refactor without changing behavior:

- Split key handling into a focused module such as `src/ui/input.rs`.
- Split rendering/layout/widgets into a focused module such as `src/ui/render.rs`.
- Split response/details/raw-output string construction into a focused module such as `src/ui/text.rs` or `src/presentation.rs`.
- Keep terminal session setup/teardown in a small obvious location, e.g. `src/ui/terminal.rs` or the UI module root.
- Preserve public API compatibility where practical: callers should still be able to use `ui::render`, `ui::handle_key_event`, `ui::current_tab_text`, and `TerminalSession` unless a clearly better small API is introduced.
- Move tests next to the responsibility they verify or keep integration-style UI tests compiling with minimal churn.

## Verification

- No behavior changes to startup picker, session key handling, help/environment modals, run-result text, or render smoke behavior.
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`

## Implementation Notes

- Replaced the monolithic `src/ui.rs` module with `src/ui/mod.rs` plus focused `input`, `render`, `text`, and `terminal` submodules so key handling, rendering, presentation text, and terminal lifecycle each have a smaller seam.
- Kept the external UI API stable by re-exporting `ui::render`, `ui::handle_key_event`, `ui::current_tab_text`, `ui::TerminalSession`, and `ui::AppTerminal` from the new module root.
- Preserved the existing integration-style UI tests under `src/ui/mod.rs`, which keeps coverage for startup picker behavior, focus/key handling, modal behavior, and render smoke checks while avoiding churn in higher-level callers.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`
