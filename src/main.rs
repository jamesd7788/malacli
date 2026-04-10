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
use bible::{BOOK_COUNT, Bible, VerseId, book_abbrev, book_name, parse_reference};
use color_eyre::Result;
use tui::Tui;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        return launch_tui();
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => print_help(),
        "-V" | "--version" => println!("malacli {VERSION}"),
        "set-bible-dir" => {
            let path = args.get(1).unwrap_or_else(|| {
                eprintln!("usage: malacli set-bible-dir <path>");
                process::exit(1);
            });
            return set_bible_dir(path);
        }
        "unset-bible-dir" => return unset_bible_dir(),
        "set" => cmd_set(&args[1..]),
        "get" => cmd_get(&args[1..]),
        "config" => cmd_config(),
        "search" => cmd_search(&args[1..]),
        "books" => cmd_books(),
        "toc" => cmd_toc(),
        "ref" => cmd_ref(&args[1..]),
        "history" => cmd_history(),
        "info" => cmd_info(),
        "notes" => cmd_notes(),
        "random" => cmd_random(),
        "count" => cmd_count(&args[1..]),
        "chapter" => cmd_chapter(&args[1..]),
        "context" => cmd_context(&args[1..]),
        "json" => cmd_json(&args[1..]),
        "parallel" => cmd_parallel(&args[1..]),
        "outline" => cmd_outline(&args[1..]),
        _ => {
            // Try as verse reference
            let input = args.join(" ");
            if parse_reference(&input).is_some() || input.contains('-') {
                cmd_verse(&input);
            } else {
                eprintln!("unknown command or reference: {input}");
                eprintln!("run `malacli --help` for usage");
                process::exit(1);
            }
        }
    }

    Ok(())
}

fn launch_tui() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let mut tui = Tui::new(terminal);
    let result = run_tui(&mut tui);
    ratatui::restore();
    result
}

fn run_tui(tui: &mut Tui) -> Result<()> {
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
    let editor = std::env::var("EDITOR")
        .ok()
        .or_else(|| config::load().editor)
        .unwrap_or_else(|| "vim".to_string());
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

// -- CLI commands --

fn load_bible() -> Bible {
    let registry =
        translation::TranslationRegistry::load().expect("failed to load translation registry");
    let preferred = registry.preferred_code().map(str::to_string);
    let mut entries = registry.into_entries();

    // Try preferred translation first
    if let Some(code) = &preferred {
        for entry in &mut entries {
            if entry.code == *code && entry.ensure_full_loaded().unwrap_or(false) {
                return entry.take_bible().expect("loaded bible should exist");
            }
        }
    }

    // Fall back to KJV
    for entry in &mut entries {
        if entry.code == "kjv" && entry.ensure_full_loaded().unwrap_or(false) {
            return entry.take_bible().expect("kjv should always load");
        }
    }

    // Last resort: embedded directly
    let cross_refs = data::cross_references();
    Bible::load_from_str(data::kjv_xml(), cross_refs).expect("failed to load embedded bible")
}

fn cmd_verse(input: &str) {
    let bible = load_bible();
    // Check if it's a range like "john 3:16-18"
    if let Some(range) = parse_verse_range(input, &bible) {
        for vid in &range {
            if let Some(v) = bible.verse(*vid) {
                println!("  {} {}", v.id.verse, v.text);
            }
        }
        if let (Some(first), Some(last)) = (range.first(), range.last()) {
            if first.chapter == last.chapter {
                println!(
                    "  — {} {}:{}-{}",
                    book_name(first.book),
                    first.chapter,
                    first.verse,
                    last.verse
                );
            }
        }
        return;
    }
    let Some(id) = bible.parse_reference(input) else {
        eprintln!("could not resolve: {input}");
        process::exit(1);
    };
    if let Some(v) = bible.verse(id) {
        println!("{}", v.text);
        println!("  — {}", v.id.display());
    }
}

fn cmd_chapter(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli chapter <reference>");
        process::exit(1);
    }
    let input = args.join(" ");
    let bible = load_bible();
    let Some(id) = bible.parse_reference(&input) else {
        eprintln!("could not resolve: {input}");
        process::exit(1);
    };
    let chapter = bible.chapter_for(id);
    if chapter.is_empty() {
        eprintln!("no verses found for {}", id.display());
        process::exit(1);
    }
    println!("{} {}\n", book_name(id.book), id.chapter);
    for verse in chapter {
        println!("  {} {}", verse.id.verse, verse.text);
    }
}

fn cmd_search(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli search <query>");
        process::exit(1);
    }
    let query = args.join(" ");
    let bible = load_bible();
    let results = bible.search(&query, 25);
    if results.is_empty() {
        println!("no results for \"{query}\"");
        return;
    }
    println!("{} results for \"{query}\"\n", results.len());
    for hit in &results {
        if let Some(v) = bible.verse(hit.verse) {
            let preview: String = v.text.chars().take(80).collect();
            println!("  {}  {}", v.id.display(), preview);
        }
    }
}

fn cmd_books() {
    println!("{:<6} {:<20} Index", "OSIS", "Name");
    println!("{}", "-".repeat(40));
    for i in 0..BOOK_COUNT {
        println!("{:<6} {:<20} {}", book_abbrev(i), book_name(i), i);
    }
}

fn cmd_toc() {
    let bible = load_bible();
    println!("OLD TESTAMENT\n");
    for i in 0..39 {
        let chapters = bible.chapters_for_book(i);
        println!("  {:<20} {} chapters", book_name(i), chapters.len());
    }
    println!("\nNEW TESTAMENT\n");
    for i in 39..BOOK_COUNT {
        let chapters = bible.chapters_for_book(i);
        println!("  {:<20} {} chapters", book_name(i), chapters.len());
    }
    let total_chapters = bible.chapter_list().len();
    println!(
        "\n{} books, {} chapters, {} verses",
        BOOK_COUNT,
        total_chapters,
        bible.verse_count()
    );
}

fn cmd_ref(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli ref <verse>");
        process::exit(1);
    }
    let input = args.join(" ");
    let bible = load_bible();
    let Some(id) = bible.parse_reference(&input) else {
        eprintln!("could not resolve: {input}");
        process::exit(1);
    };
    let refs = bible.cross_references(id, 24);
    if refs.is_empty() {
        println!("no cross references for {}", id.display());
        return;
    }
    println!("cross references for {}\n", id.display());
    for r in &refs {
        if let Some(target) = r.target {
            if let Some(v) = bible.verse(target) {
                let preview: String = v.text.chars().take(60).collect();
                println!("  {} ({})  {}", target.display(), r.votes, preview);
            } else {
                println!("  {} ({})", r.target_label, r.votes);
            }
        } else {
            println!("  {} ({})", r.target_label, r.votes);
        }
    }
}

fn cmd_history() {
    let Some(session) = session::load() else {
        println!("no session history found.");
        return;
    };
    if session.history.is_empty() {
        println!("no session history.");
        return;
    }
    println!("session history ({} entries)\n", session.history.len());
    for (i, verse) in session.history.iter().enumerate() {
        let marker = if i == session.history_index { ">" } else { " " };
        println!("{} {}", marker, verse.display());
    }
}

fn cmd_info() {
    let cfg = config::load();
    let session = session::load();
    let note_index = note::NoteIndex::load(&note::notes_dir());

    println!("malacli {VERSION}\n");

    println!("config:     ~/.config/malacli/config.toml");
    println!("session:    ~/.config/malacli/session.toml");
    println!("notes dir:  {}", note::notes_dir().display());
    println!("notes:      {}", note_index.all_notes().len());

    if let Some(dir) = &cfg.bible_dir {
        println!("bible dir:  {}", dir.display());
    } else {
        println!("bible dir:  (not set)");
    }

    if let Some(s) = &session {
        println!("last verse: {}", s.current_verse.display());
        println!("translation: {}", s.translation);
        println!("history:    {} entries", s.history.len());
    } else {
        println!("session:    (none)");
    }
}

fn cmd_notes() {
    let note_index = note::NoteIndex::load(&note::notes_dir());
    let notes = note_index.all_notes();
    if notes.is_empty() {
        println!("no notes found in {}", note::notes_dir().display());
        return;
    }
    println!("{} notes\n", notes.len());
    for note in notes {
        let preview = note.body.lines().next().unwrap_or("(empty)");
        let refs: Vec<String> = note.verses.iter().map(|v| v.display()).collect();
        let refs_str = if refs.is_empty() {
            "(no verses)".to_string()
        } else {
            refs.join(", ")
        };
        println!("  {}  [{}]", preview, refs_str);
        println!("    {}\n", note.path.display());
    }
}

fn cmd_random() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let bible = load_bible();
    let count = bible.verse_count();
    if count == 0 {
        return;
    }
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    let index = seed % count;
    // Access verse by iterating chapter list
    let mut remaining = index;
    for &(book, chapter) in bible.chapter_list() {
        let verses = bible.chapter(book, chapter);
        if remaining < verses.len() {
            let v = &verses[remaining];
            println!("{}", v.text);
            println!("  — {}", v.id.display());
            return;
        }
        remaining -= verses.len();
    }
}

fn cmd_count(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli count <word>");
        process::exit(1);
    }
    let query = args.join(" ").to_ascii_lowercase();
    let bible = load_bible();
    let mut total = 0usize;
    let mut verse_count = 0usize;
    for &(book, chapter) in bible.chapter_list() {
        for verse in bible.chapter(book, chapter) {
            let lower = verse.text.to_ascii_lowercase();
            let count = lower.matches(&query).count();
            if count > 0 {
                total += count;
                verse_count += 1;
            }
        }
    }
    println!("\"{query}\" appears {total} times in {verse_count} verses");
}

fn cmd_context(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli context <verse>");
        process::exit(1);
    }
    let input = args.join(" ");
    let bible = load_bible();
    let Some(id) = bible.parse_reference(&input) else {
        eprintln!("could not resolve: {input}");
        process::exit(1);
    };
    let chapter = bible.chapter_for(id);
    let Some(pos) = chapter.iter().position(|v| v.id == id) else {
        eprintln!("verse not found in chapter");
        process::exit(1);
    };
    let start = pos.saturating_sub(5);
    let end = (pos + 6).min(chapter.len());
    println!("{}\n", id.display());
    for verse in &chapter[start..end] {
        let marker = if verse.id == id { ">" } else { " " };
        println!("{} {} {}", marker, verse.id.verse, verse.text);
    }
}

fn cmd_json(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli json <verse>");
        process::exit(1);
    }
    let input = args.join(" ");
    let bible = load_bible();
    let Some(id) = bible.parse_reference(&input) else {
        eprintln!("could not resolve: {input}");
        process::exit(1);
    };
    // Check if it looks like a chapter reference (no verse specified)
    if let Some(range) = parse_verse_range(&input, &bible) {
        print!("[");
        for (i, vid) in range.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            if let Some(v) = bible.verse(*vid) {
                print_verse_json(v);
            }
        }
        println!("]");
    } else if let Some(v) = bible.verse(id) {
        print_verse_json(v);
        println!();
    }
}

fn print_verse_json(v: &bible::Verse) {
    let text = v.text.replace('\\', "\\\\").replace('"', "\\\"");
    print!(
        "{{\"book\":\"{}\",\"chapter\":{},\"verse\":{},\"text\":\"{}\"}}",
        book_name(v.id.book),
        v.id.chapter,
        v.id.verse,
        text
    );
}

fn cmd_parallel(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli parallel <verse>");
        process::exit(1);
    }
    let input = args.join(" ");
    let Some(id) = parse_reference(&input) else {
        eprintln!("could not resolve: {input}");
        process::exit(1);
    };

    // Load all translations via registry (includes embedded KJV)
    let registry = translation::TranslationRegistry::load().unwrap_or_else(|_| {
        eprintln!("failed to load translation registry");
        process::exit(1);
    });
    let mut entries = registry.into_entries();
    let mut found_any = false;

    println!("{}\n", id.display());
    for entry in &mut entries {
        if entry.ensure_full_loaded().unwrap_or(false) {
            if let Some(bible) = entry.bible() {
                if let Some(v) = bible.verse(id) {
                    println!("  {}: {}", entry.code.to_ascii_uppercase(), v.text);
                    found_any = true;
                }
            }
        }
    }

    if !found_any {
        println!("verse not found: {}", id.display());
        return;
    }

    if entries.len() <= 1 {
        println!(
            "\nonly KJV loaded. set a bible directory with `malacli set-bible-dir <path>` for parallel comparison."
        );
    }
}

fn cmd_outline(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli outline <book>");
        process::exit(1);
    }
    let input = args.join(" ");
    let Some(id) = parse_reference(&input) else {
        eprintln!("could not resolve book: {input}");
        process::exit(1);
    };
    let bible = load_bible();
    let chapters = bible.chapters_for_book(id.book);
    if chapters.is_empty() {
        eprintln!("no chapters found for {}", book_name(id.book));
        process::exit(1);
    }
    println!("{} — {} chapters\n", book_name(id.book), chapters.len());
    for ch in &chapters {
        let verses = bible.chapter(id.book, *ch);
        let first = verses.first().map(|v| v.text.as_str()).unwrap_or("");
        let preview: String = first.chars().take(60).collect();
        println!("  {:>3}  ({:>3} verses)  {}", ch, verses.len(), preview);
    }
}

// -- helpers --

fn parse_verse_range(input: &str, bible: &Bible) -> Option<Vec<VerseId>> {
    // Handle "john 3:16-18" style ranges
    let input = input.trim();
    if !input.contains('-') {
        return None;
    }
    // Try to split on the last hyphen that's part of a verse range
    // e.g. "john 3:16-18" -> base "john 3:16", end verse 18
    let colon_pos = input.rfind(':')?;
    let after_colon = &input[colon_pos + 1..];
    let dash_pos = after_colon.find('-')?;
    let start_str = &input[..colon_pos + 1 + dash_pos];
    let end_verse_str = &after_colon[dash_pos + 1..];
    let start = bible.parse_reference(start_str)?;
    let end_verse: u16 = end_verse_str.trim().parse().ok()?;
    let chapter = bible.chapter_for(start);
    let range: Vec<VerseId> = chapter
        .iter()
        .filter(|v| v.id.verse >= start.verse && v.id.verse <= end_verse)
        .map(|v| v.id)
        .collect();
    if range.is_empty() { None } else { Some(range) }
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

fn cmd_config() {
    let cfg = config::load();
    cfg.display();
}

fn cmd_get(args: &[String]) {
    if args.is_empty() {
        cmd_config();
        return;
    }
    let cfg = config::load();
    match args[0].as_str() {
        "bible-dir" => println!(
            "{}",
            cfg.bible_dir
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(not set)".to_string())
        ),
        "translation" => println!(
            "{}",
            cfg.translation.unwrap_or_else(|| "(not set)".to_string())
        ),
        "theme" => println!("{}", cfg.theme.unwrap_or_else(|| "(not set)".to_string())),
        "editor" => println!("{}", cfg.editor.unwrap_or_else(|| "(not set)".to_string())),
        key => {
            eprintln!("unknown config key: {key}");
            eprintln!("valid keys: bible-dir, translation, theme, editor");
            process::exit(1);
        }
    }
}

fn cmd_set(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: malacli set <key> [value]");
        eprintln!("       malacli set <key> --unset");
        eprintln!("keys:  bible-dir, translation, theme, editor");
        process::exit(1);
    }
    let key = args[0].as_str();
    let value = args.get(1).map(|s| s.as_str());
    let unset = value == Some("--unset") || value == Some("-u");

    let mut cfg = config::load();

    match key {
        "bible-dir" => {
            if unset {
                cfg.bible_dir = None;
                println!("bible-dir cleared.");
            } else {
                let path = value.unwrap_or_else(|| {
                    eprintln!("usage: malacli set bible-dir <path>");
                    process::exit(1);
                });
                let path = PathBuf::from(path);
                if !path.is_dir() {
                    eprintln!("not a directory: {}", path.display());
                    process::exit(1);
                }
                let canonical = path.canonicalize().unwrap_or(path);
                println!("bible-dir = {}", canonical.display());
                cfg.bible_dir = Some(canonical);
            }
        }
        "translation" => {
            if unset {
                cfg.translation = None;
                println!("translation cleared (defaults to kjv).");
            } else {
                let val = value.unwrap_or_else(|| {
                    eprintln!("usage: malacli set translation <code>");
                    process::exit(1);
                });
                cfg.translation = Some(val.to_ascii_lowercase());
                println!("translation = {}", val.to_ascii_lowercase());
            }
        }
        "theme" => {
            if unset {
                cfg.theme = None;
                println!("theme cleared (defaults to monastic).");
            } else {
                let val = value.unwrap_or_else(|| {
                    eprintln!("usage: malacli set theme <monastic|terminal>");
                    process::exit(1);
                });
                match val.to_ascii_lowercase().as_str() {
                    "monastic" | "terminal" => {}
                    _ => {
                        eprintln!("unknown theme: {val}");
                        eprintln!("valid themes: monastic, terminal");
                        process::exit(1);
                    }
                }
                cfg.theme = Some(val.to_ascii_lowercase());
                println!("theme = {}", val.to_ascii_lowercase());
            }
        }
        "editor" => {
            if unset {
                cfg.editor = None;
                println!("editor cleared (uses $EDITOR or vim).");
            } else {
                let val = value.unwrap_or_else(|| {
                    eprintln!("usage: malacli set editor <command>");
                    process::exit(1);
                });
                cfg.editor = Some(val.to_string());
                println!("editor = {val}");
            }
        }
        _ => {
            eprintln!("unknown config key: {key}");
            eprintln!("valid keys: bible-dir, translation, theme, editor");
            process::exit(1);
        }
    }

    config::save(&cfg).unwrap_or_else(|e| {
        eprintln!("failed to save config: {e}");
        process::exit(1);
    });
}

fn print_help() {
    println!(
        "
  ┌┬┐┌─┐┬  ┌─┐┌─┐┬  ┬
  │││├─┤│  ├─┤│  │  │
  ┴ ┴┴ ┴┴─┘┴ ┴└─┘┴─┘┴
  {VERSION}

USAGE
  malacli                         launch the reader
  malacli <reference>             print verse (e.g. john 3:16, gen 1:1-5)
  malacli chapter <ref>           print full chapter
  malacli context <verse>         print surrounding verses
  malacli search <query>          search scripture
  malacli ref <verse>             show cross references
  malacli count <word>            count occurrences
  malacli parallel <verse>        compare across translations
  malacli json <verse>            output verse as json
  malacli books                   list all books with OSIS codes
  malacli toc                     table of contents with chapter counts
  malacli outline <book>          chapter outline for a book
  malacli random                  print a random verse
  malacli history                 show session navigation history
  malacli notes                   list all notes
  malacli info                    show config and session state
  malacli config                  show all config values
  malacli set <key> <value>       set a config value
  malacli set <key> --unset       clear a config value
  malacli get <key>               get a config value
  malacli -h, --help              show this help
  malacli -V, --version           show version

CONFIG KEYS
  bible-dir      path to local translations directory
  translation    default translation code (e.g. esv, nkjv)
  theme          monastic (default) or terminal
  editor         command for note editing (defaults to $EDITOR or vim)

READER CONTROLS (TUI mode)
  q        quit
  g        jump to a passage
  /        search scripture
  x        show cross references
  n        show notes for current chapter
  a        create note at current verse (or add to pinned note)
  P        pin/unpin a note
  tab      toggle reader / side pane focus
  j / k    move verse or selection
  J / K    extend verse selection
  h / l    previous / next chapter
  enter    open selected item
  u / p    back / forward in history
  t        cycle loaded translations
  esc      cancel / clear selection

ENVIRONMENT
  MALACLI_OSIS_DIR        override translations directory
  MALACLI_TRANSLATION     preferred translation on startup
  MALACLI_THEME           'terminal' for color passthrough
  MALACLI_SESSION         override session file path

CONFIG
  ~/.config/malacli/config.toml    settings
  ~/.config/malacli/session.toml   session state
  ~/.config/malacli/notes/         notes directory"
    );
}
