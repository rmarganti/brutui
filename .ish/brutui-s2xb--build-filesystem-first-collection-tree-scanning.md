---
# brutui-s2xb
title: Build filesystem-first collection tree scanning
status: completed
type: task
priority: high
tags:
- collections
- tree
created_at: 2026-05-29T17:28:16.832070Z
updated_at: 2026-05-29T17:45:53.250048Z
parent: brutui-7sjr
blocked_by:
- brutui-iblr
---

## Context

The PRD requires a collection tree pane that reliably shows folders and runnable requests without depending on deep Bruno internals. The tree is filesystem-first and must preserve paths needed for `bru run` targets.

Reference: `.local/prds/1780075033-brutui-rust-tui-prd.md`, user stories 17-25 and collection scanner module notes.

## Scope

- Define in-memory collection, folder, request, and node selection models.
- Recursively scan collection contents into a stable, deterministic tree.
- Include collection root, folders, and request files for supported formats.
- Preserve display names, filenames, relative paths, and absolute paths needed by runner command construction.
- Keep malformed requests visible/runnable; scanner must not fail on metadata parse errors.

## Validation

- Tests verify nested folder/request tree construction, deterministic ordering, root/folder/request node identity, path preservation, and resilience to malformed request files.



## Implementation Notes

- Added `CollectionNodeId` plus node helper methods in `src/collection/model.rs` so later state/UI work can refer to stable root/folder/request selections without re-deriving identity from display text.
- Implemented `src/collection/scanner.rs` as a filesystem-first recursive scanner that emits a deterministic preorder tree: root first, then sorted folders depth-first, then sorted request files within each folder.
- Scanner preserves absolute paths, collection-relative paths, and display names for every folder/request node while staying format-aware for classic `.bru` and OpenCollection YAML request files.
- Scanner ignores non-runnable collection content that should not appear in the request tree, including collection root markers, collection-local environment directories, and common irrelevant directories like `.git`, `node_modules`, `target`, `build`, and `dist`.
- Malformed request files remain visible because scanning is path/extension based and does not depend on metadata parsing.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
- `ish check`

## Notes For Follow-on Work

- `Collection::nodes` is now a flat deterministic preorder traversal, which should be straightforward for upcoming tree-selection/state code to render without introducing another traversal layer.
- The scanner currently excludes the top-level `environments/` subtree from request nodes so environment discovery can own those files independently without polluting the runnable request tree.
- Request inclusion is intentionally shallow and extension-based; upcoming metadata parsing can decorate request nodes without changing scanner semantics or hiding malformed requests.
