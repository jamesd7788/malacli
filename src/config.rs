use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.toml";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bible_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,
}

impl Config {
    pub fn display(&self) {
        println!(
            "bible-dir:    {}",
            self.bible_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(not set)".to_string())
        );
        println!(
            "translation:  {}",
            self.translation
                .as_deref()
                .unwrap_or("(not set, defaults to kjv)")
        );
        println!(
            "theme:        {}",
            self.theme
                .as_deref()
                .unwrap_or("(not set, defaults to monastic)")
        );
        println!(
            "editor:       {}",
            self.editor
                .as_deref()
                .unwrap_or("(not set, uses $EDITOR or vim)")
        );
    }
}

pub fn load() -> Config {
    let path = config_path();
    let Ok(text) = fs::read_to_string(path) else {
        return Config::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

pub fn save(config: &Config) -> io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(config)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, text)
}

fn config_path() -> PathBuf {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return Path::new(&config_home).join("malacli").join(CONFIG_FILE);
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("malacli")
            .join(CONFIG_FILE);
    }

    PathBuf::from(".malacli-config.toml")
}
