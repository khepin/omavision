//! Enrichment: what to fetch and how to ask the provider for it. Network only; the caller
//! runs it on a worker thread and stores what comes back through the cache.
use crate::config::Config;
use crate::index::{Index, Item, Kind};
use crate::meta::{Meta, VERSION};
use crate::tmdb::{self, Client, MediaKind};
use anyhow::Result;
use log::warn;
use std::collections::HashMap;

/// Items whose metadata is missing, was fetched for another first language, or was written
/// by a build with another cache version, in index order.
pub fn missing<'a>(index: &'a Index, have: &HashMap<String, Meta>, config: &Config) -> Vec<&'a Item> {
    let want = config.metadata.languages.first().map(String::as_str).unwrap_or("en");
    index
        .categories
        .iter()
        .flat_map(|c| c.items.iter())
        .filter(|it| match have.get(&it.id) {
            None => true,
            Some(m) => m.requested != want || m.version != VERSION,
        })
        .collect()
}

/// Metadata for one item: search, details with language fallback, episode titles for shows.
/// A miss returns the parsed name, so the caller can store it and stop asking every launch.
pub fn lookup(client: &Client, item: &Item, langs: &[&str]) -> Result<Meta> {
    let first = langs.first().copied().unwrap_or("en");
    let kind = match item.kind {
        Kind::Show => MediaKind::Tv,
        _ => MediaKind::Movie,
    };
    let mut id = None;
    for variant in title_variants(&item.title) {
        let cands = client.search(kind, &variant, first)?;
        id = tmdb::pick(&cands, item.year);
        if id.is_some() {
            break;
        }
    }
    let Some(id) = id else {
        return Ok(Meta { version: VERSION, title: item.title.clone(), year: item.year, language: first.to_string(), requested: first.to_string(), fetched_at: crate::now(), ..Default::default() });
    };
    let mut meta = details(client, kind, id, langs)?;
    meta.version = VERSION;
    if meta.title.is_empty() {
        meta.title = item.title.clone();
    }
    if kind == MediaKind::Tv {
        for s in &item.seasons {
            match client.season_episodes(id, s.number, &meta.language) {
                Ok(eps) => meta.episodes.extend(eps),
                Err(e) => warn!("season {} of {}: {e:#}", s.number, item.title),
            }
        }
    }
    Ok(meta)
}

/// Details in the first configured language, then the next ones until the overview is not empty.
fn details(client: &Client, kind: MediaKind, id: u64, langs: &[&str]) -> Result<Meta> {
    let first = langs.first().copied().unwrap_or("en");
    let mut meta = client.details(kind, id, first)?;
    meta.requested = first.to_string();
    if meta.overview.is_empty() {
        for lang in langs.iter().skip(1) {
            let alt = client.details(kind, id, lang)?;
            if !alt.overview.is_empty() {
                meta.overview = alt.overview;
                if meta.title.is_empty() {
                    meta.title = alt.title;
                }
                meta.language = lang.to_string();
                break;
            }
        }
    }
    Ok(meta)
}

/// Search strings to try, in order: the title as parsed, then with the " - " subtitle
/// separator turned into a colon, then the part before the separator alone.
pub fn title_variants(title: &str) -> Vec<String> {
    let mut v = vec![title.to_string()];
    if let Some((head, _)) = title.split_once(" - ") {
        v.push(title.replacen(" - ", ": ", 1));
        v.push(head.to_string());
    }
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_change_marks_items_for_refetch() {
        use crate::index::{Category, Kind};
        let item = Item { id: "films/x.mkv".into(), kind: Kind::Movie, title: "x".into(), year: None, path: Some("films/x.mkv".into()), seasons: vec![] };
        let index = Index { scanned_at: 0, categories: vec![Category { id: "films".into(), label: "Films".into(), items: vec![item] }] };
        let mut have = HashMap::new();
        have.insert("films/x.mkv".to_string(), Meta { version: VERSION, requested: "fr".into(), ..Default::default() });
        let mut c = Config::default();
        c.metadata.languages = vec!["fr".into(), "en".into()];
        assert!(missing(&index, &have, &c).is_empty());
        c.metadata.languages = vec!["en".into(), "fr".into()];
        assert_eq!(missing(&index, &have, &c).len(), 1);
    }

    #[test]
    fn old_cache_version_marks_items_for_refetch() {
        use crate::index::{Category, Kind};
        let item = Item { id: "films/x.mkv".into(), kind: Kind::Movie, title: "x".into(), year: None, path: Some("films/x.mkv".into()), seasons: vec![] };
        let index = Index { scanned_at: 0, categories: vec![Category { id: "films".into(), label: "Films".into(), items: vec![item] }] };
        let mut c = Config::default();
        c.metadata.languages = vec!["en".into()];
        let mut have = HashMap::new();
        have.insert("films/x.mkv".to_string(), Meta { version: VERSION - 1, requested: "en".into(), ..Default::default() });
        assert_eq!(missing(&index, &have, &c).len(), 1);
    }

    #[test]
    fn title_variants_fall_back_to_the_head() {
        assert_eq!(title_variants("Dragons 3 - Le Monde caché"), vec!["Dragons 3 - Le Monde caché", "Dragons 3: Le Monde caché", "Dragons 3"]);
        assert_eq!(title_variants("Ponyo"), vec!["Ponyo"]);
    }
}
