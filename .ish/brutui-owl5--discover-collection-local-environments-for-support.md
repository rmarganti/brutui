---
# brutui-owl5
title: Discover collection-local environments for supported formats
status: todo
type: task
priority: normal
tags:
- environments
created_at: 2026-05-29T17:28:16.839239Z
updated_at: 2026-05-29T17:28:16.839239Z
parent: brutui-7sjr
blocked_by:
- brutui-s2xb
---

## Context

V1 supports collection-local environments only, with an explicit "no environment" option and no manual/global/workspace environment support.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 29-31 and environment discovery implementation/testing notes.

## Scope

- Discover environments according to the detected collection format.
- Expose a stable environment model with display name and value passed to `bru run --env`.
- Always include/select an explicit "no environment" option.
- Exclude unsupported global/workspace/manual environment concepts from v1.

## Validation

- Tests cover classic and YAML/OpenCollection environment discovery, unsupported file exclusion, no-env behavior, selected env mapping, and empty environment directories.
