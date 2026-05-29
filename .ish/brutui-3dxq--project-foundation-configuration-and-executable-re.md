---
# brutui-3dxq
title: Project foundation, configuration, and executable resolution
status: todo
type: epic
priority: high
tags:
- prd
- epic
created_at: 2026-05-29T17:28:16.787950Z
updated_at: 2026-05-29T17:28:16.787950Z
parent: brutui-omh0
---

## Goal

Implement this slice of Brutui v1 in alignment with `.local/prds/1780075033-brutui-rust-tui-prd.md`.

## Guardrails

- Execution correctness belongs to `bru`; do not import Bruno JavaScript internals.
- Keep inspection shallow, best-effort, and read-only.
- Do not add out-of-scope editing, global environments, tag filtering, concurrent runs, or project-local Brutui files inside collections.

## Validation

This epic is complete when all child ishes pass their validation and the overall roadmap still satisfies the PRD user stories and implementation decisions.
