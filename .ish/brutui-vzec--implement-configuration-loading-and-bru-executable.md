---
# brutui-vzec
title: Implement configuration loading and bru executable resolution
status: todo
type: task
priority: high
tags:
- config
- bru
created_at: 2026-05-29T17:28:16.824088Z
updated_at: 2026-05-29T17:28:16.824088Z
parent: brutui-3dxq
blocked_by:
- brutui-xxoo
- brutui-ui1v
---

## Context

Brutui must work without config from inside a collection, but support TOML config, configured collection directories, configured `bru` executable path, and an environment-variable override for scripting/testing.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 11-16 and implementation decisions for configuration/executable resolution.

## Scope

- Define platform-appropriate config lookup using a TOML file.
- Model configured collection directories and optional `bru` path.
- Normalize paths and return helpful diagnostics for invalid config.
- Implement executable resolution precedence: environment override first, config second, PATH lookup third.
- Make missing executable an actionable setup error, not a panic.

## Validation

- Unit tests cover missing config defaults, valid config, invalid TOML/paths diagnostics, configured executable precedence, env override precedence, PATH detection, and missing executable errors.
- Tests use temp config homes and fake executables.
