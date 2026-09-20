(function () {
  const COLS = 5;
  const FONT = "https://fonts.googleapis.com/css2?family=Fraunces:ital,opsz,wght@0,9..144,300..900;1,9..144,300..900&family=Newsreader:ital,opsz,wght@0,6..72,400..600;1,6..72,400..600&display=swap";
  let root, model, api;
  let S = null; // state
  const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function ensureFont() {
    if (document.getElementById("omv-v6-font")) return;
    const l = document.createElement("link");
    l.id = "omv-v6-font"; l.rel = "stylesheet"; l.href = FONT;
    document.head.appendChild(l);
  }

  function sections(query) {
    const out = [];
    const cw = model.continueWatching();
    if (cw.length) out.push({ id: "reprendre", label: "Reprendre", items: cw });
    model.categories.forEach(c => out.push({ id: c.id, label: c.label, items: c.items }));
    return out
      .map(s => ({ ...s, items: query ? model.filter(query, s.items) : s.items }))
      .filter(s => s.items.length);
  }

  function flatten(secs) {
    const flat = [];
    secs.forEach((s, si) => s.items.forEach((it, pos) => flat.push({ item: it, si, pos, len: s.items.length })));
    return flat;
  }

  function fmtRuntime(m) {
    if (!m) return "";
    const h = Math.floor(m / 60), mm = m % 60;
    return h ? `${h} h ${String(mm).padStart(2, "0")} min` : `${mm} min`;
  }
  const fmtRating = r => r ? String(r).replace(".", ",") : "";
  const pad2 = n => String(n).padStart(2, "0");

  function mount(r, m, a) {
    root = r; model = m; api = a;
    ensureFont();
    S = { query: "", mode: "browse", secs: [], flat: [], focus: 0, show: null, eps: [], ep: 0, tiles: [] };
    root.innerHTML = `
      <div class="ed">
        <aside class="feature"><div class="feat" id="v6-feat"></div></aside>
        <main class="browse">
          <header class="runhead"><span class="rh-q" id="v6-q"></span><span class="rh-n" id="v6-n"></span></header>
          <div class="scroll" id="v6-scroll"></div>
        </main>
      </div>`;
    rebuild(true);
  }

  function rebuild(keepFocus) {
    S.secs = sections(S.query);
    S.flat = flatten(S.secs);
    if (!keepFocus || S.focus >= S.flat.length) S.focus = 0;
    const scroll = root.querySelector("#v6-scroll");
    scroll.innerHTML = "";
    S.tiles = [];
    let k = 0;
    S.secs.forEach(s => {
      const sec = document.createElement("section");
      sec.innerHTML = `<h2><em>${api.esc(s.label)}</em><span class="rule"></span><span class="n">${s.items.length}</span></h2>`;
      const grid = document.createElement("div"); grid.className = "grid";
      s.items.forEach(it => {
        const t = document.createElement("div"); t.className = "tile"; t.dataset.k = k++;
        t.appendChild(api.posterEl(it));
        grid.appendChild(t); S.tiles.push(t);
      });
      sec.appendChild(grid); scroll.appendChild(sec);
    });
    if (!S.flat.length) {
      scroll.innerHTML = `<p class="empty">Aucun titre ne correspond à « ${api.esc(S.query)} ».</p>`;
    }
    renderHead();
    setFocus(S.focus, true);
  }

  function renderHead() {
    const q = root.querySelector("#v6-q"), n = root.querySelector("#v6-n");
    if (S.query) {
      q.textContent = S.query; q.classList.remove("idle");
      n.textContent = `${S.flat.length} ${S.flat.length === 1 ? "titre" : "titres"}`;
    } else {
      q.textContent = "Tapez pour filtrer"; q.classList.add("idle");
      n.textContent = `${model.items.length} titres`;
    }
  }

  function setFocus(i, instant) {
    if (!S.flat.length) { renderFeature(null); return; }
    S.focus = Math.max(0, Math.min(i, S.flat.length - 1));
    S.tiles.forEach((t, k) => t.classList.toggle("focus", k === S.focus));
    const t = S.tiles[S.focus];
    t.scrollIntoView({ block: "center", behavior: instant || reduced() ? "auto" : "smooth" });
    renderFeature(S.flat[S.focus].item);
  }

  function crossfade(el) {
    if (reduced()) return;
    el.classList.remove("fade"); void el.offsetWidth; el.classList.add("fade");
  }

  function renderFeature(it) {
    const f = root.querySelector("#v6-feat");
    if (!it) { f.innerHTML = `<p class="syn pending">Rien à afficher.</p>`; return; }
    const meta = model.meta(it);
    const cat = model.label(it.category);
    let eyebrow, metaLine;
    if (it.kind === "show") {
      const n = it.seasons.reduce((a, s) => a + s.episodes.length, 0);
      const nx = model.nextUnwatched(it);
      eyebrow = `Série · ${it.seasons.length} saisons · ${n} épisodes`;
      metaLine = `Reprendre S${pad2(nx.season)}E${pad2(nx.episode)}${nx.title ? " · " + api.esc(nx.title) : ""}`;
    } else {
      eyebrow = [it.year, cat].filter(Boolean).join(" · ");
      metaLine = meta ? [fmtRuntime(meta.runtime), fmtRating(meta.rating), (meta.genres || []).join(", ")].filter(Boolean).join(" · ") : "";
    }
    const syn = meta ? `<p class="syn">${api.esc(meta.synopsis)}</p>`
      : `<p class="syn pending">Métadonnées à venir. Titre lu depuis le fichier.</p>`;
    f.innerHTML = `
      <div class="feat-poster"></div>
      <div class="eyebrow">${api.esc(eyebrow)}</div>
      <h1 class="title">${api.esc(it.title)}</h1>
      ${syn}
      <hr>
      <div class="meta">${metaLine || "&nbsp;"}</div>`;
    f.querySelector(".feat-poster").appendChild(api.posterEl(it));
    crossfade(f);
  }

  // ---- show mode -------------------------------------------------------
  function openShow(show) {
    S.mode = "show"; S.show = show;
    S.eps = show.seasons.flatMap(s => s.episodes);
    const nx = model.nextUnwatched(show);
    S.ep = Math.max(0, S.eps.findIndex(e => e.path === nx.path));
    const f = root.querySelector("#v6-feat");
    const n = S.eps.length;
    let toc = "";
    show.seasons.forEach(s => {
      toc += `<li class="season">Saison ${s.number}</li>`;
      s.episodes.forEach(e => {
        const k = S.eps.indexOf(e);
        toc += `<li class="ep" data-k="${k}"><span class="ep-t">${e.title ? api.esc(e.title) : "Épisode " + e.episode}</span><span class="leader"></span><span class="num">${pad2(e.episode)}</span></li>`;
      });
    });
    f.innerHTML = `
      <div class="eyebrow">Série · ${show.seasons.length} saisons · ${n} épisodes</div>
      <h1 class="title small">${api.esc(show.title)}</h1>
      <ol class="toc">${toc}</ol>
      <hr>
      <div class="meta">Entrée pour lire · Échap pour revenir</div>`;
    crossfade(f);
    setEp(S.ep, true);
  }
  function setEp(i, instant) {
    S.ep = Math.max(0, Math.min(i, S.eps.length - 1));
    const rows = root.querySelectorAll(".toc .ep");
    rows.forEach(r => r.classList.toggle("focus", +r.dataset.k === S.ep));
    const r = rows[S.ep];
    if (r) r.scrollIntoView({ block: "center", behavior: instant || reduced() ? "auto" : "smooth" });
  }
  function closeShow() {
    S.mode = "browse"; S.show = null;
    renderFeature(S.flat[S.focus] && S.flat[S.focus].item);
  }
  function seasonJump(dir) {
    const cur = S.eps[S.ep].season;
    const seasons = S.show.seasons.map(s => s.number);
    const idx = seasons.indexOf(cur) + dir;
    if (idx < 0 || idx >= seasons.length) return;
    setEp(S.eps.findIndex(e => e.season === seasons[idx]));
  }

  // ---- browse navigation ------------------------------------------------
  function moveVert(dir) {
    const cur = S.flat[S.focus]; if (!cur) return;
    const col = cur.pos % COLS, row = Math.floor(cur.pos / COLS), lastRow = Math.floor((cur.len - 1) / COLS);
    if (dir < 0) {
      if (row > 0) return setFocus(S.focus - COLS);
      const ps = S.secs[cur.si - 1]; if (!ps) return;
      const plen = ps.items.length, plast = Math.floor((plen - 1) / COLS);
      const pos = Math.min(plast * COLS + col, plen - 1);
      return setFocus(S.focus - cur.pos - plen + pos);
    } else {
      if (cur.pos + COLS < cur.len) return setFocus(S.focus + COLS);
      if (row < lastRow) return setFocus(S.focus - cur.pos + cur.len - 1);
      const ns = S.secs[cur.si + 1]; if (!ns) return;
      const pos = Math.min(col, ns.items.length - 1);
      return setFocus(S.focus - cur.pos + cur.len + pos);
    }
  }

  function onKey(e) {
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    const k = e.key;
    if (S.mode === "show") {
      if (k === "ArrowDown") { setEp(S.ep + 1); return true; }
      if (k === "ArrowUp") { setEp(S.ep - 1); return true; }
      if (k === "ArrowRight") { seasonJump(1); return true; }
      if (k === "ArrowLeft") { seasonJump(-1); return true; }
      if (k === "Enter") { api.play(S.eps[S.ep]); return true; }
      if (k === "Escape" || k === "Backspace") { closeShow(); return true; }
      return false;
    }
    if (k === "ArrowRight") { setFocus(S.focus + 1); return true; }
    if (k === "ArrowLeft") { setFocus(S.focus - 1); return true; }
    if (k === "ArrowDown") { moveVert(1); return true; }
    if (k === "ArrowUp") { moveVert(-1); return true; }
    if (k === "Enter") {
      const cur = S.flat[S.focus]; if (!cur) return true;
      if (cur.item.kind === "show") openShow(cur.item); else api.play(cur.item);
      return true;
    }
    if (k === "Escape") { if (S.query) { S.query = ""; rebuild(false); } return true; }
    if (k === "Backspace") { if (S.query) { S.query = S.query.slice(0, -1); rebuild(false); } return true; }
    if (k.length === 1) { S.query += k; rebuild(false); return true; }
    return false;
  }

  function unmount() { S = null; }

  window.OMV.register({
    id: "v6",
    name: "Éditorial",
    thesis: "Une double page de revue de cinéma : le titre choisi occupe la moitié gauche en grande typographie serif, la bibliothèque défile à droite en petites affiches ; le film compte plus que la grille.",
    mount, unmount, onKey,
  });
})();
