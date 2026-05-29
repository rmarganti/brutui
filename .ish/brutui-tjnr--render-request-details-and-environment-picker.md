---
# brutui-tjnr
title: Render request details and environment picker
status: todo
type: task
priority: normal
tags:
- ui
- environments
- metadata
created_at: 2026-05-29T17:28:16.863900Z
updated_at: 2026-05-29T17:28:16.863900Z
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
