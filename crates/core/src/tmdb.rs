//! Thin TMDB v3 client. Only what enrichment needs.
use crate::config::{expand_home, Config};
use crate::meta::Meta;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const ATTRIBUTION: &str = "This product uses the TMDb API but is not endorsed or certified by TMDb.";
const BASE: &str = "https://api.themoviedb.org/3";
const IMAGE: &str = "https://image.tmdb.org/t/p/w500";

#[derive(Debug, Clone)]
pub struct Client {
    key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Movie,
    Tv,
}

/// The TMDB credential: `TMDB_API_READ_ACCESS_TOKEN`, then `TMDB_API_KEY` from the
/// environment, then the key file. Empty values count as absent.
pub fn credential(config: &Config) -> Option<String> {
    let non_empty = |s: String| { let s = s.trim().to_string(); if s.is_empty() { None } else { Some(s) } };
    std::env::var("TMDB_API_READ_ACCESS_TOKEN").ok().and_then(non_empty)
        .or_else(|| std::env::var("TMDB_API_KEY").ok().and_then(non_empty))
        .or_else(|| std::fs::read_to_string(expand_home(&config.metadata.api_key_file)).ok().and_then(non_empty))
}

impl Client {
    pub fn new(key: impl Into<String>) -> Client {
        Client { key: key.into() }
    }

    fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value> {
        // Query strings are encoded here so non-ASCII titles reach TMDB as UTF-8.
        let mut url = format!("{BASE}{path}?");
        // A v4 read access token is a JWT; a v3 key is a short hex string.
        let bearer = self.key.starts_with("eyJ");
        let mut pairs: Vec<(&str, &str)> = params.to_vec();
        if !bearer {
            pairs.push(("api_key", &self.key));
        }
        url.push_str(&pairs.iter().map(|(k, v)| format!("{k}={}", encode(v))).collect::<Vec<_>>().join("&"));
        let mut req = ureq::get(&url);
        if bearer {
            req = req.header("Authorization", &format!("Bearer {}", self.key));
        }
        let mut resp = req.call().with_context(|| format!("GET {path}"))?;
        let v: Value = resp.body_mut().read_json().with_context(|| format!("decoding {path}"))?;
        Ok(v)
    }

    /// Search candidates for a title. The year is not sent as a filter; `pick` uses it to rank.
    pub fn search(&self, kind: MediaKind, title: &str, language: &str) -> Result<Vec<Candidate>> {
        let params = [("query", title), ("language", language), ("include_adult", "false")];
        let path = match kind { MediaKind::Movie => "/search/movie", MediaKind::Tv => "/search/tv" };
        let v = self.get(path, &params)?;
        Ok(candidates(&v))
    }

    /// Details with credits and keywords appended, so one call fills everything search reads.
    pub fn details(&self, kind: MediaKind, id: u64, language: &str) -> Result<Meta> {
        let path = match kind { MediaKind::Movie => format!("/movie/{id}"), MediaKind::Tv => format!("/tv/{id}") };
        let v = self.get(&path, &[("language", language), ("append_to_response", "credits,keywords")])?;
        Ok(meta_from_details(kind, &v, language))
    }

    /// Episode titles for one season.
    pub fn season_episodes(&self, id: u64, season: u16, language: &str) -> Result<BTreeMap<String, String>> {
        let v = self.get(&format!("/tv/{id}/season/{season}"), &[("language", language)])?;
        Ok(episodes_from_season(&v))
    }

    pub fn download(&self, url: &str) -> Result<Vec<u8>> {
        let mut resp = ureq::get(url).call().with_context(|| format!("GET {url}"))?;
        let bytes = resp.body_mut().read_to_vec().context("reading poster")?;
        if bytes.is_empty() {
            bail!("empty poster body");
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub id: u64,
    pub year: Option<u16>,
    pub votes: u64,
    pub popularity: f64,
}

pub fn candidates(search: &Value) -> Vec<Candidate> {
    search
        .get("results")
        .and_then(|r| r.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|r| {
                    let date = r.get("release_date").or_else(|| r.get("first_air_date")).and_then(|d| d.as_str()).unwrap_or("");
                    Some(Candidate {
                        id: r.get("id")?.as_u64()?,
                        year: date.get(..4).and_then(|y| y.parse().ok()),
                        votes: r.get("vote_count").and_then(|v| v.as_u64()).unwrap_or(0),
                        popularity: r.get("popularity").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The best candidate: among those within a year of the file's year when any are, the one with
/// the most votes. Featurettes and making-ofs share the year but never the votes.
pub fn pick(cands: &[Candidate], year: Option<u16>) -> Option<u64> {
    let near: Vec<&Candidate> = match year {
        Some(y) => cands.iter().filter(|c| c.year.map(|cy| (cy as i32 - y as i32).abs() <= 1).unwrap_or(false)).collect(),
        None => vec![],
    };
    let pool: Vec<&Candidate> = if near.is_empty() { cands.iter().collect() } else { near };
    pool.into_iter().max_by(|a, b| a.votes.cmp(&b.votes).then(a.popularity.total_cmp(&b.popularity))).map(|c| c.id)
}

/// Percent-encodes a query value as UTF-8, keeping unreserved characters.
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The leading cast is capped so a long ensemble does not drown the search haystack.
const CAST_LIMIT: usize = 5;

pub fn meta_from_details(kind: MediaKind, v: &Value, language: &str) -> Meta {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let names = |v: Option<&Value>| -> Vec<String> {
        v.and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|e| e.get("name")?.as_str().map(String::from)).collect()).unwrap_or_default()
    };
    let (title, original, date) = match kind {
        MediaKind::Movie => (s("title"), s("original_title"), s("release_date")),
        MediaKind::Tv => (s("name"), s("original_name"), s("first_air_date")),
    };
    let runtime = match kind {
        MediaKind::Movie => v.get("runtime").and_then(|x| x.as_u64()).map(|x| x as u32),
        MediaKind::Tv => v.get("episode_run_time").and_then(|x| x.as_array()).and_then(|a| a.first()).and_then(|x| x.as_u64()).map(|x| x as u32),
    };
    Meta {
        tmdb_id: v.get("id").and_then(|x| x.as_u64()),
        kind: Some(kind),
        title,
        original_title: original,
        year: date.get(..4).and_then(|y| y.parse().ok()),
        overview: s("overview"),
        runtime: runtime.filter(|r| *r > 0),
        rating: v.get("vote_average").and_then(|x| x.as_f64()).map(|x| x as f32).filter(|r| *r > 0.0),
        genres: names(v.get("genres")),
        companies: [names(v.get("production_companies")), names(v.get("networks"))].concat(),
        people: {
            let credits = v.get("credits");
            let crew = credits.and_then(|c| c.get("crew")).and_then(|x| x.as_array());
            let directors: Vec<String> = crew.map(|a| a.iter().filter(|e| e.get("job").and_then(|j| j.as_str()) == Some("Director")).filter_map(|e| e.get("name")?.as_str().map(String::from)).collect()).unwrap_or_default();
            let cast: Vec<String> = names(credits.and_then(|c| c.get("cast"))).into_iter().take(CAST_LIMIT).collect();
            let mut people = [names(v.get("created_by")), directors, cast].concat();
            people.dedup();
            people
        },
        keywords: names(v.get("keywords").and_then(|k| k.get("keywords").or_else(|| k.get("results")))),
        poster_url: v.get("poster_path").and_then(|p| p.as_str()).map(|p| format!("{IMAGE}{p}")),
        language: language.to_string(),
        fetched_at: crate::now(),
        ..Default::default()
    }
}

pub fn episodes_from_season(v: &Value) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let season = v.get("season_number").and_then(|x| x.as_u64()).unwrap_or(0);
    if let Some(eps) = v.get("episodes").and_then(|e| e.as_array()) {
        for e in eps {
            let n = e.get("episode_number").and_then(|x| x.as_u64()).unwrap_or(0);
            let name = e.get("name").and_then(|x| x.as_str()).unwrap_or("");
            if !name.is_empty() {
                m.insert(episode_key(season as u16, n as u16), name.to_string());
            }
        }
    }
    m
}

pub fn episode_key(season: u16, episode: u16) -> String {
    format!("S{season:02}E{episode:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_movie_details() {
        let v = json!({"id": 129, "title": "Le Voyage de Chihiro", "original_title": "千と千尋の神隠し", "release_date": "2001-07-20",
            "overview": "Chihiro...", "runtime": 125, "vote_average": 8.5, "genres": [{"name": "Animation"}, {"name": "Familial"}], "poster_path": "/abc.jpg",
            "production_companies": [{"name": "Studio Ghibli"}],
            "credits": {"cast": [{"name": "Rumi Hiiragi"}, {"name": "Miyu Irino"}], "crew": [{"name": "Hayao Miyazaki", "job": "Director"}, {"name": "Joe Hisaishi", "job": "Original Music Composer"}]},
            "keywords": {"keywords": [{"name": "witch"}, {"name": "anime"}]}});
        let m = meta_from_details(MediaKind::Movie, &v, "fr");
        assert_eq!(m.tmdb_id, Some(129));
        assert_eq!(m.title, "Le Voyage de Chihiro");
        assert_eq!(m.year, Some(2001));
        assert_eq!(m.runtime, Some(125));
        assert_eq!(m.rating, Some(8.5));
        assert_eq!(m.genres, vec!["Animation", "Familial"]);
        assert_eq!(m.companies, vec!["Studio Ghibli"]);
        assert_eq!(m.people, vec!["Hayao Miyazaki", "Rumi Hiiragi", "Miyu Irino"]);
        assert_eq!(m.keywords, vec!["witch", "anime"]);
        assert_eq!(m.poster_url.as_deref(), Some("https://image.tmdb.org/t/p/w500/abc.jpg"));
    }

    #[test]
    fn parses_tv_details_and_season() {
        let v = json!({"id": 1668, "name": "Friends", "original_name": "Friends", "first_air_date": "1994-09-22", "overview": "", "episode_run_time": [22], "genres": [],
            "networks": [{"name": "NBC"}], "created_by": [{"name": "David Crane"}], "keywords": {"results": [{"name": "sitcom"}]}});
        let m = meta_from_details(MediaKind::Tv, &v, "fr");
        assert_eq!(m.runtime, Some(22));
        assert_eq!(m.year, Some(1994));
        assert_eq!(m.companies, vec!["NBC"]);
        assert_eq!(m.people, vec!["David Crane"]);
        assert_eq!(m.keywords, vec!["sitcom"]);
        let s = json!({"season_number": 2, "episodes": [{"episode_number": 3, "name": "Celui qui a une bosse"}, {"episode_number": 4, "name": ""}]});
        let eps = episodes_from_season(&s);
        assert_eq!(eps.get("S02E03").map(String::as_str), Some("Celui qui a une bosse"));
        assert!(!eps.contains_key("S02E04"));
    }

    #[test]
    fn picks_the_voted_film_over_the_same_year_featurette() {
        let v = json!({"results": [
            {"id": 729191, "release_date": "2002-07-11", "vote_count": 2, "popularity": 1.1},
            {"id": 15370, "release_date": "2002-07-19", "vote_count": 2535, "popularity": 10.6},
            {"id": 348909, "release_date": "1992-06-17", "vote_count": 16, "popularity": 2.2}]});
        let c = candidates(&v);
        assert_eq!(pick(&c, Some(2002)), Some(15370));
        assert_eq!(pick(&c, Some(1992)), Some(348909));
        assert_eq!(pick(&c, None), Some(15370));
        assert_eq!(pick(&[], Some(2002)), None);
    }

    #[test]
    fn year_off_by_one_still_counts() {
        let c = vec![Candidate { id: 1, year: Some(2003), votes: 10, popularity: 1.0 }, Candidate { id: 2, year: Some(1990), votes: 999, popularity: 9.0 }];
        assert_eq!(pick(&c, Some(2002)), Some(1));
    }

    #[test]
    fn encodes_utf8_queries() {
        assert_eq!(encode("L'Âge de glace"), "L%27%C3%82ge%20de%20glace");
        assert_eq!(encode("John Wick - Chapter 2"), "John%20Wick%20-%20Chapter%202");
    }
}
