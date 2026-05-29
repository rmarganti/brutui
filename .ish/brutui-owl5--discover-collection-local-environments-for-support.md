---
# brutui-owl5
title: Discover collection-local environments for supported formats
status: completed
type: task
priority: normal
tags:
- environments
created_at: 2026-05-29T17:28:16.839239Z
updated_at: 2026-05-29T17:49:02.619687Z
parent: brutui-7sjr
blocked_by:
- brutui-s2xb
---

## Context

V1 supports collection-local environments only, with an explicit "no environment" option and no manual/global/workspace environment support.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 29-31 and environment discovery implementation/testing notes.

## Scope

- Discover environments according to the detected collection format.
- Expose a stable environment model with display name and value passed to `bru run --env`.
- Always include/select an explicit "no environment" option.
- Exclude unsupported global/workspace/manual environment concepts from v1.

## Validation

- Tests cover classic and YAML/OpenCollection environment discovery, unsupported file exclusion, no-env behavior, selected env mapping, and empty environment directories.



## Implementation Notes

- Added `src/environments.rs::discover()` with format-aware collection-local environment discovery rooted at `<collection>/environments` and always returning an explicit `No environment` option first.
- Classic collections now surface only direct `environments/*.bru` files, using the filename stem for both display and CLI value.
- OpenCollection/YAML collections now surface only direct `environments/*.yml`/`*.yaml` files, using the filename stem as the stable CLI value and the top-level `name:` field as the preferred display label when present.
- Unsupported file types and nested environment files/directories are ignored so v1 stays collection-local and format-scoped.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- Runner command construction can pass `EnvironmentOption::cli_value` directly to `bru run --env` and omit the flag when it is `None`.
- The discovery contract currently treats environment lookup as shallow (`<collection>/environments/*`) and non-recursive for both supported formats.
