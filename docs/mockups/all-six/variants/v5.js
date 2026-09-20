(function(){
  const S={pane:1,cat:0,row:0,mode:"NORMAL",filter:"",expanded:new Set(),rows:[]};
  let root,model,api,els={};
  const esc=s=>window.OMV.__esc?window.OMV.__esc(s):String(s).replace(/[&<>"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]));
  const pad2=n=>String(n).padStart(2,"0");
  function font(){if(document.getElementById("omv-v5-font"))return;const l=document.createElement("link");l.id="omv-v5-font";l.rel="stylesheet";l.href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700&display=swap";document.head.appendChild(l);}
  function lastPlayedIds(){return new Set(model.continueWatching().map(i=>i.id));}
  function buildRows(){
    const cat=model.categories[S.cat]; const items=model.filter(S.filter,cat.items); const rows=[];
    items.forEach(it=>{rows.push({type:"item",item:it});
      if(it.kind==="show"&&S.expanded.has(it.id)){
        it.seasons.forEach((se,si)=>{const lastS=si===it.seasons.length-1;
          rows.push({type:"season",item:it,season:se,glyph:(lastS?"└─":"├─")});
          se.episodes.forEach((ep,ei)=>{const lastE=ei===se.episodes.length-1;
            rows.push({type:"ep",item:it,ep,glyph:(lastS?"   ":"│  ")+(lastE?"└─":"├─")});});});}});
    S.rows=rows; if(S.row>=rows.length)S.row=Math.max(0,rows.length-1);
  }
  function build(){
    root.innerHTML=`
      <section class="pane cats"><span class="title"> catégories </span><div class="body" id="v5cats"></div></section>
      <section class="pane list"><span class="title" id="v5ltitle"> films </span><div class="body" id="v5list"></div></section>
      <section class="pane detail"><span class="title"> détails </span><div class="body" id="v5detail"></div></section>
      <div class="status"><span class="seg mode" id="v5mode">NORMAL</span><span class="seg fil" id="v5fil"></span><span class="seg hint" id="v5hint"></span><span class="seg pos" id="v5pos"></span></div>`;
    ["v5cats","v5list","v5ltitle","v5detail","v5mode","v5fil","v5hint","v5pos"].forEach(id=>els[id]=root.querySelector("#"+id));
    render();
  }
  function render(){
    buildRows();
    root.querySelectorAll(".pane").forEach((p,i)=>p.classList.toggle("active",i===S.pane));
    els.v5cats.innerHTML=model.categories.map((c,i)=>{const n=S.filter?model.filter(S.filter,c.items).length:c.items.length;
      return `<div class="row${i===S.cat?" cur":""}"><span class="t">${esc(c.label.toLowerCase())}</span><span class="n">${n}</span></div>`;}).join("");
    const cat=model.categories[S.cat];
    els.v5ltitle.textContent=` ${cat.label.toLowerCase()} (${S.rows.filter(r=>r.type==="item").length}) `;
    const lp=lastPlayedIds();
    els.v5list.innerHTML=S.rows.length?S.rows.map((r,i)=>{const cur=i===S.row?" cur":"";
      if(r.type==="item"){const m=lp.has(r.item.id)?`<span class="mark">▶ </span>`:"  ";const tail=r.item.kind==="show"?(S.expanded.has(r.item.id)?"▾":"▸"):(r.item.year||"");
        return `<div class="row${cur}"><span class="t">${m}${esc(r.item.title)}</span><span class="y">${tail}</span></div>`;}
      if(r.type==="season")return `<div class="row tree${cur}"><span class="t">  ${r.glyph} <b>saison ${pad2(r.season.number)}</b></span><span class="y">${r.season.episodes.length} ép.</span></div>`;
      const nx=model.nextUnwatched(r.item);const m=nx&&nx.path===r.ep.path?`<span class="mark">▶</span>`:" ";
      return `<div class="row tree${cur}"><span class="t">  ${r.glyph} ${m}<b>S${pad2(r.ep.season)}E${pad2(r.ep.episode)}</b>  ${esc(r.ep.title||"")}</span><span class="y"></span></div>`;
    }).join(""):`<div class="row"><span class="t">  aucun résultat pour « ${esc(S.filter)} »</span></div>`;
    const curEl=els.v5list.querySelector(".row.cur"); if(curEl)curEl.scrollIntoView({block:"nearest"});
    renderDetail(); renderStatus();
  }
  function renderDetail(){
    const r=S.rows[S.row]; if(!r){els.v5detail.innerHTML="";return;}
    const it=r.item, meta=model.meta(it); const kv=[];
    kv.push(["titre",esc(it.title)]);
    if(r.type==="ep"){kv.push(["épisode",`S${pad2(r.ep.season)}E${pad2(r.ep.episode)}${r.ep.title?"  "+esc(r.ep.title):""}`]);}
    if(r.type==="season"){kv.push(["saison",`${pad2(r.season.number)}  (${r.season.episodes.length} épisodes)`]);}
    if(it.year)kv.push(["année",String(it.year)]);
    if(it.kind==="show")kv.push(["épisodes",String(it.seasons.reduce((a,s)=>a+s.episodes.length,0))+" sur "+it.seasons.length+" saisons"]);
    if(meta){if(meta.runtime)kv.push(["durée",it.kind==="show"?meta.runtime+" min / ép.":fmtDur(meta.runtime)]);if(meta.rating)kv.push(["note",`<span class="acc">${"★".repeat(Math.round(meta.rating/2))}</span> ${meta.rating.toFixed(1)}`]);if(meta.genres)kv.push(["genres",esc(meta.genres.join(", ").toLowerCase())]);}
    else kv.push(["méta",`<span class="note">métadonnées à venir</span>`]);
    const lp=model.state.lastPlayed[it.id]; if(lp)kv.push(["vu",esc(lp.at.slice(0,10))+(lp.episode?"  "+esc(lp.episode.match(/S\d+E\d+/)?.[0]||""):"")]);
    const path=r.type==="ep"?r.ep.path:(it.path||(it.kind==="show"?model.nextUnwatched(it).path:""));
    kv.push(["fichier",`<span class="path">${esc(path)}</span>`]);
    const poster=api.posterEl(it); 
    els.v5detail.innerHTML=`<div class="kv">${kv.map(([k,v])=>`<span class="k">${k}</span><span class="v">${v}</span>`).join("")}</div>${meta&&meta.synopsis?`<div class="syn">${esc(meta.synopsis)}</div>`:""}`;
    els.v5detail.prepend(poster);
  }
  function fmtDur(m){return `${Math.floor(m/60)}h${pad2(m%60)}`;}
  function renderStatus(){
    els.v5mode.textContent=" "+S.mode+" "; els.v5mode.classList.toggle("filter",S.mode==="FILTER");
    els.v5fil.innerHTML=(S.filter||S.mode==="FILTER")?`/${esc(S.filter)}${S.mode==="FILTER"?'<span class="cur"></span>':""}`:"";
    els.v5hint.innerHTML=S.mode==="FILTER"?`<b>esc</b> normal  <b>enter</b> ouvrir  <b>⌫</b> effacer`:`<b>j/k</b> déplacer  <b>h/l</b> panneau  <b>enter</b> ${hintEnter()}  <b>/</b> filtrer  <b>esc</b> ${S.filter?"effacer le filtre":"replier"}`;
    const n=S.rows.length; els.v5pos.textContent=S.pane===0?` ${S.cat+1}/${model.categories.length} `:` ${n?S.row+1:0}/${n} `;
  }
  function hintEnter(){const r=S.rows[S.row];if(!r)return"ouvrir";if(S.pane===0)return"liste";if(r.type==="item"&&r.item.kind==="show")return S.expanded.has(r.item.id)?"replier":"déplier";return"lire";}
  function move(d){
    if(S.pane===0){S.cat=(S.cat+d+model.categories.length)%model.categories.length;S.row=0;}
    else{const n=S.rows.length;if(!n)return;S.row=(S.row+d+n)%n;}
    render();
  }
  function enter(){
    if(S.pane===0){S.pane=1;S.row=0;render();return;}
    const r=S.rows[S.row]; if(!r)return;
    if(r.type==="item"){const it=r.item;
      if(it.kind==="show"){if(S.expanded.has(it.id))S.expanded.delete(it.id);else{S.expanded.add(it.id);buildRows();const nx=model.nextUnwatched(it);const k=S.rows.findIndex(x=>x.type==="ep"&&x.item===it&&x.ep.path===nx.path);if(k>=0)S.row=k;}render();return;}
      api.play(it);return;}
    if(r.type==="season"){const k=S.rows.findIndex((x,i)=>i>S.row&&x.type==="ep");if(k>=0){S.row=k;render();}return;}
    api.play(r.ep);
  }
  function escape(){
    if(S.mode==="FILTER"){S.mode="NORMAL";render();return;}
    if(S.filter){S.filter="";S.row=0;render();return;}
    const r=S.rows[S.row]; if(r&&r.item.kind==="show"&&S.expanded.has(r.item.id)){S.expanded.delete(r.item.id);buildRows();S.row=S.rows.findIndex(x=>x.item===r.item);render();return;}
    if(S.pane!==1){S.pane=1;render();}
  }
  function setPane(p){S.pane=Math.max(0,Math.min(2,p));render();}
  function onKey(e){
    if(e.ctrlKey||e.metaKey||e.altKey)return false;
    const k=e.key;
    if(S.mode==="FILTER"){
      if(k==="Escape"){escape();return true;}
      if(k==="Enter"){S.mode="NORMAL";enter();return true;}
      if(k==="Backspace"){S.filter=S.filter.slice(0,-1);S.row=0;render();return true;}
      if(k==="ArrowDown"){move(1);return true;} if(k==="ArrowUp"){move(-1);return true;}
      if(k.length===1){S.filter+=k;S.row=0;S.pane=1;render();return true;}
      return false;
    }
    switch(k){
      case"ArrowDown":case"j":move(1);return true;
      case"ArrowUp":case"k":move(-1);return true;
      case"ArrowLeft":case"h":setPane(S.pane-1);return true;
      case"ArrowRight":case"l":setPane(S.pane+1);return true;
      case"Tab":setPane(e.shiftKey?(S.pane+2)%3:(S.pane+1)%3);return true;
      case"Enter":enter();return true;
      case"Escape":escape();return true;
      case"Backspace":if(S.filter){S.mode="FILTER";S.filter=S.filter.slice(0,-1);S.row=0;render();}else escape();return true;
      case"/":S.mode="FILTER";S.pane=1;render();return true;
    }
    if(k.length===1&&/[\p{L}\p{N} '’-]/u.test(k)){S.mode="FILTER";S.filter+=k;S.row=0;S.pane=1;render();return true;}
    return false;
  }
  window.OMV.register({
    id:"v5",name:"Terminal",
    thesis:"un TUI à l'échelle du téléviseur : trois panneaux encadrés, une seule fonte mono, les couleurs exactes du terminal Omarchy, une ligne d'état à la vim.",
    mount(r,m,a){root=r;model=m;api=a;font();S.pane=1;S.cat=0;S.row=0;S.mode="NORMAL";S.filter="";S.expanded=new Set();build();},
    unmount(){els={};},
    onKey,
  });
})();
