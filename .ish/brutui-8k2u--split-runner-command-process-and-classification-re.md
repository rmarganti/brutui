---
# brutui-8k2u
title: Split runner command process and classification responsibilities
status: todo
type: task
priority: normal
tags:
- runner
- architecture
- refactor
created_at: 2026-05-30T17:24:03.256933Z
updated_at: 2026-05-30T17:24:03.256933Z
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
