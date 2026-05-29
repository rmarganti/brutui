---
# brutui-1z83
title: Parse Bruno JSON reports into a result model
status: todo
type: task
priority: high
tags:
- reports
created_at: 2026-05-29T17:28:16.843046Z
updated_at: 2026-05-29T17:28:16.843046Z
parent: brutui-6zk1
blocked_by:
- brutui-xxoo
- brutui-ui1v
---

## Context

Every run should produce a Bruno JSON reporter output and the UI should show structured summaries and failure details after completion.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 37-44 and report parser module notes.

## Scope

- Define Rust models for run summary, request results, failed tests/assertions/errors, and parse diagnostics.
- Parse successful, failed, and partial Bruno JSON reports defensively.
- Preserve enough raw/report path context to let the UI expose debugging information for the latest temp report.
- Treat missing required semantics as parse errors distinct from API/test failures.

## Validation

- Tests cover success reports, failed request/test/assertion reports, missing optional fields, partial reports, malformed JSON, and parse failure diagnostics.
