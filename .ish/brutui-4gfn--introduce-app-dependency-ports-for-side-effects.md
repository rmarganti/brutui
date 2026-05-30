---
# brutui-4gfn
title: Introduce app dependency ports for side effects
status: todo
type: task
priority: high
tags:
- app
- testability
- architecture
created_at: 2026-05-30T17:23:41.926183Z
updated_at: 2026-05-30T17:24:20.626982Z
parent: brutui-hrnz
blocking:
- brutui-0jl8
blocked_by:
- brutui-ev42
---

## Context

`AppBootstrap` and `AppController` directly call process-global or concrete side effects: environment/config loading, `std::env::current_dir`, discovery, scanning, executable resolution, clipboard, crossterm event loop integration, and a concrete `ProcessRunner`. Tests therefore mutate cwd/env and duplicate guard helpers. As features grow, this will make controller/bootstrap behavior harder to test without real globals, threads, or terminal state.

## Dependencies

- Blocked by `brutui-ev42` so the baseline is clean before changing app seams.

## Work

- Introduce small dependency boundaries/ports for app-level side effects. Keep them minimal and Rust-idiomatic; avoid a large framework.
- At minimum, make controller tests able to use a fake runner and fake clipboard without invoking `ProcessRunner` or `arboard`.
- Make startup/bootstrap tests able to avoid direct global env/cwd mutation where practical by injecting current-dir/config/discovery inputs, or centralize the remaining process-state guard in shared test support.
- Preserve existing startup behavior, missing-`bru` messaging, run/cancel behavior, and copy behavior.
- Remove duplicated env/cwd test helper implementations if dependency injection or shared support makes them unnecessary.

## Verification

- Existing `src/app.rs` unit tests and `tests/run_ui_flow.rs` still pass.
- Add at least one controller-level test using a fake runner/clipboard or equivalent injected dependency to prove the seam works.
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`
