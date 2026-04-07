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
        return Path::new(&config_home).join("tui-bible").join(CONFIG_FILE);
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("tui-bible")
            .join(CONFIG_FILE);
    }

    PathBuf::from(".tui-bible-config.toml")
}
