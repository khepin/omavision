//! A scan walks the library and produces an index. Nothing else.
use crate::config::Config;
use crate::index::{Category, Episode, Index, Item, Kind, Season};
use crate::parse::{nfc, parse_stem, Parsed};
use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::path::Path;
use walkdir::WalkDir;

pub fn scan(config: &Config) -> Result<Index> {
    let root = match config.root() {
        Some(r) => r,
        None => bail!("library.root is not set"),
    };
    if !root.is_dir() {
        bail!("library root {} is not a directory", root.display());
    }
    let mut categories = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    dirs.sort_by_key(|n| (rank(&config.library.order, n), n.to_lowercase()));
    for name in dirs {
        let items = scan_category(config, &root, &name);
        categories.push(Category { id: name.clone(), label: label_for(&name), items });
    }
    Ok(Index { scanned_at: crate::now(), categories })
}

/// Position in the configured order, with unlisted categories after every listed one.
fn rank(order: &[String], name: &str) -> usize {
    order.iter().position(|o| o.eq_ignore_ascii_case(name)).unwrap_or(order.len())
}

fn scan_category(config: &Config, root: &Path, cat: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut shows: BTreeMap<String, BTreeMap<u16, Vec<Episode>>> = BTreeMap::new();
    let walker = WalkDir::new(root.join(cat))
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !hidden(e.file_name().to_string_lossy().as_ref()));
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() || !config.is_video(entry.path()) {
            continue;
        }
        let rel = match entry.path().strip_prefix(root) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => continue,
        };
        let stem = entry.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        match parse_stem(&stem) {
            Parsed::Episode { show, season, episode } => {
                shows.entry(show).or_default().entry(season).or_default().push(Episode { season, episode, path: rel, title: None });
            }
            Parsed::Movie { title, year } => {
                items.push(Item { id: nfc(&rel), kind: Kind::Movie, title, year: Some(year), path: Some(rel), seasons: vec![] });
            }
            Parsed::Flat { title } => {
                items.push(Item { id: nfc(&rel), kind: Kind::Flat, title, year: None, path: Some(rel), seasons: vec![] });
            }
        }
    }
    for (show, seasons) in shows {
        let seasons = seasons
            .into_iter()
            .map(|(number, mut episodes)| {
                episodes.sort_by_key(|e| e.episode);
                Season { number, episodes }
            })
            .collect();
        items.push(Item { id: nfc(&format!("{cat}/{show}")), kind: Kind::Show, title: show, year: None, path: None, seasons });
    }
    items.sort_by_key(|i| i.title.to_lowercase());
    items
}

/// Dot files, dot directories and macOS AppleDouble `._*` files are never media.
fn hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn label_for(folder: &str) -> String {
    let words = folder.replace(['-', '_'], " ");
    let mut c = words.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"").unwrap();
    }

    #[test]
    fn groups_episodes_into_shows_and_skips_hidden() {
        let dir = std::env::temp_dir().join(format!("omv-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        touch(&dir.join("films/John Wick (2014).mkv"));
        touch(&dir.join("films/._John Wick (2014).mkv"));
        touch(&dir.join("films/notes.txt"));
        touch(&dir.join("films/Nested Folder/Spirited Away 2001 1080p.mkv"));
        touch(&dir.join("series/Friends/Season 01/Friends S01E02.mp4"));
        touch(&dir.join("series/Friends/Season 01/Friends S01E01.mp4"));
        touch(&dir.join("series/Friends/Season 02/Friends S02E01.mp4"));
        touch(&dir.join(".omavision/index.json"));
        let mut c = Config::default();
        c.library.root = dir.to_string_lossy().to_string();
        let idx = scan(&c).unwrap();
        assert_eq!(idx.categories.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["films", "series"]);
        let films = &idx.categories[0].items;
        assert_eq!(films.len(), 2);
        assert_eq!(films[0].title, "John Wick");
        assert_eq!(films[1].title, "Spirited Away");
        let friends = &idx.categories[1].items[0];
        assert_eq!(friends.kind, Kind::Show);
        assert_eq!(friends.id, "series/Friends");
        assert_eq!(friends.seasons.len(), 2);
        assert_eq!(friends.seasons[0].episodes[0].episode, 1);
        assert_eq!(friends.episode_count(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn configured_order_comes_first_then_alphabetical() {
        let dir = std::env::temp_dir().join(format!("omv-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for c in ["concerts", "divers", "films", "series"] {
            touch(&dir.join(c).join("x.mkv"));
        }
        let mut c = Config::default();
        c.library.root = dir.to_string_lossy().to_string();
        c.library.order = ["Films", "series", "missing"].map(String::from).to_vec();
        let idx = scan(&c).unwrap();
        assert_eq!(idx.categories.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["films", "series", "concerts", "divers"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// An id must survive a round trip through TMDB and a cache file name, so it is composed.
    /// The path must open the file, so it is whatever readdir gave us.
    #[test]
    fn ids_are_composed_while_paths_stay_raw() {
        let dir = std::env::temp_dir().join(format!("omv-nfd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        touch(&dir.join("films/Le Monde cache\u{301} (2019).mkv"));
        let mut c = Config::default();
        c.library.root = dir.to_string_lossy().to_string();
        let idx = scan(&c).unwrap();
        let item = &idx.categories[0].items[0];
        assert_eq!(item.id, "films/Le Monde cach\u{e9} (2019).mkv");
        assert!(dir.join(item.path.as_ref().unwrap()).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn labels_folders() {
        assert_eq!(label_for("dessins-animes"), "Dessins animes");
        assert_eq!(label_for("films"), "Films");
    }
}
