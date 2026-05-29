---
# brutui-e0n0
title: Wire run action, cancellation, raw output, and structured results UI
status: completed
type: task
priority: critical
tags:
- ui
- runner
- results
created_at: 2026-05-29T17:28:16.867415Z
updated_at: 2026-05-29T18:25:21.246868Z
parent: brutui-carh
blocked_by:
- brutui-hlnk
- brutui-8r3c
---

## Context

The central interaction is pressing one key on root/folder/request to run, watching live output, optionally canceling, then inspecting structured results while raw output remains available.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 32-47.

## Scope

- Bind run action for request, folder recursive, and collection root targets.
- Disable or clearly handle run controls while a run is active.
- Bind cancel for active run.
- Stream stdout/stderr into the output pane live.
- Render structured post-run summary and failed requests/tests/assertions/errors.
- Keep raw output accessible after completion and expose latest temp report path/session context for debugging.
- Distinguish test failures from tool/execution errors.

## Validation

- Fake `bru` manual/automated flows cover success, failed tests (exit 1 + report), missing report tool error, stderr output, cancellation, and rejected overlapping run.


## Implementation Notes

- Added `app::AppController` as the run-wiring seam between UI key handling, `AppState`, and `ProcessRunner`, with `r` to launch the selected root/folder/request target, `c` to cancel, `1/2/3` to switch summary/failures/raw views, and non-fatal stderr feedback when users try to start an overlapping run or cancel while idle.
- Kept rendering/state responsibilities separated: controller code owns process lifecycle and event pumping, while `src/ui.rs` now renders structured summary, failure-detail, and raw-output views from `SessionState` without reparsing runner output.
- Output rendering now distinguishes success, completed-with-failures, cancellation, and tool/report errors in both summary and failures views, while retaining raw stdout/stderr plus the latest report path for post-run debugging.
- Updated the details/help/footer copy so the run/cancel/view-switch bindings are visible in-session.
- Added `tests/run_ui_flow.rs` coverage for success with stdout/stderr streaming, failed-test reports, missing-report tool errors, cancellation, and rejected overlapping run requests using fake `bru` executables and fixture collections.

## Verification

- `cargo fmt --all`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `ish check`

## Notes For Follow-on Work

- `AppController::pump_run_events()` is the current integration point for any future real terminal event loop once the startup-flow ish loads collections and enters the TUI.
- Startup work can construct `AppController` after discovery/executable resolution instead of duplicating run wiring.
- Result-view keybindings are currently `1/2/3`; if later UX work prefers tabs or pane-local navigation, the controller/render split should make that easy to remap without touching runner semantics.
