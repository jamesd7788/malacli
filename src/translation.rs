use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;

use crate::bible::Bible;
use crate::config;
use crate::data;

const OSIS_DIR_ENV: &str = "MALACLI_OSIS_DIR";
const DEFAULT_TRANSLATION_ENV: &str = "MALACLI_TRANSLATION";
const FALLBACK_LOCAL_TRANSLATIONS_DIR: &str = "/Users/james/Downloads/media-tool-kit-xml-bibles";

pub struct TranslationEntry {
    pub code: String,
    pub source_path: Option<PathBuf>,
    embedded: bool,
    bible: Option<Bible>,
    failed: bool,
}

impl TranslationEntry {
    pub fn new(code: String, source_path: PathBuf) -> Self {
        Self {
            code,
            source_path: Some(source_path),
            embedded: false,
            bible: None,
            failed: false,
        }
    }

    fn embedded(code: String) -> Self {
        Self {
            code,
            source_path: None,
            embedded: true,
            bible: None,
            failed: false,
        }
    }

    pub fn bible(&self) -> Option<&Bible> {
        self.bible.as_ref()
    }

    pub fn is_ready(&self) -> bool {
        self.bible.as_ref().is_some_and(Bible::is_complete)
    }

    pub fn is_embedded(&self) -> bool {
        self.embedded
    }

    pub fn load_window(&mut self, center: crate::bible::VerseId) -> Result<bool> {
        if self.failed {
            return Ok(false);
        }

        if self
            .bible
            .as_ref()
            .is_some_and(|bible| bible.is_complete() || !bible.chapter_for(center).is_empty())
        {
            return Ok(true);
        }

        let cross_refs = data::cross_references();
        let bible = if self.embedded {
            Bible::load_window_from_str(data::kjv_xml(), cross_refs, center)?
        } else {
            Bible::load_window(self.source_path.as_ref().unwrap(), cross_refs, center)?
        };
        if bible.first_verse().is_none() {
            self.failed = true;
            return Ok(false);
        }

        self.bible = Some(bible);
        Ok(true)
    }

    pub fn ensure_full_loaded(&mut self) -> Result<bool> {
        if self.bible.as_ref().is_some_and(Bible::is_complete) {
            return Ok(true);
        }
        if self.failed {
            return Ok(false);
        }

        let cross_refs = data::cross_references();
        let bible = if self.embedded {
            Bible::load_from_str(data::kjv_xml(), cross_refs)?
        } else {
            Bible::load(self.source_path.as_ref().unwrap(), cross_refs)?
        };
        if bible.first_verse().is_none() {
            self.failed = true;
            return Ok(false);
        }

        self.bible = Some(bible);
        Ok(true)
    }

    pub fn set_loaded_bible(&mut self, bible: Bible) {
        self.bible = Some(bible);
        self.failed = false;
    }

    pub fn mark_failed(&mut self) {
        self.failed = true;
    }
}

pub struct TranslationRegistry {
    entries: Vec<TranslationEntry>,
    preferred: Option<String>,
}

impl TranslationRegistry {
    pub fn load() -> Result<Self> {
        let mut by_code = BTreeMap::new();

        by_code.insert(
            "kjv".to_string(),
            TranslationEntry::embedded("kjv".to_string()),
        );

        let cfg = config::load();
        let local_root = env::var(OSIS_DIR_ENV)
            .ok()
            .map(PathBuf::from)
            .or(cfg.bible_dir)
            .or_else(|| {
                let fallback = PathBuf::from(FALLBACK_LOCAL_TRANSLATIONS_DIR);
                fallback.exists().then_some(fallback)
            });

        if let Some(root) = local_root {
            for path in discover_xml_files(&root)? {
                let code = translation_code(&path);
                if by_code.contains_key(&code) {
                    continue;
                }
                by_code.insert(code.clone(), TranslationEntry::new(code, path));
            }
        }

        let preferred = env::var(DEFAULT_TRANSLATION_ENV)
            .ok()
            .map(|value| value.to_ascii_lowercase());

        Ok(Self {
            entries: by_code.into_values().collect(),
            preferred,
        })
    }

    pub fn into_entries(self) -> Vec<TranslationEntry> {
        self.entries
    }

    pub fn preferred_code(&self) -> Option<&str> {
        self.preferred.as_deref()
    }
}

fn discover_xml_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if root.is_file() {
        if is_xml_file(root) {
            files.push(root.to_path_buf());
        }
        return Ok(files);
    }

    let english_dir = root.join("en");
    if english_dir.is_dir() {
        collect_xml_files(&english_dir, &mut files)?;
    } else {
        collect_xml_files(root, &mut files)?;
    }

    files.sort();
    Ok(files)
}

fn collect_xml_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some(".git") {
                continue;
            }
            collect_xml_files(&path, files)?;
        } else if is_xml_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_xml_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("xml")
}

fn translation_code(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase()
}
