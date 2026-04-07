use std::{
    collections::VecDeque,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    app::{Focus, SidePanel},
    bible::VerseId,
};

const SESSION_FILE: &str = "session.toml";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionState {
    pub translation: String,
    pub current_verse: VerseId,
    pub focus: Focus,
    pub side_panel: SidePanel,
    pub history: Vec<VerseId>,
    pub history_index: usize,
}

pub fn load() -> Option<SessionState> {
    let path = session_path();
    let text = fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

pub fn save(state: &SessionState) -> io::Result<()> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(state)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, text)
}

pub fn state_from_parts(
    translation: String,
    current_verse: VerseId,
    focus: Focus,
    side_panel: SidePanel,
    history: &VecDeque<VerseId>,
    history_index: usize,
) -> SessionState {
    SessionState {
        translation,
        current_verse,
        focus,
        side_panel,
        history: history.iter().copied().collect(),
        history_index,
    }
}

fn session_path() -> PathBuf {
    std::env::var("TUI_BIBLE_SESSION")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_session_path())
}

fn default_session_path() -> PathBuf {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return Path::new(&config_home).join("tui-bible").join(SESSION_FILE);
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("tui-bible")
            .join(SESSION_FILE);
    }

    PathBuf::from(".tui-bible-session.toml")
}
