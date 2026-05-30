---
# brutui-u3c9
title: Evolve collection model toward indexed tree access
status: todo
type: task
priority: normal
tags:
- collections
- state
- architecture
created_at: 2026-05-30T17:24:12.753762Z
updated_at: 2026-05-30T17:24:12.753762Z
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
