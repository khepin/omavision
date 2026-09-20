//! Name parsing: the only source of identity for an item.
use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Movie { title: String, year: u16 },
    Episode { show: String, season: u16, episode: u16 },
    Flat { title: String },
}

static EPISODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?P<show>.+?)[\s.\-_]+S(?P<s>\d{1,2})[\s.]?E(?P<e>\d{1,3})").unwrap());
static PAREN_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<title>.+?)\s*\((?P<year>(?:19|20)\d{2})\)").unwrap());
static SCENE_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<title>.+?)[\s.]+(?P<year>(?:19|20)\d{2})(?:[\s.\[\(]|$)").unwrap());

/// Parses a file stem (name without extension). macOS file systems hand back decomposed
/// accents (NFD); everything downstream, TMDB included, expects composed text (NFC).
pub fn parse_stem(stem: &str) -> Parsed {
    let stem = nfc(stem);
    let stem = stem.as_str();
    if let Some(c) = EPISODE.captures(stem) {
        return Parsed::Episode {
            show: clean(&c["show"]),
            season: c["s"].parse().unwrap_or(0),
            episode: c["e"].parse().unwrap_or(0),
        };
    }
    if let Some(c) = PAREN_YEAR.captures(stem) {
        return Parsed::Movie { title: clean(&c["title"]), year: c["year"].parse().unwrap_or(0) };
    }
    if let Some(c) = SCENE_YEAR.captures(stem) {
        return Parsed::Movie { title: clean(&c["title"]), year: c["year"].parse().unwrap_or(0) };
    }
    Parsed::Flat { title: clean(stem) }
}

/// Composed form. macOS hands back decomposed accents; ids, titles and queries use this.
pub fn nfc(s: &str) -> String {
    s.nfc().collect()
}

/// Scene names use dots for spaces; ordinary names keep their punctuation.
fn clean(s: &str) -> String {
    let s = s.trim();
    let dotted = s.contains('.') && !s.contains(' ');
    let s = if dotted { s.replace('.', " ") } else { s.to_string() };
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_with_paren_year() {
        assert_eq!(parse_stem("My Neighbor Totoro (1988)"), Parsed::Movie { title: "My Neighbor Totoro".into(), year: 1988 });
        assert_eq!(parse_stem("L'Âge de glace (2002)"), Parsed::Movie { title: "L'Âge de glace".into(), year: 2002 });
        assert_eq!(parse_stem("John Wick - Chapter 3 Parabellum (2019)"), Parsed::Movie { title: "John Wick - Chapter 3 Parabellum".into(), year: 2019 });
    }

    #[test]
    fn episode() {
        assert_eq!(parse_stem("Friends S03E05"), Parsed::Episode { show: "Friends".into(), season: 3, episode: 5 });
        assert_eq!(parse_stem("The.Office.S02E10.720p"), Parsed::Episode { show: "The Office".into(), season: 2, episode: 10 });
    }

    #[test]
    fn scene_name_with_bare_year() {
        assert_eq!(
            parse_stem("The Boy and the Heron 2023 1080p (DAUL) WEB-DL HEVC x265 5.1 BONE"),
            Parsed::Movie { title: "The Boy and the Heron".into(), year: 2023 }
        );
        assert_eq!(parse_stem("Spirited.Away.2001.1080p.BluRay"), Parsed::Movie { title: "Spirited Away".into(), year: 2001 });
    }

    #[test]
    fn composes_decomposed_accents() {
        let nfd = "L'A\u{302}ge de glace (2002)";
        assert_eq!(parse_stem(nfd), Parsed::Movie { title: "L'\u{c2}ge de glace".into(), year: 2002 });
        assert_eq!(parse_stem("Dragons 3 - Le Monde cache\u{301} (2019)"), Parsed::Movie { title: "Dragons 3 - Le Monde caché".into(), year: 2019 });
    }

    #[test]
    fn flat_when_nothing_matches() {
        assert_eq!(parse_stem("La Pat Patrouille - Vol 8"), Parsed::Flat { title: "La Pat Patrouille - Vol 8".into() });
        assert_eq!(parse_stem("Yakari"), Parsed::Flat { title: "Yakari".into() });
    }

    #[test]
    fn a_number_in_a_title_is_not_a_year() {
        assert_eq!(parse_stem("Dragons 3 - Le Monde caché (2019)"), Parsed::Movie { title: "Dragons 3 - Le Monde caché".into(), year: 2019 });
        assert_eq!(parse_stem("Metallica - S&M2 (2019)"), Parsed::Movie { title: "Metallica - S&M2".into(), year: 2019 });
    }
}
