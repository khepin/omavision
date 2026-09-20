# Design direction: Filmothèque

Chosen on 2026-09-19 from six explorations. A working HTML mockup lives in `docs/mockups/filmotheque/index.html`; open it in a browser to see the reference behaviour. The mockup's UI copy is French; the app's UI copy is English. Metadata language follows `metadata.languages` in the config; the current setting is English first, French fallback. Metadata language follows `metadata.languages` in the config; the current setting is English first, French fallback.

## Thesis

A video-club catalog: a dense list you scan with the keyboard, and a full card on the right. No chrome that does not carry information. Colours come from the Omarchy theme, everything else from this document.

## Layout on a 1920x1080 stage, scale 1.6

- Top line, full width: the filter prompt. `> ` followed by the typed text, a blinking caret, a hint when empty, and a match count right-aligned as `n / total`.
- Left column, about two thirds: category tabs as plain uppercase words with the count in small type, active tab underlined in accent. Below, the list: one item per line with a running number, a dot in accent2 for shows, the title, a muted "played on …" note for continue-watching items, and the year right-aligned in tabular numerals.
- Right column, about one third: the card. Poster with no radius, a small label such as "Card · Animation No. 017", title, facts line with year, runtime, rating, and for shows seasons and episodes, genres in accent2, synopsis in a humanist serif. Without metadata the card reads "Metadata pending".
- Hairline rules in border between regions. Zero border radius anywhere. The focused line is a full-width accent bar with bg-coloured text.

## Type

- List, labels, prompt: IBM Plex Mono, fallback ui-monospace.
- Synopsis and episode titles: Alegreya, fallback Georgia, serif.

## Keys

- Up/Down move one line. PageUp/PageDown move ten. Home/End jump.
- Left/Right switch tabs and keep the filter.
- Any letter or digit types into the filter, Backspace edits it, Esc clears it.
- Enter plays a movie or flat item, or opens a show.
- In a show: episodes grouped under "Season N · 24 episodes" headings, lines like `S02 · E03 · The One Where Heckles Dies`, next unwatched preselected, last played marked. Left/Right jump between seasons. Esc or Backspace returns to the list.

## Theme token use

| Token | Use |
| --- | --- |
| bg | page ground, text on the focused line |
| surface | poster area ground |
| text | primary text |
| dark_foreground | secondary text: years, counts, notes |
| muted | hairline rules only |
| accent | active tab underline, focused line, caret |
| accent2 | show dot, genres |
