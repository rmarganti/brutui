---
# brutui-h39n
title: Implement application state transitions for selection, environments, runs, and focus
status: todo
type: task
priority: high
tags:
- state
created_at: 2026-05-29T17:28:16.853461Z
updated_at: 2026-05-29T17:28:16.853461Z
parent: brutui-carh
blocked_by:
- brutui-s2xb
- brutui-g1xq
- brutui-owl5
- brutui-8r3c
---

## Context

The UI should be thin over a well-tested state machine: current collection, selected node, selected environment, focused pane, modal state, run state, raw output, and result summary.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 17-22, 29-47 and application state module/testing notes.

## Scope

- Model startup states, current collection, tree selection, details, selected environment, focus, modal picker/help state, active run, raw output, latest report, and completed results.
- Implement transitions for tree navigation, focus switching, env selection, run start/finish/error/cancel, output events, result tab/view changes, and disabled run controls while active.
- Keep business rules independent from terminal rendering where practical.

## Validation

- Unit tests cover selection movement, env changes, focus changes, run start/finish, exit-code classifications as surfaced to state, cancellation, raw output retention, latest report retention, and prevention of overlapping runs.
