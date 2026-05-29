---
# brutui-tjnr
title: Render request details and environment picker
status: completed
type: task
priority: normal
tags:
- ui
- environments
- metadata
created_at: 2026-05-29T17:28:16.863900Z
updated_at: 2026-05-29T18:39:19.401870Z
parent: brutui-carh
blocked_by:
- brutui-hlnk
- brutui-owl5
- brutui-g1xq
---

## Context

Users need filenames/paths always, shallow metadata when available, tags read-only, and a collection-local environment picker with explicit no-env.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 23-31.

## Scope

- Populate details pane from selected root/folder/request node.
- Show filename/path as minimum request information.
- Show parsed name, method, URL, and tags when available, with parse limitations visible but non-blocking.
- Implement environment picker modal and selected environment display.
- Do not support editing tags/environments or manual/global environments.

## Validation

- Tests or render smoke checks verify minimum display on malformed requests, optional metadata display, tag read-only display, env picker navigation/selection/cancel, and no-env option.

## Implementation Notes

- Extended `src/ui.rs` details rendering so every selected node shows path plus the currently selected environment, while request nodes additionally show best-effort request name, method/URL, read-only tags, and metadata diagnostics without hiding malformed requests.
- Added an interactive environment picker flow on `e` in `src/app.rs`, keeping selection/cancel behavior in existing modal state transitions and making the active environment visible both in the details pane and footer.
- Improved environment-picker and help rendering to show the current selection and the new keyboard affordance without expanding scope into environment editing.
- Added focused UI coverage in `src/ui.rs` plus flow coverage in `tests/run_ui_flow.rs` for metadata/tags/diagnostics rendering, explicit no-env display, picker open/navigate/confirm/cancel behavior, and persisted environment selection.

## Verification

- `cargo fmt --all`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- The environment picker is currently opened with `e` from the loaded-session controller path; if future UX wants pane-local shortcuts or mouse support, keep that wiring in `AppController` so modal state remains UI-agnostic.
- Request details intentionally stay shallow and read-only; richer inspection should extend the existing metadata display rather than making scanner visibility depend on parse completeness.
