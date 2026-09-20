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

/// Every word of the query must appear somewhere in the item's text: parsed title and year,
/// then what enrichment brought back. The overview is left out because common words would
/// match half the library with no visible reason.
pub fn matches(item: &Item, meta: Option<&Meta>, folded_query: &str) -> bool {
    let hay = fold(&haystack(item, meta));
    folded_query.split_whitespace().all(|w| hay.contains(w))
}

fn haystack(item: &Item, meta: Option<&Meta>) -> String {
    let mut hay = item.title.clone();
    if let Some(y) = item.year {
        hay.push(' ');
        hay.push_str(&y.to_string());
    }
    if let Some(m) = meta {
        for s in [&m.title, &m.original_title].into_iter().chain(&m.genres).chain(&m.companies).chain(&m.people).chain(&m.keywords) {
            hay.push(' ');
            hay.push_str(s);
        }
    }
    hay
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

    #[test]
    fn filters_on_enrichment_fields_and_every_word() {
        let items = vec![item("Spirited Away", Some(2001)), item("Ponyo", Some(2008)), item("Cars", Some(2006))];
        let mut meta = HashMap::new();
        meta.insert("Spirited Away".to_string(), Meta { title: "Le Voyage de Chihiro".into(), original_title: "千と千尋の神隠し".into(), genres: vec!["Animation".into()], companies: vec!["Studio Ghibli".into()], people: vec!["Hayao Miyazaki".into()], keywords: vec!["witch".into()], overview: "A girl".into(), ..Default::default() });
        meta.insert("Ponyo".to_string(), Meta { companies: vec!["Studio Ghibli".into()], ..Default::default() });
        meta.insert("Cars".to_string(), Meta { companies: vec!["Pixar".into()], ..Default::default() });
        assert_eq!(filter(&items, &meta, "ghibli"), vec![0, 1]);
        assert_eq!(filter(&items, &meta, "GHIBLI 2008"), vec![1]);
        assert_eq!(filter(&items, &meta, "chihiro"), vec![0]);
        assert_eq!(filter(&items, &meta, "千尋"), vec![0]);
        assert_eq!(filter(&items, &meta, "miyazaki witch"), vec![0]);
        assert_eq!(filter(&items, &meta, "animation"), vec![0]);
        assert!(filter(&items, &meta, "girl").is_empty());
        assert!(filter(&items, &meta, "ghibli pixar").is_empty());
    }
}
