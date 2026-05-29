---
# brutui-ui1v
title: Create Bruno fixture collections and test harness utilities
status: todo
type: task
priority: critical
tags:
- tests
- fixtures
created_at: 2026-05-29T17:28:16.819767Z
updated_at: 2026-05-29T17:28:16.819767Z
parent: brutui-dz57
blocked_by:
- brutui-xxoo
---

## Context

The PRD requires behavior-focused tests using temporary directories, fixtures for classic and YAML/OpenCollection formats, and fake `bru` executables instead of network or globally installed Bruno.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, "Testing Decisions".

## Scope

- Add classic collection fixtures rooted by `bruno.json` with nested folders, `.bru` requests, environments, tags, malformed request examples, and ignorable directories.
- Add YAML/OpenCollection fixtures rooted by `opencollection.yml` with representative request/environment files.
- Add test helpers for temp workspaces, cwd-sensitive discovery, fixture copying, fake `bru` executables, and report temp files.
- Ensure fixture helpers can assert that collection directories remain unchanged after runs.

## Validation

- Tests can create isolated temp collections for both formats.
- A fake `bru` can emit stdout/stderr, exit with configurable codes, and write configurable JSON reports.
- The harness supports tests without network access and without a globally installed Bruno CLI.
