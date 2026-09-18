# CLAUDE.md

## Project

malacli — terminal bible reader in rust/ratatui. binary name is `malacli` (a play on malachi, the last OT prophet). the repo is still named `tui-bible` locally but `malacli` on github.

## Build & Validate

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

clippy runs with `-D warnings` so all warnings are errors. the `collapsible_if` lint is suppressed crate-wide (`#![allow(clippy::collapsible_if)]` in main.rs) bc the suggested let-chains aren't stable on older rust toolchains. don't try to collapse `if let` + `if` patterns — they'll fail to compile on rust < 1.94.

## Release Cycle

1. bump version in `Cargo.toml`
2. commit + push to master
3. `git tag vX.Y.Z && git push origin vX.Y.Z`
4. release workflow builds binaries for 4 targets (linux x86_64/aarch64, macos x86_64/aarch64)
5. download tarballs, compute sha256s, update `pkg/homebrew/malacli.rb`
6. copy formula to `/opt/homebrew/Library/Taps/jamesd7788/homebrew-tap/Formula/malacli.rb`, commit + push to tap repo
7. commit formula update back to this repo

the homebrew tap is `jamesd7788/homebrew-tap` (shared tap, not a dedicated one). the formula file in this repo is `pkg/homebrew/malacli.rb`.

## Architecture

- KJV bible + cross references are compressed at build time (build.rs + flate2) and embedded via `include_bytes!`. decompressed once on first access via `OnceLock`. no external data files needed.
- additional translations are discovered from a configurable directory (`malacli set bible-dir <path>` or `MALACLI_OSIS_DIR` env var). env vars override config.
- notes are markdown files in `~/.config/malacli/notes/` with a trailing `---\nverses: [...]` metadata block (NOT top frontmatter). the parser supports both formats for backwards compat but new notes use trailing refs.
- verse files live in `notes/verses/` subfolder, created lazily on note creation. named `book_abbrevChapter-Verse.md` e.g. `john3-16.md`. notes use `[[john3-16]]` wikilinks for obsidian integration.
- `$EDITOR` integration: tui suspends via `ratatui::restore()`, spawns editor, then `ratatui::init()` to resume. the Tui struct must be replaced after restore.
- config lives at `~/.config/malacli/config.toml`. fields: `bible_dir`, `translation`, `theme`, `editor`. all optional, `skip_serializing_if = "Option::is_none"`.
- omarchy theme integration lives in `src/omarchy.rs`. reads the live palette from `~/.local/state/omarchy/current/theme/colors.toml` (`XDG_STATE_HOME` respected). `MALACLI_OMARCHY_THEME` points at a colors.toml directly, which is how the tests fake a desktop theme. everything returns `Option` and falls back to the monastic palette, so the binary stays portable off omarchy.
- `colors.toml` is parsed by hand rather than via the `toml` crate: a malformed line should cost one colour, not the whole theme.
- theme resolution is cached per-frame in `ui::Theme::cached()`, keyed on `omarchy::revision()` (newest mtime of the hook stamp + colors.toml). that cache IS the live-reload mechanism — the palette rebuilds when the token moves. before this, `Theme::current()` did a config read on every single frame.
- `pkg/omarchy/malacli-theme-hook` is a theme-set hook that only touches a stamp file. it's an optimisation: colors.toml is watched too, so live reload works without it.

## Key Learnings (stuff that bit us)

- **private repos + github releases**: release assets created while a repo is private remain 404 even after making the repo public. must delete the release and re-tag to regenerate assets.
- **`#![allow(...)]` is crate-root only**: can't use inner attributes in non-root modules. use `#[allow(...)]` on the `mod` declaration in main.rs instead.
- **test timeouts for embedded data**: the search-dependent tests need the full bible loaded in a background thread. decompressing embedded data adds latency. test timeout is 30s — don't lower it.
- **session state affects tests**: `App::load()` reads the real session file. tests that assume `focus == Reader` can fail if the session has `Side` saved. this is a known pre-existing issue.
- **colons in filenames**: don't work on macos (classic mac path separator) or windows. verse file slugs use hyphens: `john3-16.md` not `john3:16.md`.
- **homebrew requires a tap**: can't `brew install` a local formula file directly. must be in a tap directory structure.
- **`cargo fmt` import ordering**: build.rs imports get reordered by fmt. don't fight it.
- **env vars in tests race**: `MALACLI_OMARCHY_THEME` is process-global and the test runner is multi-threaded, so the omarchy + ui theme tests flaked ~1 in 5 runs until both took `omarchy::env_lock()`. any new test that sets an env var must take that guard.
- **raw strings vs hex colours**: `r#"..."#` terminates early on a `"#rrggbb"` value. theme fixtures need `r##"..."##`.

## CLI vs TUI

`malacli` with no args launches the TUI. `malacli theme-info` is an undocumented diagnostic that dumps what the omarchy integration currently sees. with args it's a CLI tool. the `load_bible()` helper in main.rs respects the configured translation (config → env var → kjv fallback) for all CLI output.
