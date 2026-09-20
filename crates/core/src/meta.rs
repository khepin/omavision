//! What the app stores for an item. The provider fills the fields it knows; poster_file,
//! requested and episodes are the app's own bookkeeping.
use crate::tmdb::MediaKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Meta {
    pub tmdb_id: Option<u64>,
    pub kind: Option<MediaKind>,
    pub title: String,
    pub original_title: String,
    pub year: Option<u16>,
    pub overview: String,
    pub runtime: Option<u32>,
    pub rating: Option<f32>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    /// Poster file name inside the posters cache directory. Its presence means one was stored.
    pub poster_file: Option<String>,
    /// "S02E03" -> episode title.
    #[serde(default)]
    pub episodes: BTreeMap<String, String>,
    /// Language the text actually came in.
    pub language: String,
    /// First configured language at fetch time. A different setting now means a refetch.
    #[serde(default)]
    pub requested: String,
    pub fetched_at: u64,
}
