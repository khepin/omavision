//! The UI thread: one message type, one place where state changes, one render.
use crate::{tasks, Card, MainWindow, Row, Tab, Theme};
use log::warn;
use omavision_core::browse::{self, Action, Browser, Effect};
use omavision_core::cache::Cache;
use omavision_core::config::{expand_home, Config};
use omavision_core::player::Outcome;
use omavision_core::theme::{Palette, Rgb};
use omavision_core::{enrich, tmdb, Index, Item, Meta};
use slint::platform::Key;
use slint::{ComponentHandle, ModelRc, VecModel, Weak};
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub config: Config,
    pub browser: Browser,
    pub cache: Option<Cache>,
    pub enriching: bool,
}

/// Everything a worker thread can tell the UI thread.
pub enum Msg {
    Scanned(anyhow::Result<Index>),
    Enriched { id: String, done: usize, total: usize, result: anyhow::Result<Meta> },
    Played(anyhow::Result<Outcome>),
    Theme(Palette),
}

/// The way back to the UI thread. Cheap to clone, safe to hand to a worker thread.
#[derive(Clone)]
pub struct Post {
    ui: Weak<MainWindow>,
    state: Arc<Mutex<AppState>>,
}

impl Post {
    pub fn new(ui: Weak<MainWindow>, state: Arc<Mutex<AppState>>) -> Post {
        Post { ui, state }
    }

    pub fn send(&self, msg: Msg) {
        let post = self.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(ui) = post.ui.upgrade() else { return };
            let state = post.state.clone();
            dispatch(&mut state.lock().unwrap(), &ui, &post, msg);
        });
    }

    pub fn state(&self) -> Arc<Mutex<AppState>> {
        self.state.clone()
    }
}

/// Every state transition in the app. Runs on the UI thread only.
pub fn dispatch(state: &mut AppState, ui: &MainWindow, post: &Post, msg: Msg) {
    match msg {
        Msg::Scanned(Ok(index)) => {
            if let Some(c) = &state.cache {
                if let Err(e) = c.save_index(&index) {
                    warn!("could not save index: {e:#}");
                }
            }
            let meta = state.cache.as_ref().map(|c| c.load_all_meta(&index)).unwrap_or_default();
            let n = index.item_count();
            state.browser.set_index(index, meta);
            ui.set_status(format!("{n} items · scan complete").into());
            start_enrichment(state, ui, post);
            render(ui, state);
        }
        Msg::Scanned(Err(e)) => {
            ui.set_status(format!("scan failed: {e:#}").into());
            render(ui, state);
        }
        Msg::Enriched { id, done, total, result } => {
            match result {
                Ok(meta) => state.browser.set_meta(id.clone(), meta),
                Err(e) => warn!("enrichment of {id}: {e:#}"),
            }
            let finished = done == total;
            if finished {
                state.enriching = false;
            }
            let status = if finished { format!("{} items · metadata complete", state.browser.item_count()) } else { format!("fetching metadata {done}/{total}") };
            ui.set_status(status.into());
            if state.browser.selected_id() == Some(id.as_str()) {
                if state.browser.is_in_show() {
                    render(ui, state);
                } else {
                    render_card(ui, state);
                }
            }
        }
        Msg::Played(outcome) => {
            ui.set_playing(false);
            ui.set_status(
                match outcome {
                    Ok(o) if o.player.success() => "player exited".to_string(),
                    Ok(o) => format!("player exited with {}", o.player),
                    Err(e) => format!("playback failed: {e:#}"),
                }
                .into(),
            );
            ui.window().show().ok();
        }
        Msg::Theme(palette) => apply_palette(ui, &palette),
    }
}

/// Keys the browser understands. Anything else belongs to Slint.
pub fn action_for(text: &str) -> Option<Action> {
    let c = text.chars().next()?;
    let is = |k: Key| c == char::from(k);
    Some(if is(Key::DownArrow) {
        Action::Down
    } else if is(Key::UpArrow) {
        Action::Up
    } else if is(Key::PageDown) {
        Action::Page(10)
    } else if is(Key::PageUp) {
        Action::Page(-10)
    } else if is(Key::Home) {
        Action::Home
    } else if is(Key::End) {
        Action::End
    } else if is(Key::RightArrow) {
        Action::NextTab
    } else if is(Key::LeftArrow) {
        Action::PrevTab
    } else if is(Key::Backspace) {
        Action::Backspace
    } else if is(Key::Escape) {
        Action::Clear
    } else if is(Key::Return) {
        Action::Activate
    } else if !c.is_control() && text.chars().count() == 1 {
        Action::Type(c)
    } else {
        return None;
    })
}

pub fn handle_key(ui: &MainWindow, post: &Post, text: &str) -> bool {
    if ui.get_playing() {
        return true;
    }
    let Some(action) = action_for(text) else { return false };
    let state = post.state();
    let mut state = state.lock().unwrap();
    match state.browser.apply(action) {
        Some(Effect::Unhandled) => return false,
        Some(Effect::Play(path)) => tasks::play(state.config.player.clone(), path, ui, post),
        None => {}
    }
    render(ui, &state);
    true
}

/// Fetches metadata for every item without it, one at a time on a worker thread.
fn start_enrichment(state: &mut AppState, ui: &MainWindow, post: &Post) {
    if state.enriching || state.cache.is_none() {
        return;
    }
    let todo: Vec<Item> = enrich::missing(state.browser.index(), state.browser.meta(), &state.config).into_iter().cloned().collect();
    if todo.is_empty() {
        return;
    }
    let Some(key) = tmdb::credential(&state.config) else {
        let file = expand_home(&state.config.metadata.api_key_file);
        ui.set_status(format!("{} · no TMDB key in env or {}", ui.get_status(), file.display()).into());
        return;
    };
    state.enriching = true;
    tasks::enrich(state.config.clone(), key, todo, post.clone());
}

pub fn render(ui: &MainWindow, state: &AppState) {
    let v = state.browser.view();
    let tabs: Vec<Tab> = v.tabs.iter().map(|t| Tab { id: t.id.clone().into(), label: t.label.to_uppercase().into(), count: t.count.to_string().into() }).collect();
    ui.set_tabs(ModelRc::new(VecModel::from(tabs)));
    ui.set_active_tab(v.active_tab as i32);
    let rows: Vec<Row> = v
        .rows
        .iter()
        .map(|r| Row {
            id: r.id.clone().into(),
            num: format!("{:03}", r.number).into(),
            title: r.title.clone().into(),
            year: r.year.map(|y| y.to_string()).unwrap_or_default().into(),
            is_show: r.is_show,
            note: match (r.is_show, r.heading) {
                (true, _) => format!("{} ep.", r.episodes),
                (_, true) => format!("{} episodes", r.episodes),
                _ => String::new(),
            }
            .into(),
            code: r.code.clone().into(),
            heading: r.heading,
        })
        .collect();
    ui.set_rows(ModelRc::new(VecModel::from(rows)));
    ui.set_selected(v.selected as i32);
    ui.set_filter(v.filter.clone().into());
    ui.set_crumb(v.crumb.clone().into());
    ui.set_count_label(if v.crumb.is_empty() { format!("{} / {}", v.shown, v.total) } else { format!("{} episodes", v.total) }.into());
    ui.invoke_ensure_visible();
    ui.set_card(to_card(&v.card, state.cache.as_ref()));
}

pub fn render_card(ui: &MainWindow, state: &AppState) {
    ui.set_card(to_card(&state.browser.card(), state.cache.as_ref()));
}

/// The only place where a card becomes Slint. Slint caches decoded images by path.
fn to_card(card: &browse::Card, cache: Option<&Cache>) -> Card {
    let poster = match (card.has_poster, cache) {
        (true, Some(c)) => slint::Image::load_from_path(&c.poster_path(&card.item_id)).ok(),
        _ => None,
    };
    Card {
        label: card.label.clone().into(),
        title: card.title.clone().into(),
        original: card.original.clone().into(),
        facts: card.facts.clone().into(),
        genres: card.genres.clone().into(),
        note: card.note.clone().into(),
        note_italic: card.note_italic,
        has_poster: poster.is_some(),
        poster: poster.unwrap_or_default(),
        episode: card.episode.clone().into(),
    }
}

pub fn apply_palette(ui: &MainWindow, p: &Palette) {
    let c = |r: Rgb| slint::Color::from_rgb_u8(r.0, r.1, r.2);
    let t = ui.global::<Theme>();
    t.set_bg(c(p.bg));
    t.set_surface(c(p.surface));
    t.set_text(c(p.text));
    t.set_text2(c(p.text2));
    t.set_muted(c(p.muted));
    t.set_accent(c(p.accent));
    t.set_accent2(c(p.accent2));
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::SharedString;

    #[test]
    fn replayed_keys_map_to_actions() {
        let left: SharedString = Key::LeftArrow.into();
        assert_eq!(action_for(&left), Some(Action::PrevTab));
        let ret: SharedString = Key::Return.into();
        assert_eq!(action_for(&ret), Some(Action::Activate));
        assert_eq!(action_for("a"), Some(Action::Type('a')));
    }
}
