# tui-bible

A fast, keyboard-first terminal Bible reader in Rust with `ratatui`, real Bible text, full-text search, cross references, local translation loading, and session restore.

The current product direction is reader-first with study power: beautiful reading, fast passage movement, a contextual side pane, and no network dependency in the core app.

<img width="2298" height="1322" alt="image" src="https://github.com/user-attachments/assets/21711638-0863-47c3-a757-2a529c23f561" />

## Features

- Paragraph-style reader with inline verse numbers and a focused reading column
- Jump to passages like `john`, `jn 3:16`, `gen 1`, or `1 cor 13`
- Full-text search in the side pane
- Cross-reference side pane backed by the OpenBible cross-reference dataset
- Contextual preview pane for search and cross-reference results
- Back/forward history with short reference tabs in the header
- Session restore for current passage, pane state, translation, and history
- Lazy local translation loading with bundled KJV fallback
- Optional terminal color passthrough theme

## Quick Start

```bash
cargo run
```

Release build:

```bash
cargo build --release
./target/release/tui-bible
```

## Controls

- `q`: quit
- `g`: jump to a passage
- `/`: search scripture
- `x`: switch side pane back to cross references
- `tab`: toggle reader/side pane focus
- `j` / `k`: move current verse in reader, or selected item in side pane
- `h` / `l`: previous/next chapter
- `enter`: open selected search hit or cross reference
- `u`: back in history
- `p`: forward in history
- `t`: cycle loaded translations
- `esc`: cancel search/jump entry

## History Behavior

History tracks intentional navigation, not every small reading movement.

- Jumping, opening a search hit, and opening a cross-reference push a new history item.
- `j/k` reader movement replaces the current history item.
- `h/l` within the same book replaces the current history item.
- `h/l` across a book boundary pushes a new history item.
- `u/p` move backward and forward through that trail.

Example:

```text
jump Col 3 -> l to Col 4 -> l to 1 Thess 1 -> u returns to Col 4
```

## Themes

Default theme is the built-in warm/monastic palette.

Use your terminal foreground/background colors instead:

```bash
TUI_BIBLE_THEME=terminal cargo run
```

## Session Restore

The app saves session state on normal quit and restores it on startup.

Default session path:

```text
$XDG_CONFIG_HOME/tui-bible/session.toml
```

Fallback:

```text
$HOME/.config/tui-bible/session.toml
```

Override the session path:

```bash
TUI_BIBLE_SESSION=/path/to/session.toml cargo run
```

Clear session state:

```bash
rm "$HOME/.config/tui-bible/session.toml"
```

If another app instance is still running, it can recreate the file on exit.

## Data Sources

Bundled data:

- Bible text: `data/raw/eng-kjv.osis.xml` from Open Bibles
- Cross references: `data/raw/cross_references.txt` from OpenBible

Refresh open data:

```bash
./scripts/fetch_open_data.sh
```

The downloaded zip artifact is ignored; extracted app data is tracked.

## Local Translations

The app always includes bundled KJV and can discover additional local XML Bible files.

Supported local formats currently include:

- OSIS XML
- `XMLBIBLE` style XML
- simple `<bible><b><c><v>` style XML

Point the app at a local translation directory:

```bash
export TUI_BIBLE_OSIS_DIR=$HOME/src/osis-bibles
cargo run
```

Prefer a translation on startup:

```bash
export TUI_BIBLE_TRANSLATION=esv
cargo run
```

Notes:

- If `TUI_BIBLE_OSIS_DIR` contains an `en/` directory, the app scans that subtree for `*.xml`.
- On this development machine, the app also checks `/Users/james/Downloads/media-tool-kit-xml-bibles` if it exists.
- Additional translations are loaded lazily; the current chapter appears first, then the full translation warms in memory.
- Do not commit or redistribute private/licensed translations.

## Validation

Run before committing:

```bash
cargo fmt
cargo test
cargo clippy -- -D warnings
```

## Repo Hygiene

Ignored paths include:

- `target/`
- `data/private/`
- `data/local/`
- `*.local.xml`
- downloaded zip artifacts in `data/raw/`

## Distribution Notes

The app currently expects bundled data under `data/raw/` relative to the working directory. Before a polished release, add release-safe data path resolution such as:

1. `TUI_BIBLE_DATA_DIR`
2. data directory beside the binary
3. platform data dir
4. development fallback to repo `data/raw/`

A minimal distributable should include:

```text
tui-bible
data/raw/eng-kjv.osis.xml
data/raw/cross_references.txt
README.md
LICENSE
NOTICE
```

## Backburner Ideas

- Persistent normalized translation cache for faster cold starts
- Dedicated translation picker instead of cycling with `t`
- Semantic search as optional local sidecar data/model, likely behind `?`
- Snapshot tests for key UI states
