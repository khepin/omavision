# Mockup variant contract

Files: `variants/vN.js` and `variants/vN.css` in this directory (N = your number). Nothing else. Do not edit shell files.

## Registration (vN.js)
```js
window.OMV.register({
  id: "v3",                      // also the class added to the stage element
  name: "Shelf",                 // short name shown in the switcher
  thesis: "one sentence: what this look is and why it suits a family film library on a TV",
  mount(root, model, api) {},    // build DOM under root; root is the 1920x1080 stage, already has class "stage vN"
  unmount() {},                  // clear timers/listeners; the shell empties root itself
  onKey(e) { return true; },     // keydown; return true when handled (shell then preventDefault). Reserved by shell: [ ] \
  onTheme(t) {},                 // optional; called when the Omarchy theme changes
});
```
Wrap your file in an IIFE. No modules, no imports, no external scripts. Google Fonts are allowed: inject a `<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=...&display=swap">` into document.head from mount (once), and always give a real fallback stack.

## Stage
- Fixed 1920x1080 design canvas, scaled by the shell. Root font-size is 24px (the 1.6 scale is already applied). Use rem. Nothing may overflow the stage; assume a viewer 3 m away, so minimum readable text is 1rem and focused text larger.
- CSS must be scoped under `.stage.vN` (e.g. `.stage.v3 .grid {}`); the stage only holds your variant while selected.
- A base `.poster` element exists (placeholder gradient with the title typeset, `--hue` per item, 2:3). Get one with `api.posterEl(item)`; restyle it under your scope if you want. Real posters arrive later from TMDB, so treat it as an image slot: do not rely on the placeholder text being visible.

## Theme tokens (the Omarchy theme drives colour; you drive everything else)
`--bg --surface --surface2 --text --muted --accent --accent2 --border`, set on :root by the shell. `document.documentElement.dataset.mode` is "dark" or "light". Use only these tokens for colour, plus `--hue` from items and transparency/mixing via `color-mix()` or hsl alpha. No hard-coded palette colours. The look must hold on every theme in the switcher, light one included.

## Model
```
model.categories: [{id, label, items:[Item]}]  in display order: Films, Animation, Séries, Concerts, Divers
model.items: all items flat
Item: {id, kind:"movie"|"show"|"flat", title, year|null, hue, path, category, meta?:{synopsis, runtime, rating, genres[]}, seasons?:[{number, episodes:[{season, episode, path, title|null}]}]}
model.meta(item) -> meta or null       (only ~18 items have sample metadata; the rest show parsed name only, as in real life before enrichment)
model.filter(query, list?) -> items    (accent/case insensitive substring; use this for type-to-filter)
model.nextUnwatched(show) -> episode
model.continueWatching() -> items, most recent first (3 items in the sample state)
model.label(categoryId) -> display label
```

## API
```
api.posterEl(item, extraClass?) -> element
api.play(itemOrEpisode)          -> shows a toast with the mpv command (mock playback). Call it on Enter.
api.toast(text)
api.esc(str)                     -> html escape
```

## Required behaviour (keyboard only, no mouse needed)
- Arrows move focus. Enter opens or plays. Esc/Backspace goes back or clears the filter. Typing any letter/digit starts a live filter (space included, Backspace edits it). Show the current filter text somewhere.
- A show (kind "show") opens into its episodes grouped by season, with `model.nextUnwatched` preselected. Enter on an episode plays it.
- The focused item is unmistakable from 3 m. Show the item's details somewhere when focused (title, year, synopsis/runtime/rating when meta exists; otherwise a quiet "métadonnées à venir" style note in the same language as your UI copy; UI copy is French).
- Continue-watching data exists; use it if your direction has a place for it, otherwise skip it.
- The page must look complete at rest: first paint shows a focused item and content, no empty state.
- Navigation layout is yours to decide (tabs, rows, single grid, sidebar, whatever the direction calls for). That is what this exploration is for.

## Craft
This is a look-and-feel pitch. Make deliberate, specific choices in typography, spacing, layout, motion. Real content only. Respect prefers-reduced-motion. Keep the code self-contained and under ~400 lines per file.
