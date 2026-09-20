use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Movie,
    Show,
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub season: u16,
    pub episode: u16,
    /// Relative to the library root.
    pub path: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub number: u16,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// Stable id: the relative path for files, `<category>/<show>` for shows. Always NFC,
    /// so it survives a macOS disk; `path` stays as the file system spelled it.
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub year: Option<u16>,
    /// Relative path of the video file. None for shows.
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seasons: Vec<Season>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub label: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Index {
    /// Seconds since the Unix epoch.
    pub scanned_at: u64,
    pub categories: Vec<Category>,
}

impl Item {
    pub fn episode_count(&self) -> usize {
        self.seasons.iter().map(|s| s.episodes.len()).sum()
    }
}

impl Index {
    pub fn item_count(&self) -> usize {
        self.categories.iter().map(|c| c.items.len()).sum()
    }
}
