---
# brutui-8r3c
title: Implement process runner with live stdout/stderr, completion interpretation, and cancellation
status: todo
type: task
priority: critical
tags:
- runner
- process
created_at: 2026-05-29T17:28:16.850056Z
updated_at: 2026-05-29T17:28:16.850056Z
parent: brutui-6zk1
blocked_by:
- brutui-cwlm
---

## Context

The runner is the execution boundary. It must stream Bruno CLI output live, keep one active run at a time, support cancellation, and interpret Bruno exit codes/reports exactly as the PRD specifies.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 36-47 and Bruno runner/exit-code testing notes.

## Scope

- Launch the resolved `bru` process with argv from command construction.
- Emit stdout/stderr events as data arrives.
- Track a single active subprocess and reject or clearly report overlapping run attempts.
- Cancel/kill the active process on user request.
- On completion, parse the temp JSON report and classify: exit 0 + valid report = success, exit 1 + valid report = completed with failures, other non-zero/missing/unparseable report = execution/tool error.
- Keep the latest temp report during the session.

## Validation

- Tests with fake processes cover stdout/stderr streaming, success, exit-code-1 failure reports, other non-zero tool errors, missing/unparseable reports, cancellation, and concurrent-run rejection.
