---
# brutui-8r3c
title: Implement process runner with live stdout/stderr, completion interpretation, and cancellation
status: completed
type: task
priority: critical
tags:
- runner
- process
created_at: 2026-05-29T17:28:16.850056Z
updated_at: 2026-05-29T18:02:20.457979Z
parent: brutui-6zk1
blocked_by:
- brutui-cwlm
---

## Context

The runner is the execution boundary. It must stream Bruno CLI output live, keep one active run at a time, support cancellation, and interpret Bruno exit codes/reports exactly as the PRD specifies.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 36-47 and Bruno runner/exit-code testing notes.

## Scope

- Launch the resolved `bru` process with argv from command construction.
- Emit stdout/stderr events as data arrives.
- Track a single active subprocess and reject or clearly report overlapping run attempts.
- Cancel/kill the active process on user request.
- On completion, parse the temp JSON report and classify: exit 0 + valid report = success, exit 1 + valid report = completed with failures, other non-zero/missing/unparseable report = execution/tool error.
- Keep the latest temp report during the session.

## Validation

- Tests with fake processes cover stdout/stderr streaming, success, exit-code-1 failure reports, other non-zero tool errors, missing/unparseable reports, cancellation, and concurrent-run rejection.


## Implementation Notes

- Expanded `src/runner.rs` from pure argv construction into a `ProcessRunner` that launches `bru`, streams stdout/stderr line events on background reader threads, tracks one active run, and supports cancellation via a run-local signal channel.
- Added explicit completion classification types (`RunOutcome`, `RunToolError`, `RunToolErrorKind`) so downstream state/UI work can distinguish success, failed-test completions, cancellation, and tool/report failures without reparsing subprocess state.
- The runner now remembers the latest temp report path as soon as a run starts and clears only the active-run slot after completion, matching the PRD requirement to keep the latest report available for the session.
- Exit handling is PRD-aligned: exit `0` + valid report => `Success`, exit `1` + valid report => `CompletedWithFailures`, missing/unparseable reports => tool errors, and other non-zero exits => tool errors even if a report exists.
- Added `tests/runner_process.rs` coverage for live stdout/stderr delivery, success, exit-code-1 failure reports, other non-zero exits, missing reports, invalid reports, cancellation, and overlapping-run rejection using fake shell `bru` executables.
- Hardened process-global test helpers with `tests/support::lock_process_state()` and used it in env/cwd-sensitive tests to eliminate parallel-test flakiness around `HOME`, `PATH`, and current-directory mutation.

## Verification

- `cargo test --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `ProcessRunner::start()` currently returns a standard `mpsc::Receiver<RunEvent>` and uses line-buffered output events; UI/state wiring can consume that directly without needing async runtime ownership.
- `RunCompletion.report_path` and `ProcessRunner::latest_report_path()` are ready for future output/debug panes that expose the most recent JSON report location.
- Cancellation is modeled distinctly from tool errors via `RunOutcome::Cancelled`, which should let upcoming state/UI ishes disable/re-enable run controls cleanly without conflating user intent with Bruno/report failures.
