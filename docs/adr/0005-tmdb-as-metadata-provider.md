---
status: accepted
---

# TMDB is the metadata provider

Posters are the point of a TV grid, and the library mixes French and English titles with episodes. TMDB is free for non-commercial use with attribution, covers movies, shows and episodes, and returns French text with English fallback. Enrichment is lazy and in the background; name parsing is the fallback when the key or the network is missing.

## Considered options

- **OMDb**: free tier, but posters need a paid tier and coverage of French titles and episodes is weak.
- **TheTVDB v4**: paid subscription, TV focused.
- **Wikidata and IMDb dumps**: free and keyless, but no posters.

## Consequences

- The attribution notice "This product uses the TMDb API but is not endorsed or certified by TMDb" must be visible in the app.
- Fixing wrong matches is deferred until real misses appear; the interim behaviour is top hit plus placeholder poster.
