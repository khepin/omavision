(function(){
  const CARD=9.5, GAP=1, PAD=4; // rem
  let root, model, api, rows=[], r=0, c=0, q="", overlay=null, els={};
  function h(tag,cls,html){const e=document.createElement(tag);if(cls)e.className=cls;if(html!=null)e.innerHTML=html;return e;}
  function fonts(){if(document.getElementById("omv-v1-fonts"))return;const l=document.createElement("link");l.id="omv-v1-fonts";l.rel="stylesheet";l.href="https://fonts.googleapis.com/css2?family=Bebas+Neue&family=Source+Sans+3:wght@400;600&display=swap";document.head.appendChild(l);}
  function buildRows(){
    const all=[];
    const cw=model.continueWatching();
    if(cw.length) all.push({id:"resume",label:"Reprendre",items:cw});
    model.categories.forEach(cat=>all.push({id:cat.id,label:cat.label,items:cat.items}));
    return all;
  }
  function mount(rt,m,a){
    root=rt;model=m;api=a;fonts();q="";overlay=null;
    els.backdrop=h("div","backdrop");
    els.hero=h("div","hero");
    els.stub=h("div","stub",'<span>Filtre</span><span class="q"></span>');
    els.rows=h("div","rows"); els.inner=h("div","rows-inner"); els.rows.appendChild(els.inner);
    els.empty=h("div","empty","Aucun titre ne correspond."); els.empty.style.display="none";
    root.append(els.backdrop,els.hero,els.stub,els.rows,els.empty);
    rows=buildRows().map(row=>{
      const el=h("section","row"); el.appendChild(h("h2","",api.esc(row.label)+'<span class="n"></span>'));
      const strip=h("div","strip"); el.appendChild(strip); els.inner.appendChild(el);
      return {...row,el,strip,cards:[],visible:row.items};
    });
    r=0;c=0;
    // preselect: continue-watching row, first item
    applyFilter();
  }
  function applyFilter(){
    let anyVisible=false;
    rows.forEach(row=>{
      row.visible=q?model.filter(q,row.items):row.items;
      row.strip.innerHTML="";row.cards=[];
      row.visible.forEach(it=>{const card=h("div","card");card.appendChild(api.posterEl(it));card.appendChild(h("div","cap",api.esc(it.title)+(it.year?" · "+it.year:"")));row.strip.appendChild(card);row.cards.push(card);});
      row.el.querySelector(".n").textContent=row.visible.length;
      row.el.classList.toggle("hidden",row.visible.length===0);
      if(row.visible.length) anyVisible=true;
    });
    els.stub.classList.toggle("active",!!q); els.stub.querySelector(".q").textContent=q;
    els.empty.style.display=anyVisible?"none":"block";
    const vis=visibleRows(); if(!vis.includes(r)) r=vis[0]??0; c=Math.min(c,Math.max(0,(rows[r]?.visible.length||1)-1));
    render();
  }
  function visibleRows(){return rows.map((row,i)=>row.visible.length?i:-1).filter(i=>i>=0);}
  function focused(){return rows[r]&&rows[r].visible[c];}
  function render(){
    rows.forEach((row,i)=>{
      row.el.classList.toggle("active",i===r);
      row.cards.forEach((cd,j)=>cd.classList.toggle("focus",i===r&&j===c));
      const off=i===r?Math.max(0,c-1)*(CARD+GAP):0; // keep focused card second from the left
      const maxOff=Math.max(0,row.visible.length*(CARD+GAP)-(80-PAD*2));
      row.strip.style.transform=`translateX(${-Math.min(off,maxOff)}rem)`;
    });
    // vertical: keep active row near top of rows area
    const vis=visibleRows(); const k=Math.max(0,vis.indexOf(r)); const rowH=20.5; // rem per row incl. gap
    els.inner.style.transform=`translateY(${-k*rowH}rem)`;
    const it=focused(); if(!it){els.hero.innerHTML="";return;}
    root.style.setProperty("--hue",it.hue);
    const meta=model.meta(it);
    const facts=[];
    if(it.year) facts.push(`<b>${it.year}</b>`);
    if(it.kind==="show"){const n=it.seasons.reduce((s,x)=>s+x.episodes.length,0);facts.push(`${it.seasons.length} saisons`,`${n} épisodes`);}
    if(meta){if(meta.runtime&&it.kind!=="show")facts.push(`${meta.runtime} min`);if(meta.rating)facts.push(`<span class="rating">★ ${meta.rating.toFixed(1)}</span>`);if(meta.genres)facts.push(meta.genres.join(" · "));}
    const rowLabel=rows[r].label;
    let extra="";
    if(it.kind==="show"){const nx=model.nextUnwatched(it);extra=` · reprendre S${String(nx.season).padStart(2,"0")}E${String(nx.episode).padStart(2,"0")}`;}
    els.hero.innerHTML=`<div class="eyebrow">${api.esc(rowLabel)}${extra}</div><h1>${api.esc(it.title)}</h1><div class="facts">${facts.join('<span>·</span>')}</div>`+
      (meta?`<p class="synopsis">${api.esc(meta.synopsis)}</p>`:`<p class="synopsis pending">Métadonnées à venir — affiche et synopsis seront récupérés depuis TMDB.</p>`);
  }
  // ---- show overlay
  function openShow(show){
    const seasons=show.seasons; const nx=model.nextUnwatched(show); const lp=model.state.lastPlayed[show.id];
    let s=seasons.findIndex(x=>x.number===nx.season), e=seasons[s].episodes.findIndex(x=>x.path===nx.path);
    const ov=h("div","overlay"); ov.style.setProperty("--hue",show.hue);
    const head=h("div","head"); head.appendChild(api.posterEl(show));
    const n=seasons.reduce((a,x)=>a+x.episodes.length,0);
    head.appendChild(h("div","",`<h1>${api.esc(show.title)}</h1><div class="sub"><b>${seasons.length} saisons</b> · ${n} épisodes${lp?` · dernier vu S${String(seasons.find(x=>x.episodes.some(y=>y.path===lp.episode))?.number).padStart(2,"0")}`:""}</div>`));
    head.appendChild(h("div","hint","Entrée lire · Échap retour"));
    ov.appendChild(head);
    const cols=h("div","seasons");
    const seasonEls=seasons.map(sn=>{const col=h("div","season");col.appendChild(h("h3","",`Saison ${String(sn.number).padStart(2,"0")}`));const list=h("div","eps");
      const eps=sn.episodes.map(ep=>{const el=h("div","ep",`<span class="num">${String(ep.episode).padStart(2,"0")}</span><span class="t${ep.title?"":" none"}">${ep.title?api.esc(ep.title):"Titre à venir"}</span>`);if(ep.path===nx.path)el.classList.add("next");list.appendChild(el);return el;});
      col.appendChild(list);cols.appendChild(col);return {col,list,eps};});
    ov.appendChild(cols); root.appendChild(ov);
    overlay={show,seasons,s,e,seasonEls,el:ov};
    renderOverlay();
  }
  function renderOverlay(){
    const o=overlay; o.seasonEls.forEach((se,i)=>{se.col.classList.toggle("active",i===o.s);se.eps.forEach((el,j)=>el.classList.toggle("focus",i===o.s&&j===o.e));
      // scroll column so focused ep visible (24 rows max fit ~ 22)
      const rowH=1.35*1+.64+.2; const fit=Math.floor(28/rowH); const first=i===o.s?Math.max(0,o.e-fit+2):0; se.list.style.transform=`translateY(${-first*rowH}rem)`;});
  }
  function closeOverlay(){if(overlay){overlay.el.remove();overlay=null;}}
  function onKey(ev){
    const k=ev.key;
    if(overlay){
      const o=overlay;
      if(k==="Escape"||k==="Backspace"){closeOverlay();return true;}
      if(k==="ArrowDown"){o.e=Math.min(o.e+1,o.seasons[o.s].episodes.length-1);renderOverlay();return true;}
      if(k==="ArrowUp"){o.e=Math.max(o.e-1,0);renderOverlay();return true;}
      if(k==="ArrowRight"){o.s=Math.min(o.s+1,o.seasons.length-1);o.e=Math.min(o.e,o.seasons[o.s].episodes.length-1);renderOverlay();return true;}
      if(k==="ArrowLeft"){o.s=Math.max(o.s-1,0);o.e=Math.min(o.e,o.seasons[o.s].episodes.length-1);renderOverlay();return true;}
      if(k==="Enter"){api.play(o.seasons[o.s].episodes[o.e]);return true;}
      return false;
    }
    const vis=visibleRows();
    if(k==="ArrowRight"){if(rows[r]&&c<rows[r].visible.length-1){c++;render();}return true;}
    if(k==="ArrowLeft"){if(c>0){c--;render();}return true;}
    if(k==="ArrowDown"){const i=vis.indexOf(r);if(i<vis.length-1){r=vis[i+1];c=Math.min(c,rows[r].visible.length-1);render();}return true;}
    if(k==="ArrowUp"){const i=vis.indexOf(r);if(i>0){r=vis[i-1];c=Math.min(c,rows[r].visible.length-1);render();}return true;}
    if(k==="Enter"){const it=focused();if(!it)return true;if(it.kind==="show")openShow(it);else api.play(it);return true;}
    if(k==="Escape"){if(q){q="";applyFilter();}return true;}
    if(k==="Backspace"){if(q){q=q.slice(0,-1);applyFilter();}return true;}
    if(k.length===1&&!ev.ctrlKey&&!ev.metaKey&&!ev.altKey){if(k===" "&&!q)return true;q+=k;c=0;applyFilter();return true;}
    return false;
  }
  window.OMV.register({id:"v1",name:"Marquee",thesis:"Une façade de cinéma : l'affiche en vedette teinte tout l'écran, les rangées défilent dessous comme des programmes, et la couleur vient du thème Omarchy.",
    mount,unmount(){rows=[];overlay=null;els={};},onKey});
})();
