---
# brutui-8k2u
title: Split runner command process and classification responsibilities
status: completed
type: task
priority: normal
tags:
- runner
- architecture
- refactor
created_at: 2026-05-30T17:24:03.256933Z
updated_at: 2026-05-30T17:59:19.069618Z
parent: brutui-hrnz
blocked_by:
- brutui-ev42
---

## Context

`src/runner.rs` currently combines command construction, temp report path generation, concrete process spawning, stdout/stderr reader threads, cancellation, report parsing, and completion classification. This is still manageable, but future work like run history, timeouts, configurable report paths, retries, or async execution will be harder unless these responsibilities are separated.

## Dependencies

- Blocked by `brutui-ev42` so the baseline is clean.
- Coordinate with `brutui-4gfn`; if app dependency ports introduce a runner trait first, use that seam rather than inventing a competing abstraction.

## Work

- Preserve the public run behavior while splitting responsibilities into smaller units/modules, for example:
  - run command construction
  - report path creation
  - process execution/event streaming/cancellation
  - completion/report classification
- Keep `build_run_command` pure and unit-testable.
- Make completion classification independently testable without spawning processes or writing collection files.
- Avoid changing the external Bruno CLI contract unless tests prove the existing contract is wrong.

## Verification

- Existing command construction and runner process integration tests pass.
- Add or retain unit tests for command construction and completion classification that do not spawn processes.
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`

## Implementation Notes

- Split `src/runner.rs` into focused internal modules: `runner::command` for pure command construction, `runner::report_path` for temp report naming, `runner::process` for concrete process/event streaming, and `runner::completion` for exit/report classification.
- Kept the public runner API stable (`ProcessRunner`, `build_run_command`, run event/completion types) so app/controller code did not need behavioral changes while the internals became smaller and easier to test in isolation.
- Added completion-classification unit coverage that exercises success, failed-tests, missing-report, invalid-report, non-zero-exit, and cancellation paths without spawning Bruno processes.
- Retained the existing process-level integration coverage in `tests/runner_process.rs` and command-construction coverage in `tests/run_command_construction.rs` to prove the refactor preserved the Bruno CLI contract and live event behavior.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- If future runner work needs timeouts, retries, alternate report destinations, or async execution, extend the focused `runner::process`, `runner::report_path`, or `runner::completion` modules rather than re-centralizing those concerns in the root module.
- `runner::completion::classify_completion(...)` is now the narrow seam for behavior-only tests around exit-code/report interpretation; keep new classification rules there so they remain testable without process spawning.
