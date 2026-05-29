---
# brutui-ui1v
title: Create Bruno fixture collections and test harness utilities
status: completed
type: task
priority: critical
tags:
- tests
- fixtures
created_at: 2026-05-29T17:28:16.819767Z
updated_at: 2026-05-29T17:35:58.288891Z
parent: brutui-dz57
blocked_by:
- brutui-xxoo
---

## Context

The PRD requires behavior-focused tests using temporary directories, fixtures for classic and YAML/OpenCollection formats, and fake `bru` executables instead of network or globally installed Bruno.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, "Testing Decisions".

## Scope

- Add classic collection fixtures rooted by `bruno.json` with nested folders, `.bru` requests, environments, tags, malformed request examples, and ignorable directories.
- Add YAML/OpenCollection fixtures rooted by `opencollection.yml` with representative request/environment files.
- Add test helpers for temp workspaces, cwd-sensitive discovery, fixture copying, fake `bru` executables, and report temp files.
- Ensure fixture helpers can assert that collection directories remain unchanged after runs.

## Validation

- Tests can create isolated temp collections for both formats.
- A fake `bru` can emit stdout/stderr, exit with configurable codes, and write configurable JSON reports.
- The harness supports tests without network access and without a globally installed Bruno CLI.


## Implementation Notes

- Added reusable integration-test helpers in `tests/support/mod.rs` for temp workspaces, scoped cwd/env changes, fixture copying, fake `bru` executables, report-file paths, and collection immutability snapshots.
- Added representative classic and OpenCollection fixture trees under `tests/fixtures/` with root markers, nested requests, collection-local environments, malformed request examples, tags/metadata hints, and ignorable directories like `node_modules`/`target`.
- Added `tests/fixture_harness.rs` coverage proving fixture isolation, cwd restoration, fake `bru` stdout/stderr + exit/report behavior, scoped env restoration, and no-write assertions against copied collections.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `ish check`

## Notes For Follow-on Work

- `copy_fixture_collection("classic", "sample-classic")` and `copy_fixture_collection("opencollection", "sample-open")` are ready for discovery/scanner/config tests without mutating the repo fixtures.
- `install_fake_bru` currently recognizes `--report-file`, `--report-path`, and `--output` so future runner tests can evolve command naming without replacing the harness.
- Use `TempCollection::assert_unchanged()` to enforce the PRD guardrail that Brutui does not write inside Bruno collection directories.
