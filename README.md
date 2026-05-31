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

## Initializing a config file

Generate a config file at the platform default location with all keys shown as commented-out annotated defaults:

```bash
brutui init
```

## Bruno executable resolution

Brutui resolves `bru` in this order:

1. `BRUTUI_BRU_PATH`
2. `bru_path` from config
3. `bru` on `PATH`

## Configuration

Brutui uses a TOML config file at `~/.config/brutui/config.toml`.

Run `brutui init` to generate a config file at the default location with every supported key annotated and commented out.

Use `BRUTUI_CONFIG=/path/to/config.toml` to point Brutui at a different config file.

### Supported keys

| Key               | Type           | Description                                                         |
| ----------------- | -------------- | ------------------------------------------------------------------- |
| `collection_dirs` | array of paths | Directories to scan for Bruno collections on startup                |
| `bru_path`        | path           | Explicit path to the `bru` binary (overridden by `BRUTUI_BRU_PATH`) |
| `[theme.*]`       | style sections | TUI color and style overrides (see below)                           |

### Theme

Each theme section (`base`, `panel_border`, `focused_panel_border`, `selected_item`, `focused_selected_item`, `emphasized_text`) accepts these optional fields:

| Field        | Type  | Description      |
| ------------ | ----- | ---------------- |
| `fg`         | color | Foreground color |
| `bg`         | color | Background color |
| `bold`       | bool  | Bold text        |
| `italic`     | bool  | Italic text      |
| `underlined` | bool  | Underlined text  |
| `reversed`   | bool  | Swap fg/bg       |
| `dim`        | bool  | Dim text         |

Colors may be named terminal colors (`black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `white`) or RGB hex values in `#RRGGBB` format. Invalid color names are reported as configuration errors on startup.

Example config:

```toml
collection_dirs = [
  "/Users/you/code/apis",
  "/Users/you/work/bruno-collections"
]

bru_path = "/opt/homebrew/bin/bru"

# Optional TUI theme overrides. Omitted fields keep their defaults.
[theme.focused_panel_border]
fg = "yellow"

[theme.focused_selected_item]
fg = "#f0f0f0"
bold = true
reversed = true
```

A fully annotated template is also provided at [`config/brutui.example.toml`](config/brutui.example.toml).

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
