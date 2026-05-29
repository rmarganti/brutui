---
# brutui-h39n
title: Implement application state transitions for selection, environments, runs, and focus
status: completed
type: task
priority: high
tags:
- state
created_at: 2026-05-29T17:28:16.853461Z
updated_at: 2026-05-29T17:28:16.853461Z
parent: brutui-carh
blocked_by:
- brutui-s2xb
- brutui-g1xq
- brutui-owl5
- brutui-8r3c
---

## Context

The UI should be thin over a well-tested state machine: current collection, selected node, selected environment, focused pane, modal state, run state, raw output, and result summary.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 17-22, 29-47 and application state module/testing notes.

## Scope

- Model startup states, current collection, tree selection, details, selected environment, focus, modal picker/help state, active run, raw output, latest report, and completed results.
- Implement transitions for tree navigation, focus switching, env selection, run start/finish/error/cancel, output events, result tab/view changes, and disabled run controls while active.
- Keep business rules independent from terminal rendering where practical.

## Validation

- Unit tests cover selection movement, env changes, focus changes, run start/finish, exit-code classifications as surfaced to state, cancellation, raw output retention, latest report retention, and prevention of overlapping runs.

## Implementation Notes

- Added a new `src/state.rs` application-state module with startup state, collection picker state, loaded session state, modal state, focus, run lifecycle, raw output capture, latest report retention, and completed-run classification.
- Kept business rules separate from rendering by modeling pure transitions for tree movement, focus cycling, help/env modal behavior, environment selection, run start/cancel/finish, and result-view switching.
- Surface runner outcomes into UI-friendly state via `CompletedRunStatus`, preserving the distinction between success, completed-with-failures, cancellation, and tool/report errors without forcing the renderer to reinterpret exit codes.
- Added focused unit coverage for collection picker filtering, selection edge-clamping, environment changes, focus movement, overlapping-run rejection, cancellation, completion classification, and retention of raw output plus the latest report path.

## Verification

- `cargo test --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `AppState::show_collection_picker` and the embedded `CollectionPickerState` are ready for the startup-flow ish to wire zero/one/many discovery outcomes into a searchable picker without mixing discovery logic into rendering.
- `SessionState::selected_node`, `selected_environment`, `result_view`, and `completed_run` provide the core read model the upcoming TUI/layout and details-pane ishes can render directly.
- `start_run_on_selected_node`, `append_stdout`/`append_stderr`, `request_run_cancellation`, and `finish_run` already encode the one-active-run guardrail and latest-report retention expected by the run/results UI work.
