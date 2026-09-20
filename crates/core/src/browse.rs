//! The browse model: what is on screen, and what a key does to it. No UI types here; the
//! binary maps keys to `Action`, renders a `View` and runs the `Effect`.
use crate::index::{Episode, Index, Item, Kind};
use crate::meta::Meta;
use crate::search;
use crate::tmdb::episode_key;
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
    /// Next tab in the list, next season inside a show.
    NextTab,
    PrevTab,
    Type(char),
    /// Edits the filter in the list, leaves an open show.
    Backspace,
    /// Clears the filter in the list, leaves an open show.
    Clear,
    /// Plays a movie, flat item or episode; opens a show.
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

/// One line of the list: an item, or inside a show a season heading or an episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    /// Position in the list, from 1. 0 for a heading.
    pub number: usize,
    pub title: String,
    pub year: Option<u16>,
    pub is_show: bool,
    pub episodes: usize,
    /// `S02 · E03` for an episode line.
    pub code: String,
    /// A season heading: not selectable, `title` is the heading and `episodes` its count.
    pub heading: bool,
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
    /// `S02 · E03 · The One Where…` for the selected episode of an open show.
    pub episode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub rows: Vec<Row>,
    /// Index into `rows`.
    pub selected: usize,
    pub filter: String,
    /// Rows shown out of the items in the active category; episodes when a show is open.
    pub shown: usize,
    pub total: usize,
    /// Title of the open show, empty in the list.
    pub crumb: String,
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
    /// The show drilled into, if any. The list selection and filter wait underneath it.
    open: Option<Open>,
}

struct Open {
    id: String,
    /// Index into the show's episodes, seasons flattened in order.
    selected: usize,
}

/// The tab that searches every category at once. Only shown when there is something to merge.
pub const ALL_TAB_ID: &str = "all";

impl Browser {
    pub fn new(index: Index, meta: HashMap<String, Meta>, root: PathBuf) -> Browser {
        let mut b = Browser { root, index, meta, active_tab: 0, selected: 0, filter: String::new(), visible: vec![], open: None };
        b.refilter();
        b
    }

    /// A fresh scan. The tab, the selection, the filter and an open show survive it, unless
    /// the show is gone.
    pub fn set_index(&mut self, index: Index, meta: HashMap<String, Meta>) {
        self.index = index;
        self.meta = meta;
        self.refilter();
        if let Some(open) = &mut self.open {
            match self.index.categories.iter().flat_map(|c| &c.items).find(|it| it.id == open.id) {
                Some(show) => open.selected = open.selected.min(show.episode_count().saturating_sub(1)),
                None => self.open = None,
            }
        }
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

    /// The item the card is about: the open show, else the selected line.
    pub fn selected_id(&self) -> Option<&str> {
        self.open.as_ref().map(|o| o.id.as_str()).or_else(|| self.selected_item().map(|it| it.id.as_str()))
    }

    pub fn is_in_show(&self) -> bool {
        self.open.is_some()
    }

    pub fn apply(&mut self, action: Action) -> Option<Effect> {
        if self.open.is_some() {
            return self.apply_in_show(action);
        }
        let n = self.visible.len();
        if let Some(to) = moved(self.selected, n, action) {
            self.selected = to;
            return None;
        }
        match action {
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
                let it = self.selected_item()?;
                if it.kind == Kind::Show {
                    self.open = Some(Open { id: it.id.clone(), selected: 0 });
                } else if let Some(p) = &it.path {
                    return Some(Effect::Play(self.root.join(p)));
                }
            }
            Action::Down | Action::Up | Action::Page(_) | Action::Home | Action::End => {}
        }
        None
    }

    fn apply_in_show(&mut self, action: Action) -> Option<Effect> {
        let show = self.open_show()?;
        let episodes: Vec<&Episode> = show.seasons.iter().flat_map(|s| &s.episodes).collect();
        let n = episodes.len();
        let selected = self.open.as_ref()?.selected;
        let to = match action {
            Action::NextTab | Action::PrevTab => {
                let season = episodes.get(selected).map(|e| e.season)?;
                let seasons: Vec<u16> = show.seasons.iter().map(|s| s.number).collect();
                let at = seasons.iter().position(|&s| s == season)?;
                let next = if action == Action::NextTab { at.checked_add(1).filter(|&i| i < seasons.len()) } else { at.checked_sub(1) };
                next.and_then(|i| episodes.iter().position(|e| e.season == seasons[i]))
            }
            Action::Backspace | Action::Clear => {
                self.open = None;
                return None;
            }
            Action::Activate => return episodes.get(selected).map(|e| Effect::Play(self.root.join(&e.path))),
            Action::Type(_) => None,
            _ => moved(selected, n, action),
        };
        if let (Some(to), Some(open)) = (to, &mut self.open) {
            open.selected = to;
        }
        None
    }

    fn open_show(&self) -> Option<&Item> {
        let id = &self.open.as_ref()?.id;
        self.index.categories.iter().flat_map(|c| &c.items).find(|it| &it.id == id)
    }

    pub fn view(&self) -> View {
        let mut tabs = Vec::with_capacity(self.tab_count());
        if self.has_all_tab() {
            tabs.push(Tab { id: ALL_TAB_ID.into(), label: "All".into(), count: self.index.item_count() });
        }
        tabs.extend(self.index.categories.iter().map(|c| Tab { id: c.id.clone(), label: c.label.clone(), count: c.items.len() }));
        let mut view = View {
            tabs,
            active_tab: self.active_tab,
            rows: vec![],
            selected: self.selected,
            filter: self.filter.clone(),
            shown: self.visible.len(),
            total: self.items().len(),
            crumb: String::new(),
            card: self.card(),
        };
        match (self.open_show(), &self.open) {
            (Some(show), Some(open)) => {
                let titles = self.meta.get(&show.id).map(|m| &m.episodes);
                let mut number = 0;
                for s in &show.seasons {
                    view.rows.push(Row { id: format!("{}/S{:02}", show.id, s.number), title: format!("Season {}", s.number), episodes: s.episodes.len(), heading: true, ..Row::empty() });
                    for e in &s.episodes {
                        if number == open.selected {
                            view.selected = view.rows.len();
                        }
                        number += 1;
                        let key = episode_key(e.season, e.episode);
                        let title = titles.and_then(|t| t.get(&key)).cloned().unwrap_or_default();
                        view.rows.push(Row { id: e.path.clone(), number, title, code: episode_code(e), ..Row::empty() });
                    }
                }
                view.shown = number;
                view.total = number;
                view.crumb = show.title.clone();
            }
            _ => {
                view.rows = self
                    .visible
                    .iter()
                    .enumerate()
                    .map(|(n, &(c, i))| {
                        let it = &self.index.categories[c].items[i];
                        Row { id: it.id.clone(), number: n + 1, title: it.title.clone(), year: it.year, is_show: it.kind == Kind::Show, episodes: it.episode_count(), ..Row::empty() }
                    })
                    .collect();
            }
        }
        view
    }

    pub fn card(&self) -> Card {
        let Some(it) = self.selected_item() else {
            let note = if self.filter.is_empty() { "Nothing here yet." } else { "No match." };
            return Card { note: note.into(), note_italic: true, ..Default::default() };
        };
        let category = self.visible.get(self.selected).map(|&(c, _)| self.index.categories[c].label.as_str()).unwrap_or("");
        let mut card = card_for(it, self.meta.get(&it.id), category, self.selected + 1);
        if let (Some(show), Some(open)) = (self.open_show(), &self.open) {
            if let Some(e) = show.seasons.iter().flat_map(|s| &s.episodes).nth(open.selected) {
                let title = self.meta.get(&show.id).and_then(|m| m.episodes.get(&episode_key(e.season, e.episode)));
                card.episode = match title {
                    Some(t) => format!("{} · {t}", episode_code(e)),
                    None => episode_code(e),
                };
            }
        }
        card
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

    /// The open show, else the selected line of the list.
    fn selected_item(&self) -> Option<&Item> {
        self.open_show().or_else(|| self.visible.get(self.selected).map(|&(c, i)| &self.index.categories[c].items[i]))
    }

    fn refilter(&mut self) {
        let q = search::fold(&self.filter);
        self.visible = self.items().into_iter().filter(|&(c, i)| q.is_empty() || search::matches(&self.index.categories[c].items[i], self.meta.get(&self.index.categories[c].items[i].id), &q)).collect();
        if self.selected >= self.visible.len() {
            self.selected = self.visible.len().saturating_sub(1);
        }
    }
}

/// Where a movement action lands in a list of `n` lines. None when the action is not a movement.
fn moved(selected: usize, n: usize, action: Action) -> Option<usize> {
    let last = n.saturating_sub(1);
    Some(match action {
        Action::Down => (selected + 1).min(last),
        Action::Up => selected.saturating_sub(1),
        Action::Page(by) if by < 0 => selected.saturating_sub(by.unsigned_abs() as usize),
        Action::Page(by) => (selected + by as usize).min(last),
        Action::Home => 0,
        Action::End => last,
        _ => return None,
    })
}

fn episode_code(e: &Episode) -> String {
    format!("S{:02} · E{:02}", e.season, e.episode)
}

impl Row {
    fn empty() -> Row {
        Row { id: String::new(), number: 0, title: String::new(), year: None, is_show: false, episodes: 0, code: String::new(), heading: false }
    }
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
        episode: String::new(),
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
        let ep = |season, episode| Episode { season, episode, path: format!("{cat}/{title}/S{season:02}E{episode:02}.mkv"), title: None };
        let seasons = vec![Season { number: 1, episodes: vec![ep(1, 1), ep(1, 2)] }, Season { number: 2, episodes: vec![ep(2, 1)] }];
        Item { id: format!("{cat}/{title}"), kind: Kind::Show, title: title.into(), year: None, path: None, seasons }
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
    fn activating_a_show_opens_it_on_its_first_episode() {
        let mut b = browser();
        b.apply(Action::NextTab);
        b.apply(Action::NextTab);
        assert_eq!(b.apply(Action::Activate), None);
        let v = b.view();
        assert_eq!(v.crumb, "Friends");
        assert_eq!(v.rows.iter().map(|r| (r.heading, r.number, r.code.as_str())).collect::<Vec<_>>(), vec![(true, 0, ""), (false, 1, "S01 · E01"), (false, 2, "S01 · E02"), (true, 0, ""), (false, 3, "S02 · E01")]);
        assert_eq!((v.rows[0].title.as_str(), v.rows[0].episodes), ("Season 1", 2));
        assert_eq!(v.selected, 1);
        assert_eq!((v.shown, v.total), (3, 3));
        assert_eq!(v.card.title, "Friends");
        assert_eq!(v.card.facts, "2 seasons · 3 episodes");
        assert_eq!(v.card.episode, "S01 · E01");
        assert_eq!(b.apply(Action::Activate), Some(Effect::Play(PathBuf::from("/lib/series/Friends/S01E01.mkv"))));
    }

    #[test]
    fn inside_a_show_left_and_right_jump_seasons_and_movement_skips_headings() {
        let mut b = browser();
        b.apply(Action::End);
        b.apply(Action::Activate);
        b.apply(Action::NextTab);
        assert_eq!(b.view().selected, 4);
        assert_eq!(b.apply(Action::Activate), Some(Effect::Play(PathBuf::from("/lib/series/Friends/S02E01.mkv"))));
        b.apply(Action::NextTab);
        assert_eq!(b.view().selected, 4);
        b.apply(Action::PrevTab);
        assert_eq!(b.view().selected, 1);
        b.apply(Action::Down);
        assert_eq!(b.view().selected, 2);
        b.apply(Action::Down);
        assert_eq!(b.view().selected, 4);
        b.apply(Action::Home);
        assert_eq!(b.view().selected, 1);
        b.apply(Action::Page(10));
        assert_eq!(b.view().selected, 4);
    }

    #[test]
    fn leaving_a_show_returns_to_the_same_line_and_typing_inside_is_ignored() {
        let mut b = browser();
        type_in(&mut b, "fri");
        b.apply(Action::Activate);
        assert_eq!(b.apply(Action::Type('x')), None);
        assert_eq!(b.view().filter, "fri");
        assert_eq!(b.apply(Action::Clear), None);
        let v = b.view();
        assert_eq!(v.crumb, "");
        assert_eq!(v.filter, "fri");
        assert_eq!(v.rows.len(), 1);
        assert_eq!(v.card.episode, "");
        b.apply(Action::Activate);
        b.apply(Action::Backspace);
        assert_eq!(b.view().crumb, "");
        assert_eq!(b.view().filter, "fri");
    }

    #[test]
    fn episode_titles_come_from_metadata() {
        let mut b = browser();
        let mut episodes = std::collections::BTreeMap::new();
        episodes.insert("S01E02".to_string(), "The One with the Sonogram".to_string());
        b.set_meta("series/Friends".into(), Meta { tmdb_id: Some(1), title: "Friends".into(), episodes, ..Default::default() });
        b.apply(Action::End);
        b.apply(Action::Activate);
        b.apply(Action::Down);
        assert_eq!(b.selected_id(), Some("series/Friends"));
        let v = b.view();
        assert_eq!(v.rows[2].title, "The One with the Sonogram");
        assert_eq!(v.rows[1].title, "");
        assert_eq!(v.card.episode, "S01 · E02 · The One with the Sonogram");
    }

    #[test]
    fn a_rescan_keeps_an_open_show_unless_it_is_gone() {
        let mut b = browser();
        b.apply(Action::End);
        b.apply(Action::Activate);
        b.apply(Action::End);
        b.set_index(library(), HashMap::new());
        assert_eq!(b.view().crumb, "Friends");
        assert_eq!(b.view().selected, 4);
        let mut index = library();
        index.categories.truncate(1);
        b.set_index(index, HashMap::new());
        assert_eq!(b.view().crumb, "");
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
        assert_eq!(b.apply(Action::Activate), None);
        assert_eq!(b.card().label, "Card · Series No. 005");
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
