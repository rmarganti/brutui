---
# brutui-ev42
title: Establish pre-feature quality gate and fix hygiene drift
status: completed
type: task
priority: high
tags:
- quality
- docs
- ci
created_at: 2026-05-30T17:23:08.125647Z
updated_at: 2026-05-30T17:26:36.657092Z
parent: brutui-hrnz
blocking:
- brutui-uamc
- brutui-4gfn
- brutui-8k2u
---

## Context

Before structural refactors, establish a clean verification baseline. Current `cargo test` passes, but `cargo clippy --all-targets --all-features -- -D warnings` fails on redundant closures in `src/report.rs`. README keybinding documentation has also drifted from code: README describes `1/2/3` as summary/failures/raw, while the code uses body/headers/tests. `Cargo.toml` also includes `tokio` even though current code does not reference it.

## Dependencies

None. This should be completed first so later refactors have a trustworthy quality gate.

## Work

- Fix the current Clippy failures without changing report parsing behavior.
- Decide whether unused `tokio` is intentionally reserved for near-term async runner work. If not, remove it from `Cargo.toml`/`Cargo.lock`; if it stays, document why in the relevant code or issue body.
- Update README keybinding text so it matches the current UI (`1` body, `2` headers, `3` tests, `[/]` result navigation, `y` copy if present).
- Add or document a single local validation command/workflow for future agents, e.g. a script, make target, or explicit README/contributor section containing:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-targets --all-features`
  - `ish check`

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`



## Implementation Notes

- Replaced the redundant `and_then(|map| first_body(map))` closures in `src/report.rs` with direct function pointers so Clippy passes without changing report parsing semantics.
- Removed the unused `tokio` dependency from `Cargo.toml` and refreshed `Cargo.lock`; current runner/app code does not reference Tokio yet, so keeping it would add unused maintenance surface.
- Updated `README.md` loaded-session keybindings to match the current UI (`1` body, `2` headers, `3` tests, `[`/`]` request navigation, `y` copy).
- Added `scripts/validate.sh` and documented it in `README.md` as the single local quality-gate entrypoint for future agents.

## Verification

- `./scripts/validate.sh`
