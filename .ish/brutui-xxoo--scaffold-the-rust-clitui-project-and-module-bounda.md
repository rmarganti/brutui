---
# brutui-xxoo
title: Scaffold the Rust CLI/TUI project and module boundaries
status: completed
type: task
priority: critical
tags:
- rust
- architecture
- setup
created_at: 2026-05-29T17:28:16.815579Z
updated_at: 2026-05-29T17:32:13.026887Z
parent: brutui-3dxq
---

## Context

Start Brutui from the empty repository. The architecture should make the PRD's recommended deep modules explicit so downstream work can land without reshaping the project.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, especially "Recommended deep modules", "Implementation Decisions", and "Testing Decisions".

## Scope

- Create a Cargo binary crate for `brutui`.
- Add initial module skeletons for configuration, executable resolution, collection discovery, collection model/scanner, metadata parsing, environment discovery, Bruno runner, report parsing, app state, and UI rendering.
- Choose and wire baseline dependencies for a Rust TUI/CLI stack (for example `ratatui`, `crossterm`, `clap`, `serde`, `toml`, `anyhow/thiserror`, `tempfile`, `tokio` or another async/process strategy if selected).
- Add a minimal `brutui --help` and placeholder launch path that fails gracefully until later ishes fill behavior.
- Establish test/fixture directories.

## Validation

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run -- --help` prints a useful command synopsis.
- No files are written inside any Bruno collection fixture during tests.



## Implementation Notes

- Scaffolded a single-crate Rust application with `src/lib.rs` exposing the PRD-aligned module boundaries: config, executable resolution, discovery, collection model/scanner, metadata, environments, runner, report parsing, app bootstrap, and UI frame state.
- Added baseline dependencies for CLI/TUI/config/process work: `clap`, `anyhow`, `thiserror`, `serde`, `serde_json`, `toml`, `directories`, `tempfile`, `tokio`, `crossterm`, `ratatui`, and `walkdir`.
- Implemented a minimal Clap-based `brutui` entrypoint plus placeholder startup bootstrap that cleanly errors instead of panicking until discovery/TUI execution is implemented.
- Established fixture roots at `tests/fixtures/classic/` and `tests/fixtures/opencollection/` for follow-on discovery/scanner tests.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run -- --help`
- `ish check`

## Notes For Follow-on Work

- Startup wiring currently lives in `src/app.rs::AppBootstrap`; extend that path for discovery and early setup diagnostics instead of replacing the CLI surface.
- The scanner and report modules are intentionally skeletal and should be filled in-place by downstream ishes so module paths stay stable.
- Current tests use temp directories only; fixture directories remain untouched, satisfying the no-collection-write guardrail at this stage.
