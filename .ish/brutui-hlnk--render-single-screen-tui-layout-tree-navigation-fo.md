---
# brutui-hlnk
title: Render single-screen TUI layout, tree navigation, focus, and help overlay
status: todo
type: task
priority: high
tags:
- ui
- ratatui
created_at: 2026-05-29T17:28:16.860250Z
updated_at: 2026-05-29T17:28:16.860250Z
parent: brutui-carh
blocked_by:
- brutui-h39n
---

## Context

The PRD calls for a single-screen TUI with collection tree, inspection/details area, output/results areas, keyboard navigation, focus switching, lightweight modals, and help overlay.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 17-24 and UI rendering module notes.

## Scope

- Implement terminal setup/teardown using the chosen TUI stack.
- Render collection tree, details pane placeholder, output/results panes, status/footer, focus indication, and active modal layer.
- Implement keyboard navigation through the tree and focus switching between panes.
- Implement `?` help overlay and predictable quit behavior.
- Keep rendering tests minimal; prioritize state tests and smoke-level render checks.

## Validation

- Manual smoke: launch against fixtures and navigate tree/focus/help without terminal corruption.
- Automated tests cover key-to-state mapping where practical and at least one render smoke test for small/normal terminal sizes.
