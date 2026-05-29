---
# brutui-g1xq
title: Implement best-effort shallow request metadata parsing
status: todo
type: task
priority: normal
tags:
- metadata
- parser
created_at: 2026-05-29T17:28:16.835649Z
updated_at: 2026-05-29T17:28:16.835649Z
parent: brutui-7sjr
blocked_by:
- brutui-s2xb
---

## Context

Request inspection in v1 is shallow and read-only. Parsing must be native Rust, best-effort, and never block runnability.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 23-28 and best-effort metadata parser module notes.

## Scope

- Parse common classic `.bru` request metadata for name, method, URL, and tags where available.
- Parse representative YAML/OpenCollection request metadata for name, method, URL, and tags where available.
- Return partial metadata with diagnostics instead of hard failures.
- Integrate metadata with the scanner/details model while retaining filename/path as the minimum display contract.

## Validation

- Tests cover common classic and YAML/OpenCollection cases, missing fields, tags, malformed files, unsupported shapes, and regression that parse failure does not remove or disable a request node.
