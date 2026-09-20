(function () {
  const COLS = 8, EP_COLS = 6;
  let root, model, api, els = {}, tiles = [], epTiles = [];
  let view = "grid", zone = "grid", focus = 0, barIdx = 0, epFocus = 0, query = "";
  let catOn = new Set(), showItem = null, raf = 0, fontsDone = false;

  function fonts() {
    if (fontsDone) return; fontsDone = true;
    const l = document.createElement("link"); l.rel = "stylesheet";
    l.href = "https://fonts.googleapis.com/css2?family=Archivo:wdth,wght@62..125,400..700&family=IBM+Plex+Mono:wght@500&display=swap";
    document.head.appendChild(l);
  }
  const h = (tag, cls, html) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; };
  const pad = n => String(n).padStart(2, "0");

  function mount(r, m, a) {
    root = r; model = m; api = a; fonts();
    catOn = new Set(model.categories.map(c => c.id));
    const cw = model.continueWatching();
    els.bar = h("div", "m-bar");
    els.tags = h("div", "m-tags");
    model.categories.forEach((c, i) => {
      const t = h("button", "m-tag on", `<span class="m-n">${c.items.length}</span>${api.esc(c.label)}`);
      t.dataset.id = c.id; els.tags.appendChild(t);
    });
    els.query = h("div", "m-query");
    els.bar.append(els.tags, els.query);
    els.area = h("div", "m-area");
    els.grid = h("div", "m-grid");
    tiles = model.items.map((it, i) => {
      const t = h("div", "m-tile" + (cw.includes(it) ? " cw" : ""));
      t.dataset.cat = it.category; t.appendChild(api.posterEl(it)); t.item = it; els.grid.appendChild(t); return t;
    });
    els.area.appendChild(els.grid);
    els.strip = h("div", "m-strip");
    root.append(els.bar, els.area, els.strip);
    focus = 0; zone = "grid"; view = "grid"; query = "";
    renderGrid(); renderStrip();
  }
  function unmount() { cancelAnimationFrame(raf); tiles = []; epTiles = []; els = {}; showItem = null; }

  function active(t) { return catOn.has(t.item.category) && (!query || matchSet.has(t.item)); }
  let matchSet = new Set();
  function renderGrid() {
    matchSet = new Set(query ? model.filter(query) : model.items);
    tiles.forEach((t, i) => { t.classList.toggle("dim", !active(t)); t.classList.toggle("focus", zone === "grid" && i === focus); });
    [...els.tags.children].forEach((b, i) => {
      b.classList.toggle("on", catOn.has(b.dataset.id)); b.classList.toggle("focus", zone === "bar" && i === barIdx);
    });
    els.query.innerHTML = query ? `<span class="m-q">${api.esc(query)}</span><span class="m-cur"></span>` : `<span class="m-hint">tapez pour filtrer</span>`;
    scrollTo(els.grid, zone === "grid" ? tiles[focus] : null);
  }
  function renderStrip() {
    let it, ep;
    if (view === "show") { ep = epTiles[epFocus]?.ep; it = showItem; }
    else it = tiles[focus]?.item;
    if (!it) return;
    const meta = model.meta(it);
    let title = api.esc(it.title), sub = [];
    if (ep) { title = `${api.esc(it.title)} <span class="m-ep">S${pad(ep.season)}E${pad(ep.episode)}</span>`; if (ep.title) sub.push(api.esc(ep.title)); }
    else if (zone === "bar") { const c = model.categories[barIdx]; title = api.esc(c.label); sub.push(`${c.items.length} titres · Entrée pour ${catOn.has(c.id) ? "masquer" : "afficher"}`); }
    if (!ep && zone !== "bar") {
      if (it.year) sub.push(String(it.year));
      if (it.kind === "show") { const n = it.seasons.reduce((s, x) => s + x.episodes.length, 0); sub.push(`${it.seasons.length} saisons · ${n} épisodes`); }
      if (meta) { if (meta.runtime && it.kind !== "show") sub.push(`${meta.runtime} min`); if (meta.rating) sub.push(`★ ${meta.rating.toFixed(1)}`); }
    }
    const syn = meta?.synopsis ? api.esc(meta.synopsis) : `<span class="m-pending">métadonnées à venir</span>`;
    els.strip.innerHTML = `<div class="m-title">${title}</div><div class="m-meta">${sub.map(s => `<span>${s}</span>`).join("")}</div><div class="m-syn">${ep && ep.title ? "" : syn}</div>`;
    els.strip.classList.remove("in"); void els.strip.offsetWidth; els.strip.classList.add("in");
  }
  function scrollTo(container, el) {
    if (!el) return;
    const areaH = els.area.clientHeight - 200; // keep clear of the strip
    const centre = el.offsetTop + el.offsetHeight / 2;
    let y = areaH / 2 - centre;
    const min = Math.min(0, areaH - container.scrollHeight);
    y = Math.max(min, Math.min(0, y));
    container.style.transform = `translateY(${y}px)`;
  }

  // ---- navigation over the grid, skipping dimmed tiles
  function moveGrid(dx, dy) {
    const n = tiles.length;
    if (dy < 0 && Math.floor(focus / COLS) === 0) { zone = "bar"; barIdx = Math.min(barIdx, els.tags.children.length - 1); renderGrid(); renderStrip(); return; }
    if (dx) { let i = focus; for (let k = 0; k < n; k++) { i += dx; if (i < 0 || i >= n) break; if (active(tiles[i])) { focus = i; break; } } }
    else { let i = focus + dy * COLS; while (i >= 0 && i < n) { if (active(tiles[i])) { focus = i; break; } i += dy * COLS; } }
    renderGrid(); renderStrip();
  }
  function firstActive() { const i = tiles.findIndex(active); return i < 0 ? focus : i; }

  // ---- show view
  function openShow(it) {
    showItem = it; view = "show"; epTiles = [];
    els.grid.style.display = "none";
    els.eps = h("div", "m-eps");
    const next = model.nextUnwatched(it), last = model.state.lastPlayed[it.id]?.episode;
    it.seasons.forEach(s => {
      els.eps.appendChild(h("div", "m-rule", `<span>Saison ${pad(s.number)}</span><span class="m-rule-n">${s.episodes.length} épisodes</span>`));
      const g = h("div", "m-epgrid");
      s.episodes.forEach(ep => {
        const t = h("div", "m-ep-tile" + (ep.path === last ? " cw" : ""));
        t.style.setProperty("--hue", it.hue); t.ep = ep;
        t.innerHTML = `<span class="m-ep-n">S${pad(ep.season)}E${pad(ep.episode)}</span>${ep.title ? `<span class="m-ep-t">${api.esc(ep.title)}</span>` : ""}`;
        g.appendChild(t); epTiles.push(t);
      });
      els.eps.appendChild(g);
    });
    els.area.appendChild(els.eps);
    epFocus = Math.max(0, epTiles.findIndex(t => t.ep === next));
    els.bar.classList.add("show"); els.tags.innerHTML = `<span class="m-crumb"><span class="m-back">‹ Esc</span>${api.esc(it.title)}</span>`;
    renderEps(); renderStrip();
  }
  function closeShow() {
    els.eps.remove(); els.eps = null; epTiles = []; showItem = null; view = "grid";
    els.grid.style.display = ""; els.bar.classList.remove("show");
    els.tags.innerHTML = "";
    model.categories.forEach(c => { const t = h("button", "m-tag", `<span class="m-n">${c.items.length}</span>${api.esc(c.label)}`); t.dataset.id = c.id; els.tags.appendChild(t); });
    renderGrid(); renderStrip();
  }
  function renderEps() { epTiles.forEach((t, i) => t.classList.toggle("focus", i === epFocus)); scrollTo(els.eps, epTiles[epFocus]); }
  function moveEps(dx, dy) {
    const n = epTiles.length; let i = epFocus + (dx || dy * EP_COLS);
    if (i < 0 || i >= n) return; epFocus = i; renderEps(); renderStrip();
  }

  function onKey(e) {
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    const k = e.key;
    if (view === "show") {
      if (k === "ArrowLeft") moveEps(-1, 0); else if (k === "ArrowRight") moveEps(1, 0);
      else if (k === "ArrowUp") moveEps(0, -1); else if (k === "ArrowDown") moveEps(0, 1);
      else if (k === "Enter") api.play(epTiles[epFocus].ep);
      else if (k === "Escape" || k === "Backspace") closeShow();
      else return false;
      return true;
    }
    if (k === "Backspace") { if (query) { query = query.slice(0, -1); focus = firstActive(); renderGrid(); renderStrip(); } return true; }
    if (k === "Escape") { if (!query) return false; query = ""; renderGrid(); renderStrip(); return true; }
    if (k.length === 1) { query += k; zone = "grid"; focus = firstActive(); renderGrid(); renderStrip(); return true; }
    if (zone === "bar") {
      const n = els.tags.children.length;
      if (k === "ArrowLeft") barIdx = (barIdx + n - 1) % n;
      else if (k === "ArrowRight") barIdx = (barIdx + 1) % n;
      else if (k === "ArrowDown") { zone = "grid"; focus = firstActive(); }
      else if (k === "Enter") { const id = model.categories[barIdx].id; catOn.has(id) ? catOn.delete(id) : catOn.add(id); if (!catOn.size) catOn.add(id); }
      else return false;
      renderGrid(); renderStrip(); return true;
    }
    if (k === "ArrowLeft") moveGrid(-1, 0); else if (k === "ArrowRight") moveGrid(1, 0);
    else if (k === "ArrowUp") moveGrid(0, -1); else if (k === "ArrowDown") moveGrid(0, 1);
    else if (k === "Enter") { const it = tiles[focus].item; it.kind === "show" ? openShow(it) : api.play(it); }
    else return false;
    return true;
  }

  window.OMV.register({
    id: "v4", name: "Mosaic",
    thesis: "Une seule mosaïque de toutes les affiches, bord à bord, sans chrome : la bibliothèque se lit comme un mur de cinéma et le focus fait tout le travail.",
    mount, unmount, onKey,
  });
})();
