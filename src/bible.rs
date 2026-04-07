use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::Path,
};

use color_eyre::eyre::{Result, WrapErr};
use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize)]
pub struct VerseId {
    pub book: usize,
    pub chapter: u16,
    pub verse: u16,
}

impl VerseId {
    pub fn display(self) -> String {
        let book = &BOOKS[self.book].name;
        format!("{book} {}:{}", self.chapter, self.verse)
    }
}

#[derive(Clone, Debug)]
pub struct Verse {
    pub id: VerseId,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct CrossReference {
    pub target_label: String,
    pub target: Option<VerseId>,
    pub votes: i16,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub verse: VerseId,
    pub score: usize,
}

pub struct Bible {
    verses: Vec<Verse>,
    by_id: HashMap<VerseId, usize>,
    chapter_ranges: HashMap<(usize, u16), (usize, usize)>,
    chapter_order: Vec<(usize, u16)>,
    cross_references: HashMap<VerseId, Vec<CrossReference>>,
    complete: bool,
}

impl Bible {
    pub fn load(osis_path: &Path, cross_refs: &str) -> Result<Self> {
        let verses = load_verses(osis_path)?;
        Self::from_verses(verses, cross_refs, true)
    }

    pub fn load_from_str(xml: &str, cross_refs: &str) -> Result<Self> {
        let verses = load_verses_from_str(xml)?;
        Self::from_verses(verses, cross_refs, true)
    }

    pub fn load_window(osis_path: &Path, cross_refs: &str, center: VerseId) -> Result<Self> {
        let verses = load_window_verses(osis_path, center)?;
        Self::from_verses(verses, cross_refs, false)
    }

    pub fn load_window_from_str(xml: &str, cross_refs: &str, center: VerseId) -> Result<Self> {
        let verses = load_window_verses_from_str(xml, center)?;
        Self::from_verses(verses, cross_refs, false)
    }

    fn from_verses(verses: Vec<Verse>, cross_refs: &str, complete: bool) -> Result<Self> {
        let mut by_id = HashMap::with_capacity(verses.len());
        let mut chapter_ranges = HashMap::new();
        let mut chapter_order = Vec::new();

        let mut range_start = 0usize;
        for (index, verse) in verses.iter().enumerate() {
            by_id.insert(verse.id, index);

            if index == 0 {
                range_start = 0;
                chapter_order.push((verse.id.book, verse.id.chapter));
                continue;
            }

            let previous = verses[index - 1].id;
            if previous.book != verse.id.book || previous.chapter != verse.id.chapter {
                chapter_ranges.insert((previous.book, previous.chapter), (range_start, index));
                chapter_order.push((verse.id.book, verse.id.chapter));
                range_start = index;
            }
        }

        if let Some(last) = verses.last() {
            chapter_ranges.insert((last.id.book, last.id.chapter), (range_start, verses.len()));
        }

        let cross_references = parse_cross_references(cross_refs, &by_id);

        Ok(Self {
            verses,
            by_id,
            chapter_ranges,
            chapter_order,
            cross_references,
            complete,
        })
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn first_verse(&self) -> Option<VerseId> {
        self.verses.first().map(|verse| verse.id)
    }

    pub fn verse(&self, id: VerseId) -> Option<&Verse> {
        self.by_id.get(&id).map(|index| &self.verses[*index])
    }

    pub fn chapter(&self, book: usize, chapter: u16) -> &[Verse] {
        self.chapter_ranges
            .get(&(book, chapter))
            .map(|(start, end)| &self.verses[*start..*end])
            .unwrap_or(&[])
    }

    pub fn chapter_for(&self, id: VerseId) -> &[Verse] {
        self.chapter(id.book, id.chapter)
    }

    pub fn chapter_index(&self, id: VerseId) -> Option<usize> {
        self.chapter_order
            .iter()
            .position(|&(book, chapter)| book == id.book && chapter == id.chapter)
    }

    pub fn next_chapter(&self, id: VerseId) -> Option<VerseId> {
        let current = self.chapter_index(id)?;
        self.chapter_order
            .get(current + 1)
            .and_then(|&(book, chapter)| self.chapter(book, chapter).first().map(|verse| verse.id))
    }

    pub fn previous_chapter(&self, id: VerseId) -> Option<VerseId> {
        let current = self.chapter_index(id)?;
        current
            .checked_sub(1)
            .and_then(|index| self.chapter_order.get(index))
            .and_then(|&(book, chapter)| self.chapter(book, chapter).first().map(|verse| verse.id))
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let normalized = query.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Vec::new();
        }

        let terms = normalized
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .collect::<Vec<_>>();

        let mut hits = self
            .verses
            .iter()
            .filter_map(|verse| {
                let haystack = verse.text.to_ascii_lowercase();
                let mut score = 0usize;

                for term in &terms {
                    let count = haystack.matches(term).count();
                    if count == 0 {
                        return None;
                    }
                    score += count;
                }

                if haystack.contains(&normalized) {
                    score += 8;
                }

                Some(SearchHit {
                    verse: verse.id,
                    score,
                })
            })
            .collect::<Vec<_>>();

        hits.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.verse.cmp(&right.verse))
        });
        hits.truncate(limit);
        hits
    }

    pub fn cross_references(&self, id: VerseId, limit: usize) -> Vec<CrossReference> {
        self.cross_references
            .get(&id)
            .map(|refs| refs.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    pub fn parse_reference(&self, input: &str) -> Option<VerseId> {
        parse_reference(input).and_then(|reference| {
            if self.verse(reference).is_some() {
                return Some(reference);
            }

            let chapter = self.chapter(reference.book, reference.chapter);
            if chapter.is_empty() {
                return None;
            }

            chapter.first().map(|verse| verse.id)
        })
    }

    pub fn verse_preview(&self, id: VerseId, max_len: usize) -> String {
        let Some(verse) = self.verse(id) else {
            return id.display();
        };

        let text = truncate_chars(&verse.text, max_len);
        format!("{}  {}", id.display(), text)
    }
}

fn truncate_chars(text: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }

    let mut result = String::new();
    let mut chars = text.chars();

    for _ in 0..max_len {
        let Some(ch) = chars.next() else {
            return text.to_string();
        };
        result.push(ch);
    }

    if chars.next().is_some() {
        if max_len > 1 {
            result.pop();
        }
        result.push('…');
    }

    result
}

fn load_verses(path: &Path) -> Result<Vec<Verse>> {
    let xml = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read OSIS bible data from {}", path.display()))?;
    load_verses_from_str(&xml)
}

fn load_verses_from_str(xml: &str) -> Result<Vec<Verse>> {
    if xml.contains("<XMLBIBLE") {
        return load_xmlbible_verses(xml);
    }

    if xml.contains("<bible>") {
        return load_simple_bible_verses(xml);
    }

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut verses = Vec::with_capacity(31_200);
    let mut active_id: Option<VerseId> = None;
    let mut active_text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"verse" => {
                let mut osis_id = None;
                let mut is_end = false;

                for attr in event.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"osisID" => {
                            osis_id = Some(
                                attr.decode_and_unescape_value(reader.decoder())?
                                    .into_owned(),
                            )
                        }
                        b"eID" => is_end = true,
                        _ => {}
                    }
                }

                if let Some(raw_id) = osis_id {
                    finalize_verse(&mut verses, &mut active_id, &mut active_text);
                    active_id = parse_osis_id(&raw_id);
                } else if is_end {
                    finalize_verse(&mut verses, &mut active_id, &mut active_text);
                }
            }
            Event::Text(event) => {
                if active_id.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut active_text, &text);
                }
            }
            Event::CData(event) => {
                if active_id.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut active_text, &text);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    finalize_verse(&mut verses, &mut active_id, &mut active_text);
    Ok(verses)
}

fn load_window_verses(path: &Path, center: VerseId) -> Result<Vec<Verse>> {
    let file = File::open(path)
        .wrap_err_with(|| format!("failed to open bible data from {}", path.display()))?;
    load_window_verses_from_bufreader(BufReader::new(file), Some(path), center)
}

fn load_window_verses_from_str(xml: &str, center: VerseId) -> Result<Vec<Verse>> {
    load_window_verses_from_bufreader(std::io::Cursor::new(xml.as_bytes()), None, center)
}

fn load_window_verses_from_bufreader<R: std::io::BufRead>(
    source: R,
    reopen_path: Option<&Path>,
    center: VerseId,
) -> Result<Vec<Verse>> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(true);

    let target_start = center.chapter.saturating_sub(1);
    let target_end = center.chapter.saturating_add(1);
    let mut buf = Vec::new();

    // Detect format from the first meaningful element.
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) | Event::Empty(event) => {
                let name = event.name();
                if name.as_ref() == b"XMLBIBLE" {
                    return load_xmlbible_window_from_reader(
                        reader,
                        center.book,
                        target_start,
                        target_end,
                    );
                }
                if name.as_ref() == b"bible" {
                    return load_simple_bible_window_from_reader(
                        reader,
                        center.book,
                        target_start,
                        target_end,
                    );
                }
                if name.as_ref() == b"osis"
                    || name.as_ref() == b"osisText"
                    || name.as_ref() == b"div"
                    || name.as_ref() == b"chapter"
                    || name.as_ref() == b"verse"
                {
                    if let Some(path) = reopen_path {
                        let file = File::open(path).wrap_err_with(|| {
                            format!("failed to reopen bible data from {}", path.display())
                        })?;
                        let mut osis_reader = Reader::from_reader(BufReader::new(file));
                        osis_reader.config_mut().trim_text(true);
                        return load_osis_window_from_reader(
                            osis_reader,
                            center.book,
                            target_start,
                            target_end,
                        );
                    }
                    // Embedded or string source: can't reopen, so parse the full content
                    // and filter to the window. The reader has already consumed some bytes,
                    // but for OSIS the detection happens early. We'll restart via the
                    // remaining reader.
                    return load_osis_window_from_reader(
                        reader,
                        center.book,
                        target_start,
                        target_end,
                    );
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(Vec::new())
}

fn load_xmlbible_verses(xml: &str) -> Result<Vec<Verse>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut verses = Vec::with_capacity(31_200);
    let mut current_book: Option<usize> = None;
    let mut current_chapter: Option<u16> = None;
    let mut current_verse: Option<u16> = None;
    let mut current_text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"BIBLEBOOK" => {
                current_book =
                    parse_book_from_attrs(&event, reader.decoder(), b"bname", b"bnumber");
            }
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"CHAPTER" => {
                current_chapter = parse_u16_attr(&event, reader.decoder(), b"cnumber");
            }
            Event::Start(event) => {
                if event.name().as_ref() == b"VERS" {
                    current_verse = parse_u16_attr(&event, reader.decoder(), b"vnumber");
                    current_text.clear();
                }
            }
            Event::Text(event) => {
                if current_verse.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut current_text, &text);
                }
            }
            Event::End(event) if event.name().as_ref() == b"VERS" => {
                if let (Some(book), Some(chapter), Some(verse)) =
                    (current_book, current_chapter, current_verse.take())
                {
                    let text = clean_text(&current_text);
                    if !text.is_empty() {
                        verses.push(Verse {
                            id: VerseId {
                                book,
                                chapter,
                                verse,
                            },
                            text,
                        });
                    }
                }
                current_text.clear();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(verses)
}

fn load_xmlbible_window_from_reader<R: std::io::BufRead>(
    mut reader: Reader<R>,
    target_book: usize,
    target_start: u16,
    target_end: u16,
) -> Result<Vec<Verse>> {
    let mut verses = Vec::new();
    let mut current_book: Option<usize> = None;
    let mut current_chapter: Option<u16> = None;
    let mut current_verse: Option<u16> = None;
    let mut current_text = String::new();
    let mut buf = Vec::new();
    let mut entered_window = false;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"BIBLEBOOK" => {
                let book = parse_book_from_attrs(&event, reader.decoder(), b"bname", b"bnumber");
                if entered_window && book.is_some_and(|value| value > target_book) {
                    break;
                }
                current_book = book;
            }
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"CHAPTER" => {
                let chapter = parse_u16_attr(&event, reader.decoder(), b"cnumber");
                if current_book == Some(target_book)
                    && chapter.is_some_and(|value| value >= target_start && value <= target_end)
                {
                    entered_window = true;
                } else if entered_window
                    && current_book == Some(target_book)
                    && chapter.is_some_and(|value| value > target_end)
                {
                    break;
                }
                current_chapter = chapter;
            }
            Event::Start(event) => {
                if event.name().as_ref() == b"VERS" {
                    current_verse = parse_u16_attr(&event, reader.decoder(), b"vnumber");
                    current_text.clear();
                }
            }
            Event::Text(event) => {
                if current_verse.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut current_text, &text);
                }
            }
            Event::End(event) if event.name().as_ref() == b"VERS" => {
                if let (Some(book), Some(chapter), Some(verse)) =
                    (current_book, current_chapter, current_verse.take())
                {
                    if book == target_book && chapter >= target_start && chapter <= target_end {
                        let text = clean_text(&current_text);
                        if !text.is_empty() {
                            verses.push(Verse {
                                id: VerseId {
                                    book,
                                    chapter,
                                    verse,
                                },
                                text,
                            });
                        }
                    }
                }
                current_text.clear();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(verses)
}

fn load_simple_bible_verses(xml: &str) -> Result<Vec<Verse>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut verses = Vec::with_capacity(31_200);
    let mut current_book: Option<usize> = None;
    let mut current_chapter: Option<u16> = None;
    let mut current_verse: Option<u16> = None;
    let mut current_text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"b" => {
                current_book = parse_book_from_attrs(&event, reader.decoder(), b"n", b"");
            }
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"c" => {
                current_chapter = parse_u16_attr(&event, reader.decoder(), b"n");
            }
            Event::Start(event) => {
                if event.name().as_ref() == b"v" {
                    current_verse = parse_u16_attr(&event, reader.decoder(), b"n");
                    current_text.clear();
                }
            }
            Event::Text(event) => {
                if current_verse.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut current_text, &text);
                }
            }
            Event::End(event) if event.name().as_ref() == b"v" => {
                if let (Some(book), Some(chapter), Some(verse)) =
                    (current_book, current_chapter, current_verse.take())
                {
                    let text = clean_text(&current_text);
                    if !text.is_empty() {
                        verses.push(Verse {
                            id: VerseId {
                                book,
                                chapter,
                                verse,
                            },
                            text,
                        });
                    }
                }
                current_text.clear();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(verses)
}

fn load_simple_bible_window_from_reader<R: std::io::BufRead>(
    mut reader: Reader<R>,
    target_book: usize,
    target_start: u16,
    target_end: u16,
) -> Result<Vec<Verse>> {
    let mut verses = Vec::new();
    let mut current_book: Option<usize> = None;
    let mut current_chapter: Option<u16> = None;
    let mut current_verse: Option<u16> = None;
    let mut current_text = String::new();
    let mut buf = Vec::new();
    let mut entered_window = false;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"b" => {
                let book = parse_book_from_attrs(&event, reader.decoder(), b"n", b"");
                if entered_window && book.is_some_and(|value| value > target_book) {
                    break;
                }
                current_book = book;
            }
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"c" => {
                let chapter = parse_u16_attr(&event, reader.decoder(), b"n");
                if current_book == Some(target_book)
                    && chapter.is_some_and(|value| value >= target_start && value <= target_end)
                {
                    entered_window = true;
                } else if entered_window
                    && current_book == Some(target_book)
                    && chapter.is_some_and(|value| value > target_end)
                {
                    break;
                }
                current_chapter = chapter;
            }
            Event::Start(event) => {
                if event.name().as_ref() == b"v" {
                    current_verse = parse_u16_attr(&event, reader.decoder(), b"n");
                    current_text.clear();
                }
            }
            Event::Text(event) => {
                if current_verse.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut current_text, &text);
                }
            }
            Event::End(event) if event.name().as_ref() == b"v" => {
                if let (Some(book), Some(chapter), Some(verse)) =
                    (current_book, current_chapter, current_verse.take())
                {
                    if book == target_book && chapter >= target_start && chapter <= target_end {
                        let text = clean_text(&current_text);
                        if !text.is_empty() {
                            verses.push(Verse {
                                id: VerseId {
                                    book,
                                    chapter,
                                    verse,
                                },
                                text,
                            });
                        }
                    }
                }
                current_text.clear();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(verses)
}

fn load_osis_window_from_reader<R: std::io::BufRead>(
    mut reader: Reader<R>,
    target_book: usize,
    target_start: u16,
    target_end: u16,
) -> Result<Vec<Verse>> {
    let mut verses = Vec::new();
    let mut active_id: Option<VerseId> = None;
    let mut active_text = String::new();
    let mut buf = Vec::new();
    let mut entered_window = false;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) | Event::Empty(event) if event.name().as_ref() == b"verse" => {
                let mut osis_id = None;
                let mut is_end = false;

                for attr in event.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"osisID" => {
                            osis_id = Some(
                                attr.decode_and_unescape_value(reader.decoder())?
                                    .into_owned(),
                            )
                        }
                        b"eID" => is_end = true,
                        _ => {}
                    }
                }

                if let Some(raw_id) = osis_id {
                    finalize_window_verse(
                        &mut verses,
                        &mut active_id,
                        &mut active_text,
                        target_book,
                        target_start,
                        target_end,
                    );
                    let parsed = parse_osis_id(&raw_id);
                    if let Some(id) = parsed {
                        if id.book == target_book
                            && id.chapter >= target_start
                            && id.chapter <= target_end
                        {
                            entered_window = true;
                        } else if entered_window
                            && (id.book > target_book
                                || (id.book == target_book && id.chapter > target_end))
                        {
                            break;
                        }
                    }
                    active_id = parsed;
                } else if is_end {
                    finalize_window_verse(
                        &mut verses,
                        &mut active_id,
                        &mut active_text,
                        target_book,
                        target_start,
                        target_end,
                    );
                }
            }
            Event::Text(event) => {
                if active_id.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut active_text, &text);
                }
            }
            Event::CData(event) => {
                if active_id.is_some() {
                    let text = event.decode()?.into_owned();
                    push_text(&mut active_text, &text);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    finalize_window_verse(
        &mut verses,
        &mut active_id,
        &mut active_text,
        target_book,
        target_start,
        target_end,
    );
    Ok(verses)
}

fn parse_book_from_attrs(
    event: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
    name_key: &[u8],
    number_key: &[u8],
) -> Option<usize> {
    let mut book_name = None;
    let mut book_number = None;

    for attr in event.attributes().flatten() {
        if attr.key.as_ref() == name_key {
            book_name = Some(attr.decode_and_unescape_value(decoder).ok()?.into_owned());
        } else if !number_key.is_empty() && attr.key.as_ref() == number_key {
            book_number = attr
                .decode_and_unescape_value(decoder)
                .ok()?
                .parse::<usize>()
                .ok()
                .and_then(|value| value.checked_sub(1));
        }
    }

    book_name
        .as_deref()
        .and_then(book_index_by_name)
        .or(book_number)
}

fn parse_u16_attr(
    event: &quick_xml::events::BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
    key: &[u8],
) -> Option<u16> {
    for attr in event.attributes().flatten() {
        if attr.key.as_ref() == key {
            return attr
                .decode_and_unescape_value(decoder)
                .ok()?
                .parse::<u16>()
                .ok();
        }
    }
    None
}

fn finalize_verse(
    verses: &mut Vec<Verse>,
    active_id: &mut Option<VerseId>,
    active_text: &mut String,
) {
    if let Some(id) = active_id.take() {
        let text = clean_text(active_text);
        if !text.is_empty() {
            verses.push(Verse { id, text });
        }
    }
    active_text.clear();
}

fn finalize_window_verse(
    verses: &mut Vec<Verse>,
    active_id: &mut Option<VerseId>,
    active_text: &mut String,
    target_book: usize,
    target_start: u16,
    target_end: u16,
) {
    if let Some(id) = active_id.take() {
        if id.book == target_book && id.chapter >= target_start && id.chapter <= target_end {
            let text = clean_text(active_text);
            if !text.is_empty() {
                verses.push(Verse { id, text });
            }
        }
    }
    active_text.clear();
}

fn push_text(buffer: &mut String, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    if !buffer.is_empty() {
        buffer.push(' ');
    }
    buffer.push_str(trimmed);
}

fn clean_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_cross_references(
    text: &str,
    verses: &HashMap<VerseId, usize>,
) -> HashMap<VerseId, Vec<CrossReference>> {
    let mut by_verse: HashMap<VerseId, Vec<CrossReference>> = HashMap::new();

    for (index, line) in text.lines().enumerate() {
        if index == 0 || line.trim().is_empty() {
            continue;
        }

        let mut parts = line.split('\t');
        let Some(from) = parts.next() else { continue };
        let Some(to) = parts.next() else { continue };
        let Some(votes) = parts.next() else { continue };

        let Some(from_id) = parse_osis_id(from) else {
            continue;
        };
        if !verses.contains_key(&from_id) {
            continue;
        }

        let to_verse = parse_single_or_range_target(to).and_then(parse_osis_id);
        let votes = votes.parse::<i16>().unwrap_or_default();

        by_verse.entry(from_id).or_default().push(CrossReference {
            target_label: to.replace('.', " ").replace('-', " - "),
            target: to_verse,
            votes,
        });
    }

    for refs in by_verse.values_mut() {
        refs.sort_by(|left, right| right.votes.cmp(&left.votes));
        refs.truncate(24);
    }

    by_verse
}

fn parse_single_or_range_target(input: &str) -> Option<&str> {
    if let Some((start, _)) = input.split_once('-') {
        Some(start)
    } else {
        Some(input)
    }
}

fn parse_osis_id(input: &str) -> Option<VerseId> {
    let mut parts = input.split('.');
    let book = parts.next()?;
    let chapter = parts.next()?.parse().ok()?;
    let verse = parts.next()?.parse().ok()?;
    let book = book_index_by_osis(book)?;
    Some(VerseId {
        book,
        chapter,
        verse,
    })
}

pub fn parse_reference(input: &str) -> Option<VerseId> {
    let normalized = input.trim().replace('.', " ");
    let mut tokens = normalized.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return None;
    }

    if tokens.len() == 1 {
        let book = book_index_by_name(tokens[0])?;
        return Some(VerseId {
            book,
            chapter: 1,
            verse: 1,
        });
    }

    let last = tokens.pop()?;
    let (chapter, verse) = if let Some((chapter, verse)) = last.split_once(':') {
        (chapter.parse().ok()?, verse.parse().ok()?)
    } else {
        (last.parse().ok()?, 1)
    };

    let book_name = tokens.join(" ");
    let book = book_index_by_name(&book_name)?;
    Some(VerseId {
        book,
        chapter,
        verse,
    })
}

fn book_index_by_osis(osis: &str) -> Option<usize> {
    BOOKS
        .iter()
        .position(|book| book.osis.eq_ignore_ascii_case(osis))
}

fn book_index_by_name(input: &str) -> Option<usize> {
    let normalized = normalize_book_name(input);
    BOOKS.iter().position(|book| {
        book.aliases
            .iter()
            .any(|alias| normalize_book_name(alias) == normalized)
    })
}

fn normalize_book_name(input: &str) -> String {
    input
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

struct BookDef {
    osis: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
}

const BOOKS: &[BookDef] = &[
    BookDef {
        osis: "Gen",
        name: "Genesis",
        aliases: &["gen", "ge", "gn", "genesis"],
    },
    BookDef {
        osis: "Exod",
        name: "Exodus",
        aliases: &["exod", "exo", "ex", "exodus"],
    },
    BookDef {
        osis: "Lev",
        name: "Leviticus",
        aliases: &["lev", "le", "lv", "leviticus"],
    },
    BookDef {
        osis: "Num",
        name: "Numbers",
        aliases: &["num", "nm", "nb", "numbers"],
    },
    BookDef {
        osis: "Deut",
        name: "Deuteronomy",
        aliases: &["deut", "deu", "dt", "deuteronomy"],
    },
    BookDef {
        osis: "Josh",
        name: "Joshua",
        aliases: &["josh", "jos", "jsh", "joshua"],
    },
    BookDef {
        osis: "Judg",
        name: "Judges",
        aliases: &["judg", "jdg", "jg", "judges"],
    },
    BookDef {
        osis: "Ruth",
        name: "Ruth",
        aliases: &["ruth", "ru", "rth"],
    },
    BookDef {
        osis: "1Sam",
        name: "1 Samuel",
        aliases: &["1 samuel", "1 sam", "1sam", "i samuel", "first samuel"],
    },
    BookDef {
        osis: "2Sam",
        name: "2 Samuel",
        aliases: &["2 samuel", "2 sam", "2sam", "ii samuel", "second samuel"],
    },
    BookDef {
        osis: "1Kgs",
        name: "1 Kings",
        aliases: &["1 kings", "1 kgs", "1kgs", "1 ki", "first kings"],
    },
    BookDef {
        osis: "2Kgs",
        name: "2 Kings",
        aliases: &["2 kings", "2 kgs", "2kgs", "2 ki", "second kings"],
    },
    BookDef {
        osis: "1Chr",
        name: "1 Chronicles",
        aliases: &["1 chronicles", "1 chr", "1chr", "first chronicles"],
    },
    BookDef {
        osis: "2Chr",
        name: "2 Chronicles",
        aliases: &["2 chronicles", "2 chr", "2chr", "second chronicles"],
    },
    BookDef {
        osis: "Ezra",
        name: "Ezra",
        aliases: &["ezra", "ezr"],
    },
    BookDef {
        osis: "Neh",
        name: "Nehemiah",
        aliases: &["neh", "ne", "nehemiah"],
    },
    BookDef {
        osis: "Esth",
        name: "Esther",
        aliases: &["esth", "est", "esther"],
    },
    BookDef {
        osis: "Job",
        name: "Job",
        aliases: &["job"],
    },
    BookDef {
        osis: "Ps",
        name: "Psalms",
        aliases: &["ps", "psa", "psalm", "psalms"],
    },
    BookDef {
        osis: "Prov",
        name: "Proverbs",
        aliases: &["prov", "pr", "pro", "proverbs"],
    },
    BookDef {
        osis: "Eccl",
        name: "Ecclesiastes",
        aliases: &["eccl", "ecc", "ec", "ecclesiastes"],
    },
    BookDef {
        osis: "Song",
        name: "Song of Solomon",
        aliases: &["song", "song of solomon", "song of songs", "songs", "sos"],
    },
    BookDef {
        osis: "Isa",
        name: "Isaiah",
        aliases: &["isa", "is", "isaiah"],
    },
    BookDef {
        osis: "Jer",
        name: "Jeremiah",
        aliases: &["jer", "je", "jr", "jeremiah"],
    },
    BookDef {
        osis: "Lam",
        name: "Lamentations",
        aliases: &["lam", "la", "lamentations"],
    },
    BookDef {
        osis: "Ezek",
        name: "Ezekiel",
        aliases: &["ezek", "eze", "ezk", "ezekiel"],
    },
    BookDef {
        osis: "Dan",
        name: "Daniel",
        aliases: &["dan", "da", "dn", "daniel"],
    },
    BookDef {
        osis: "Hos",
        name: "Hosea",
        aliases: &["hos", "ho", "hosea"],
    },
    BookDef {
        osis: "Joel",
        name: "Joel",
        aliases: &["joel", "jl"],
    },
    BookDef {
        osis: "Amos",
        name: "Amos",
        aliases: &["amos", "am"],
    },
    BookDef {
        osis: "Obad",
        name: "Obadiah",
        aliases: &["obad", "ob", "obadiah"],
    },
    BookDef {
        osis: "Jonah",
        name: "Jonah",
        aliases: &["jonah", "jon"],
    },
    BookDef {
        osis: "Mic",
        name: "Micah",
        aliases: &["mic", "mc", "micah"],
    },
    BookDef {
        osis: "Nah",
        name: "Nahum",
        aliases: &["nah", "na", "nahum"],
    },
    BookDef {
        osis: "Hab",
        name: "Habakkuk",
        aliases: &["hab", "hb", "habakkuk"],
    },
    BookDef {
        osis: "Zeph",
        name: "Zephaniah",
        aliases: &["zeph", "zep", "zp", "zephaniah"],
    },
    BookDef {
        osis: "Hag",
        name: "Haggai",
        aliases: &["hag", "hg", "haggai"],
    },
    BookDef {
        osis: "Zech",
        name: "Zechariah",
        aliases: &["zech", "zec", "zc", "zechariah"],
    },
    BookDef {
        osis: "Mal",
        name: "Malachi",
        aliases: &["mal", "ml", "malachi"],
    },
    BookDef {
        osis: "Matt",
        name: "Matthew",
        aliases: &["matt", "mt", "mat", "matthew"],
    },
    BookDef {
        osis: "Mark",
        name: "Mark",
        aliases: &["mark", "mrk", "mk", "mr"],
    },
    BookDef {
        osis: "Luke",
        name: "Luke",
        aliases: &["luke", "luk", "lk"],
    },
    BookDef {
        osis: "John",
        name: "John",
        aliases: &["john", "jn", "jhn"],
    },
    BookDef {
        osis: "Acts",
        name: "Acts",
        aliases: &["acts", "ac"],
    },
    BookDef {
        osis: "Rom",
        name: "Romans",
        aliases: &["rom", "ro", "romans"],
    },
    BookDef {
        osis: "1Cor",
        name: "1 Corinthians",
        aliases: &["1 corinthians", "1 cor", "1cor", "first corinthians"],
    },
    BookDef {
        osis: "2Cor",
        name: "2 Corinthians",
        aliases: &["2 corinthians", "2 cor", "2cor", "second corinthians"],
    },
    BookDef {
        osis: "Gal",
        name: "Galatians",
        aliases: &["gal", "ga", "galatians"],
    },
    BookDef {
        osis: "Eph",
        name: "Ephesians",
        aliases: &["eph", "ep", "ephesians"],
    },
    BookDef {
        osis: "Phil",
        name: "Philippians",
        aliases: &["phil", "php", "philippians"],
    },
    BookDef {
        osis: "Col",
        name: "Colossians",
        aliases: &["col", "co", "colossians"],
    },
    BookDef {
        osis: "1Thess",
        name: "1 Thessalonians",
        aliases: &[
            "1 thessalonians",
            "1 thess",
            "1thess",
            "first thessalonians",
        ],
    },
    BookDef {
        osis: "2Thess",
        name: "2 Thessalonians",
        aliases: &[
            "2 thessalonians",
            "2 thess",
            "2thess",
            "second thessalonians",
        ],
    },
    BookDef {
        osis: "1Tim",
        name: "1 Timothy",
        aliases: &["1 timothy", "1 tim", "1tim", "first timothy"],
    },
    BookDef {
        osis: "2Tim",
        name: "2 Timothy",
        aliases: &["2 timothy", "2 tim", "2tim", "second timothy"],
    },
    BookDef {
        osis: "Titus",
        name: "Titus",
        aliases: &["titus", "tit"],
    },
    BookDef {
        osis: "Phlm",
        name: "Philemon",
        aliases: &["philemon", "phlm", "phm"],
    },
    BookDef {
        osis: "Heb",
        name: "Hebrews",
        aliases: &["heb", "he", "hebrews"],
    },
    BookDef {
        osis: "Jas",
        name: "James",
        aliases: &["james", "jas", "jm"],
    },
    BookDef {
        osis: "1Pet",
        name: "1 Peter",
        aliases: &["1 peter", "1 pet", "1pet", "first peter"],
    },
    BookDef {
        osis: "2Pet",
        name: "2 Peter",
        aliases: &["2 peter", "2 pet", "2pet", "second peter"],
    },
    BookDef {
        osis: "1John",
        name: "1 John",
        aliases: &["1 john", "1jn", "1 jn", "first john"],
    },
    BookDef {
        osis: "2John",
        name: "2 John",
        aliases: &["2 john", "2jn", "2 jn", "second john"],
    },
    BookDef {
        osis: "3John",
        name: "3 John",
        aliases: &["3 john", "3jn", "3 jn", "third john"],
    },
    BookDef {
        osis: "Jude",
        name: "Jude",
        aliases: &["jude", "jud"],
    },
    BookDef {
        osis: "Rev",
        name: "Revelation",
        aliases: &["revelation", "rev", "re", "the revelation"],
    },
];

pub fn book_name(book: usize) -> &'static str {
    BOOKS[book].name
}

pub fn book_abbrev(book: usize) -> &'static str {
    BOOKS[book].osis
}

pub fn suggest_books(input: &str, limit: usize) -> Vec<&'static str> {
    let normalized = normalize_book_name(input);
    if normalized.is_empty() {
        return BOOKS.iter().take(limit).map(|book| book.name).collect();
    }

    let mut matches = BOOKS
        .iter()
        .filter(|book| {
            book.aliases
                .iter()
                .any(|alias| normalize_book_name(alias).starts_with(&normalized))
        })
        .map(|book| book.name)
        .collect::<Vec<_>>();
    matches.dedup();
    matches.truncate(limit);
    matches
}

#[cfg(test)]
mod tests {
    use super::{book_name, parse_osis_id, parse_reference, truncate_chars};

    #[test]
    fn parses_common_reference_forms() {
        let john = parse_reference("jn 3:16").unwrap();
        assert_eq!(book_name(john.book), "John");
        assert_eq!(john.chapter, 3);
        assert_eq!(john.verse, 16);

        let cor = parse_reference("1 cor 13").unwrap();
        assert_eq!(book_name(cor.book), "1 Corinthians");
        assert_eq!(cor.chapter, 13);
        assert_eq!(cor.verse, 1);

        let book_only = parse_reference("john").unwrap();
        assert_eq!(book_name(book_only.book), "John");
        assert_eq!(book_only.chapter, 1);
        assert_eq!(book_only.verse, 1);
    }

    #[test]
    fn parses_osis_verse_ids() {
        let verse = parse_osis_id("Gen.1.1").unwrap();
        assert_eq!(book_name(verse.book), "Genesis");
        assert_eq!(verse.chapter, 1);
        assert_eq!(verse.verse, 1);
    }

    #[test]
    fn suggests_books_from_partial_input() {
        let suggestions = super::suggest_books("jo", 5);
        assert!(suggestions.contains(&"John"));
    }

    #[test]
    fn truncates_on_char_boundaries() {
        let text = "mother’s";
        assert_eq!(truncate_chars(text, 6), "mothe…");
    }
}
