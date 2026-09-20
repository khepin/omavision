pub mod browse;
pub mod cache;
pub mod config;
pub mod enrich;
pub mod index;
pub mod meta;
pub mod parse;
pub mod player;
pub mod scan;
pub mod search;
pub mod theme;
pub mod tmdb;

pub use config::Config;
pub use index::{Category, Episode, Index, Item, Kind, Season};
pub use meta::Meta;

/// Seconds since the Unix epoch. The only clock in the app.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
