---
# brutui-omh0
title: Brutui v1 PRD implementation
status: todo
type: milestone
priority: critical
tags:
- prd
- v1
created_at: 2026-05-29T17:28:16.779654Z
updated_at: 2026-05-29T17:28:16.779654Z
---

## Goal

Deliver Brutui v1 exactly as specified by `.local/prds/1780075033-brutui-rust-tui-prd.md`: a Rust terminal UI that discovers Bruno collections, inspects them read-only, selects collection-local environments, delegates execution to `bru`, streams output, parses JSON reports, and presents results without writing into collection directories.

## PRD Alignment

This milestone exists only to implement the referenced PRD. Future work must re-read the PRD before changing scope. Keep v1 constrained to runner/browser behavior and avoid out-of-scope authoring features.

## Completion Criteria

- The executable can be launched from inside a Bruno collection with no config.
- Explicit launch paths, cwd discovery, and configured discovery follow the PRD precedence.
- Classic `bruno.json` and YAML/OpenCollection `opencollection.yml` roots are supported.
- The TUI supports tree browsing, shallow request details, env selection, help, run/cancel, live raw output, and structured post-run results.
- Exit-code/report semantics match the PRD.
- Automated tests cover the dedicated modules listed in the PRD testing section.
