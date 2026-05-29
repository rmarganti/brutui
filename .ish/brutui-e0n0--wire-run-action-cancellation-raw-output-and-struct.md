---
# brutui-e0n0
title: Wire run action, cancellation, raw output, and structured results UI
status: todo
type: task
priority: critical
tags:
- ui
- runner
- results
created_at: 2026-05-29T17:28:16.867415Z
updated_at: 2026-05-29T17:28:16.867415Z
parent: brutui-carh
blocked_by:
- brutui-hlnk
- brutui-8r3c
---

## Context

The central interaction is pressing one key on root/folder/request to run, watching live output, optionally canceling, then inspecting structured results while raw output remains available.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 32-47.

## Scope

- Bind run action for request, folder recursive, and collection root targets.
- Disable or clearly handle run controls while a run is active.
- Bind cancel for active run.
- Stream stdout/stderr into the output pane live.
- Render structured post-run summary and failed requests/tests/assertions/errors.
- Keep raw output accessible after completion and expose latest temp report path/session context for debugging.
- Distinguish test failures from tool/execution errors.

## Validation

- Fake `bru` manual/automated flows cover success, failed tests (exit 1 + report), missing report tool error, stderr output, cancellation, and rejected overlapping run.
