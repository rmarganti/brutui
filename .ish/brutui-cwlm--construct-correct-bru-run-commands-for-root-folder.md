---
# brutui-cwlm
title: Construct correct bru run commands for root, folder, and request targets
status: todo
type: task
priority: high
tags:
- runner
- commands
created_at: 2026-05-29T17:28:16.846630Z
updated_at: 2026-05-29T17:28:16.846630Z
parent: brutui-6zk1
blocked_by:
- brutui-vzec
- brutui-s2xb
- brutui-owl5
- brutui-1z83
---

## Context

Brutui delegates execution correctness to `bru`. Command construction must align with selected tree nodes and environment choices while generating a temporary JSON report and not writing into collection dirs.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 32-37, 48-50 and Bruno command construction testing notes.

## Scope

- Build command argv for running the collection root, selected folder recursively, or selected request.
- Include selected environment via `--env` only when not using the explicit no-env option.
- Always request a temporary JSON reporter output outside the collection directory.
- Keep command construction testable without spawning processes.

## Validation

- Tests verify root, folder-recursive, and request argv; env/no-env mapping; temp report path placement; and no collection-local writes.
