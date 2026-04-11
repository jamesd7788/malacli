# malacli

A fast, keyboard-first terminal Bible reader in Rust with `ratatui`, real Bible text, full-text search, cross references, notes with obsidian integration, local translation loading, and session restore.

The current product direction is reader-first with study power: beautiful reading, fast passage movement, a contextual side pane, and no network dependency in the core app.

<img width="2340" height="1358" alt="image" src="https://github.com/user-attachments/assets/b09ee135-da6b-4ed9-8053-ac8e1b0d1d9a" />


## Features

- Paragraph-style reader with inline verse numbers and a focused reading column
- Jump to passages like `john`, `jn 3:16`, `gen 1`, or `1 cor 13`
- Full-text search in the side pane
- Cross-reference side pane backed by the OpenBible cross-reference dataset
- Contextual preview pane for search and cross-reference results
- Notes with `$EDITOR` integration, obsidian wikilinks, and verse pinning
- Visual verse selection with Shift+j/k for multi-verse notes
- Back/forward history with short reference tabs in the header
- Session restore for current passage, pane state, translation, and history
- Lazy local translation loading with bundled KJV fallback
- Optional terminal color passthrough theme

## Install

### Homebrew (macOS / Linux)

```bash
brew tap jamesd7788/tap
brew install malacli
```

### AUR (Arch Linux)

```bash
yay -S malacli
```

### Cargo

```bash
cargo install --git https://github.com/jamesd7788/malacli
```

### From source

```bash
cargo build --release
./target/release/malacli
```

## CLI Commands

```bash
malacli                         # launch the TUI reader
malacli john 3:16               # print a verse
malacli john 3:16-18            # print a verse range
malacli chapter john 3          # print full chapter
malacli context john 3:16      # surrounding verses with marker
malacli search "love"           # search scripture
malacli ref john 3:16           # cross references
malacli count love              # word occurrence count
malacli parallel john 3:16     # compare across loaded translations
malacli json john 3:16          # structured json output
malacli books                   # list all books with OSIS codes
malacli toc                     # table of contents
malacli outline genesis         # chapter outline for a book
malacli random                  # random verse
malacli history                 # session navigation trail
malacli notes                   # list all notes
malacli info                    # config and session state
```

## Configuration

```bash
malacli config                  # show all settings
malacli set translation esv     # default translation
malacli set theme terminal      # terminal color passthrough
malacli set editor nvim         # note editor
malacli set bible-dir ~/bibles  # translations directory
malacli set <key> --unset       # clear any setting
malacli get <key>               # get a single value
```

Environment variables (`MALACLI_OSIS_DIR`, `MALACLI_TRANSLATION`, `MALACLI_THEME`, `MALACLI_SESSION`) override config when set.

## Reader Controls

- `q`: quit
- `g`: jump to a passage
- `/`: search scripture
- `x`: switch side pane to cross references
- `n`: switch side pane to notes (press again to toggle chapter/all)
- `a`: create note at current verse (or add to pinned note)
- `P`: pin/unpin a note
- `tab`: toggle reader/side pane focus
- `j` / `k`: move current verse in reader, or selected item in side pane
- `J` / `K`: extend verse selection (shift+j/k)
- `h` / `l`: previous/next chapter
- `enter`: open selected item (search hit, cross ref, or note in $EDITOR)
- `u`: back in history
- `p`: forward in history
- `t`: cycle loaded translations
- `esc`: cancel input / clear selection

## Notes

Notes are markdown files stored in `~/.config/malacli/notes/` with verse metadata. They integrate with obsidian via wikilinks and a `verses/` subfolder of individual verse pages.

Workflow:
1. Press `a` on a verse to create a note and open in `$EDITOR`
2. Use `Shift+j/k` to select multiple verses before pressing `a`
3. Press `n` to view notes for the current chapter
4. Press `P` on a note to pin it, then navigate and press `a` to add verses from anywhere
5. Press `P` again to unpin

Notes format:

```markdown
> For God so loved the world...
>
> — [[john3-16]]

your notes here

---
verses: [John.3.16]
```

Symlink into your obsidian vault:

```bash
ln -s ~/.config/malacli/notes ~/your-vault/bible-notes
```

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

Default theme is the built-in warm/monastic palette. Switch to terminal color passthrough:

```bash
malacli set theme terminal
```

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

KJV is bundled in the binary. Additional Bible XML files can be loaded from a local directory.

Supported local formats:

- OSIS XML
- `XMLBIBLE` style XML
- simple `<bible><b><c><v>` style XML

Set a translations directory:

```bash
malacli set bible-dir ~/bibles
```

Set a default translation:

```bash
malacli set translation esv
```

Notes:

- If the directory contains an `en/` subdirectory, the app scans that subtree for `*.xml`.
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

## Distribution

KJV text and cross-reference data are compressed and embedded in the binary at build time. The release binary is self-contained (~6 MB) with no external data dependencies.

## Backburner Ideas

- Persistent normalized translation cache for faster cold starts
- Dedicated translation picker instead of cycling with `t`
- Semantic search as optional local sidecar data/model, likely behind `?`
- Snapshot tests for key UI states
