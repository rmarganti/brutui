---
# brutui-iblr
title: Implement PRD-aligned collection root discovery
status: todo
type: task
priority: high
tags:
- collections
- discovery
created_at: 2026-05-29T17:28:16.828013Z
updated_at: 2026-05-29T17:28:16.828013Z
parent: brutui-7sjr
blocked_by:
- brutui-vzec
---

## Context

Discovery is the entrypoint to Brutui's promise: explicit launch path wins, cwd upward search wins over config, and configured directories are searched recursively with ignored irrelevant dirs. Roots must match Bruno CLI-compatible markers.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 1-13 and implementation decisions for collection discovery.

## Scope

- Support an explicit collection path launch option that overrides all automatic discovery.
- Search upward from cwd for collection roots.
- Search configured directories recursively to a bounded depth; configured dirs may be roots or containers.
- Detect classic roots by `bruno.json` and YAML/OpenCollection roots by `opencollection.yml`.
- Ignore common irrelevant directories such as `.git`, `node_modules`, build outputs, and dependency folders.
- Return zero/one/many discovery outcomes suitable for startup UI.

## Validation

- Tests cover explicit path precedence, cwd ancestry precedence, zero/one/many discovered collections, recursive bounded search, ignored dirs, and both root markers.
- Discovery never accepts directories that lack the PRD's Bruno-compatible markers.
