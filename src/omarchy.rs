//! Omarchy desktop theme integration.
//!
//! Omarchy keeps the live theme under `~/.local/state/omarchy/current/theme`,
//! a directory of per-app colour files symlinked into place whenever
//! `omarchy theme set` runs. The one we care about is `colors.toml`: a flat
//! table of `key = "#rrggbb"` pairs that every Omarchy theme ships.
//!
//! Reading that file is the whole integration. The `theme-set` hook in
//! `pkg/omarchy/` exists only to nudge us — it touches a stamp file so a
//! running TUI notices the change without the user restarting it. The stamp
//! is an optimisation, not a requirement: we also watch `colors.toml` itself,
//! so live reloading still works on a plain `omarchy theme set` with no hook
//! installed at all.
//!
//! Everything here degrades to `None` off Omarchy, which keeps malacli a
//! portable terminal app on macOS and non-Omarchy Linux.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// A parsed `colors.toml`. Only the keys malacli maps onto its own palette are
/// kept; Omarchy themes carry a good many more.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Palette {
    /// `mode = "dark"` or `"light"`. Themes are overwhelmingly dark, so an
    /// absent or unrecognised mode is treated as dark.
    pub light: bool,
    pub background: Option<String>,
    pub lighter_background: Option<String>,
    pub foreground: Option<String>,
    pub bright_foreground: Option<String>,
    pub dark_foreground: Option<String>,
    pub muted: Option<String>,
    pub accent: Option<String>,
    pub selection: Option<String>,
    pub yellow: Option<String>,
    pub bright_yellow: Option<String>,
}

/// The current Omarchy theme, or `None` when we aren't on an Omarchy system
/// (or the state directory has no `colors.toml`).
pub fn palette() -> Option<Palette> {
    let path = colors_path()?;
    parse(&fs::read_to_string(path).ok()?).into()
}

/// The slug of the live theme, e.g. `tokyo-night`. Shown in `malacli config`
/// so it's obvious which desktop theme is being followed.
pub fn theme_name() -> Option<String> {
    if theme_override().is_some() {
        // An override points straight at a colors.toml; its parent directory
        // is the closest thing to a name we have.
        let dir = colors_path()?.parent()?.file_name()?.to_owned();
        return Some(dir.to_string_lossy().into_owned());
    }
    let name = fs::read_to_string(state_dir()?.join("current/theme.name")).ok()?;
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// A change token for the live theme: the newest mtime across the stamp file
/// the hook touches and `colors.toml` itself.
///
/// The render loop compares this between frames and rebuilds the palette when
/// it moves. Both files are watched because either can change alone: the hook
/// stamp covers the case where a theme is re-applied with identical colours,
/// and `colors.toml` covers systems where the hook was never installed.
pub fn revision() -> Option<SystemTime> {
    let stamp = stamp_path().and_then(|p| mtime(&p));
    let colors = colors_path().and_then(|p| mtime(&p));
    match (stamp, colors) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

/// Where the theme-set hook writes its stamp. Also used by the installer.
pub fn stamp_path() -> Option<PathBuf> {
    Some(state_home()?.join("malacli/theme-stamp"))
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// `MALACLI_OMARCHY_THEME` points at a `colors.toml` directly, which makes the
/// integration testable and lets non-Omarchy machines borrow a palette.
fn theme_override() -> Option<PathBuf> {
    let raw = std::env::var_os("MALACLI_OMARCHY_THEME")?;
    (!raw.is_empty()).then(|| PathBuf::from(raw))
}

fn colors_path() -> Option<PathBuf> {
    if let Some(path) = theme_override() {
        return path.is_file().then_some(path);
    }
    let path = state_dir()?.join("current/theme/colors.toml");
    path.is_file().then_some(path)
}

fn state_dir() -> Option<PathBuf> {
    Some(state_home()?.join("omarchy"))
}

fn state_home() -> Option<PathBuf> {
    if let Some(state) = std::env::var_os("XDG_STATE_HOME")
        && !state.is_empty()
    {
        return Some(PathBuf::from(state));
    }
    let home = std::env::var_os("HOME")?;
    Some(Path::new(&home).join(".local/state"))
}

/// Parse `colors.toml`.
///
/// Hand-rolled rather than handed to the `toml` crate on purpose: the file is
/// a flat table of string values, and some themes in the wild carry keys that
/// aren't valid TOML on the strict parser. A malformed line should cost us one
/// colour, not the whole desktop theme.
fn parse(text: &str) -> Palette {
    let mut palette = Palette::default();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches(['"', '\''].as_slice()).trim();

        if key == "mode" {
            palette.light = value.eq_ignore_ascii_case("light");
            continue;
        }

        let Some(color) = normalize_hex(value) else {
            continue;
        };
        let slot = match key {
            "background" => &mut palette.background,
            "lighter_background" => &mut palette.lighter_background,
            "foreground" => &mut palette.foreground,
            "bright_foreground" => &mut palette.bright_foreground,
            "dark_foreground" => &mut palette.dark_foreground,
            "muted" => &mut palette.muted,
            "accent" => &mut palette.accent,
            "selection" => &mut palette.selection,
            "yellow" => &mut palette.yellow,
            "bright_yellow" => &mut palette.bright_yellow,
            _ => continue,
        };
        *slot = Some(color);
    }

    palette
}

/// Accept `#rrggbb`, `rrggbb`, and the `#rgb` shorthand, returning the value
/// in `rrggbb` form. Anything else is rejected so a stray non-colour value
/// can't reach the renderer.
fn normalize_hex(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        6 => Some(hex.to_ascii_lowercase()),
        3 => Some(
            hex.chars()
                .flat_map(|c| [c, c])
                .collect::<String>()
                .to_ascii_lowercase(),
        ),
        _ => None,
    }
}

/// Split `rrggbb` into components for `ratatui`'s `Color::Rgb`.
pub fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = normalize_hex(hex)?;
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some((byte(0)?, byte(2)?, byte(4)?))
}

/// Serialises tests that mutate process-global environment variables.
///
/// `MALACLI_OMARCHY_THEME` is read by both this module and `ui`, and the test
/// runner is multi-threaded, so without this the two suites race and one
/// intermittently reads the other's fixture.
#[cfg(test)]
pub fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // A poisoned lock just means another env test panicked; the guard is still
    // sound to take, and failing here would only mask the real failure.
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKYO_NIGHT: &str = r##"
mode = "dark"

accent = "#7aa2f7"
selection = "#292e42"
muted = "#414868"

background = "#1a1b26"
lighter_background = "#24283b"

foreground = "#a9b1d6"
dark_foreground = "#565f89"
bright_foreground = "#c0caf5"

yellow = "#e0af68"
bright_yellow = "#ff9e64"
"##;

    #[test]
    fn parses_a_real_theme() {
        let palette = parse(TOKYO_NIGHT);
        assert!(!palette.light);
        assert_eq!(palette.background.as_deref(), Some("1a1b26"));
        assert_eq!(palette.accent.as_deref(), Some("7aa2f7"));
        assert_eq!(palette.bright_foreground.as_deref(), Some("c0caf5"));
        assert_eq!(palette.selection.as_deref(), Some("292e42"));
    }

    #[test]
    fn detects_light_mode() {
        assert!(parse(r#"mode = "light""#).light);
        assert!(parse(r#"mode = "Light""#).light);
        assert!(!parse(r#"mode = "dark""#).light);
        // An absent mode means dark, which is the overwhelming default.
        assert!(!parse("accent = \"#ffffff\"").light);
    }

    #[test]
    fn skips_junk_without_losing_the_rest() {
        let palette = parse(
            r##"
# a comment
[section]
accent = "#ff0000"
muted = not-a-colour
malformed line with no equals
background = "#00ff00"
"##,
        );
        assert_eq!(palette.accent.as_deref(), Some("ff0000"));
        assert_eq!(palette.background.as_deref(), Some("00ff00"));
        assert_eq!(palette.muted, None);
    }

    #[test]
    fn normalizes_hex_forms() {
        assert_eq!(normalize_hex("#AABBCC").as_deref(), Some("aabbcc"));
        assert_eq!(normalize_hex("aabbcc").as_deref(), Some("aabbcc"));
        assert_eq!(normalize_hex("#abc").as_deref(), Some("aabbcc"));
        assert_eq!(normalize_hex("#ggg"), None);
        assert_eq!(normalize_hex("#abcd"), None);
        assert_eq!(normalize_hex(""), None);
    }

    #[test]
    fn revision_moves_when_the_theme_file_changes() {
        let _guard = env_lock();
        // The whole live-reload story rests on this: touch the palette, and
        // the revision token must move so the render cache invalidates.
        let dir = std::env::temp_dir().join("malacli-revision-test");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let colors = dir.join("colors.toml");
        std::fs::write(&colors, "accent = \"#111111\"\n").expect("write");

        unsafe { std::env::set_var("MALACLI_OMARCHY_THEME", &colors) };
        let before = revision();
        assert!(
            before.is_some(),
            "a readable colors.toml must yield a revision"
        );

        // mtime granularity can be coarse, so set it explicitly rather than
        // racing the clock with a sleep.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(10);
        std::fs::write(&colors, "accent = \"#222222\"\n").expect("rewrite");
        let file = std::fs::File::options()
            .write(true)
            .open(&colors)
            .expect("open");
        file.set_modified(later).expect("set mtime");

        assert_ne!(
            before,
            revision(),
            "a changed palette must move the revision"
        );
        assert_eq!(palette().unwrap().accent.as_deref(), Some("222222"));

        unsafe { std::env::remove_var("MALACLI_OMARCHY_THEME") };
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn converts_to_rgb() {
        assert_eq!(rgb("#1a1b26"), Some((26, 27, 38)));
        assert_eq!(rgb("fff"), Some((255, 255, 255)));
        assert_eq!(rgb("nope"), None);
    }

    #[test]
    fn strips_trailing_comments_never_at_cost_of_the_value() {
        // Values are quoted in every theme we've seen; make sure the quote
        // stripping doesn't mangle a bare value.
        assert_eq!(parse("accent = #7aa2f7").accent.as_deref(), Some("7aa2f7"));
    }
}
