//! The cache directory: `index.json`, `meta/`, `posters/`. Nothing else knows that layout.
//! Everything in here is derived from disk or the provider and safe to delete.
use crate::index::Index;
use crate::meta::Meta;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const INDEX_FILE: &str = "index.json";

pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn open(dir: impl Into<PathBuf>) -> Cache {
        Cache { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn index(&self) -> Result<Option<Index>> {
        let p = self.dir.join(INDEX_FILE);
        if !p.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        Ok(Some(serde_json::from_str(&text).with_context(|| format!("parsing {}", p.display()))?))
    }

    pub fn save_index(&self, index: &Index) -> Result<()> {
        std::fs::create_dir_all(&self.dir).with_context(|| format!("creating {}", self.dir.display()))?;
        let p = self.dir.join(INDEX_FILE);
        let tmp = self.dir.join(format!("{INDEX_FILE}.tmp"));
        std::fs::write(&tmp, serde_json::to_vec_pretty(index)?).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &p).with_context(|| format!("moving into place {}", p.display()))?;
        Ok(())
    }

    pub fn meta(&self, item_id: &str) -> Option<Meta> {
        let text = std::fs::read_to_string(self.meta_path(item_id)).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save_meta(&self, item_id: &str, meta: &Meta) -> Result<()> {
        let dir = self.dir.join("meta");
        std::fs::create_dir_all(&dir)?;
        let p = self.meta_path(item_id);
        std::fs::write(&p, serde_json::to_vec_pretty(meta)?).with_context(|| format!("writing {}", p.display()))
    }

    /// Everything already cached for the items of an index.
    pub fn load_all_meta(&self, index: &Index) -> HashMap<String, Meta> {
        let mut m = HashMap::new();
        for c in &index.categories {
            for it in &c.items {
                if let Some(meta) = self.meta(&it.id) {
                    m.insert(it.id.clone(), meta);
                }
            }
        }
        m
    }

    /// Writes poster bytes and returns the file name to record in the item's metadata.
    pub fn store_poster(&self, item_id: &str, bytes: &[u8]) -> Result<String> {
        let dir = self.dir.join("posters");
        std::fs::create_dir_all(&dir)?;
        let p = self.poster_path(item_id);
        std::fs::write(&p, bytes).with_context(|| format!("writing {}", p.display()))?;
        Ok(poster_file(item_id))
    }

    pub fn poster_path(&self, item_id: &str) -> PathBuf {
        self.dir.join("posters").join(poster_file(item_id))
    }

    fn meta_path(&self, item_id: &str) -> PathBuf {
        self.dir.join("meta").join(format!("{}.json", cache_key(item_id)))
    }
}

fn poster_file(item_id: &str) -> String {
    format!("{}.jpg", cache_key(item_id))
}

/// Stable file name for an item id: FNV-1a 64 as hex.
pub fn cache_key(item_id: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in item_id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Category, Item, Kind};

    #[test]
    fn key_is_stable_and_filename_safe() {
        assert_eq!(cache_key("films/John Wick (2014).mkv"), cache_key("films/John Wick (2014).mkv"));
        assert_ne!(cache_key("a"), cache_key("b"));
        assert!(cache_key("séries/x").chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn round_trips_index_meta_and_posters() {
        let dir = std::env::temp_dir().join(format!("omv-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = Cache::open(&dir);
        assert!(cache.index().unwrap().is_none());
        let item = Item { id: "films/x.mkv".into(), kind: Kind::Movie, title: "X".into(), year: Some(2001), path: Some("films/x.mkv".into()), seasons: vec![] };
        let index = Index { scanned_at: 42, categories: vec![Category { id: "films".into(), label: "Films".into(), items: vec![item] }] };
        cache.save_index(&index).unwrap();
        assert_eq!(cache.index().unwrap().unwrap().scanned_at, 42);
        assert!(cache.meta("films/x.mkv").is_none());
        cache.save_meta("films/x.mkv", &Meta { title: "X".into(), ..Default::default() }).unwrap();
        assert_eq!(cache.load_all_meta(&index).len(), 1);
        let file = cache.store_poster("films/x.mkv", b"jpeg").unwrap();
        assert_eq!(cache.poster_path("films/x.mkv").file_name().unwrap().to_string_lossy(), file);
        assert_eq!(std::fs::read(cache.poster_path("films/x.mkv")).unwrap(), b"jpeg");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
