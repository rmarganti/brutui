---
# brutui-elma
title: End-to-end PRD acceptance pass, documentation, and release readiness
status: todo
type: task
priority: critical
tags:
- acceptance
- docs
created_at: 2026-05-29T17:28:16.870840Z
updated_at: 2026-05-29T17:28:16.870840Z
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
