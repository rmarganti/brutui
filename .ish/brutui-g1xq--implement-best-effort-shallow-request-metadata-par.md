---
# brutui-g1xq
title: Implement best-effort shallow request metadata parsing
status: completed
type: task
priority: normal
tags:
- metadata
- parser
created_at: 2026-05-29T17:28:16.835649Z
updated_at: 2026-05-29T18:09:24.392450Z
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


## Implementation Notes

- Expanded `src/metadata.rs` into a format-aware best-effort parser with `parse_request_file`/`parse_request_str` entrypoints and a `ParsedRequestMetadata` result carrying partial fields plus non-fatal diagnostics.
- Classic `.bru` parsing now extracts shallow `meta.name`, the first HTTP method block, `url`, and simple `tags` block entries while recovering from malformed block structure instead of failing hard.
- OpenCollection/YAML parsing now extracts top-level `name`, `method`, `url`, and list/inline tags while reporting unsupported tag shapes as diagnostics.
- Extended `src/collection/model.rs::RequestNode` with parsed metadata and metadata diagnostics, and wired `src/collection/scanner.rs` to decorate request nodes during filesystem scanning without affecting node visibility or runnability.
- Added focused unit coverage in `src/metadata.rs` plus integration coverage in `tests/metadata_parsing.rs` for classic and YAML/OpenCollection parsing, missing fields, malformed files, unsupported tag shapes, and scanner-level regression that malformed requests remain visible.

## Verification

- `cargo test --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- Request metadata is now attached directly to `CollectionNode::Request`, so upcoming details-pane and app-state work can render filename/path first and opportunistically layer name/method/url/tags/diagnostics without reparsing files.
- Classic tag values are surfaced as plain tag names for boolean/empty values and `key:value` strings otherwise; UI work should treat them as read-only display text, not structured filters.
- Metadata parsing is intentionally shallow and line-oriented; if later compatibility work broadens supported Bruno/YAML shapes, extend `parse_request_str` while preserving the current non-fatal diagnostics contract.
