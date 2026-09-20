//! Matching for the filter line. Case and accent insensitive, because the library mixes
//! French and English titles and nobody types accents on a remote.
use crate::index::Item;
use crate::meta::Meta;
use std::collections::HashMap;

/// Case and accent insensitive form for matching.
pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'œ' => 'o',
            _ => c,
        })
        .collect()
}

/// Indices of the items that match, in item order. An empty query matches everything.
pub fn filter(items: &[Item], meta: &HashMap<String, Meta>, query: &str) -> Vec<usize> {
    let q = fold(query);
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| q.is_empty() || matches(it, meta.get(&it.id), &q))
        .map(|(i, _)| i)
        .collect()
}

/// Matching is on the parsed title and year. `meta` is taken so it can grow to the localized
/// title, genres or the overview without moving the seam.
pub fn matches(item: &Item, _meta: Option<&Meta>, folded_query: &str) -> bool {
    let year = item.year.map(|y| y.to_string()).unwrap_or_default();
    fold(&format!("{} {}", item.title, year)).contains(folded_query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Kind;

    fn item(title: &str, year: Option<u16>) -> Item {
        Item { id: title.into(), kind: Kind::Movie, title: title.into(), year, path: Some(title.into()), seasons: vec![] }
    }

    #[test]
    fn folds_case_and_accents() {
        assert_eq!(fold("L'Âge de glace"), "l'age de glace");
        assert_eq!(fold("Le Monde caché"), "le monde cache");
    }

    #[test]
    fn filters_on_title_and_year_and_ignores_accents() {
        let items = vec![item("L'Âge de glace", Some(2002)), item("Ponyo", Some(2008)), item("Yakari", None)];
        let meta = HashMap::new();
        assert_eq!(filter(&items, &meta, ""), vec![0, 1, 2]);
        assert_eq!(filter(&items, &meta, "age"), vec![0]);
        assert_eq!(filter(&items, &meta, "GLACE"), vec![0]);
        assert_eq!(filter(&items, &meta, "2008"), vec![1]);
        assert_eq!(filter(&items, &meta, "yak"), vec![2]);
        assert!(filter(&items, &meta, "ghibli").is_empty());
    }
}
