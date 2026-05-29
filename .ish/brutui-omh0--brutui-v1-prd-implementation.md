---
# brutui-omh0
title: Brutui v1 PRD implementation
status: completed
type: milestone
priority: critical
tags:
- prd
- v1
created_at: 2026-05-29T17:28:16.779654Z
updated_at: 2026-05-29T18:45:19.008367Z
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

## Milestone Closeout

- Foundation/config/executable resolution landed in `src/app.rs`, `src/cli.rs`, `src/config.rs`, and `src/executable.rs`, giving v1 a usable CLI startup path and actionable setup diagnostics.
- Discovery/scanning/metadata/environment work now supports classic and OpenCollection roots, deterministic tree rendering, best-effort shallow request inspection, and collection-local environment selection only.
- Runner/report/UI work now provides single-run execution with cancellation, live stdout/stderr streaming, structured Bruno JSON result parsing, startup collection selection, request details, and result/raw-output views.
- Release-readiness work added fixture collections, fake-`bru` harness coverage, README/config documentation, and an acceptance pass confirming v1 stays read-only and avoids collection-local writes.

## Final Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Future Workers

- Keep `README.md`, `config/brutui.example.toml`, and `src/cli.rs` help text aligned whenever startup/discovery/runtime behavior changes.
- Preserve the v1 guardrails from the PRD: no editing flows, no global/workspace/manual environments, no tag filtering, no concurrent runs, and no writes inside Bruno collection directories during normal operation.
- If terminal startup/run regressions become common, promote the ad hoc fake-`bru` PTY smoke flow into a checked-in release-smoke script.
