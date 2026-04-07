#![allow(clippy::collapsible_if)]

mod app;
mod bible;
mod config;
mod data;
mod event;
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
                println!("tui-bible {VERSION}");
                return Ok(());
            }
            "set-bible-dir" => {
                let path = args.get(1).unwrap_or_else(|| {
                    eprintln!("usage: tui-bible set-bible-dir <path>");
                    process::exit(1);
                });
                return set_bible_dir(path);
            }
            "unset-bible-dir" => {
                return unset_bible_dir();
            }
            other => {
                eprintln!("unknown command: {other}");
                eprintln!("run `tui-bible --help` for usage");
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
    }

    let _ = app.save_session();
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
        "\
tui-bible {VERSION} — a terminal Bible reader

USAGE
  tui-bible                  launch the reader
  tui-bible set-bible-dir <path>   set local translations directory
  tui-bible unset-bible-dir        clear the translations directory
  tui-bible -h, --help             show this help
  tui-bible -V, --version          show version

READER CONTROLS
  q        quit
  g        jump to a passage (e.g. john 3:16, gen 1, 1 cor 13)
  /        search scripture
  x        show cross references
  tab      toggle reader / side pane focus
  j / k    move verse (reader) or selection (side pane)
  h / l    previous / next chapter
  enter    open selected search hit or cross reference
  u / p    back / forward in history
  t        cycle loaded translations
  esc      cancel input

ENVIRONMENT
  TUI_BIBLE_OSIS_DIR        override translations directory (takes precedence over config)
  TUI_BIBLE_TRANSLATION     preferred translation code on startup (e.g. esv)
  TUI_BIBLE_THEME           set to 'terminal' for terminal color passthrough
  TUI_BIBLE_SESSION          override session file path

CONFIG
  ~/.config/tui-bible/config.toml    persistent settings (bible_dir)
  ~/.config/tui-bible/session.toml   session state (auto-saved on quit)"
    );
}
