use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const DEFAULT_TOML: &str = include_str!("../assets/config.default.toml");

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub library: Library,
    pub player: Player,
    pub metadata: Metadata,
    pub ui: Ui,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Library {
    pub root: String,
    pub cache_dir: String,
    pub extensions: Vec<String>,
    pub order: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Player {
    pub command: Vec<String>,
    pub pre: Vec<String>,
    pub post: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Metadata {
    pub provider: String,
    pub api_key_file: String,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Ui {
    pub scale: f32,
    pub fullscreen: bool,
    pub theme_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Config { library: Library::default(), player: Player::default(), metadata: Metadata::default(), ui: Ui::default() }
    }
}
impl Default for Library {
    fn default() -> Self {
        Library {
            root: String::new(),
            cache_dir: "{root}/.omavision".into(),
            extensions: ["mp4", "mkv", "avi", "m4v", "webm", "mov"].map(String::from).to_vec(),
            order: vec![],
        }
    }
}
impl Default for Player {
    fn default() -> Self {
        Player { command: ["mpv", "--fs", "--save-position-on-quit"].map(String::from).to_vec(), pre: vec![], post: vec![] }
    }
}
impl Default for Metadata {
    fn default() -> Self {
        Metadata { provider: "tmdb".into(), api_key_file: "~/.config/omavision/tmdb.key".into(), languages: ["fr", "en"].map(String::from).to_vec() }
    }
}
impl Default for Ui {
    fn default() -> Self { Ui { scale: 1.6, fullscreen: true, theme_file: String::new() } }
}

/// Outcome of loading the config: whether the file had to be created.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: Config,
    pub path: PathBuf,
    pub created: bool,
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("omavision")
}

pub fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("omavision")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Loads the config, writing the documented default first when none exists.
pub fn load_or_init() -> Result<LoadedConfig> {
    let path = config_path();
    let created = !path.exists();
    if created {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        std::fs::write(&path, DEFAULT_TOML).with_context(|| format!("writing {}", path.display()))?;
    }
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: Config = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(LoadedConfig { config, path, created })
}

pub fn expand_home(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(h) = dirs::home_dir() {
            return h.join(rest);
        }
    }
    PathBuf::from(s)
}

impl Config {
    pub fn root(&self) -> Option<PathBuf> {
        let r = self.library.root.trim();
        if r.is_empty() {
            None
        } else {
            Some(expand_home(r))
        }
    }

    pub fn cache_dir(&self) -> Option<PathBuf> {
        let root = self.root()?;
        let s = self.library.cache_dir.replace("{root}", &root.to_string_lossy());
        Some(expand_home(&s))
    }

    pub fn theme_file(&self) -> Option<PathBuf> {
        let t = self.ui.theme_file.trim();
        if t.is_empty() { None } else { Some(expand_home(t)) }
    }

    pub fn is_video(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                let e = e.to_ascii_lowercase();
                self.library.extensions.iter().any(|x| x.eq_ignore_ascii_case(&e))
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped template is documentation; the code defaults are truth. They must agree.
    #[test]
    fn template_matches_code_defaults() {
        let t: Config = toml::from_str(DEFAULT_TOML).unwrap();
        let d = Config::default();
        assert_eq!(t.library.root, d.library.root);
        assert_eq!(t.library.cache_dir, d.library.cache_dir);
        assert_eq!(t.library.extensions, d.library.extensions);
        assert_eq!(t.library.order, d.library.order);
        assert_eq!(t.player.command, d.player.command);
        assert_eq!(t.player.pre, d.player.pre);
        assert_eq!(t.player.post, d.player.post);
        assert_eq!(t.metadata.provider, d.metadata.provider);
        assert_eq!(t.metadata.api_key_file, d.metadata.api_key_file);
        assert_eq!(t.metadata.languages, d.metadata.languages);
        assert_eq!(t.ui.scale, d.ui.scale);
        assert_eq!(t.ui.fullscreen, d.ui.fullscreen);
        assert_eq!(t.ui.theme_file, d.ui.theme_file);
    }

    #[test]
    fn cache_dir_expands_root_placeholder() {
        let mut c = Config::default();
        c.library.root = "/media/movies".into();
        assert_eq!(c.cache_dir().unwrap(), PathBuf::from("/media/movies/.omavision"));
    }

    #[test]
    fn partial_config_fills_defaults() {
        let c: Config = toml::from_str("[library]\nroot = \"/x\"\n").unwrap();
        assert_eq!(c.library.root, "/x");
        assert_eq!(c.player.command[0], "mpv");
    }
}
