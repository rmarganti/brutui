---
# brutui-elma
title: End-to-end PRD acceptance pass, documentation, and release readiness
status: completed
type: task
priority: critical
tags:
- acceptance
- docs
created_at: 2026-05-29T17:28:16.870840Z
updated_at: 2026-05-29T18:43:36.559647Z
parent: brutui-dz57
blocked_by:
- brutui-fe0z
- brutui-tjnr
- brutui-e0n0
---

## Context

Before v1 is considered complete, verify the implementation still matches the PRD and has not drifted into out-of-scope behavior.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, entire document, especially "Out of Scope" and "Further Notes".

## Scope

- Add concise README usage covering launch from collection, configured discovery dirs, `bru` resolution, env selection, run/cancel, and result inspection.
- Add configuration example TOML.
- Run an end-to-end manual pass using fixture/fake `bru` and, optionally, a real Bruno CLI if installed.
- Audit that Brutui does not write project-local files into collection directories during normal operation.
- Audit that v1 does not include out-of-scope editing/global env/tag filtering/concurrent run features.

## Validation

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- README/config examples are accurate.
- `ish check` passes.
- A short acceptance note is appended to this ish documenting the manual PRD pass.

## Implementation Notes

- Added `README.md` with v1 scope, launch flows, `bru` resolution precedence, config guidance, keybindings, and explicit notes about read-only/out-of-scope behavior.
- Added `config/brutui.example.toml` so future users and packagers have a concrete config template for `collection_dirs` and optional `bru_path`.
- Updated the CLI `--help` long description in `src/cli.rs` so the binary synopsis matches the implemented feature set instead of the initial scaffold wording.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `cargo run -- --help`
- `ish check`

## Acceptance Note

- Ran a pseudo-TTY smoke pass against `tests/fixtures/classic/sample-classic` with a fake `bru` executable, launched via explicit collection path, triggered a run from the TUI, and exited cleanly.
- Verified the fake `bru` was invoked with `--reporter-json`, confirming report generation stays in the Bruno CLI boundary instead of collection-local writes.
- Compared fixture file hashes before/after the smoke pass; the collection tree was unchanged.
- Audited the shipped v1 surface against the PRD out-of-scope list: the app remains read-only and does not implement editing, global/workspace/manual environments, tag-filtered runs, or concurrent/queued runs.

## Notes For Follow-on Work

- If Brutui is packaged beyond source builds, keep `README.md`, `config/brutui.example.toml`, and `src/cli.rs` help text in sync so install docs do not drift from the actual runtime behavior.
- The acceptance smoke harness used here is ad hoc; if startup/run regressions become common, promoting it into a checked-in PTY smoke script would provide stronger release confidence.
