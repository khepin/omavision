//! The browse model: what is on screen, and what a key does to it. No UI types here; the
//! binary maps keys to `Action`, renders a `View` and runs the `Effect`.
use crate::index::{Index, Item, Kind};
use crate::meta::Meta;
use crate::search;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Down,
    Up,
    /// Lines to move, negative for up.
    Page(i32),
    Home,
    End,
    NextTab,
    PrevTab,
    Type(char),
    Backspace,
    Clear,
    Activate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Play(PathBuf),
    /// The action changed nothing and the key belongs to whoever asked next.
    Unhandled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    /// Position in the filtered list, from 1.
    pub number: usize,
    pub title: String,
    pub year: Option<u16>,
    pub is_show: bool,
    pub episodes: usize,
}

/// The right-hand card, already worded. `has_poster` means one was stored in the cache.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Card {
    pub item_id: String,
    pub label: String,
    pub title: String,
    pub original: String,
    pub facts: String,
    pub genres: String,
    pub note: String,
    pub note_italic: bool,
    pub has_poster: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub rows: Vec<Row>,
    pub selected: usize,
    pub filter: String,
    /// Rows shown out of the items in the active category.
    pub shown: usize,
    pub total: usize,
    pub card: Card,
}

pub struct Browser {
    root: PathBuf,
    index: Index,
    meta: HashMap<String, Meta>,
    /// 0 is the All tab when there is more than one category; the rest are categories in index order.
    active_tab: usize,
    selected: usize,
    filter: String,
    /// `(category, item)` indices into the index, after filtering.
    visible: Vec<(usize, usize)>,
}

/// The tab that searches every category at once. Only shown when there is something to merge.
pub const ALL_TAB_ID: &str = "all";

impl Browser {
    pub fn new(index: Index, meta: HashMap<String, Meta>, root: PathBuf) -> Browser {
        let mut b = Browser { root, index, meta, active_tab: 0, selected: 0, filter: String::new(), visible: vec![] };
        b.refilter();
        b
    }

    /// A fresh scan. The tab, the selection and the filter survive it.
    pub fn set_index(&mut self, index: Index, meta: HashMap<String, Meta>) {
        self.index = index;
        self.meta = meta;
        self.refilter();
    }

    pub fn set_meta(&mut self, item_id: String, meta: Meta) {
        self.meta.insert(item_id, meta);
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn meta(&self) -> &HashMap<String, Meta> {
        &self.meta
    }

    pub fn item_count(&self) -> usize {
        self.index.item_count()
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected_item().map(|it| it.id.as_str())
    }

    pub fn apply(&mut self, action: Action) -> Option<Effect> {
        let n = self.visible.len();
        match action {
            Action::Down => {
                if n > 0 {
                    self.selected = (self.selected + 1).min(n - 1);
                }
            }
            Action::Up => self.selected = self.selected.saturating_sub(1),
            Action::Page(by) if by < 0 => self.selected = self.selected.saturating_sub(by.unsigned_abs() as usize),
            Action::Page(by) => {
                if n > 0 {
                    self.selected = (self.selected + by as usize).min(n - 1);
                }
            }
            Action::Home => self.selected = 0,
            Action::End => self.selected = n.saturating_sub(1),
            Action::NextTab | Action::PrevTab => {
                let count = self.tab_count();
                if count > 0 {
                    self.active_tab = match action {
                        Action::NextTab => (self.active_tab + 1) % count,
                        _ => (self.active_tab + count - 1) % count,
                    };
                    self.selected = 0;
                    self.refilter();
                }
            }
            Action::Type(c) => {
                self.filter.push(c);
                self.refilter();
            }
            Action::Backspace => {
                self.filter.pop();
                self.refilter();
            }
            Action::Clear => {
                if self.filter.is_empty() {
                    return Some(Effect::Unhandled);
                }
                self.filter.clear();
                self.refilter();
            }
            Action::Activate => {
                return self.selected_item().and_then(playable_path).map(|p| Effect::Play(self.root.join(p)));
            }
        }
        None
    }

    pub fn view(&self) -> View {
        let mut tabs = Vec::with_capacity(self.tab_count());
        if self.has_all_tab() {
            tabs.push(Tab { id: ALL_TAB_ID.into(), label: "All".into(), count: self.index.item_count() });
        }
        tabs.extend(self.index.categories.iter().map(|c| Tab { id: c.id.clone(), label: c.label.clone(), count: c.items.len() }));
        View {
            tabs,
            active_tab: self.active_tab,
            rows: self
                .visible
                .iter()
                .enumerate()
                .map(|(n, &(c, i))| {
                    let it = &self.index.categories[c].items[i];
                    Row { id: it.id.clone(), number: n + 1, title: it.title.clone(), year: it.year, is_show: it.kind == Kind::Show, episodes: it.episode_count() }
                })
                .collect(),
            selected: self.selected,
            filter: self.filter.clone(),
            shown: self.visible.len(),
            total: self.items().len(),
            card: self.card(),
        }
    }

    pub fn card(&self) -> Card {
        let Some(it) = self.selected_item() else {
            let note = if self.filter.is_empty() { "Nothing here yet." } else { "No match." };
            return Card { note: note.into(), note_italic: true, ..Default::default() };
        };
        let category = self.visible.get(self.selected).map(|&(c, _)| self.index.categories[c].label.as_str()).unwrap_or("");
        card_for(it, self.meta.get(&it.id), category, self.selected + 1)
    }

    fn has_all_tab(&self) -> bool {
        self.index.categories.len() > 1
    }

    fn tab_count(&self) -> usize {
        self.index.categories.len() + usize::from(self.has_all_tab())
    }

    /// `(category, item)` for everything under the active tab, in index order.
    fn items(&self) -> Vec<(usize, usize)> {
        let all = self.has_all_tab();
        let categories = if all && self.active_tab == 0 {
            0..self.index.categories.len()
        } else {
            let c = self.active_tab - usize::from(all);
            c..c + 1
        };
        categories.filter_map(|c| self.index.categories.get(c).map(|cat| (c, cat.items.len()))).flat_map(|(c, n)| (0..n).map(move |i| (c, i))).collect()
    }

    fn selected_item(&self) -> Option<&Item> {
        self.visible.get(self.selected).map(|&(c, i)| &self.index.categories[c].items[i])
    }

    fn refilter(&mut self) {
        let q = search::fold(&self.filter);
        self.visible = self.items().into_iter().filter(|&(c, i)| q.is_empty() || search::matches(&self.index.categories[c].items[i], self.meta.get(&self.index.categories[c].items[i].id), &q)).collect();
        if self.selected >= self.visible.len() {
            self.selected = self.visible.len().saturating_sub(1);
        }
    }
}

/// What Enter plays: the item's own file, or the first episode of a show.
fn playable_path(item: &Item) -> Option<&str> {
    item.path.as_deref().or_else(|| item.seasons.first().and_then(|s| s.episodes.first()).map(|e| e.path.as_str()))
}

/// The card for one item, with or without metadata. The only place that words it.
pub fn card_for(item: &Item, meta: Option<&Meta>, category: &str, number: usize) -> Card {
    let found = meta.filter(|m| m.tmdb_id.is_some());
    let mut facts: Vec<String> = Vec::new();
    if let Some(y) = found.and_then(|m| m.year).or(item.year) {
        facts.push(y.to_string());
    }
    if item.kind == Kind::Show {
        facts.push(format!("{} seasons · {} episodes", item.seasons.len(), item.episode_count()));
    }
    if let Some(m) = found {
        if let Some(r) = m.runtime {
            facts.push(format!("{r} min"));
        }
        if let Some(r) = m.rating {
            facts.push(format!("★ {r:.1}"));
        }
    }
    let (note, note_italic) = match found {
        Some(m) if !m.overview.is_empty() => (m.overview.clone(), false),
        Some(_) => ("No synopsis available.".to_string(), true),
        None if meta.is_some() => ("No match on TMDB.".to_string(), true),
        None => ("Metadata pending".to_string(), true),
    };
    Card {
        item_id: item.id.clone(),
        label: format!("Card · {category} No. {number:03}"),
        title: found.map(|m| m.title.clone()).unwrap_or_else(|| item.title.clone()),
        original: found.filter(|m| m.original_title != m.title && !m.original_title.is_empty()).map(|m| m.original_title.clone()).unwrap_or_default(),
        facts: facts.join(" · "),
        genres: found.map(|m| m.genres.join(" / ")).unwrap_or_default(),
        note,
        note_italic,
        has_poster: meta.and_then(|m| m.poster_file.as_deref()).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Category, Episode, Season};

    fn movie(cat: &str, title: &str, year: u16) -> Item {
        let path = format!("{cat}/{title} ({year}).mkv");
        Item { id: path.clone(), kind: Kind::Movie, title: title.into(), year: Some(year), path: Some(path), seasons: vec![] }
    }

    fn show(cat: &str, title: &str) -> Item {
        let episodes = vec![Episode { season: 1, episode: 1, path: format!("{cat}/{title}/S01E01.mkv"), title: None }];
        Item { id: format!("{cat}/{title}"), kind: Kind::Show, title: title.into(), year: None, path: None, seasons: vec![Season { number: 1, episodes }] }
    }

    fn library() -> Index {
        Index {
            scanned_at: 0,
            categories: vec![
                Category {
                    id: "films".into(),
                    label: "Films".into(),
                    items: vec![
                        movie("films", "John Wick", 2014),
                        movie("films", "John Wick - Chapter 2", 2017),
                        movie("films", "John Wick - Chapter 3", 2019),
                        movie("films", "Ponyo", 2008),
                    ],
                },
                Category { id: "series".into(), label: "Series".into(), items: vec![show("series", "Friends")] },
            ],
        }
    }

    fn browser() -> Browser {
        Browser::new(library(), HashMap::new(), PathBuf::from("/lib"))
    }

    fn type_in(b: &mut Browser, text: &str) {
        for c in text.chars() {
            b.apply(Action::Type(c));
        }
    }

    #[test]
    fn typing_then_moving_down_plays_the_second_match() {
        let mut b = browser();
        type_in(&mut b, "wick");
        assert_eq!(b.view().rows.len(), 3);
        assert_eq!(b.apply(Action::Down), None);
        assert_eq!(b.apply(Action::Activate), Some(Effect::Play(PathBuf::from("/lib/films/John Wick - Chapter 2 (2017).mkv"))));
    }

    #[test]
    fn activating_a_show_plays_its_first_episode() {
        let mut b = browser();
        b.apply(Action::NextTab);
        b.apply(Action::NextTab);
        assert_eq!(b.apply(Action::Activate), Some(Effect::Play(PathBuf::from("/lib/series/Friends/S01E01.mkv"))));
    }

    #[test]
    fn the_all_tab_comes_first_and_merges_every_category() {
        let b = browser();
        let v = b.view();
        assert_eq!(v.tabs.iter().map(|t| (t.id.as_str(), t.count)).collect::<Vec<_>>(), vec![("all", 5), ("films", 4), ("series", 1)]);
        assert_eq!(v.active_tab, 0);
        assert_eq!(v.rows.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), vec!["John Wick", "John Wick - Chapter 2", "John Wick - Chapter 3", "Ponyo", "Friends"]);
        assert_eq!(v.rows.iter().map(|r| r.number).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
        assert_eq!((v.shown, v.total), (5, 5));
    }

    #[test]
    fn the_all_tab_card_names_the_item_own_category() {
        let mut b = browser();
        b.apply(Action::End);
        assert_eq!(b.card().label, "Card · Series No. 005");
        assert_eq!(b.apply(Action::Activate), Some(Effect::Play(PathBuf::from("/lib/series/Friends/S01E01.mkv"))));
    }

    #[test]
    fn a_single_category_has_no_all_tab() {
        let mut index = library();
        index.categories.truncate(1);
        let b = Browser::new(index, HashMap::new(), PathBuf::from("/lib"));
        assert_eq!(b.view().tabs.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["films"]);
        assert_eq!(b.view().rows.len(), 4);
    }

    #[test]
    fn tabs_wrap_both_ways() {
        let mut b = browser();
        b.apply(Action::NextTab);
        assert_eq!(b.view().active_tab, 1);
        b.apply(Action::NextTab);
        assert_eq!(b.view().active_tab, 2);
        b.apply(Action::NextTab);
        assert_eq!(b.view().active_tab, 0);
        b.apply(Action::PrevTab);
        assert_eq!(b.view().active_tab, 2);
    }

    #[test]
    fn the_filter_survives_a_tab_switch() {
        let mut b = browser();
        b.apply(Action::NextTab);
        type_in(&mut b, "friends");
        assert!(b.view().rows.is_empty());
        b.apply(Action::NextTab);
        let v = b.view();
        assert_eq!(v.filter, "friends");
        assert_eq!(v.rows.len(), 1);
        assert_eq!(v.shown, 1);
        assert_eq!(v.total, 1);
    }

    #[test]
    fn escape_on_an_empty_filter_is_not_handled() {
        let mut b = browser();
        assert_eq!(b.apply(Action::Clear), Some(Effect::Unhandled));
        type_in(&mut b, "po");
        assert_eq!(b.apply(Action::Clear), None);
        assert_eq!(b.view().filter, "");
        assert_eq!(b.view().rows.len(), 5);
    }

    #[test]
    fn moving_clamps_and_filtering_pulls_the_selection_back() {
        let mut b = browser();
        b.apply(Action::NextTab);
        b.apply(Action::End);
        assert_eq!(b.view().selected, 3);
        b.apply(Action::Down);
        assert_eq!(b.view().selected, 3);
        b.apply(Action::Page(10));
        assert_eq!(b.view().selected, 3);
        type_in(&mut b, "ponyo");
        assert_eq!(b.view().selected, 0);
        b.apply(Action::Backspace);
        b.apply(Action::Page(-10));
        assert_eq!(b.view().selected, 0);
        assert_eq!(b.apply(Action::Up), None);
    }

    #[test]
    fn the_card_words_an_item_with_and_without_metadata() {
        let mut b = browser();
        assert_eq!(b.card().note, "Metadata pending");
        assert_eq!(b.card().label, "Card · Films No. 001");
        assert_eq!(b.card().facts, "2014");
        let meta = Meta { tmdb_id: Some(1), title: "John Wick".into(), original_title: "John Wick".into(), year: Some(2014), overview: "A hitman.".into(), runtime: Some(101), rating: Some(7.4), genres: vec!["Action".into()], poster_file: Some("x.jpg".into()), ..Default::default() };
        b.set_meta("films/John Wick (2014).mkv".into(), meta);
        let c = b.card();
        assert_eq!(c.facts, "2014 · 101 min · ★ 7.4");
        assert_eq!(c.genres, "Action");
        assert_eq!(c.original, "");
        assert_eq!(c.note, "A hitman.");
        assert!(!c.note_italic);
        assert!(c.has_poster);
    }

    #[test]
    fn an_empty_list_still_has_a_card() {
        let mut b = Browser::new(Index::default(), HashMap::new(), PathBuf::from("/lib"));
        assert_eq!(b.card().note, "Nothing here yet.");
        assert_eq!(b.apply(Action::Activate), None);
        let mut b2 = browser();
        type_in(&mut b2, "zzz");
        assert_eq!(b2.card().note, "No match.");
        assert!(b2.card().item_id.is_empty());
    }
}
