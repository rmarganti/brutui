---
# brutui-1z83
title: Parse Bruno JSON reports into a result model
status: completed
type: task
priority: high
tags:
- reports
created_at: 2026-05-29T17:28:16.843046Z
updated_at: 2026-05-29T17:53:37.244668Z
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


## Implementation Notes

- Replaced the placeholder `src/report.rs` structs with a defensive Bruno report parser that can load from a file or string and preserves the source report path for later UI/debugging work.
- Added result models for request-level failures, assertion failures, execution errors, and parser diagnostics so downstream runner/UI code can distinguish test failures from tooling/report-shape problems.
- Summary parsing accepts both snake_case and camelCase counter fields and derives missing counts from request results when the report is partial.
- Missing summary/request sections now surface parser diagnostics, while malformed JSON, invalid top-level shapes, and empty reports without usable semantics return explicit parse errors.
- Added focused integration coverage in `tests/report_parsing.rs` for success, failure, partial/missing-field, malformed JSON, invalid-shape, and missing-semantics cases.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `brutui::report::parse_report_file` is ready for runner completion handling; it preserves `source_path` so the UI can expose the latest temp report path during a session.
- `RunReport::diagnostics` is intended for partial-but-usable reports; runner logic should treat `ReportParseError` as a tool/report error and successful parses with diagnostics as completed runs with caveats.
- The parser currently tolerates both snake_case and camelCase summary fields plus several common request-level aliases (`requestName`, `failedTests`, `failedAssertions`, `errors`), so command-runner tests can use concise fake reports without matching one exact serialized shape.
