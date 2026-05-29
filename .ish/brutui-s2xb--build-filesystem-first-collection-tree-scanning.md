---
# brutui-s2xb
title: Build filesystem-first collection tree scanning
status: todo
type: task
priority: high
tags:
- collections
- tree
created_at: 2026-05-29T17:28:16.832070Z
updated_at: 2026-05-29T17:28:16.832070Z
parent: brutui-7sjr
blocked_by:
- brutui-iblr
---

## Context

The PRD requires a collection tree pane that reliably shows folders and runnable requests without depending on deep Bruno internals. The tree is filesystem-first and must preserve paths needed for `bru run` targets.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 17-25 and collection scanner module notes.

## Scope

- Define in-memory collection, folder, request, and node selection models.
- Recursively scan collection contents into a stable, deterministic tree.
- Include collection root, folders, and request files for supported formats.
- Preserve display names, filenames, relative paths, and absolute paths needed by runner command construction.
- Keep malformed requests visible/runnable; scanner must not fail on metadata parse errors.

## Validation

- Tests verify nested folder/request tree construction, deterministic ordering, root/folder/request node identity, path preservation, and resilience to malformed request files.
