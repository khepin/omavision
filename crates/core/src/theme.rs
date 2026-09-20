//! Reads the Omarchy theme (`colors.toml`) and resolves it into the palette the UI uses.
//! Format and fallback chain: docs/omarchy-theme.md.
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub mode: Mode,
    pub bg: Rgb,
    pub surface: Rgb,
    pub surface2: Rgb,
    pub text: Rgb,
    pub text2: Rgb,
    pub muted: Rgb,
    pub accent: Rgb,
    pub accent2: Rgb,
}

impl Palette {
    /// The built-in palette, used when no Omarchy theme is found. Tokyo Night.
    pub fn builtin() -> Palette {
        Palette {
            mode: Mode::Dark,
            bg: Rgb(0x1a, 0x1b, 0x26),
            surface: Rgb(0x24, 0x28, 0x3b),
            surface2: Rgb(0x29, 0x2e, 0x42),
            text: Rgb(0xa9, 0xb1, 0xd6),
            text2: Rgb(0x56, 0x5f, 0x89),
            muted: Rgb(0x41, 0x48, 0x68),
            accent: Rgb(0x7a, 0xa2, 0xf7),
            accent2: Rgb(0xad, 0x8e, 0xe6),
        }
    }
}

/// Candidate locations of the active Omarchy theme, most current first.
fn candidate_files() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(p) = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).filter(|p| p.is_absolute()) {
        v.push(p.join("omarchy/current/theme/colors.toml"));
    }
    if let Some(h) = dirs::home_dir() {
        v.push(h.join(".local/state/omarchy/current/theme/colors.toml"));
        v.push(h.join(".config/omarchy/current/theme/colors.toml"));
    }
    v
}

/// The first candidate that exists.
pub fn find_file() -> Option<PathBuf> {
    candidate_files().into_iter().find(|p| p.is_file())
}

/// The directory to watch for theme changes: the parent of the `theme` dir, because Omarchy
/// replaces that directory wholesale.
pub fn watch_dir(file: &Path) -> Option<PathBuf> {
    file.parent()?.parent().map(|p| p.to_path_buf())
}

pub fn load(file: &Path) -> Result<Palette> {
    let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    parse(&text).with_context(|| format!("parsing {}", file.display()))
}

pub fn parse(text: &str) -> Result<Palette> {
    let table: toml::Table = toml::from_str(text)?;
    let mut keys: HashMap<String, String> = HashMap::new();
    for (k, v) in table {
        if let Some(s) = v.as_str() {
            keys.insert(k, s.to_string());
        }
    }
    Ok(resolve(&keys))
}

fn resolve(k: &HashMap<String, String>) -> Palette {
    let get = |names: &[&str]| names.iter().find_map(|n| k.get(*n).and_then(|v| parse_hex(v)));
    let fallback = Palette::builtin();
    let bg = get(&["background", "bg", "color0"]).unwrap_or(fallback.bg);
    let fg = get(&["foreground", "fg", "color7"]).unwrap_or(fallback.text);
    let mode = match k.get("mode").or_else(|| k.get("theme_type")).map(|s| s.as_str()) {
        Some("light") => Mode::Light,
        Some("dark") => Mode::Dark,
        _ => {
            if bg.0 as u32 + bg.1 as u32 + bg.2 as u32 > 382 { Mode::Light } else { Mode::Dark }
        }
    };
    let toward = |a: Rgb, b: Rgb, t: f32| Rgb(
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t) as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t) as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t) as u8,
    );
    let surface = get(&["lighter_background", "lighter_bg"]).unwrap_or_else(|| toward(bg, fg, 0.08));
    let muted = get(&["muted", "color8", "dark_foreground"]).unwrap_or_else(|| toward(bg, fg, 0.3));
    let surface2 = get(&["selection", "selection_background", "color8"]).unwrap_or(muted);
    let text2 = get(&["dark_foreground", "dark_fg"]).unwrap_or_else(|| toward(fg, bg, 0.4));
    let accent = get(&["accent", "blue", "color4"]).unwrap_or(fallback.accent);
    let accent2 = get(&["magenta", "color5", "purple"]).unwrap_or(fallback.accent2);
    Palette { mode, bg, surface, surface2, text: fg, text2, muted, accent, accent2 }
}

/// `#rrggbb` only. Anything else (Hyprland gradients, names) is ignored.
fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.trim().strip_prefix('#')?;
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// Calls `on_change` whenever the theme directory changes. Keeps watching while the returned
/// watcher lives.
pub fn watch(file: &Path, on_change: impl Fn() + Send + 'static) -> Result<notify::RecommendedWatcher> {
    use notify::{Event, RecursiveMode, Watcher};
    let dir = watch_dir(file).context("theme file has no watchable parent")?;
    let mut w = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
        if res.is_ok() {
            on_change();
        }
    })?;
    w.watch(&dir, RecursiveMode::Recursive).with_context(|| format!("watching {}", dir.display()))?;
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKYO: &str = r##"
mode = "dark"
accent = "#7aa2f7"
selection = "#292e42"
muted = "#414868"
background = "#1a1b26"
dark_background = "#13141c"
lighter_background = "#24283b"
foreground = "#a9b1d6"
dark_foreground = "#565f89"
bright_foreground = "#c0caf5"
red = "#f7768e"
blue = "#7aa2f7"
magenta = "#ad8ee6"
hyprland_active_border = "rgba(33ccffee) rgba(00ff99ee) 45deg"
"##;

    #[test]
    fn tokyo_night_maps_to_tokens() {
        let p = parse(TOKYO).unwrap();
        assert_eq!(p.mode, Mode::Dark);
        assert_eq!(p.bg, Rgb(0x1a, 0x1b, 0x26));
        assert_eq!(p.surface, Rgb(0x24, 0x28, 0x3b));
        assert_eq!(p.surface2, Rgb(0x29, 0x2e, 0x42));
        assert_eq!(p.text, Rgb(0xa9, 0xb1, 0xd6));
        assert_eq!(p.text2, Rgb(0x56, 0x5f, 0x89));
        assert_eq!(p.muted, Rgb(0x41, 0x48, 0x68));
        assert_eq!(p.accent, Rgb(0x7a, 0xa2, 0xf7));
        assert_eq!(p.accent2, Rgb(0xad, 0x8e, 0xe6));
    }

    #[test]
    fn legacy_keys_and_light_detection() {
        let p = parse("bg = \"#faf4ed\"\nfg = \"#575279\"\ncolor4 = \"#56949f\"\ncolor5 = \"#907aa9\"\ncolor8 = \"#cecacd\"\n").unwrap();
        assert_eq!(p.mode, Mode::Light);
        assert_eq!(p.accent, Rgb(0x56, 0x94, 0x9f));
        assert_eq!(p.accent2, Rgb(0x90, 0x7a, 0xa9));
        assert_eq!(p.muted, Rgb(0xce, 0xca, 0xcd));
        assert_eq!(p.surface2, p.muted);
    }

    #[test]
    fn empty_file_gives_builtin_like_palette() {
        let p = parse("").unwrap();
        assert_eq!(p.bg, Palette::builtin().bg);
        assert_eq!(p.accent, Palette::builtin().accent);
    }

    #[test]
    fn light_mode_marker_file_is_not_needed_when_mode_is_set() {
        let p = parse("mode = \"light\"\nbackground = \"#000000\"\n").unwrap();
        assert_eq!(p.mode, Mode::Light);
    }
}
