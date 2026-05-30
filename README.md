# Brutui

Brutui is a read-only terminal UI for browsing and running Bruno collections via the Bruno CLI.

## What v1 does

- Launch from inside a Bruno collection or pass a collection path explicitly
- Discover collections from configured directories when launched elsewhere
- Browse collection roots, folders, and requests in a keyboard-driven TUI
- Show shallow read-only request metadata when available
- Pick collection-local environments, including an explicit **No environment** option
- Run the selected collection root, folder, or request through `bru`
- Stream live stdout/stderr while a run is active
- Show structured summary/failure details from Bruno JSON reporter output
- Keep raw output and the latest temp report path visible for debugging

## Requirements

- Rust toolchain for building from source
- Bruno CLI installed and reachable as `bru`, or configured via `bru_path`, or overridden with `BRUTUI_BRU_PATH`

## Build

```bash
cargo build --all-features
```

## Launch

Open the nearest Bruno collection from your current directory:

```bash
cargo run --
```

Open a specific collection directly:

```bash
cargo run -- /path/to/collection
```

Brutui recognizes collection roots that contain either `bruno.json` or `opencollection.yml`.

## Bruno executable resolution

Brutui resolves `bru` in this order:

1. `BRUTUI_BRU_PATH`
2. `bru_path` from config
3. `bru` on `PATH`

## Configuration

Brutui uses a TOML config file in the platform-appropriate app config directory (`dev/rmarganti/brutui/config.toml` via the Rust `directories` crate).

Use `BRUTUI_CONFIG=/path/to/config.toml` to point Brutui at a different config file.

Example config:

```toml
collection_dirs = [
  "/Users/you/code/apis",
  "/Users/you/work/bruno-collections"
]

bru_path = "/opt/homebrew/bin/bru"

# Optional TUI theme overrides. Omitted fields keep their defaults.
[theme]
focused_panel_border = { fg = "yellow" }
selected_item = { reversed = true }
focused_selected_item = { fg = "#f0f0f0", reversed = true, bold = true }
emphasized_text = { bold = true }
```

A copy is also provided at [`config/brutui.example.toml`](config/brutui.example.toml).

Theme colors may be named terminal colors (such as `yellow`, `cyan`, or `light_blue`) or RGB hex values in `#RRGGBB` format. Invalid color names are reported as configuration errors on startup.

## Keyboard usage

### Startup collection picker

- Type to filter collections
- `Enter` open selected collection
- `↑/↓` or `j/k` move
- `q` or `Esc` quit

### Loaded session

- `↑/↓` or `j/k` move in the collection tree
- `Tab` cycle focus between tree, details, and output
- `e` open environment picker
- `r` run the selected root, folder, or request
- `c` cancel the active run
- `1` response body view
- `2` response/request headers view
- `3` tests/assertions/errors view
- `[` / `]` move between requests in the latest report
- `y` copy the current response tab text
- `?` help overlay
- `q` quit

## Validation

Run the full local quality gate with:

```bash
./scripts/validate.sh
```

It runs:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `ish check`

## Notes

- Folder runs are recursive.
- Brutui is intentionally read-only in v1: no editing, no global/workspace environments, no tag filtering, and no concurrent runs.
- Brutui writes Bruno JSON reports to temporary files outside collection directories during normal operation.
