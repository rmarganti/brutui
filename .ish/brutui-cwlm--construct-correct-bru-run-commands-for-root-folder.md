---
# brutui-cwlm
title: Construct correct bru run commands for root, folder, and request targets
status: completed
type: task
priority: high
tags:
- runner
- commands
created_at: 2026-05-29T17:28:16.846630Z
updated_at: 2026-05-29T17:56:28.655403Z
parent: brutui-6zk1
blocked_by:
- brutui-vzec
- brutui-s2xb
- brutui-owl5
- brutui-1z83
---

## Context

Brutui delegates execution correctness to `bru`. Command construction must align with selected tree nodes and environment choices while generating a temporary JSON report and not writing into collection dirs.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 32-37, 48-50 and Bruno command construction testing notes.

## Scope

- Build command argv for running the collection root, selected folder recursively, or selected request.
- Include selected environment via `--env` only when not using the explicit no-env option.
- Always request a temporary JSON reporter output outside the collection directory.
- Keep command construction testable without spawning processes.

## Validation

- Tests verify root, folder-recursive, and request argv; env/no-env mapping; temp report path placement; and no collection-local writes.



## Implementation Notes

- Implemented `brutui::runner::build_run_command` in `src/runner.rs` so command construction stays pure/testable and future process-runner work can consume a `RunCommand` without spawning during tests.
- Root and folder selections now map to `bru run <target> -r`, while request selections map to `bru run <request>` without recursive mode.
- Selected environments map to `--env <name>` only when `EnvironmentOption::cli_value` is present; the explicit no-env option omits the flag.
- Every built command now includes `--reporter-json <temp-path>` with a unique report path under the system temp directory so Brutui avoids writing into collection trees.
- Extended `tests/support/mod.rs` fake-`bru` argument parsing to recognize `--reporter-json`, which should make the upcoming process-runner ish easier to wire against the same harness.
- Added `tests/run_command_construction.rs` coverage for root/folder/request argv, env/no-env mapping, temp report placement, and fixture immutability.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `RunCommand.report_path` is already generated up front and returned alongside argv, so the upcoming process runner can keep that path as session state and classify completion results against the same file.
- If later manual/compatibility testing shows Bruno prefers a different JSON-report flag shape, only `build_run_command` and the fake-`bru` helper should need adjustment because the selection-to-command contract is now centralized.
