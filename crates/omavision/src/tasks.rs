//! Worker threads. They touch no state: they do the slow thing and post one Msg back.
use crate::ui::{apply_palette, Msg, Post};
use crate::MainWindow;
use log::warn;
use omavision_core::cache::Cache;
use omavision_core::config::{Config, Player};
use omavision_core::theme::{self, Palette};
use omavision_core::tmdb::Client;
use omavision_core::{enrich, Item, Meta};
use std::path::PathBuf;

pub fn scan(config: Config, post: Post) {
    std::thread::spawn(move || post.send(Msg::Scanned(omavision_core::scan::scan(&config))));
}

pub fn play(player: Player, path: PathBuf, ui: &MainWindow, post: &Post) {
    ui.set_playing(true);
    ui.set_status(format!("playing {}", path.display()).into());
    let post = post.clone();
    std::thread::spawn(move || post.send(Msg::Played(omavision_core::player::play(&player, &path))));
}

/// One item at a time: look it up, store what came back, tell the UI.
pub fn enrich(config: Config, key: String, todo: Vec<Item>, post: Post) {
    std::thread::spawn(move || {
        let client = Client::new(key);
        let cache = config.cache_dir().map(Cache::open);
        let langs: Vec<&str> = config.metadata.languages.iter().map(String::as_str).collect();
        let total = todo.len();
        for (n, item) in todo.iter().enumerate() {
            let result = enrich::lookup(&client, item, &langs).map(|mut meta| {
                if let Some(cache) = &cache {
                    store(&client, cache, item, &mut meta);
                }
                meta
            });
            post.send(Msg::Enriched { id: item.id.clone(), done: n + 1, total, result });
        }
    });
}

fn store(client: &Client, cache: &Cache, item: &Item, meta: &mut Meta) {
    if let Some(url) = meta.poster_url.clone() {
        match client.download(&url).and_then(|bytes| cache.store_poster(&item.id, &bytes)) {
            Ok(file) => meta.poster_file = Some(file),
            Err(e) => warn!("poster for {}: {e:#}", item.title),
        }
    }
    if let Err(e) = cache.save_meta(&item.id, meta) {
        warn!("saving metadata for {}: {e:#}", item.title);
    }
}

/// Applies the theme now and keeps applying it when Omarchy switches themes.
pub fn watch_theme(config: &Config, ui: &MainWindow, post: Post) -> Option<notify::RecommendedWatcher> {
    let Some(file) = config.theme_file().or_else(theme::find_file) else {
        apply_palette(ui, &Palette::builtin());
        return None;
    };
    match theme::load(&file) {
        Ok(p) => apply_palette(ui, &p),
        Err(e) => {
            warn!("theme: {e:#}");
            apply_palette(ui, &Palette::builtin());
        }
    }
    let f = file.clone();
    match theme::watch(&file, move || match theme::load(&f) {
        Ok(p) => post.send(Msg::Theme(p)),
        Err(e) => warn!("theme: {e:#}"),
    }) {
        Ok(w) => Some(w),
        Err(e) => {
            warn!("theme watch: {e:#}");
            None
        }
    }
}
