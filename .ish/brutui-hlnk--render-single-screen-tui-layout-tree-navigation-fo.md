---
# brutui-hlnk
title: Render single-screen TUI layout, tree navigation, focus, and help overlay
status: completed
type: task
priority: high
tags:
- ui
- ratatui
created_at: 2026-05-29T17:28:16.860250Z
updated_at: 2026-05-29T18:18:54.354719Z
parent: brutui-carh
blocked_by:
- brutui-h39n
---

## Context

The PRD calls for a single-screen TUI with collection tree, inspection/details area, output/results areas, keyboard navigation, focus switching, lightweight modals, and help overlay.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 17-24 and UI rendering module notes.

## Scope

- Implement terminal setup/teardown using the chosen TUI stack.
- Render collection tree, details pane placeholder, output/results panes, status/footer, focus indication, and active modal layer.
- Implement keyboard navigation through the tree and focus switching between panes.
- Implement `?` help overlay and predictable quit behavior.
- Keep rendering tests minimal; prioritize state tests and smoke-level render checks.

## Validation

- Manual smoke: launch against fixtures and navigate tree/focus/help without terminal corruption.
- Automated tests cover key-to-state mapping where practical and at least one render smoke test for small/normal terminal sizes.


## Implementation Notes

- Replaced the placeholder `src/ui.rs` with a ratatui/crossterm UI module that now owns terminal enter/leave handling (`TerminalSession`), whole-screen rendering, and session-level key handling.
- Added a single-screen layout with a collection tree pane, details pane placeholder, output/results pane, footer hints, focus-aware borders, and centered modal rendering for help plus the future environment picker.
- Tree rendering now reflects stable collection-node identity from `AppState`, while keyboard handling maps `↑/↓` (and `j/k`) to tree navigation only when the tree is focused and `Tab` cycles focus across the three panes.
- Added predictable modal/quit behavior via `handle_key_event`: `?` opens/closes help, `Esc` closes modals or quits when no modal is open, and `q` always requests application exit.
- Added focused UI tests covering key-to-state mapping plus render smoke checks for both small and normal terminal sizes using `ratatui::backend::TestBackend`.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `ui::render()` already supports both startup-only and loaded-session states, so the startup-flow ish can plug discovery outcomes into the same renderer instead of introducing a separate screen model.
- `ui::handle_key_event()` intentionally keeps run/env-specific bindings minimal for now; the request-details/environment-picker and run/results ishes can extend that mapping in place without moving tree/help/quit behavior.
- `TerminalSession` is ready for `AppBootstrap` to own once startup discovery is wired into the real TUI loop.
