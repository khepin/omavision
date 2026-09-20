(function(){
  const COLS=6, EP_COLS=4;
  let root, model, api;
  let S; // state
  let els={};
  let fontLoaded=false;

  function reset(){
    S={view:"grid",focus:"grid",cat:0,idx:0,chip:0,query:"",show:null,season:0,ep:0,list:[],scopeAll:false};
  }
  function currentItems(){
    const cat=model.categories[S.cat];
    if(!S.query){S.scopeAll=false;return cat.items;}
    let r=model.filter(S.query,cat.items);
    if(r.length){S.scopeAll=false;return r;}
    S.scopeAll=true;return model.filter(S.query);
  }
  function lastPlayedLabel(item){
    const lp=model.state.lastPlayed[item.id]; if(!lp) return null;
    if(item.kind==="show"){const e=model.nextUnwatched(item);return e?`S${pad(e.season)}E${pad(e.episode)}`:"vu";}
    return "repris";
  }
  const pad=n=>String(n).padStart(2,"0");

  function mount(r,m,a){
    root=r;model=m;api=a;reset();
    if(!fontLoaded){const l=document.createElement("link");l.rel="stylesheet";l.href="https://fonts.googleapis.com/css2?family=Nunito:wght@400;600;700;800&display=swap";document.head.appendChild(l);fontLoaded=true;}
    render();
  }
  function unmount(){els={};}

  function render(){
    root.innerHTML="";
    if(S.view==="show") return renderShow();
    const top=document.createElement("div");top.className="top";
    const chips=document.createElement("ul");chips.className="chips";
    model.categories.forEach((c,i)=>{const li=document.createElement("li");li.className="chip";li.innerHTML=`${api.esc(c.label)}<span class="n">${c.items.length}</span>`;chips.appendChild(li);});
    const search=document.createElement("div");search.className="search";search.innerHTML=`<span class="q"></span><span class="scope"></span>`;
    top.append(chips,search);
    const shelf=document.createElement("div");shelf.className="shelf";
    const grid=document.createElement("div");grid.className="grid";
    shelf.appendChild(grid);
    const sheet=document.createElement("div");sheet.className="sheet";
    root.append(top,shelf,sheet);
    els={chips:[...chips.children],search,grid,sheet,shelf};
    renderGrid();
  }
  function renderGrid(){
    S.list=currentItems();
    if(S.idx>=S.list.length) S.idx=Math.max(0,S.list.length-1);
    const g=els.grid;g.innerHTML="";
    if(!S.list.length){const e=document.createElement("div");e.className="empty";e.textContent=`Rien pour « ${S.query} »`;g.appendChild(e);}
    S.list.forEach(item=>{
      const t=document.createElement("div");t.className="tile";
      t.appendChild(api.posterEl(item));
      const seen=lastPlayedLabel(item); if(seen){const s=document.createElement("span");s.className="seen";s.textContent=seen;t.appendChild(s);}
      const cap=document.createElement("div");cap.className="cap";cap.textContent=item.title;t.appendChild(cap);
      g.appendChild(t);
    });
    updateFocus();
  }
  function updateFocus(){
    els.chips.forEach((c,i)=>{c.classList.toggle("active",i===S.cat);c.classList.toggle("focus",S.focus==="chips"&&i===S.chip);});
    const tiles=[...els.grid.querySelectorAll(".tile")];
    tiles.forEach((t,i)=>t.classList.toggle("focus",S.focus==="grid"&&i===S.idx));
    els.search.classList.toggle("on",!!S.query);
    els.search.querySelector(".q").textContent=S.query;
    els.search.querySelector(".scope").textContent=S.scopeAll?"· toute la bibliothèque":`· ${model.categories[S.cat].label}`;
    // keep the focused row at the top once past the first row
    const row=Math.floor(S.idx/COLS);
    const t0=tiles[0];
    let pitch=0; if(tiles[COLS]) pitch=tiles[COLS].offsetTop-t0.offsetTop;
    const rowsTotal=Math.ceil(tiles.length/COLS);
    const scrollRow=S.focus==="grid"?Math.min(row,Math.max(0,rowsTotal-2)):0;
    els.grid.style.transform=`translateY(${-scrollRow*pitch}px)`;
    renderSheet(S.focus==="grid"?S.list[S.idx]:null);
  }
  function renderSheet(item){
    const sh=els.sheet;
    if(!item){sh.classList.remove("up");return;}
    sh.style.setProperty("--hue",item.hue);
    const meta=model.meta(item);
    const facts=[];
    if(item.year) facts.push(`<b>${item.year}</b>`);
    if(item.kind==="show"){const n=item.seasons.reduce((a,s)=>a+s.episodes.length,0);facts.push(`${item.seasons.length} saisons`,`${n} épisodes`);}
    if(meta){ if(meta.runtime&&item.kind!=="show") facts.push(`${meta.runtime} min`); if(meta.rating) facts.push(`<b>★ ${meta.rating.toFixed(1)}</b>`); if(meta.genres) facts.push(meta.genres.join(" · ")); }
    facts.push(`<span>${api.esc(model.label(item.category))}</span>`);
    let cta="Lecture";
    if(item.kind==="show"){const e=model.nextUnwatched(item);cta=model.state.lastPlayed[item.id]?`Reprendre S${pad(e.season)}E${pad(e.episode)}`:"Ouvrir";}
    else if(model.state.lastPlayed[item.id]) cta="Reprendre";
    sh.innerHTML=`<div><h2>${api.esc(item.title)}</h2><div class="facts">${facts.join("<span>·</span>")}</div>${meta?`<p>${api.esc(meta.synopsis)}</p>`:`<p class="todo">Métadonnées à venir — affiche et résumé seront récupérés automatiquement.</p>`}</div><div class="cta">${cta}<span class="key">↵</span></div>`;
    sh.classList.add("up");
  }

  // ---- show view
  function renderShow(){
    const item=S.show;
    const v=document.createElement("div");v.className="showv";
    v.appendChild(api.posterEl(item));
    const meta=model.meta(item);
    const head=document.createElement("div");head.className="head";
    const n=item.seasons.reduce((a,s)=>a+s.episodes.length,0);
    const nu=model.nextUnwatched(item);
    head.innerHTML=`<h2>${api.esc(item.title)}</h2><div class="facts">${item.seasons.length} saisons · ${n} épisodes${meta&&meta.rating?` · ★ ${meta.rating.toFixed(1)}`:""}${nu?` · à suivre <b>S${pad(nu.season)}E${pad(nu.episode)}</b>`:""}</div>`;
    const seasons=document.createElement("ul");seasons.className="seasons";
    item.seasons.forEach(s=>{const li=document.createElement("li");li.className="chip";li.textContent=`Saison ${s.number}`;seasons.appendChild(li);});
    const wrap=document.createElement("div");wrap.className="eps-wrap";
    const eps=document.createElement("div");eps.className="eps";wrap.appendChild(eps);
    const back=document.createElement("div");back.className="back";back.innerHTML=`<span class="key">⌫</span>Retour`;
    v.append(head,seasons,wrap,back);
    root.appendChild(v);
    els={seasons:[...seasons.children],eps,wrap};
    renderEpisodes();
  }
  function renderEpisodes(){
    const season=S.show.seasons[S.season];
    const nu=model.nextUnwatched(S.show);
    els.eps.innerHTML="";
    season.episodes.forEach(e=>{
      const d=document.createElement("div");d.className="ep"+(nu&&nu.path===e.path?" next":"");
      d.innerHTML=`<span class="num">S${pad(e.season)}E${pad(e.episode)}</span><span class="t${e.title?"":" todo"}">${e.title?api.esc(e.title):"Titre à venir"}</span>`;
      els.eps.appendChild(d);
    });
    updateShowFocus();
  }
  function updateShowFocus(){
    els.seasons.forEach((c,i)=>{c.classList.toggle("active",i===S.season);c.classList.toggle("focus",S.focus==="seasons"&&i===S.season);});
    const eps=[...els.eps.children];
    eps.forEach((e,i)=>e.classList.toggle("focus",S.focus==="episodes"&&i===S.ep));
    const row=Math.floor(S.ep/EP_COLS);
    let pitch=0; if(eps[EP_COLS]) pitch=eps[EP_COLS].offsetTop-eps[0].offsetTop;
    const visible=Math.max(1,Math.floor(els.wrap.clientHeight/(pitch||1)));
    const rowsTotal=Math.ceil(eps.length/EP_COLS);
    const scrollRow=S.focus==="episodes"?Math.min(Math.max(0,row-1),Math.max(0,rowsTotal-visible)):0;
    els.eps.style.transform=`translateY(${-scrollRow*pitch}px)`;
  }
  function openShow(item){
    S.view="show";S.show=item;S.focus="episodes";
    const nu=model.nextUnwatched(item);
    S.season=Math.max(0,item.seasons.findIndex(s=>s.episodes.some(e=>e.path===nu.path)));
    S.ep=Math.max(0,item.seasons[S.season].episodes.findIndex(e=>e.path===nu.path));
    render();
  }
  function closeShow(){S.view="grid";S.show=null;S.focus="grid";render();}

  // ---- keys
  function onKey(e){
    const k=e.key;
    if(S.view==="show") return keyShow(k,e);
    const isChar=k.length===1&&!e.ctrlKey&&!e.metaKey&&!e.altKey;
    if(isChar){S.query+=k;S.focus="grid";S.idx=0;renderGrid();return true;}
    if(k==="Backspace"){ if(S.query){S.query=S.query.slice(0,-1);S.idx=0;renderGrid();} return true;}
    if(k==="Escape"){ if(S.query){S.query="";S.idx=0;renderGrid();} return true;}
    if(S.focus==="chips"){
      if(k==="ArrowLeft"||k==="ArrowRight"){S.chip=(S.chip+(k==="ArrowLeft"?-1:1)+model.categories.length)%model.categories.length;S.cat=S.chip;S.idx=0;S.query="";renderGrid();return true;}
      if(k==="ArrowDown"||k==="Enter"){S.focus="grid";updateFocus();return true;}
      return k==="ArrowUp";
    }
    const n=S.list.length; if(!n){ if(k==="ArrowUp"){S.focus="chips";S.chip=S.cat;updateFocus();} return true;}
    if(k==="ArrowLeft"){S.idx=Math.max(0,S.idx-1);updateFocus();return true;}
    if(k==="ArrowRight"){S.idx=Math.min(n-1,S.idx+1);updateFocus();return true;}
    if(k==="ArrowUp"){ if(S.idx<COLS){S.focus="chips";S.chip=S.cat;} else S.idx-=COLS; updateFocus();return true;}
    if(k==="ArrowDown"){ if(S.idx+COLS<n) S.idx+=COLS; else if(Math.floor(S.idx/COLS)<Math.floor((n-1)/COLS)) S.idx=n-1; updateFocus();return true;}
    if(k==="Enter"){const it=S.list[S.idx]; if(!it) return true; if(it.kind==="show") openShow(it); else api.play(it); return true;}
    return false;
  }
  function keyShow(k,e){
    const isChar=k.length===1&&!e.ctrlKey&&!e.metaKey&&!e.altKey;
    if(isChar){closeShow();S.query=k;S.idx=0;renderGrid();return true;}
    if(k==="Escape"||k==="Backspace"){closeShow();return true;}
    const eps=S.show.seasons[S.season].episodes, n=eps.length;
    if(S.focus==="seasons"){
      if(k==="ArrowLeft"||k==="ArrowRight"){S.season=Math.max(0,Math.min(S.show.seasons.length-1,S.season+(k==="ArrowLeft"?-1:1)));S.ep=0;renderEpisodes();return true;}
      if(k==="ArrowDown"||k==="Enter"){S.focus="episodes";updateShowFocus();return true;}
      return true;
    }
    if(k==="ArrowLeft"){S.ep=Math.max(0,S.ep-1);updateShowFocus();return true;}
    if(k==="ArrowRight"){S.ep=Math.min(n-1,S.ep+1);updateShowFocus();return true;}
    if(k==="ArrowUp"){ if(S.ep<EP_COLS) S.focus="seasons"; else S.ep-=EP_COLS; updateShowFocus();return true;}
    if(k==="ArrowDown"){ if(S.ep+EP_COLS<n) S.ep+=EP_COLS; else if(Math.floor(S.ep/EP_COLS)<Math.floor((n-1)/EP_COLS)) S.ep=n-1; updateShowFocus();return true;}
    if(k==="Enter"){api.play(eps[S.ep]);return true;}
    return false;
  }

  window.OMV.register({
    id:"v3",name:"Shelf",
    thesis:"Une étagère douce et lisible du canapé : grandes pastilles de catégories, affiches arrondies avec halo, et une fiche qui monte du bas — pensée pour une famille et des enfants qui choisissent seuls.",
    mount,unmount,onKey,
  });
})();
