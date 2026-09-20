(function () {
  const FONTS = "https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=Alegreya:ital,wght@0,400;0,500;1,400&display=swap";
  const PAGE = 10;
  let model, api, root;
  let els = {};
  let st;

  function reset() {
    st = { tab: 0, focus: 0, query: "", mode: "list", show: null, eps: [], epFocus: 0 };
  }

  function fmtDate(iso) {
    const d = new Date(iso);
    return d.toLocaleDateString("fr-FR", { day: "numeric", month: "short" });
  }
  function pad(n, w) { return String(n).padStart(w, "0"); }
  function seen(item) {
    const lp = model.state.lastPlayed[item.id];
    return lp ? "vu le " + fmtDate(lp.at) : "";
  }
  function visible() {
    const cat = model.categories[st.tab];
    return model.filter(st.query, cat.items);
  }

  // ---------- build ----------
  function mount(r, m, a) {
    root = r; model = m; api = a; reset();
    if (!document.getElementById("v2-fonts")) {
      const l = document.createElement("link");
      l.id = "v2-fonts"; l.rel = "stylesheet"; l.href = FONTS;
      document.head.appendChild(l);
    }
    root.innerHTML = `
      <div class="prompt"><span class="chev">&gt;</span><span class="q"></span><span class="caret"></span><span class="hint"></span><span class="count"></span></div>
      <div class="left"><div class="tabs"></div><div class="list"></div></div>
      <div class="right"><div class="fiche"></div></div>`;
    els = {
      q: root.querySelector(".q"), hint: root.querySelector(".hint"), count: root.querySelector(".count"),
      tabs: root.querySelector(".tabs"), list: root.querySelector(".list"), fiche: root.querySelector(".fiche"),
    };
    // resume where the sample state left off: the show is what a family reopens most
    renderAll();
  }
  function unmount() { els = {}; }

  function renderAll() { renderPrompt(); renderTabs(); st.mode === "list" ? renderList() : renderEpisodes(); renderFiche(); }

  function renderPrompt() {
    els.q.textContent = st.query;
    els.hint.textContent = st.query ? "" : "tapez pour filtrer";
    if (st.mode === "list") {
      const cat = model.categories[st.tab];
      const n = visible().length;
      els.count.textContent = st.query ? `${n} / ${cat.items.length}` : `${cat.items.length} fiches`;
    } else {
      els.count.textContent = `${st.eps.length} épisodes`;
    }
  }

  function renderTabs() {
    els.tabs.innerHTML = model.categories.map((c, i) =>
      `<span class="tab${i === st.tab ? " on" : ""}">${api.esc(c.label)}<small>${c.items.length}</small></span>`).join("");
    if (st.mode === "show") {
      els.tabs.innerHTML += `<span class="crumb">/ ${api.esc(st.show.title)}</span>`;
    }
  }

  function renderList() {
    const items = visible();
    if (st.focus >= items.length) st.focus = Math.max(0, items.length - 1);
    if (!items.length) {
      els.list.innerHTML = `<div class="empty">aucune fiche pour « ${api.esc(st.query)} »</div>`;
      return;
    }
    els.list.innerHTML = items.map((it, i) => `
      <div class="row${i === st.focus ? " focus" : ""}${it.kind === "show" ? " show" : ""}">
        <span class="num">${pad(i + 1, 3)}</span>
        <span class="mark">${it.kind === "show" ? "●" : ""}</span>
        <span class="title">${api.esc(it.title)}</span>
        <span class="seen">${seen(it)}</span>
        <span class="year">${it.year ?? "—"}</span>
      </div>`).join("");
    keepVisible();
  }

  function renderEpisodes() {
    const lp = model.state.lastPlayed[st.show.id];
    let html = "", k = 0;
    for (const s of st.show.seasons) {
      html += `<div class="season">Saison ${s.number}<small>${s.episodes.length} épisodes</small></div>`;
      for (const e of s.episodes) {
        const isSeen = lp && lp.episode === e.path;
        html += `
          <div class="row ep${k === st.epFocus ? " focus" : ""}">
            <span class="num">${pad(k + 1, 3)}</span>
            <span class="mark"></span>
            <span class="title"><span class="code">S${pad(e.season, 2)} · E${pad(e.episode, 2)}</span>${e.title ? " · " + api.esc(e.title) : ""}</span>
            <span class="seen">${isSeen ? "vu le " + fmtDate(lp.at) : ""}</span>
            <span class="year"></span>
          </div>`;
        k++;
      }
    }
    els.list.innerHTML = html;
    keepVisible();
  }

  function keepVisible() {
    const row = els.list.querySelector(".row.focus");
    if (!row) return;
    const top = row.offsetTop - els.list.offsetTop, h = row.offsetHeight, vh = els.list.clientHeight;
    const cur = els.list.scrollTop;
    if (top < cur + h) els.list.scrollTop = Math.max(0, top - h);           // one row of context above
    else if (top + h > cur + vh - h) els.list.scrollTop = top + 2 * h - vh; // one row of context below
  }

  function renderFiche() {
    let item, ep = null;
    if (st.mode === "list") {
      item = visible()[st.focus];
    } else {
      item = st.show; ep = st.eps[st.epFocus];
    }
    if (!item) { els.fiche.innerHTML = ""; return; }
    const meta = model.meta(item);
    const cat = model.categories[st.tab];
    const idx = cat.items.indexOf(item);
    const wrap = document.createElement("div");
    wrap.className = "card";
    wrap.appendChild(api.posterEl(item));
    let head = `<div class="label">Fiche${idx >= 0 ? ` · ${api.esc(cat.label)} № ${pad(idx + 1, 3)}` : ""}</div>
      <h2>${api.esc(item.title)}</h2>`;
    const facts = [];
    if (item.year) facts.push(String(item.year));
    if (item.kind === "show") facts.push(`${item.seasons.length} saisons`, `${item.seasons.reduce((n, s) => n + s.episodes.length, 0)} épisodes`);
    if (meta && meta.runtime) facts.push(item.kind === "show" ? `${meta.runtime} min / ép.` : `${meta.runtime} min`);
    if (meta && meta.rating) facts.push(`${meta.rating.toFixed(1)} / 10`);
    head += `<div class="facts">${facts.map(api.esc).join("&nbsp;&nbsp;·&nbsp;&nbsp;")}</div>`;
    if (meta && meta.genres) head += `<div class="genres">${meta.genres.map(api.esc).join(" / ")}</div>`;
    if (ep) {
      head += `<div class="epline"><span class="code">S${pad(ep.season, 2)} · E${pad(ep.episode, 2)}</span>${ep.title ? `<span class="eptitle">${api.esc(ep.title)}</span>` : `<span class="eptitle muted">titre à venir</span>`}</div>`;
    } else if (item.kind === "show") {
      const nx = model.nextUnwatched(item);
      head += `<div class="epline"><span class="label">Reprendre</span><span class="code">S${pad(nx.season, 2)} · E${pad(nx.episode, 2)}</span></div>`;
    }
    const s = seen(item);
    if (s) head += `<div class="seenline">${api.esc(s)}</div>`;
    head += meta && meta.synopsis
      ? `<p class="synopsis">${api.esc(meta.synopsis)}</p>`
      : `<p class="synopsis muted">Fiche à compléter — métadonnées à venir.</p>`;
    const text = document.createElement("div");
    text.className = "text"; text.innerHTML = head;
    wrap.appendChild(text);
    els.fiche.innerHTML = ""; els.fiche.appendChild(wrap);
  }

  // ---------- navigation ----------
  function move(d) {
    if (st.mode === "list") {
      const n = visible().length; if (!n) return;
      st.focus = Math.min(n - 1, Math.max(0, st.focus + d));
      renderList(); renderFiche();
    } else {
      st.epFocus = Math.min(st.eps.length - 1, Math.max(0, st.epFocus + d));
      renderEpisodes(); renderFiche();
    }
  }
  function moveTo(i) { st.mode === "list" ? (st.focus = i) : (st.epFocus = i); move(0); }
  function switchTab(d) {
    st.tab = (st.tab + d + model.categories.length) % model.categories.length;
    st.focus = 0; renderAll();
  }
  function seasonJump(d) {
    const cur = st.eps[st.epFocus].season;
    const seasons = st.show.seasons.map(s => s.number);
    const i = seasons.indexOf(cur) + d;
    if (i < 0 || i >= seasons.length) return;
    st.epFocus = st.eps.findIndex(e => e.season === seasons[i]);
    move(0);
  }
  function openShow(show) {
    st.mode = "show"; st.show = show;
    st.eps = show.seasons.flatMap(s => s.episodes);
    const nx = model.nextUnwatched(show);
    st.epFocus = Math.max(0, st.eps.indexOf(nx));
    renderAll();
  }
  function closeShow() { st.mode = "list"; st.show = null; st.eps = []; renderAll(); }

  function onKey(e) {
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    const k = e.key;
    switch (k) {
      case "ArrowDown": move(1); return true;
      case "ArrowUp": move(-1); return true;
      case "PageDown": move(PAGE); return true;
      case "PageUp": move(-PAGE); return true;
      case "Home": moveTo(0); return true;
      case "End": moveTo((st.mode === "list" ? visible().length : st.eps.length) - 1); return true;
      case "ArrowRight": st.mode === "list" ? switchTab(1) : seasonJump(1); return true;
      case "ArrowLeft": st.mode === "list" ? switchTab(-1) : seasonJump(-1); return true;
      case "Enter": {
        if (st.mode === "list") {
          const it = visible()[st.focus]; if (!it) return true;
          it.kind === "show" ? openShow(it) : api.play(it);
        } else {
          api.play(st.eps[st.epFocus]);
        }
        return true;
      }
      case "Escape":
        if (st.mode === "show") closeShow();
        else if (st.query) { st.query = ""; st.focus = 0; renderAll(); }
        return true;
      case "Backspace":
        if (st.mode === "show") closeShow();
        else if (st.query) { st.query = st.query.slice(0, -1); st.focus = 0; renderAll(); }
        return true;
    }
    if (k.length === 1 && st.mode === "list") {
      st.query += k; st.focus = 0; renderAll(); return true;
    }
    return false;
  }

  window.OMV.register({
    id: "v2",
    name: "Filmothèque",
    thesis: "Le catalogue d'un vidéo-club, en fiches : une liste dense qu'on parcourt au clavier en un clin d'œil, la fiche complète à droite, aucun chrome inutile pour une famille qui sait ce qu'elle cherche.",
    mount, unmount, onKey,
  });
})();
