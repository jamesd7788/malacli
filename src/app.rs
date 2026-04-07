use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};

use crate::bible::{
    Bible, CrossReference, SearchHit, Verse, VerseId, book_abbrev, parse_reference, suggest_books,
};
use crate::session;
use crate::translation::{TranslationEntry, TranslationRegistry};

const SEARCH_LIMIT: usize = 50;
const REF_LIMIT: usize = 24;
const READER_SCROLL_MARGIN: usize = 5;
const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Focus {
    Reader,
    Side,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SidePanel {
    CrossReferences,
    Search,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Jump,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HistoryMode {
    Push,
    Replace,
}

pub struct App {
    pub is_running: bool,
    pub translations: Vec<TranslationEntry>,
    pub active_translation: usize,
    pub current_verse: VerseId,
    pub focus: Focus,
    pub side_panel: SidePanel,
    pub mode: InputMode,
    pub input: String,
    pub status: String,
    pub search_results: Vec<SearchHit>,
    pub cross_references: Vec<CrossReference>,
    pub selected_search_result: ListState,
    pub selected_cross_reference: ListState,
    history: VecDeque<VerseId>,
    history_index: usize,
    load_rx: Option<Receiver<TranslationLoadResult>>,
    load_generation: u64,
}

pub struct HistoryItem {
    pub label: String,
    pub current: bool,
}

struct TranslationLoadResult {
    index: usize,
    generation: u64,
    bible: std::result::Result<Bible, String>,
}

impl App {
    pub fn load() -> Result<Self> {
        let registry = TranslationRegistry::load()?;
        let saved_session = session::load();
        let preferred = registry.preferred_code().map(str::to_string);
        let mut translations = registry.into_entries();
        let preferred_translation = preferred.as_deref().or_else(|| {
            saved_session
                .as_ref()
                .map(|state| state.translation.as_str())
        });
        let mut active_translation = preferred_translation
            .and_then(|code| translations.iter().position(|entry| entry.code == code))
            .unwrap_or(0);
        let startup_verse = parse_reference("john 1:1").unwrap_or(VerseId {
            book: 42,
            chapter: 1,
            verse: 1,
        });
        let saved_verse = saved_session
            .as_ref()
            .map(|state| state.current_verse)
            .unwrap_or(startup_verse);
        if !translations[active_translation].load_window(saved_verse)? {
            active_translation = 0;
        }
        let _ = translations[active_translation].load_window(saved_verse)?;
        let bible = translations[active_translation]
            .bible()
            .expect("default translation should load");
        let current_verse = bible
            .verse(saved_verse)
            .map(|verse| verse.id)
            .unwrap_or_else(|| {
                bible
                    .parse_reference("john 1:1")
                    .or_else(|| bible.first_verse())
                    .expect("bible should have at least one verse")
            });
        let cross_references = bible.cross_references(current_verse, REF_LIMIT);
        let mut selected_cross_reference = ListState::default();
        if !cross_references.is_empty() {
            selected_cross_reference.select(Some(0));
        }

        let (history, history_index) = restore_history(saved_session.as_ref(), current_verse);

        let mut app = Self {
            is_running: true,
            translations,
            active_translation,
            current_verse,
            focus: saved_session
                .as_ref()
                .map(|state| state.focus)
                .unwrap_or(Focus::Reader),
            side_panel: saved_session
                .as_ref()
                .map(|state| state.side_panel)
                .unwrap_or(SidePanel::CrossReferences),
            mode: InputMode::Normal,
            input: String::new(),
            status: "Ready. g jump, / search, tab changes pane, enter opens selected item."
                .to_string(),
            search_results: Vec::new(),
            cross_references,
            selected_search_result: ListState::default(),
            selected_cross_reference,
            history,
            history_index,
            load_rx: None,
            load_generation: 0,
        };
        app.start_translation_warmup(active_translation);
        Ok(app)
    }

    pub fn bible(&self) -> &crate::bible::Bible {
        self.translations[self.active_translation]
            .bible()
            .expect("active translation should be loaded")
    }

    pub fn current_chapter(&self) -> &[Verse] {
        self.bible().chapter_for(self.current_verse)
    }

    pub fn current_translation(&self) -> String {
        self.translations[self.active_translation]
            .code
            .to_ascii_uppercase()
    }

    pub fn current_translation_source(&self) -> String {
        self.translations[self.active_translation]
            .source_path
            .display()
            .to_string()
    }

    pub fn active_input_label(&self) -> Option<&'static str> {
        match self.mode {
            InputMode::Normal => None,
            InputMode::Search => Some("SEARCH"),
            InputMode::Jump => Some("JUMP"),
        }
    }

    pub fn side_panel_title(&self) -> String {
        match self.side_panel {
            SidePanel::CrossReferences => {
                format!("Cross References for {}", self.current_verse.display())
            }
            SidePanel::Search => {
                if self.input.is_empty() {
                    "Search".to_string()
                } else {
                    format!("Search / {}", self.input)
                }
            }
        }
    }

    pub fn side_panel_count_label(&self) -> String {
        match self.side_panel {
            SidePanel::CrossReferences => format!("{} refs", self.cross_references.len()),
            SidePanel::Search => format!("{} hits", self.search_results.len()),
        }
    }

    pub fn history_items(&self) -> Vec<HistoryItem> {
        let start = self.history_index.saturating_sub(3);
        let end = (self.history_index + 4).min(self.history.len());
        let mut items = Vec::new();
        for index in start..end {
            let verse = self.history[index];
            items.push(HistoryItem {
                label: short_history_label(verse),
                current: index == self.history_index,
            });
        }
        items
    }

    pub fn save_session(&self) -> std::io::Result<()> {
        let state = session::state_from_parts(
            self.translations[self.active_translation].code.clone(),
            self.current_verse,
            self.focus,
            self.side_panel,
            &self.history,
            self.history_index,
        );
        session::save(&state)
    }

    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }

    pub fn input_hints(&self) -> Vec<String> {
        match self.mode {
            InputMode::Search => {
                if self.input.trim().is_empty() {
                    vec![
                        "Type words or a phrase. Enter leaves search open in the side pane."
                            .to_string(),
                    ]
                } else {
                    vec![format!("{} hits", self.search_results.len())]
                }
            }
            InputMode::Jump => {
                let mut hints = Vec::new();
                if let Some(target) = parse_reference(&self.input) {
                    hints.push(format!("Enter -> {}", target.display()));
                }

                let prefix = jump_book_prefix(&self.input);
                if !prefix.trim().is_empty() {
                    let suggestions = suggest_books(&prefix, 4)
                        .into_iter()
                        .map(|name| name.to_string())
                        .collect::<Vec<_>>();
                    if !suggestions.is_empty() {
                        hints.push(format!("Books: {}", suggestions.join(" | ")));
                    }
                }

                if hints.is_empty() {
                    hints.push("Examples: john, john 3, john 3:16, 1 cor 13".to_string());
                }
                hints
            }
            InputMode::Normal => Vec::new(),
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        self.poll_background_work();

        if self.mode != InputMode::Normal {
            self.handle_input_mode(key);
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) => self.is_running = false,
            (KeyCode::Char('u'), _) => self.go_back(),
            (KeyCode::Char('p'), _) => self.go_forward(),
            (KeyCode::Tab, _) => self.cycle_focus(),
            (KeyCode::Char('/'), _) => self.enter_search_mode(),
            (KeyCode::Char('g'), _) => {
                self.enter_mode(InputMode::Jump, Focus::Reader, "Jump to passage")
            }
            (KeyCode::Char('x'), _) => self.show_cross_references(),
            (KeyCode::Char('t'), _) => self.next_translation(),
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => self.move_selection(1),
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => self.move_selection(-1),
            (KeyCode::Char('h'), _) | (KeyCode::Left, _) => self.previous_chapter(),
            (KeyCode::Char('l'), _) | (KeyCode::Right, _) => self.next_chapter(),
            (KeyCode::Enter, _) => self.open_selection(),
            _ => {}
        }
    }

    pub fn poll_background_work(&mut self) {
        let mut receiver = self.load_rx.take();
        let mut keep_receiver = true;

        if let Some(current) = &receiver {
            loop {
                match current.try_recv() {
                    Ok(result) => {
                        if result.generation != self.load_generation {
                            continue;
                        }

                        match result.bible {
                            Ok(bible) => {
                                self.translations[result.index].set_loaded_bible(bible);
                                if self.active_translation == result.index {
                                    self.ensure_active_verse_loaded();
                                    self.refresh_search();
                                    self.refresh_cross_references();
                                    self.status =
                                        format!("{} fully loaded.", self.current_translation());
                                }
                            }
                            Err(_) => {
                                self.translations[result.index].mark_failed();
                            }
                        }
                        keep_receiver = false;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        keep_receiver = false;
                        break;
                    }
                }
            }
        }

        if keep_receiver {
            self.load_rx = receiver.take();
        }
    }

    fn enter_search_mode(&mut self) {
        self.side_panel = SidePanel::Search;
        self.enter_mode(InputMode::Search, Focus::Side, "Search scripture");
    }

    fn show_cross_references(&mut self) {
        self.side_panel = SidePanel::CrossReferences;
        self.focus = Focus::Side;
        self.status =
            "Cross references focused. j/k move through the index, enter opens the selected verse."
                .to_string();
    }

    fn handle_input_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.exit_mode("Cancelled."),
            KeyCode::Enter => self.submit_input(),
            KeyCode::Backspace => {
                self.input.pop();
                if self.mode == InputMode::Search {
                    self.refresh_search();
                }
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.input.push(ch);
                if self.mode == InputMode::Search {
                    self.refresh_search();
                }
            }
            _ => {}
        }
    }

    fn enter_mode(&mut self, mode: InputMode, focus: Focus, status: &str) {
        self.mode = mode;
        self.focus = focus;
        self.input.clear();
        self.status = status.to_string();
        if mode == InputMode::Search {
            self.refresh_search();
        }
    }

    fn exit_mode(&mut self, status: &str) {
        self.mode = InputMode::Normal;
        self.input.clear();
        self.status = status.to_string();
    }

    fn submit_input(&mut self) {
        match self.mode {
            InputMode::Search => {
                self.mode = InputMode::Normal;
                self.focus = Focus::Side;
                self.side_panel = SidePanel::Search;
                self.status = format!(
                    "{} search hits for \"{}\".",
                    self.search_results.len(),
                    self.input
                );
            }
            InputMode::Jump => {
                let input = self.input.clone();
                self.mode = InputMode::Normal;
                self.input.clear();
                if let Some(target) = parse_reference(&input) {
                    self.open_verse(
                        target,
                        format!("Jumped to {}.", target.display()),
                        HistoryMode::Push,
                    );
                } else {
                    self.status = format!("Could not resolve \"{input}\".");
                }
            }
            InputMode::Normal => {}
        }
    }

    fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Reader => self.move_reader(delta),
            Focus::Side => match self.side_panel {
                SidePanel::Search => move_list_state(
                    &mut self.selected_search_result,
                    self.search_results.len(),
                    delta,
                ),
                SidePanel::CrossReferences => move_list_state(
                    &mut self.selected_cross_reference,
                    self.cross_references.len(),
                    delta,
                ),
            },
        }
    }

    fn move_reader(&mut self, delta: isize) {
        let chapter = self.current_chapter();
        let Some(index) = chapter
            .iter()
            .position(|verse| verse.id == self.current_verse)
        else {
            return;
        };
        let next = if delta.is_negative() {
            index.saturating_sub(delta.unsigned_abs())
        } else {
            (index + delta as usize).min(chapter.len().saturating_sub(1))
        };
        self.current_verse = chapter[next].id;
        self.replace_history_current(self.current_verse);
        self.refresh_cross_references();
    }

    fn open_selection(&mut self) {
        match self.focus {
            Focus::Reader => {}
            Focus::Side => match self.side_panel {
                SidePanel::Search => {
                    if let Some(index) = self.selected_search_result.selected() {
                        if let Some(hit) = self.search_results.get(index) {
                            self.open_verse(
                                hit.verse,
                                format!("Opened search hit {}.", hit.verse.display()),
                                HistoryMode::Push,
                            );
                        }
                    }
                }
                SidePanel::CrossReferences => {
                    if let Some(index) = self.selected_cross_reference.selected() {
                        if let Some(reference) = self
                            .cross_references
                            .get(index)
                            .and_then(|entry| entry.target)
                        {
                            self.open_verse(
                                reference,
                                format!("Opened cross reference {}.", reference.display()),
                                HistoryMode::Push,
                            );
                        }
                    }
                }
            },
        }
    }

    fn open_verse(&mut self, verse: VerseId, status: String, history_mode: HistoryMode) {
        match history_mode {
            HistoryMode::Push => self.push_history(verse),
            HistoryMode::Replace => self.replace_history_current(verse),
        }
        self.current_verse = verse;
        self.ensure_active_verse_loaded();
        self.focus = Focus::Reader;
        self.refresh_cross_references();
        self.status = status;
    }

    fn push_history(&mut self, verse: VerseId) {
        if self.current_verse == verse {
            return;
        }

        while self.history.len() > self.history_index + 1 {
            self.history.pop_back();
        }

        self.history.push_back(verse);
        if self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.history_index = self.history.len().saturating_sub(1);
    }

    fn replace_history_current(&mut self, verse: VerseId) {
        if self.history.is_empty() {
            self.history.push_back(verse);
            self.history_index = 0;
            return;
        }

        self.history[self.history_index] = verse;
    }

    fn go_back(&mut self) {
        if !self.can_go_back() {
            self.status = "No previous history.".to_string();
            return;
        }

        self.history_index -= 1;
        self.current_verse = self.history[self.history_index];
        self.ensure_active_verse_loaded();
        self.focus = Focus::Reader;
        self.refresh_cross_references();
        self.status = format!("Moved back to {}.", self.current_verse.display());
    }

    fn go_forward(&mut self) {
        if !self.can_go_forward() {
            self.status = "No forward history.".to_string();
            return;
        }

        self.history_index += 1;
        self.current_verse = self.history[self.history_index];
        self.ensure_active_verse_loaded();
        self.focus = Focus::Reader;
        self.refresh_cross_references();
        self.status = format!("Moved forward to {}.", self.current_verse.display());
    }

    fn refresh_search(&mut self) {
        if !self.translations[self.active_translation].is_ready() {
            self.search_results.clear();
            self.selected_search_result = ListState::default();
            return;
        }
        self.search_results = self.bible().search(&self.input, SEARCH_LIMIT);
        self.selected_search_result = ListState::default();
        if !self.search_results.is_empty() {
            self.selected_search_result.select(Some(0));
        }
    }

    fn refresh_cross_references(&mut self) {
        self.cross_references = self.bible().cross_references(self.current_verse, REF_LIMIT);
        self.selected_cross_reference = ListState::default();
        if !self.cross_references.is_empty() {
            self.selected_cross_reference.select(Some(0));
        }
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Reader => Focus::Side,
            Focus::Side => Focus::Reader,
        };
        self.status = match self.focus {
            Focus::Reader => "Reader focused. j/k move verse, h/l move chapter, t switches translation, u/p history.".to_string(),
            Focus::Side => match self.side_panel {
                SidePanel::Search => {
                    "Search focused. j/k moves the result index, enter opens the selected hit."
                        .to_string()
                }
                SidePanel::CrossReferences => {
                    "Cross references focused. j/k moves the reference index, enter opens the selected verse."
                        .to_string()
                }
            },
        };
    }

    fn next_translation(&mut self) {
        if self.translations.len() <= 1 {
            self.status = "No additional local translations loaded.".to_string();
            return;
        }
        let start = self.active_translation;

        for step in 1..=self.translations.len() {
            let index = (start + step) % self.translations.len();
            let Ok(loaded) = self.translations[index].load_window(self.current_verse) else {
                continue;
            };
            if !loaded {
                continue;
            }

            self.active_translation = index;
            if self.bible().verse(self.current_verse).is_none() {
                self.current_verse = self
                    .bible()
                    .first_verse()
                    .expect("loaded translation should contain at least one verse");
            }
            self.refresh_search();
            self.refresh_cross_references();
            self.start_translation_warmup(index);
            self.status = format!(
                "Switched to {} from {}. Warming full translation...",
                self.current_translation(),
                self.current_translation_source()
            );
            return;
        }

        self.status = "No additional valid local translations available.".to_string();
    }

    fn next_chapter(&mut self) {
        if self.bible().next_chapter(self.current_verse).is_none()
            && !self.translations[self.active_translation].is_ready()
        {
            let _ = self.translations[self.active_translation].ensure_full_loaded();
        }

        if let Some(next) = self.bible().next_chapter(self.current_verse) {
            let history_mode = if next.book == self.current_verse.book {
                HistoryMode::Replace
            } else {
                HistoryMode::Push
            };
            self.open_verse(next, format!("Moved to {}.", next.display()), history_mode);
        } else if !self.translations[self.active_translation].is_ready() {
            self.status = format!("{} is still loading.", self.current_translation());
        }
    }

    fn previous_chapter(&mut self) {
        if self.bible().previous_chapter(self.current_verse).is_none()
            && !self.translations[self.active_translation].is_ready()
        {
            let _ = self.translations[self.active_translation].ensure_full_loaded();
        }

        if let Some(previous) = self.bible().previous_chapter(self.current_verse) {
            let history_mode = if previous.book == self.current_verse.book {
                HistoryMode::Replace
            } else {
                HistoryMode::Push
            };
            self.open_verse(
                previous,
                format!("Moved to {}.", previous.display()),
                history_mode,
            );
        } else if !self.translations[self.active_translation].is_ready() {
            self.status = format!("{} is still loading.", self.current_translation());
        }
    }

    fn ensure_active_verse_loaded(&mut self) {
        let _ = self.translations[self.active_translation].load_window(self.current_verse);
        if self.bible().verse(self.current_verse).is_none() {
            if let Some(fallback) = self.bible().chapter_for(self.current_verse).first() {
                self.current_verse = fallback.id;
            } else if let Some(first) = self.bible().first_verse() {
                self.current_verse = first;
            }
        }
    }

    fn start_translation_warmup(&mut self, index: usize) {
        self.load_generation += 1;
        let generation = self.load_generation;
        let source_path = self.translations[index].source_path.clone();
        let (tx, rx) = mpsc::channel();
        self.load_rx = Some(rx);

        thread::spawn(move || {
            let bible = Bible::load(
                &source_path,
                std::path::Path::new("data/raw/cross_references.txt"),
            )
            .map_err(|error| error.to_string());

            let _ = tx.send(TranslationLoadResult {
                index,
                generation,
                bible,
            });
        });
    }

    pub fn effective_reader_scroll(
        &self,
        viewport_height: usize,
        selected_line_top: usize,
        total_lines: usize,
    ) -> usize {
        let max_scroll = total_lines.saturating_sub(viewport_height);
        let target = if selected_line_top <= READER_SCROLL_MARGIN || viewport_height == 0 {
            0
        } else {
            selected_line_top.saturating_sub(READER_SCROLL_MARGIN)
        };
        target.min(max_scroll)
    }

    pub fn selected_search_hit(&self) -> Option<&SearchHit> {
        self.selected_search_result
            .selected()
            .and_then(|index| self.search_results.get(index))
    }

    pub fn selected_cross_reference(&self) -> Option<&CrossReference> {
        self.selected_cross_reference
            .selected()
            .and_then(|index| self.cross_references.get(index))
    }
}

fn short_history_label(verse: VerseId) -> String {
    format!(
        "{} {}:{}",
        book_abbrev(verse.book),
        verse.chapter,
        verse.verse
    )
}

fn restore_history(
    saved_session: Option<&session::SessionState>,
    current_verse: VerseId,
) -> (VecDeque<VerseId>, usize) {
    let mut history = saved_session
        .map(|state| state.history.iter().copied().collect::<VecDeque<_>>())
        .filter(|history: &VecDeque<VerseId>| !history.is_empty())
        .unwrap_or_else(|| VecDeque::from([current_verse]));

    if !history.iter().any(|verse| *verse == current_verse) {
        history.push_back(current_verse);
    }

    while history.len() > HISTORY_LIMIT {
        history.pop_front();
    }

    let mut history_index = saved_session
        .map(|state| state.history_index)
        .unwrap_or_else(|| history.len().saturating_sub(1))
        .min(history.len().saturating_sub(1));

    if history[history_index] != current_verse {
        history_index = history
            .iter()
            .position(|verse| *verse == current_verse)
            .unwrap_or_else(|| history.len().saturating_sub(1));
    }

    (history, history_index)
}

fn jump_book_prefix(input: &str) -> String {
    let mut parts = Vec::new();
    for token in input.split_whitespace() {
        if token.chars().any(|ch| ch.is_ascii_digit()) && token.contains(':') {
            break;
        }
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            break;
        }
        parts.push(token);
    }
    parts.join(" ")
}

fn move_list_state(state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        state.select(None);
        return;
    }

    let current = state.selected().unwrap_or(0);
    let next = if delta.is_negative() {
        let steps = delta.unsigned_abs() % len;
        (current + len - steps) % len
    } else {
        (current + delta as usize) % len
    };
    state.select(Some(next));
}

#[cfg(test)]
mod tests {
    use super::{App, Focus, InputMode, SidePanel};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::time::{Duration, Instant};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn wait_for_translation(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !app.translations[app.active_translation].is_ready() && Instant::now() < deadline {
            app.poll_background_work();
            std::thread::sleep(Duration::from_millis(10));
        }
        app.poll_background_work();
    }

    #[test]
    fn search_mode_populates_results() {
        let mut app = App::load().unwrap();
        wait_for_translation(&mut app);
        app.handle_key_event(key(KeyCode::Char('/')));
        for ch in "beginning".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        assert_eq!(app.mode, InputMode::Search);
        assert_eq!(app.side_panel, SidePanel::Search);
        assert!(!app.search_results.is_empty());
    }

    #[test]
    fn jump_mode_resolves_real_reference() {
        let mut app = App::load().unwrap();
        app.handle_key_event(key(KeyCode::Char('g')));
        for ch in "gen 1:1".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        app.handle_key_event(key(KeyCode::Enter));
        assert_eq!(app.current_verse.display(), "Genesis 1:1");
        assert_eq!(app.focus, Focus::Reader);
    }

    #[test]
    fn tab_cycles_between_reader_and_side() {
        let mut app = App::load().unwrap();
        assert_eq!(app.focus, Focus::Reader);
        app.handle_key_event(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Side);
        app.handle_key_event(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Reader);
    }

    #[test]
    fn side_j_moves_selected_result() {
        let mut app = App::load().unwrap();
        wait_for_translation(&mut app);
        app.handle_key_event(key(KeyCode::Char('/')));
        for ch in "light".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        app.handle_key_event(key(KeyCode::Enter));
        let first = app.selected_search_result.selected();
        app.handle_key_event(key(KeyCode::Char('j')));
        assert_ne!(app.selected_search_result.selected(), first);
    }

    #[test]
    fn u_and_p_move_through_history() {
        let mut app = App::load().unwrap();
        app.handle_key_event(key(KeyCode::Char('g')));
        for ch in "john 1:1".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        app.handle_key_event(key(KeyCode::Enter));
        app.handle_key_event(key(KeyCode::Char('g')));
        for ch in "rom 1:1".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        app.handle_key_event(key(KeyCode::Enter));
        app.handle_key_event(key(KeyCode::Char('l')));
        let second = app.current_verse;
        app.handle_key_event(key(KeyCode::Char('u')));
        assert_eq!(app.current_verse.display(), "John 1:1");
        app.handle_key_event(key(KeyCode::Char('p')));
        assert_eq!(app.current_verse, second);
    }

    #[test]
    fn reader_movement_replaces_current_history_item() {
        let mut app = App::load().unwrap();
        app.handle_key_event(key(KeyCode::Char('g')));
        for ch in "john 3:16".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        app.handle_key_event(key(KeyCode::Enter));
        let history_len = app.history.len();
        app.handle_key_event(key(KeyCode::Char('j')));
        assert_eq!(app.history.len(), history_len);
        assert_eq!(app.history[app.history_index], app.current_verse);
    }

    #[test]
    fn same_book_chapter_movement_replaces_current_history_item() {
        let mut app = App::load().unwrap();
        app.handle_key_event(key(KeyCode::Char('g')));
        for ch in "col 3:1".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        app.handle_key_event(key(KeyCode::Enter));
        let history_len = app.history.len();
        app.handle_key_event(key(KeyCode::Char('l')));
        assert_eq!(app.current_verse.display(), "Colossians 4:1");
        assert_eq!(app.history.len(), history_len);
        assert_eq!(app.history[app.history_index], app.current_verse);
    }

    #[test]
    fn cross_book_chapter_movement_pushes_history_item() {
        let mut app = App::load().unwrap();
        app.handle_key_event(key(KeyCode::Char('g')));
        for ch in "col 4:1".chars() {
            app.handle_key_event(key(KeyCode::Char(ch)));
        }
        app.handle_key_event(key(KeyCode::Enter));
        let history_len = app.history.len();
        app.handle_key_event(key(KeyCode::Char('l')));
        assert_eq!(app.current_verse.display(), "1 Thessalonians 1:1");
        assert_eq!(app.history.len(), history_len + 1);
        app.handle_key_event(key(KeyCode::Char('u')));
        assert_eq!(app.current_verse.display(), "Colossians 4:1");
    }
}
