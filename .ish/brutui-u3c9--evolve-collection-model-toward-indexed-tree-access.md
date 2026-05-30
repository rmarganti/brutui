---
# brutui-u3c9
title: Evolve collection model toward indexed tree access
status: completed
type: task
priority: normal
tags:
- collections
- state
- architecture
created_at: 2026-05-30T17:24:12.753762Z
updated_at: 2026-05-30T17:51:54.353575Z
parent: brutui-hrnz
blocked_by:
- brutui-0jl8
- brutui-0f8d
---

## Context

`Collection` currently stores a flat preorder `Vec<CollectionNode>`, and selection/lookups scan that vector by `CollectionNodeId`. This is fine for v1 browsing, but it will fight likely future features such as folder collapse/expand, collection search/filtering, preserving selection across rescans, per-folder result badges, and moving only through visible nodes.

## Dependencies

- Blocked by `brutui-0jl8` so state access patterns and invariants are tightened before changing the collection read model.
- Blocked by `brutui-0f8d` so UI rendering/input are separated before adapting tree rendering to an indexed model.

## Work

- Evolve `Collection` toward an indexed tree/read model while preserving current scanner behavior and display ordering.
- Consider adding stable node indices/maps such as `by_id` and parent/children relationships while still exposing a flattened visible/preorder list for current rendering.
- Keep `CollectionNodeId` stable and path-based so runner command construction remains correct.
- Update navigation/state code to use explicit collection lookup helpers instead of ad hoc linear scans where practical.
- Do not implement new UX features like collapse/filter/search in this ish; prepare the model so those features are straightforward later.

## Verification

- Existing scanner, state navigation, command construction, and UI flow tests pass.
- Add tests for lookup by id, children/parent relationships if introduced, deterministic visible order, and preservation of root/folder/request path identities.
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`



## Implementation Notes

- Reworked `src/collection/model.rs` so `Collection` now owns a flattened visible preorder plus indexed lookup structures for node id -> index, parent relationships, and ordered child ids.
- Added `Collection::new(...)` plus helpers like `visible_nodes()`, `node(...)`, `node_index(...)`, `contains_node(...)`, `parent_id(...)`, and `child_ids(...)` so callers can use explicit collection read-model access instead of ad hoc linear scans.
- Kept `CollectionNodeId` path-based and stable, and preserved the existing scanner-visible preorder by building indexes from the already-deterministic filesystem scan output rather than changing scan order.
- Updated state navigation/selection, UI tree rendering, and runner target-path resolution to consume the new collection lookup helpers instead of reaching into a raw node vector.
- Added focused indexed-model coverage in `src/collection/model.rs` and updated scanner/metadata tests to assert against the visible-node API.

## Verification

- `./scripts/validate.sh`

## Notes For Follow-on Work

- The current `visible_nodes()` preorder remains the source for rendering and sequential navigation, while `parent_id(...)` and `child_ids(...)` provide the minimal indexed tree seams needed for future collapse/filter/selection-preservation work without re-scanning the filesystem.
- If later work needs richer per-node state (collapse flags, badges, cached visibility), extend `Collection` around the existing indexed maps rather than reintroducing repeated `Vec` scans in state/UI code.
