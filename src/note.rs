use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use crate::bible::VerseId;

#[derive(Clone, Debug)]
pub struct Note {
    pub id: String,
    pub verses: Vec<VerseId>,
    pub body: String,
    pub path: PathBuf,
}

pub struct NoteIndex {
    notes: Vec<Note>,
    by_verse: HashMap<VerseId, Vec<usize>>,
    dir: PathBuf,
}

impl NoteIndex {
    pub fn load(dir: &Path) -> Self {
        let mut index = Self {
            notes: Vec::new(),
            by_verse: HashMap::new(),
            dir: dir.to_path_buf(),
        };
        if dir.is_dir() {
            index.scan();
        }
        index
    }

    pub fn reload(&mut self) {
        self.notes.clear();
        self.by_verse.clear();
        if self.dir.is_dir() {
            self.scan();
        }
    }

    fn scan(&mut self) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Some(note) = parse_note_file(&path) {
                let idx = self.notes.len();
                for &verse in &note.verses {
                    self.by_verse.entry(verse).or_default().push(idx);
                }
                self.notes.push(note);
            }
        }
        self.notes.sort_by(|a, b| b.id.cmp(&a.id));
        self.rebuild_verse_index();
    }

    fn rebuild_verse_index(&mut self) {
        self.by_verse.clear();
        for (idx, note) in self.notes.iter().enumerate() {
            for &verse in &note.verses {
                self.by_verse.entry(verse).or_default().push(idx);
            }
        }
    }

    pub fn notes_for_chapter(&self, book: usize, chapter: u16) -> Vec<&Note> {
        let mut seen = Vec::new();
        let mut result = Vec::new();
        for (&verse, indices) in &self.by_verse {
            if verse.book == book && verse.chapter == chapter {
                for &idx in indices {
                    if !seen.contains(&idx) {
                        seen.push(idx);
                        result.push(&self.notes[idx]);
                    }
                }
            }
        }
        result.sort_by(|a, b| b.id.cmp(&a.id));
        result
    }

    pub fn all_notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn create_note(
        &self,
        verses: &[VerseId],
        quotes: &[(VerseId, &str)],
    ) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.dir)?;
        for &(id, text) in quotes {
            ensure_verse_file(&self.dir, id, text)?;
        }
        let id = timestamp_id();
        let filename = format!("{id}.md");
        let path = self.dir.join(&filename);
        let content = format_new_note(verses, quotes);
        fs::write(&path, content)?;
        Ok(path)
    }

    pub fn create_note_ranged(
        &self,
        verses: &[VerseId],
        quotes: &[(VerseId, &str)],
    ) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.dir)?;
        for &(id, text) in quotes {
            ensure_verse_file(&self.dir, id, text)?;
        }
        let id = timestamp_id();
        let filename = format!("{id}.md");
        let path = self.dir.join(&filename);
        let content = format_ranged_note(verses, quotes);
        fs::write(&path, content)?;
        Ok(path)
    }

    pub fn add_verse_to_note(
        &self,
        note: &Note,
        verse: VerseId,
        quote: Option<&str>,
    ) -> io::Result<()> {
        if let Some(text) = quote {
            ensure_verse_file(&self.dir, verse, text)?;
        }
        let content = fs::read_to_string(&note.path)?;
        let updated = add_verse_to_content(&content, verse, quote);
        fs::write(&note.path, updated)?;
        Ok(())
    }

    pub fn add_verse_to_note_by_path(
        &self,
        path: &Path,
        verse: VerseId,
        quote: Option<&str>,
    ) -> io::Result<()> {
        if let Some(text) = quote {
            ensure_verse_file(&self.dir, verse, text)?;
        }
        let content = fs::read_to_string(path)?;
        let updated = add_verse_to_content(&content, verse, quote);
        fs::write(path, updated)
    }

    pub fn add_range_to_note_by_path(
        &self,
        path: &Path,
        quotes: &[(VerseId, &str)],
    ) -> io::Result<()> {
        for &(id, text) in quotes {
            ensure_verse_file(&self.dir, id, text)?;
        }
        let content = fs::read_to_string(path)?;
        let updated = add_range_to_content(&content, quotes);
        fs::write(path, updated)
    }
}

fn timestamp_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

fn format_new_note(verses: &[VerseId], quotes: &[(VerseId, &str)]) -> String {
    let mut body = String::new();

    for (id, text) in quotes {
        body.push_str(&format!("> {}\n>\n> — [[{}]]\n\n", text, verse_slug(*id)));
    }

    body.push('\n');

    let verse_list: Vec<String> = verses.iter().map(|v| format_osis_id(*v)).collect();
    body.push_str(&format!("---\nverses: [{}]\n", verse_list.join(", ")));

    body
}

fn format_ranged_note(verses: &[VerseId], quotes: &[(VerseId, &str)]) -> String {
    let mut body = String::new();

    if quotes.len() <= 1 {
        // Single verse, use normal format
        return format_new_note(verses, quotes);
    }

    // Build contiguous quote block
    for (i, (_, text)) in quotes.iter().enumerate() {
        if i > 0 {
            body.push_str("> \n");
        }
        body.push_str(&format!("> {}\n", text));
    }
    body.push_str(">\n");

    // Range reference with individual wikilinks
    let links: Vec<String> = quotes.iter().map(|(id, _)| verse_slug(*id)).collect();
    let range_display = format_verse_range(quotes);
    body.push_str(&format!("> — {}\n", range_display));

    // Individual verse links below the quote
    body.push('\n');
    for link in &links {
        body.push_str(&format!("[[{}]] ", link));
    }
    body.push_str("\n\n\n");

    let verse_list: Vec<String> = verses.iter().map(|v| format_osis_id(*v)).collect();
    body.push_str(&format!("---\nverses: [{}]\n", verse_list.join(", ")));

    body
}

fn format_verse_range(quotes: &[(VerseId, &str)]) -> String {
    if quotes.is_empty() {
        return String::new();
    }
    let first = quotes[0].0;
    if quotes.len() == 1 {
        return format!("[[{}]]", verse_slug(first));
    }
    let last = quotes[quotes.len() - 1].0;
    let book = crate::bible::book_name(first.book);
    if first.book == last.book && first.chapter == last.chapter {
        format!("{} {}:{}-{}", book, first.chapter, first.verse, last.verse)
    } else {
        format!("{} - {}", first.display(), last.display())
    }
}

fn format_osis_id(verse: VerseId) -> String {
    let book = crate::bible::book_abbrev(verse.book);
    format!("{}.{}.{}", book, verse.chapter, verse.verse)
}

fn verse_slug(verse: VerseId) -> String {
    let book = crate::bible::book_abbrev(verse.book).to_ascii_lowercase();
    format!("{}{}-{}", book, verse.chapter, verse.verse)
}

fn ensure_verse_file(dir: &Path, verse: VerseId, text: &str) -> io::Result<()> {
    let verses_dir = dir.join("verses");
    fs::create_dir_all(&verses_dir)?;
    let slug = verse_slug(verse);
    let path = verses_dir.join(format!("{slug}.md"));
    if path.exists() {
        return Ok(());
    }
    let content = format!("> {}\n\n— {} (KJV)\n", text, verse.display());
    fs::write(path, content)
}

fn add_verse_to_content(content: &str, verse: VerseId, quote: Option<&str>) -> String {
    let osis = format_osis_id(verse);

    // Split body from trailing refs block (last occurrence of \n---\n)
    let (body, refs_block) = if let Some(pos) = content.rfind("\n---\n") {
        (&content[..pos + 1], Some(&content[pos + 1..]))
    } else {
        (content, None)
    };

    let mut result = body.to_string();

    // Insert quote before the refs block
    if let Some(text) = quote {
        result.push_str(&format!("> {}\n>\n> — [[{}]]\n\n", text, verse_slug(verse)));
    }

    // Update or create refs block
    if let Some(block) = refs_block {
        if let Some(verses_line) = block.lines().find(|l| l.starts_with("verses:")) {
            if verses_line.contains(&osis) {
                result.push_str(block);
            } else {
                let bracket_end = verses_line.rfind(']').unwrap_or(verses_line.len());
                let new_line = format!("{}, {}]", &verses_line[..bracket_end], osis);
                let updated_block = block.replacen(verses_line, &new_line, 1);
                result.push_str(&updated_block);
            }
        } else {
            result.push_str(&format!("---\nverses: [{}]\n", osis));
        }
    } else {
        result.push_str(&format!("\n---\nverses: [{}]\n", osis));
    }

    result
}

fn add_range_to_content(content: &str, quotes: &[(VerseId, &str)]) -> String {
    if quotes.is_empty() {
        return content.to_string();
    }
    if quotes.len() == 1 {
        return add_verse_to_content(content, quotes[0].0, Some(quotes[0].1));
    }

    // Split body from trailing refs block
    let (body, refs_block) = if let Some(pos) = content.rfind("\n---\n") {
        (&content[..pos + 1], Some(&content[pos + 1..]))
    } else {
        (content, None)
    };

    let mut result = body.to_string();

    // Build contiguous quote block
    for (i, (_, text)) in quotes.iter().enumerate() {
        if i > 0 {
            result.push_str("> \n");
        }
        result.push_str(&format!("> {}\n", text));
    }
    result.push_str(">\n");
    let range_display = format_verse_range(quotes);
    result.push_str(&format!("> — {}\n\n", range_display));

    // Individual verse links
    for (id, _) in quotes {
        result.push_str(&format!("[[{}]] ", verse_slug(*id)));
    }
    result.push_str("\n\n");

    // Update refs block with all new verses
    let new_osis: Vec<String> = quotes.iter().map(|(id, _)| format_osis_id(*id)).collect();
    if let Some(block) = refs_block {
        if let Some(verses_line) = block.lines().find(|l| l.starts_with("verses:")) {
            let mut updated_line = verses_line.to_string();
            for osis in &new_osis {
                if !updated_line.contains(osis.as_str()) {
                    let bracket_end = updated_line.rfind(']').unwrap_or(updated_line.len());
                    updated_line = format!("{}, {}]", &updated_line[..bracket_end], osis);
                }
            }
            let updated_block = block.replacen(verses_line, &updated_line, 1);
            result.push_str(&updated_block);
        } else {
            result.push_str(&format!("---\nverses: [{}]\n", new_osis.join(", ")));
        }
    } else {
        result.push_str(&format!("---\nverses: [{}]\n", new_osis.join(", ")));
    }

    result
}

fn parse_note_file(path: &Path) -> Option<Note> {
    let content = fs::read_to_string(path).ok()?;
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut verses = Vec::new();
    let mut body = content.as_str();

    // Try trailing refs block first (new format: body then ---\nverses:[...]\n)
    if let Some(pos) = content.rfind("\n---\n") {
        let trailing = &content[pos + 5..];
        if extract_verses(trailing, &mut verses) {
            body = content[..pos].trim();
        }
    }

    // Fall back to top frontmatter (old format: ---\n...\n---\nbody)
    if verses.is_empty() {
        if let Some(rest) = content.strip_prefix("---\n") {
            if let Some(end) = rest.find("\n---") {
                let frontmatter = &rest[..end];
                extract_verses(frontmatter, &mut verses);
                body = rest[end + 4..].trim_start_matches('\n');
            }
        }
    }

    Some(Note {
        id,
        verses,
        body: body.to_string(),
        path: path.to_path_buf(),
    })
}

fn extract_verses(text: &str, verses: &mut Vec<VerseId>) -> bool {
    let mut found = false;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("verses:") {
            found = true;
            let value = value.trim();
            let value = value
                .strip_prefix('[')
                .unwrap_or(value)
                .strip_suffix(']')
                .unwrap_or(value);
            for entry in value.split(',') {
                let entry = entry.trim();
                if let Some(vid) = parse_osis_ref(entry) {
                    verses.push(vid);
                }
            }
        }
    }
    found
}

fn parse_osis_ref(input: &str) -> Option<VerseId> {
    let mut parts = input.split('.');
    let book_str = parts.next()?.trim();
    let chapter: u16 = parts.next()?.trim().parse().ok()?;
    let verse: u16 = parts.next()?.trim().parse().ok()?;
    let book = crate::bible::book_index_by_osis(book_str)?;
    Some(VerseId {
        book,
        chapter,
        verse,
    })
}

pub fn notes_dir() -> PathBuf {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return Path::new(&config_home).join("malacli").join("notes");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("malacli")
            .join("notes");
    }
    PathBuf::from(".malacli-notes")
}
