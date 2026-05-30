---
# brutui-ovb3
title: Make render-time scroll measurement explicit
status: todo
type: task
priority: normal
tags:
- ui
- state
- testability
created_at: 2026-05-30T17:23:32.843643Z
updated_at: 2026-05-30T17:23:32.843643Z
parent: brutui-hrnz
blocked_by:
- brutui-0f8d
---

## Context

The current render path mutates session state: `render(frame, state: &mut AppState)` eventually calls `session.set_output_scroll_metrics(...)` during `render_output_pane`. This makes rendering non-pure and couples viewport measurement to domain/session state. It complicates snapshot-style render tests and will interfere with richer pane/layout work.

## Dependencies

- Blocked by `brutui-0f8d` so rendering is isolated before changing how render measurements flow.

## Work

- Make render-time viewport/content measurement explicit instead of hidden mutation inside widget rendering.
- Choose a small design, for example:
  - rendering returns `ViewMeasurements` that the controller applies after draw, or
  - the controller computes/updates scroll metrics before calling pure render functions, or
  - scroll metrics move into a dedicated view-state layer clearly separate from domain/run state.
- Preserve current scrolling behavior for output focus, page up/down, home/end, and tab changes.
- Add tests around scroll clamping/viewport updates so this contract is explicit.

## Verification

- Existing output scrolling tests/flows still pass.
- Add or update focused tests proving render/measurement behavior is deterministic and scroll offsets clamp correctly after viewport/content changes.
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`
