use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};

use crate::{
    app::{App, Focus, InputMode, SidePanel},
    bible::{Verse, book_name},
};

const BG: Color = Color::Rgb(23, 18, 14);
const PANEL: Color = Color::Rgb(35, 28, 22);
const TEXT: Color = Color::Rgb(235, 225, 204);
const MUTED: Color = Color::Rgb(145, 130, 108);
const ACCENT: Color = Color::Rgb(194, 155, 92);
const SELECT: Color = Color::Rgb(77, 52, 31);
const STRONG: Color = Color::Rgb(212, 174, 102);
const TITLE_FOCUS: Color = Color::Rgb(228, 203, 152);
const READER_SELECTED_TEXT: Color = Color::Rgb(250, 239, 216);
const READING_COLUMN_MAX: usize = 84;
const READER_PARAGRAPH_VERSES: usize = 4;

#[derive(Clone, Copy)]
struct Theme {
    transparent: bool,
    bg: Color,
    panel: Color,
    text: Color,
    muted: Color,
    accent: Color,
    select: Color,
    strong: Color,
    title_focus: Color,
    reader_selected_text: Color,
}

impl Theme {
    fn current() -> Self {
        let theme = std::env::var("MALACLI_THEME")
            .ok()
            .or_else(|| crate::config::load().theme);
        match theme.as_deref() {
            Some(v) if v.eq_ignore_ascii_case("terminal") => Self::terminal(),
            _ => Self::monastic(),
        }
    }

    fn monastic() -> Self {
        Self {
            transparent: false,
            bg: BG,
            panel: PANEL,
            text: TEXT,
            muted: MUTED,
            accent: ACCENT,
            select: SELECT,
            strong: STRONG,
            title_focus: TITLE_FOCUS,
            reader_selected_text: READER_SELECTED_TEXT,
        }
    }

    fn terminal() -> Self {
        Self {
            transparent: true,
            bg: Color::Reset,
            panel: Color::Reset,
            text: Color::Reset,
            muted: Color::DarkGray,
            accent: Color::Yellow,
            select: Color::DarkGray,
            strong: Color::Yellow,
            title_focus: Color::Yellow,
            reader_selected_text: Color::Reset,
        }
    }

    fn bg_style(self) -> Style {
        if self.transparent {
            Style::default()
        } else {
            Style::default().bg(self.bg)
        }
    }

    fn surface_style(self, color: Color) -> Style {
        if self.transparent {
            Style::default()
        } else {
            Style::default().bg(color)
        }
    }

    fn fg(self, color: Color) -> Style {
        if self.transparent && color == Color::Reset {
            Style::default()
        } else {
            Style::default().fg(color)
        }
    }

    fn selected(self) -> Style {
        if self.transparent {
            Style::default().reversed().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(self.select)
                .fg(self.text)
                .add_modifier(Modifier::BOLD)
        }
    }
}

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let theme = Theme::current();

    frame.render_widget(Block::default().style(theme.bg_style()), frame.area());

    let [header, body, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(4),
        ])
        .areas(frame.area());

    render_header(frame, app, header, theme);
    render_body(frame, app, body, theme);
    render_footer(frame, app, footer, theme);
}

fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let current = app.current_verse;
    let chapter_label = format!(
        "{} {} [{}]",
        book_name(current.book),
        current.chapter,
        app.current_translation()
    );
    let mode_label = match app.mode {
        InputMode::Normal => "READ",
        InputMode::Search => "SEARCH",
        InputMode::Jump => "JUMP",
    };

    let [top, tabs] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .areas(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "TUI BIBLE",
            theme.fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            chapter_label,
            theme.fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(mode_label, theme.fg(theme.muted)),
        Span::raw("   "),
        Span::styled(app.side_panel_count_label(), theme.fg(theme.muted)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme.fg(theme.muted))
            .padding(Padding::new(1, 1, 0, 0))
            .style(theme.bg_style()),
    );

    let side_label = match app.side_panel {
        SidePanel::CrossReferences => " Side: Cross Refs ",
        SidePanel::Search => " Side: Search ",
        SidePanel::Notes => " Side: Notes ",
    };

    let mut tabs_spans = vec![
        pane_tab(" Reader ", app.focus == Focus::Reader, theme),
        Span::raw(" "),
        pane_tab(side_label, app.focus == Focus::Side, theme),
        Span::raw("   "),
        Span::styled(
            "u back",
            theme.fg(if app.can_go_back() {
                theme.text
            } else {
                theme.muted
            }),
        ),
        Span::raw("  "),
        Span::styled(
            "p fwd",
            theme.fg(if app.can_go_forward() {
                theme.text
            } else {
                theme.muted
            }),
        ),
        Span::raw("  "),
        Span::styled("j/k move", theme.fg(theme.muted)),
        Span::raw("   "),
        Span::styled("History ", theme.fg(theme.accent)),
    ];
    tabs_spans.extend(history_spans(app, theme));
    if app.pinned_note.is_some() {
        tabs_spans.push(Span::raw("   "));
        tabs_spans.push(Span::styled(
            "PINNED",
            theme.fg(theme.strong).add_modifier(Modifier::BOLD),
        ));
    }
    let tabs_line = Line::from(tabs_spans);
    let tabs_widget = Paragraph::new(tabs_line).block(
        Block::default()
            .padding(Padding::new(1, 1, 0, 0))
            .style(theme.bg_style()),
    );

    frame.render_widget(title, top);
    frame.render_widget(tabs_widget, tabs);
}

fn history_spans(app: &App, theme: Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let items = app.history_items();
    if items.is_empty() {
        return spans;
    }

    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }

        let style = if item.current {
            theme.selected()
        } else {
            theme.fg(theme.muted)
        };
        spans.push(Span::styled(format!(" {} ", item.label), style));
    }

    spans
}

fn pane_tab(label: &'static str, active: bool, theme: Theme) -> Span<'static> {
    if active {
        Span::styled(
            label,
            theme.fg(theme.title_focus).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label, theme.fg(theme.muted))
    }
}

fn render_body(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let [reader, side] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .areas(area);

    render_reader(frame, app, reader, theme);
    render_side_panel(frame, app, side, theme);
}

fn render_reader(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let border = if app.focus == Focus::Reader {
        theme.strong
    } else {
        theme.muted
    };
    let title_style = if app.focus == Focus::Reader {
        theme.fg(theme.title_focus).add_modifier(Modifier::BOLD)
    } else {
        theme.fg(theme.accent)
    };
    let available_width = area.width.saturating_sub(8).max(12) as usize;
    let content_width = available_width.min(READING_COLUMN_MAX);
    let left_pad = available_width.saturating_sub(content_width) / 2;
    let reader_view = build_reader_view(app, content_width, left_pad, theme);
    let viewport_height = area.height.saturating_sub(4) as usize;
    let scroll = app.effective_reader_scroll(
        viewport_height,
        reader_view.selected_line_top,
        reader_view.selected_line_bottom,
        reader_view.lines.len(),
    ) as u16;
    let paragraph = Paragraph::new(reader_view.lines).scroll((scroll, 0)).block(
        Block::default()
            .title(Span::styled(
                format!("Reading  {}", app.current_verse.display()),
                title_style,
            ))
            .borders(Borders::ALL)
            .border_style(theme.fg(border))
            .padding(Padding::new(1, 2, 1, 0))
            .style(theme.bg_style()),
    );

    frame.render_widget(paragraph, area);
}

struct ReaderView {
    lines: Vec<Line<'static>>,
    selected_line_top: usize,
    selected_line_bottom: usize,
}

#[derive(Clone)]
struct StyledWord {
    text: String,
    style: Style,
    verse_selected: bool,
}

fn build_reader_view(app: &App, width: usize, left_pad: usize, theme: Theme) -> ReaderView {
    let selected = app.selected_verse_range();
    build_chapter_context_view(
        app.current_chapter(),
        app.current_verse,
        app.current_translation(),
        &[],
        &selected,
        width,
        left_pad,
        theme,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_chapter_context_view(
    chapter: &[Verse],
    current: crate::bible::VerseId,
    translation: String,
    search_terms: &[String],
    selected_verses: &[crate::bible::VerseId],
    width: usize,
    left_pad: usize,
    theme: Theme,
) -> ReaderView {
    let mut lines = Vec::new();
    let mut selected_line_top = 0usize;
    let mut selected_line_bottom = 0usize;
    let title = format!(
        "{} {}",
        book_name(current.book).to_ascii_uppercase(),
        current.chapter
    );
    let subtitle = format!("{} verses · {}", chapter.len(), translation);

    lines.push(centered_line(
        &title,
        width,
        left_pad,
        theme.fg(theme.title_focus).add_modifier(Modifier::BOLD),
    ));
    lines.push(centered_line(
        &subtitle,
        width,
        left_pad,
        theme.fg(theme.muted),
    ));
    lines.push(Line::raw(""));

    for (paragraph_index, paragraph) in chapter.chunks(READER_PARAGRAPH_VERSES).enumerate() {
        if paragraph_index > 0 {
            lines.push(Line::raw(""));
        }

        let words = paragraph_words(paragraph, current, search_terms, selected_verses, theme);
        let wrapped = wrap_styled_words(&words, width, left_pad, theme);
        for line in wrapped {
            if line.contains_selected {
                if selected_line_top == 0 {
                    selected_line_top = lines.len();
                }
                selected_line_bottom = lines.len();
            }
            lines.push(line.line);
        }
    }

    ReaderView {
        lines,
        selected_line_top,
        selected_line_bottom,
    }
}

fn paragraph_words(
    verses: &[Verse],
    current: crate::bible::VerseId,
    search_terms: &[String],
    selected_verses: &[crate::bible::VerseId],
    theme: Theme,
) -> Vec<StyledWord> {
    let mut words = Vec::new();

    for verse in verses {
        let selected = selected_verses.contains(&verse.id) || verse.id == current;
        let number_style = if selected {
            theme.fg(theme.title_focus).add_modifier(Modifier::BOLD)
        } else {
            theme.fg(theme.muted)
        };
        let text_style = if selected {
            theme
                .fg(theme.reader_selected_text)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.fg(theme.text)
        };

        words.push(StyledWord {
            text: format!("{} ", verse.id.verse),
            style: number_style,
            verse_selected: selected,
        });

        for word in verse.text.split_whitespace() {
            let search_match = word_matches_search(word, search_terms);
            let style = if search_match {
                theme.fg(theme.strong).add_modifier(Modifier::BOLD)
            } else {
                text_style
            };
            words.push(StyledWord {
                text: word.to_string(),
                style,
                verse_selected: selected,
            });
        }
    }

    words
}

fn word_matches_search(word: &str, search_terms: &[String]) -> bool {
    if search_terms.is_empty() {
        return false;
    }

    let normalized = normalize_search_word(word);
    search_terms
        .iter()
        .any(|term| !term.is_empty() && normalized.contains(term))
}

fn normalize_search_word(word: &str) -> String {
    word.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

struct WrappedLine {
    line: Line<'static>,
    contains_selected: bool,
}

fn wrap_styled_words(
    words: &[StyledWord],
    width: usize,
    left_pad: usize,
    theme: Theme,
) -> Vec<WrappedLine> {
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0usize;
    let mut contains_selected = false;
    let text_width = width.saturating_sub(2).max(12);

    for word in words {
        let word_width = word.text.chars().count();
        let spacer = usize::from(!current.is_empty());
        if current_width + spacer + word_width > text_width && !current.is_empty() {
            lines.push(styled_line(
                std::mem::take(&mut current),
                contains_selected,
                left_pad,
                theme,
            ));
            current_width = 0;
            contains_selected = false;
        }

        if !current.is_empty() {
            current.push(Span::raw(" "));
            current_width += 1;
        }
        current.push(Span::styled(word.text.clone(), word.style));
        current_width += word_width;
        contains_selected |= word.verse_selected;
    }

    if !current.is_empty() {
        lines.push(styled_line(current, contains_selected, left_pad, theme));
    }

    if lines.is_empty() {
        lines.push(WrappedLine {
            line: Line::raw(""),
            contains_selected: false,
        });
    }

    lines
}

fn styled_line(
    mut spans: Vec<Span<'static>>,
    contains_selected: bool,
    left_pad: usize,
    theme: Theme,
) -> WrappedLine {
    let mut line_spans = Vec::new();
    line_spans.push(Span::raw(" ".repeat(left_pad)));
    if contains_selected {
        line_spans.push(Span::styled("▌ ", theme.fg(theme.accent)));
    } else {
        line_spans.push(Span::raw("  "));
    }
    line_spans.append(&mut spans);

    WrappedLine {
        line: Line::from(line_spans),
        contains_selected,
    }
}

fn centered_line(text: &str, width: usize, left_pad: usize, style: Style) -> Line<'static> {
    let text_width = text.chars().count();
    let inner_pad = width.saturating_sub(text_width) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(left_pad + inner_pad)),
        Span::styled(text.to_string(), style),
    ])
}

fn render_side_panel(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let [index_area, preview_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .areas(area);

    render_side_index(frame, app, index_area, theme);
    render_side_preview(frame, app, preview_area, theme);
}

fn render_side_index(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let border = if app.focus == Focus::Side {
        theme.strong
    } else {
        theme.muted
    };
    let title_style = if app.focus == Focus::Side {
        theme.fg(theme.title_focus).add_modifier(Modifier::BOLD)
    } else {
        theme.fg(theme.accent)
    };

    let items = side_index_items(app);
    let selected = side_selected_index(app);
    let lines = side_index_lines(
        &items,
        selected,
        area.height.saturating_sub(4) as usize,
        app.focus == Focus::Side,
        theme,
    );

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(app.side_panel_title(), title_style))
            .borders(Borders::ALL)
            .border_style(theme.fg(border))
            .padding(Padding::new(1, 1, 1, 0))
            .style(theme.surface_style(theme.panel)),
    );

    frame.render_widget(paragraph, area);
}

fn render_side_preview(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let border = if app.focus == Focus::Side {
        theme.strong
    } else {
        theme.muted
    };
    let title_style = if app.focus == Focus::Side {
        theme.fg(theme.title_focus).add_modifier(Modifier::BOLD)
    } else {
        theme.fg(theme.accent)
    };

    let available_width = area.width.saturating_sub(4).max(12) as usize;
    let content_width = available_width.min(READING_COLUMN_MAX);
    let left_pad = available_width.saturating_sub(content_width) / 2;
    let viewport_height = area.height.saturating_sub(4) as usize;
    let (title, preview) = side_preview_view(app, content_width, left_pad, theme);
    let scroll = center_preview_scroll(
        viewport_height,
        preview.selected_line_top,
        preview.lines.len(),
    ) as u16;

    let paragraph = Paragraph::new(preview.lines).scroll((scroll, 0)).block(
        Block::default()
            .title(Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(theme.fg(border))
            .padding(Padding::new(1, 1, 1, 0))
            .style(theme.surface_style(theme.panel)),
    );

    frame.render_widget(paragraph, area);
}

fn side_preview_view(
    app: &App,
    width: usize,
    left_pad: usize,
    theme: Theme,
) -> (String, ReaderView) {
    match app.side_panel {
        SidePanel::CrossReferences => {
            if let Some(entry) = app.selected_cross_reference() {
                if let Some(target) = entry.target {
                    let chapter = app.bible().chapter_for(target);
                    if !chapter.is_empty() {
                        return (
                            target.display(),
                            build_chapter_context_view(
                                chapter,
                                target,
                                app.current_translation(),
                                &[],
                                &[target],
                                width,
                                left_pad,
                                theme,
                            ),
                        );
                    }
                    return (
                        target.display(),
                        message_preview("Verse text is not loaded.", theme),
                    );
                }
                return (
                    entry.target_label.clone(),
                    message_preview(entry.target_label.as_str(), theme),
                );
            }
            (
                "Cross Reference Preview".to_string(),
                message_preview("No cross reference selected.", theme),
            )
        }
        SidePanel::Search => {
            if let Some(hit) = app.selected_search_hit() {
                let chapter = app.bible().chapter_for(hit.verse);
                if !chapter.is_empty() {
                    let terms = search_terms(&app.input);
                    return (
                        hit.verse.display(),
                        build_chapter_context_view(
                            chapter,
                            hit.verse,
                            app.current_translation(),
                            &terms,
                            &[hit.verse],
                            width,
                            left_pad,
                            theme,
                        ),
                    );
                }
                return (
                    hit.verse.display(),
                    message_preview("Verse text is not loaded.", theme),
                );
            }
            (
                "Search Preview".to_string(),
                message_preview("No search result selected.", theme),
            )
        }
        SidePanel::Notes => {
            if let Some(note) = app.selected_note() {
                let mut lines = Vec::new();
                if !note.verses.is_empty() {
                    let refs: Vec<String> = note.verses.iter().map(|v| v.display()).collect();
                    lines.push(Line::from(Span::styled(
                        refs.join(", "),
                        theme.fg(theme.accent),
                    )));
                    lines.push(Line::from(""));
                }
                for text_line in note.body.lines() {
                    let wrapped = wrap_text(text_line, width);
                    for w in wrapped {
                        let pad = " ".repeat(left_pad);
                        lines.push(Line::from(Span::styled(
                            format!("{pad}{w}"),
                            theme.fg(theme.text),
                        )));
                    }
                }
                if lines.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "(empty note — press enter to edit)",
                        theme.fg(theme.muted),
                    )));
                }
                (
                    "Note Preview".to_string(),
                    ReaderView {
                        lines,
                        selected_line_top: 0,
                        selected_line_bottom: 0,
                    },
                )
            } else {
                (
                    "Note Preview".to_string(),
                    message_preview("No note selected. Press a to create one.", theme),
                )
            }
        }
    }
}

fn message_preview(message: &str, theme: Theme) -> ReaderView {
    ReaderView {
        lines: vec![Line::from(Span::styled(
            message.to_string(),
            theme.fg(theme.muted),
        ))],
        selected_line_top: 0,
        selected_line_bottom: 0,
    }
}

fn search_terms(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(normalize_search_word)
        .filter(|term| !term.is_empty())
        .collect()
}

fn center_preview_scroll(
    viewport_height: usize,
    selected_line_top: usize,
    total_lines: usize,
) -> usize {
    let max_scroll = total_lines.saturating_sub(viewport_height);
    let midpoint = viewport_height / 2;
    selected_line_top.saturating_sub(midpoint).min(max_scroll)
}

fn side_index_items(app: &App) -> Vec<String> {
    match app.side_panel {
        SidePanel::CrossReferences => {
            if app.cross_references.is_empty() {
                vec!["No cross references for this verse yet.".to_string()]
            } else {
                app.cross_references
                    .iter()
                    .map(|entry| {
                        if let Some(target) = entry.target {
                            format!("{}  {}", app.bible().verse_preview(target, 34), entry.votes)
                        } else {
                            format!("{}  {}", entry.target_label, entry.votes)
                        }
                    })
                    .collect()
            }
        }
        SidePanel::Search => {
            if app.search_results.is_empty() {
                vec!["Type / then search for any word or phrase.".to_string()]
            } else {
                app.search_results
                    .iter()
                    .map(|hit| app.bible().verse_preview(hit.verse, 40))
                    .collect()
            }
        }
        SidePanel::Notes => {
            if app.chapter_notes.is_empty() {
                vec!["No notes for this chapter. Press a to create one.".to_string()]
            } else {
                app.chapter_notes
                    .iter()
                    .map(|note| {
                        let preview = note
                            .body
                            .lines()
                            .next()
                            .unwrap_or("(empty)")
                            .chars()
                            .take(40)
                            .collect::<String>();
                        let refs = note.verses.len();
                        format!(
                            "{preview}  ({refs} ref{})",
                            if refs == 1 { "" } else { "s" }
                        )
                    })
                    .collect()
            }
        }
    }
}

fn side_selected_index(app: &App) -> Option<usize> {
    match app.side_panel {
        SidePanel::CrossReferences => app
            .selected_cross_reference()
            .map(|_| app.selected_cross_reference.selected().unwrap_or(0)),
        SidePanel::Search => app
            .selected_search_hit()
            .map(|_| app.selected_search_result.selected().unwrap_or(0)),
        SidePanel::Notes => app
            .selected_note()
            .map(|_| app.selected_note.selected().unwrap_or(0)),
    }
}

fn side_index_lines(
    items: &[String],
    selected: Option<usize>,
    viewport_height: usize,
    focused: bool,
    theme: Theme,
) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![Line::from(Span::styled("", theme.fg(theme.muted)))];
    }

    let selected = selected.unwrap_or(0).min(items.len().saturating_sub(1));
    let offset = if viewport_height == 0 || selected < viewport_height {
        0
    } else {
        selected + 1 - viewport_height
    };

    items
        .iter()
        .enumerate()
        .skip(offset)
        .take(viewport_height.max(1))
        .map(|(index, item)| {
            let active = index == selected && focused;
            let prefix = if active { "› " } else { "  " };
            let style = if active {
                theme.selected()
            } else if index == selected {
                theme.fg(theme.accent)
            } else {
                theme.fg(theme.text)
            };
            Line::from(vec![
                Span::styled(prefix.to_string(), style),
                Span::styled(item.clone(), style),
            ])
        })
        .collect()
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let mut lines = vec![Line::from(vec![
        Span::styled("Keys ", theme.fg(theme.accent)),
        Span::styled("/ search", theme.fg(theme.text)),
        Span::raw("  "),
        Span::styled("x refs", theme.fg(theme.text)),
        Span::raw("  "),
        Span::styled("g jump", theme.fg(theme.text)),
        Span::raw("  "),
        Span::styled("tab pane", theme.fg(theme.text)),
        Span::raw("  "),
        Span::styled("j/k move", theme.fg(theme.text)),
        Span::raw("  "),
        Span::styled("enter open", theme.fg(theme.text)),
        Span::raw("  "),
        Span::styled("u back", theme.fg(theme.text)),
        Span::raw("  "),
        Span::styled("p fwd", theme.fg(theme.text)),
    ])];

    if let Some(label) = app.active_input_label() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{label}> "),
                theme.fg(theme.strong).add_modifier(Modifier::BOLD),
            ),
            Span::styled(app.input.as_str(), theme.fg(theme.text)),
        ]));

        let hints = app.input_hints().join("   ");
        lines.push(Line::from(Span::styled(hints, theme.fg(theme.muted))));
    } else {
        lines.push(Line::from(Span::styled(
            app.status.as_str(),
            theme.fg(theme.muted),
        )));
        lines.push(Line::from(Span::styled(
            match (app.focus, app.side_panel) {
                (Focus::Reader, _) => "Reader uses plain j/k line movement.",
                (Focus::Side, SidePanel::Search) => {
                    "Side pane search: top index, bottom full verse preview. j/k moves the index."
                }
                (Focus::Side, SidePanel::CrossReferences) => {
                    "Side pane refs: top index, bottom full verse preview. j/k moves the index."
                }
                (Focus::Side, SidePanel::Notes) => {
                    "Notes: j/k moves, enter opens in $EDITOR, a creates new note."
                }
            },
            theme.fg(theme.muted),
        )));
    }

    let footer = Paragraph::new(lines)
        .style(theme.bg_style())
        .block(Block::default().padding(Padding::new(1, 1, 0, 0)));

    frame.render_widget(footer, area);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let max = width.max(10);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for word in text.split_whitespace() {
        let word_width = word.chars().count();
        let spacer = usize::from(!current.is_empty());
        if current_width + spacer + word_width > max && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += 1;
        }
        current.push_str(word);
        current_width += word_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
