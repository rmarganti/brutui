---
# brutui-vzec
title: Implement configuration loading and bru executable resolution
status: completed
type: task
priority: high
tags:
- config
- bru
created_at: 2026-05-29T17:28:16.824088Z
updated_at: 2026-05-29T17:39:51.257662Z
parent: brutui-3dxq
blocked_by:
- brutui-xxoo
- brutui-ui1v
---

## Context

Brutui must work without config from inside a collection, but support TOML config, configured collection directories, configured `bru` executable path, and an environment-variable override for scripting/testing.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 11-16 and implementation decisions for configuration/executable resolution.

## Scope

- Define platform-appropriate config lookup using a TOML file.
- Model configured collection directories and optional `bru` path.
- Normalize paths and return helpful diagnostics for invalid config.
- Implement executable resolution precedence: environment override first, config second, PATH lookup third.
- Make missing executable an actionable setup error, not a panic.

## Validation

- Unit tests cover missing config defaults, valid config, invalid TOML/paths diagnostics, configured executable precedence, env override precedence, PATH detection, and missing executable errors.
- Tests use temp config homes and fake executables.



## Implementation Notes

- Added `src/config.rs` config loading with platform-default TOML lookup, optional `BRUTUI_CONFIG` override, relative-path resolution against the config file, and normalized absolute paths for `collection_dirs` and `bru_path`.
- Added actionable `ConfigError` diagnostics for missing override files, TOML parse failures, invalid collection directories, and invalid configured `bru` paths.
- Added `src/executable.rs` executable resolution with `BRUTUI_BRU_PATH` override precedence, config-path precedence over PATH lookup, and explicit not-found/config-invalid errors instead of panics.
- Added `tests/config_and_executable.rs` coverage for default config behavior, temp-home config lookup, relative-path normalization, invalid TOML/path diagnostics, env/config/PATH resolution precedence, and missing-`bru` failures.
- Marked `tests/support/mod.rs` as shared dead-code-tolerant test support so Clippy can run per-test-target with `-D warnings`.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `brutui::config::load()` now returns both the normalized `AppConfig` and the resolved config path, so startup/discovery work can surface setup guidance without reimplementing path resolution.
- `brutui::executable::resolve()` is ready for discovery/startup and runner wiring; it already distinguishes env-override, config, and PATH sources for future diagnostics/UI messaging.
