# tui-bible

A fast terminal Bible reader in Rust with `ratatui`, real Bible text, and real cross references.

## Current v1 slice

- Read full KJV text from a local OSIS source
- Jump to passages like `jn 3:16` or `1 cor 13`
- Incremental full-text search
- Cross-reference side pane backed by the OpenBible dataset
- Keyboard-first navigation in a single terminal window

## Run

```bash
cargo run
```

Use your terminal's foreground/background colors instead of the built-in warm theme:

```bash
TUI_BIBLE_THEME=terminal cargo run
```

The app restores your last session on startup, including current passage and back/forward history.
Override the session file path with `TUI_BIBLE_SESSION=/path/to/session.toml` if needed.

## Controls

- `q`: quit
- `g`: jump to a passage
- `/`: search
- `tab`: cycle reader/side pane focus
- `j` / `k`: move selection or current verse
- `h` / `l`: previous or next chapter
- `enter`: open selected search hit or cross reference
- `u`: back in history
- `p`: forward in history
- `t`: cycle loaded translations
- `x`: switch side pane back to cross references
- `esc`: cancel search/jump entry

## Data sources

- Bible text: `eng-kjv.osis.xml` from Open Bibles
- Cross references: `cross_references.txt` from OpenBible

Refresh them with:

```bash
./scripts/fetch_open_data.sh
```

## Validation

```bash
cargo fmt
cargo test
```

## Local OSIS translations

You can point the app at a local checkout of an OSIS repo such as `osis-bibles` and cycle translations with `t`.

Example:

```bash
export TUI_BIBLE_OSIS_DIR=$HOME/src/osis-bibles
export TUI_BIBLE_TRANSLATION=esv
cargo run
```

Notes:

- If `TUI_BIBLE_OSIS_DIR` contains an `en/` directory, the app scans that subtree for `*.xml`.
- The app always loads the bundled `KJV` and any additional local OSIS files it can parse.
- No translation files from that repo are copied into this repo.
