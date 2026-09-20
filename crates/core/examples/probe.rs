//! Prints the search candidates and the pick for a title, the way enrichment sees them.
//! cargo run -p omavision-core --example probe -- "Dragons 3 - Le Monde caché" 2019
use omavision_core::config::Config;
use omavision_core::enrich::title_variants;
use omavision_core::tmdb::{self, Client, MediaKind};

fn main() {
    let mut args = std::env::args().skip(1);
    let title = args.next().expect("title");
    let year: Option<u16> = args.next().and_then(|y| y.parse().ok());
    let kind = if args.next().as_deref() == Some("tv") { MediaKind::Tv } else { MediaKind::Movie };
    let key = tmdb::credential(&Config::default()).expect("no TMDB key in env or key file");
    let client = Client::new(key);
    for v in title_variants(&title) {
        println!("query {v:?} -> encoded {}", tmdb::encode(&v));
        match client.search(kind, &v, "fr") {
            Ok(c) => {
                for x in &c {
                    println!("   id={} year={:?} votes={} pop={:.1}", x.id, x.year, x.votes, x.popularity);
                }
                println!("   pick: {:?}", tmdb::pick(&c, year));
                if let Some(id) = tmdb::pick(&c, year) {
                    for lang in ["fr", "en"] {
                        match client.details(kind, id, lang) {
                            Ok(m) => println!("   details[{lang}]: title={:?} overview_len={} genres={:?}", m.title, m.overview.len(), m.genres),
                            Err(e) => println!("   details[{lang}] error: {e:#}"),
                        }
                    }
                    break;
                }
            }
            Err(e) => println!("   error: {e:#}"),
        }
    }
}
