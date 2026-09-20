---
status: accepted
---

# Cargo workspace with a core library and a thin UI binary

Navigation and visual direction are deliberately undecided and will be settled through design explorations. Scanning, name parsing, the index, enrichment, search matching and state live in a core library with its own tests. Each exploration is a separate thin Slint binary against that core, so UIs can be thrown away without touching tested logic. The core exposes categories, items and the show tree in a layout-neutral shape so tabs, rows or a single grid can all be built on it.
