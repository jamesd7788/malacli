use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
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

pub fn render(frame: &mut Frame<'_>, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(BG)),
        frame.area(),
    );

    let [header, body, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(4),
        ])
        .areas(frame.area());

    render_header(frame, app, header);
    render_body(frame, app, body);
    render_footer(frame, app, footer);
}

fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
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
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            chapter_label,
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(mode_label, Style::default().fg(MUTED)),
        Span::raw("   "),
        Span::styled(app.side_panel_count_label(), Style::default().fg(MUTED)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(MUTED))
            .padding(Padding::new(1, 1, 0, 0))
            .style(Style::default().bg(BG)),
    );

    let side_label = match app.side_panel {
        SidePanel::CrossReferences => " Side: Cross Refs ",
        SidePanel::Search => " Side: Search ",
    };

    let tabs_line = Line::from(vec![
        pane_tab(" Reader ", app.focus == Focus::Reader),
        Span::raw(" "),
        pane_tab(side_label, app.focus == Focus::Side),
        Span::raw("   "),
        Span::styled(
            "u back",
            Style::default().fg(if app.can_go_back() { TEXT } else { MUTED }),
        ),
        Span::raw("  "),
        Span::styled(
            "p fwd",
            Style::default().fg(if app.can_go_forward() { TEXT } else { MUTED }),
        ),
        Span::raw("  "),
        Span::styled("j/k move", Style::default().fg(MUTED)),
    ]);
    let tabs_widget = Paragraph::new(tabs_line).block(
        Block::default()
            .padding(Padding::new(1, 1, 0, 0))
            .style(Style::default().bg(BG)),
    );

    frame.render_widget(title, top);
    frame.render_widget(tabs_widget, tabs);
}

fn pane_tab(label: &'static str, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            label,
            Style::default()
                .fg(TITLE_FOCUS)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label, Style::default().fg(MUTED))
    }
}

fn render_body(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let [reader, side] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .areas(area);

    render_reader(frame, app, reader);
    render_side_panel(frame, app, side);
}

fn render_reader(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let border = if app.focus == Focus::Reader {
        STRONG
    } else {
        MUTED
    };
    let title_style = if app.focus == Focus::Reader {
        Style::default()
            .fg(TITLE_FOCUS)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ACCENT)
    };
    let available_width = area.width.saturating_sub(8).max(12) as usize;
    let content_width = available_width.min(READING_COLUMN_MAX);
    let left_pad = available_width.saturating_sub(content_width) / 2;
    let reader_view = build_reader_view(app, content_width, left_pad);
    let viewport_height = area.height.saturating_sub(4) as usize;
    let scroll = app.effective_reader_scroll(
        viewport_height,
        reader_view.selected_line_top,
        reader_view.lines.len(),
    ) as u16;
    let paragraph = Paragraph::new(reader_view.lines).scroll((scroll, 0)).block(
        Block::default()
            .title(Span::styled(
                format!("Reading  {}", app.current_verse.display()),
                title_style,
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .padding(Padding::new(1, 2, 1, 0))
            .style(Style::default().bg(BG)),
    );

    frame.render_widget(paragraph, area);
}

struct ReaderView {
    lines: Vec<Line<'static>>,
    selected_line_top: usize,
}

#[derive(Clone)]
struct StyledWord {
    text: String,
    style: Style,
    verse_selected: bool,
}

fn build_reader_view(app: &App, width: usize, left_pad: usize) -> ReaderView {
    let mut lines = Vec::new();
    let mut selected_line_top = 0usize;
    let title = format!(
        "{} {}",
        book_name(app.current_verse.book).to_ascii_uppercase(),
        app.current_verse.chapter
    );
    let subtitle = format!(
        "{} verses · {}",
        app.current_chapter().len(),
        app.current_translation()
    );

    lines.push(centered_line(
        &title,
        width,
        left_pad,
        Style::default()
            .fg(TITLE_FOCUS)
            .add_modifier(Modifier::BOLD),
    ));
    lines.push(centered_line(
        &subtitle,
        width,
        left_pad,
        Style::default().fg(MUTED),
    ));
    lines.push(Line::raw(""));

    for (paragraph_index, paragraph) in app
        .current_chapter()
        .chunks(READER_PARAGRAPH_VERSES)
        .enumerate()
    {
        if paragraph_index > 0 {
            lines.push(Line::raw(""));
        }

        let words = paragraph_words(paragraph, app.current_verse);
        let wrapped = wrap_styled_words(&words, width, left_pad);
        for line in wrapped {
            if line.contains_selected && selected_line_top == 0 {
                selected_line_top = lines.len();
            }
            lines.push(line.line);
        }
    }

    ReaderView {
        lines,
        selected_line_top,
    }
}

fn paragraph_words(verses: &[Verse], current: crate::bible::VerseId) -> Vec<StyledWord> {
    let mut words = Vec::new();

    for verse in verses {
        let selected = verse.id == current;
        let number_style = if selected {
            Style::default()
                .fg(TITLE_FOCUS)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        };
        let text_style = if selected {
            Style::default()
                .fg(READER_SELECTED_TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT)
        };

        words.push(StyledWord {
            text: format!("{} ", verse.id.verse),
            style: number_style,
            verse_selected: selected,
        });

        for word in verse.text.split_whitespace() {
            words.push(StyledWord {
                text: word.to_string(),
                style: text_style,
                verse_selected: selected,
            });
        }
    }

    words
}

struct WrappedLine {
    line: Line<'static>,
    contains_selected: bool,
}

fn wrap_styled_words(words: &[StyledWord], width: usize, left_pad: usize) -> Vec<WrappedLine> {
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
        lines.push(styled_line(current, contains_selected, left_pad));
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
) -> WrappedLine {
    let mut line_spans = Vec::new();
    line_spans.push(Span::raw(" ".repeat(left_pad)));
    if contains_selected {
        line_spans.push(Span::styled("▌ ", Style::default().fg(ACCENT)));
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

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }

        if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn render_side_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let [index_area, preview_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .areas(area);

    render_side_index(frame, app, index_area);
    render_side_preview(frame, app, preview_area);
}

fn render_side_index(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let border = if app.focus == Focus::Side {
        STRONG
    } else {
        MUTED
    };
    let title_style = if app.focus == Focus::Side {
        Style::default()
            .fg(TITLE_FOCUS)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ACCENT)
    };

    let items = side_index_items(app);
    let selected = side_selected_index(app);
    let lines = side_index_lines(
        &items,
        selected,
        area.height.saturating_sub(4) as usize,
        app.focus == Focus::Side,
    );

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(app.side_panel_title(), title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .padding(Padding::new(1, 1, 1, 0))
            .style(Style::default().bg(PANEL)),
    );

    frame.render_widget(paragraph, area);
}

fn render_side_preview(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let border = if app.focus == Focus::Side {
        STRONG
    } else {
        MUTED
    };
    let title_style = if app.focus == Focus::Side {
        Style::default()
            .fg(TITLE_FOCUS)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ACCENT)
    };

    let (title, body) = side_preview_content(app);
    let lines = wrap_text(&body, area.width.saturating_sub(4).max(12) as usize)
        .into_iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(TEXT))))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .title(Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .padding(Padding::new(1, 1, 1, 0))
            .style(Style::default().bg(PANEL)),
    );

    frame.render_widget(paragraph, area);
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
    }
}

fn side_index_lines(
    items: &[String],
    selected: Option<usize>,
    viewport_height: usize,
    focused: bool,
) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![Line::from(Span::styled("", Style::default().fg(MUTED)))];
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
                Style::default()
                    .bg(SELECT)
                    .fg(TEXT)
                    .add_modifier(Modifier::BOLD)
            } else if index == selected {
                Style::default().fg(ACCENT)
            } else {
                Style::default().fg(TEXT)
            };
            Line::from(vec![
                Span::styled(prefix.to_string(), style),
                Span::styled(item.clone(), style),
            ])
        })
        .collect()
}

fn side_preview_content(app: &App) -> (String, String) {
    match app.side_panel {
        SidePanel::CrossReferences => {
            if let Some(entry) = app.selected_cross_reference() {
                if let Some(target) = entry.target {
                    if let Some(verse) = app.bible().verse(target) {
                        return (target.display(), verse.text.clone());
                    }
                    return (target.display(), String::new());
                }
                return (entry.target_label.clone(), entry.target_label.clone());
            }
            (
                "Cross Reference Preview".to_string(),
                "No cross reference selected.".to_string(),
            )
        }
        SidePanel::Search => {
            if let Some(hit) = app.selected_search_hit() {
                if let Some(verse) = app.bible().verse(hit.verse) {
                    return (hit.verse.display(), verse.text.clone());
                }
                return (hit.verse.display(), String::new());
            }
            (
                "Search Preview".to_string(),
                "No search result selected.".to_string(),
            )
        }
    }
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut lines = vec![Line::from(vec![
        Span::styled("Keys ", Style::default().fg(ACCENT)),
        Span::styled("/ search", Style::default().fg(TEXT)),
        Span::raw("  "),
        Span::styled("x refs", Style::default().fg(TEXT)),
        Span::raw("  "),
        Span::styled("g jump", Style::default().fg(TEXT)),
        Span::raw("  "),
        Span::styled("tab pane", Style::default().fg(TEXT)),
        Span::raw("  "),
        Span::styled("j/k move", Style::default().fg(TEXT)),
        Span::raw("  "),
        Span::styled("enter open", Style::default().fg(TEXT)),
        Span::raw("  "),
        Span::styled("u back", Style::default().fg(TEXT)),
        Span::raw("  "),
        Span::styled("p fwd", Style::default().fg(TEXT)),
    ])];

    if let Some(label) = app.active_input_label() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{label}> "),
                Style::default().fg(STRONG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(app.input.as_str(), Style::default().fg(TEXT)),
        ]));

        let hints = app.input_hints().join("   ");
        lines.push(Line::from(Span::styled(hints, Style::default().fg(MUTED))));
    } else {
        lines.push(Line::from(Span::styled(
            app.status.as_str(),
            Style::default().fg(MUTED),
        )));
        lines.push(Line::from(Span::styled(
            app.history_summary(),
            Style::default().fg(MUTED),
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
            },
            Style::default().fg(MUTED),
        )));
    }

    let footer = Paragraph::new(lines)
        .style(Style::default().bg(BG))
        .block(Block::default().padding(Padding::new(1, 1, 0, 0)));

    frame.render_widget(footer, area);
}
