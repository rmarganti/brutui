---
# brutui-fe0z
title: Implement startup flow, collection picker, and setup messaging
status: todo
type: task
priority: high
tags:
- ui
- startup
created_at: 2026-05-29T17:28:16.856780Z
updated_at: 2026-05-29T17:28:16.856780Z
parent: brutui-carh
blocked_by:
- brutui-h39n
- brutui-iblr
---

## Context

Startup must be fast and predictable: explicit path opens directly, cwd discovery is frictionless, one collection opens automatically, many collections show a searchable picker, and no discovery shows helpful setup guidance.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 1-13 and startup collection selection implementation decisions.

## Scope

- Wire CLI launch options into discovery precedence.
- Auto-open exactly one discovered collection.
- Render/search a lightweight collection picker when multiple collections are discovered.
- Show actionable setup messaging when no collection/config is found or `bru` cannot be resolved.
- Ensure config is optional for inside-collection launch.

## Validation

- Tests cover explicit/cwd/config precedence at startup, one collection auto-open, many collection picker state/search, no collection messaging, and missing `bru` messaging.
