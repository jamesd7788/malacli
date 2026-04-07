#![allow(clippy::collapsible_if)]

mod app;
mod bible;
mod config;
mod data;
mod event;
#[allow(dead_code)]
mod note;
mod session;
mod translation;
mod tui;
mod ui;

use std::path::PathBuf;
use std::process;

use app::App;
use color_eyre::Result;
use tui::Tui;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(cmd) = args.first() {
        match cmd.as_str() {
            "-h" | "--help" | "help" => {
                print_help();
                return Ok(());
            }
            "-V" | "--version" => {
                println!("malacli {VERSION}");
                return Ok(());
            }
            "set-bible-dir" => {
                let path = args.get(1).unwrap_or_else(|| {
                    eprintln!("usage: malacli set-bible-dir <path>");
                    process::exit(1);
                });
                return set_bible_dir(path);
            }
            "unset-bible-dir" => {
                return unset_bible_dir();
            }
            other => {
                eprintln!("unknown command: {other}");
                eprintln!("run `malacli --help` for usage");
                process::exit(1);
            }
        }
    }

    color_eyre::install()?;

    let terminal = ratatui::init();
    let mut tui = Tui::new(terminal);
    let result = run(&mut tui);
    ratatui::restore();
    result
}

fn run(tui: &mut Tui) -> Result<()> {
    let mut app = App::load()?;

    while app.is_running {
        tui.draw(&app)?;

        match event::next()? {
            event::Event::Key(key) => app.handle_key_event(key),
            event::Event::Resize | event::Event::Tick => app.poll_background_work(),
        }

        if let Some(path) = app.editor_request.take() {
            open_in_editor(tui, &mut app, &path)?;
        }
    }

    let _ = app.save_session();
    Ok(())
}

fn open_in_editor(tui: &mut Tui, app: &mut App, path: &std::path::Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    ratatui::restore();
    let status = std::process::Command::new(&editor).arg(path).status();
    let terminal = ratatui::init();
    *tui = Tui::new(terminal);
    match status {
        Ok(exit) if exit.success() => {
            app.reload_notes();
            app.status = "Note saved.".to_string();
        }
        Ok(_) => {
            app.reload_notes();
            app.status = "Editor exited with non-zero status.".to_string();
        }
        Err(error) => {
            app.status = format!("Failed to open {editor}: {error}");
        }
    }
    Ok(())
}

fn set_bible_dir(path: &str) -> Result<()> {
    let path = PathBuf::from(path);
    if !path.is_dir() {
        eprintln!("not a directory: {}", path.display());
        process::exit(1);
    }
    let canonical = path.canonicalize()?;
    let mut cfg = config::load();
    cfg.bible_dir = Some(canonical.clone());
    config::save(&cfg)?;
    println!("bible directory set to: {}", canonical.display());
    println!("xml files in this directory will be available as translations.");
    Ok(())
}

fn unset_bible_dir() -> Result<()> {
    let mut cfg = config::load();
    if cfg.bible_dir.is_none() {
        println!("no bible directory is currently set.");
        return Ok(());
    }
    cfg.bible_dir = None;
    config::save(&cfg)?;
    println!("bible directory cleared.");
    Ok(())
}

fn print_help() {
    println!(
        "
  ┌┬┐┌─┐┬  ┌─┐┌─┐┬  ┬
  │││├─┤│  ├─┤│  │  │
  ┴ ┴┴ ┴┴─┘┴ ┴└─┘┴─┘┴
  {VERSION}

USAGE
  malacli                       launch the reader
  malacli set-bible-dir <path>  set local translations directory
  malacli unset-bible-dir       clear the translations directory
  malacli -h, --help            show this help
  malacli -V, --version         show version

READER CONTROLS
  q        quit
  g        jump to a passage (e.g. john 3:16, gen 1, 1 cor 13)
  /        search scripture
  x        show cross references
  n        show notes for current chapter
  a        create a new note at current verse
  P        pin/unpin a note (a adds verses to pinned note)
  tab      toggle reader / side pane focus
  j / k    move verse (reader) or selection (side pane)
  J / K    extend verse selection (shift+j/k)
  h / l    previous / next chapter
  enter    open selected item (search hit, cross ref, or note in $EDITOR)
  u / p    back / forward in history
  t        cycle loaded translations
  esc      cancel input / clear selection

ENVIRONMENT
  MALACLI_OSIS_DIR        override translations directory (takes precedence over config)
  MALACLI_TRANSLATION     preferred translation code on startup (e.g. esv)
  MALACLI_THEME           set to 'terminal' for terminal color passthrough
  MALACLI_SESSION         override session file path

CONFIG
  ~/.config/malacli/config.toml    persistent settings (bible_dir)
  ~/.config/malacli/session.toml   session state (auto-saved on quit)
  ~/.config/malacli/notes/         notes directory"
    );
}
