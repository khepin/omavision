(function(){
  const variants=window.OMV._v;
  const data=window.OMV_DATA, themes=window.OMV_THEMES;
  const items=[]; data.categories.forEach(c=>c.items.forEach(i=>{i.category=c.id;items.push(i);}));
  const strip=s=>s.normalize("NFD").replace(/[̀-ͯ]/g,"").toLowerCase();
  const model={
    categories:data.categories, items, state:data.state,
    meta:i=>i.meta||null,
    label:id=>(data.categories.find(c=>c.id===id)||{}).label||id,
    filter:(q,list)=>{q=strip(q.trim()); if(!q) return list||items; return (list||items).filter(i=>strip(i.title+" "+(i.year||"")+" "+(i.meta?.genres||[]).join(" ")).includes(q));},
    nextUnwatched:show=>{const lp=data.state.lastPlayed[show.id]; const eps=show.seasons.flatMap(s=>s.episodes); if(!lp) return eps[0]; const k=eps.findIndex(e=>e.path===lp.episode); return eps[Math.min(k+1,eps.length-1)];},
    continueWatching:()=>Object.entries(data.state.lastPlayed).sort((a,b)=>b[1].at.localeCompare(a[1].at)).map(([id])=>items.find(i=>i.id===id)).filter(Boolean),
  };
  const stage=document.getElementById("stage"), toast=document.getElementById("toast");
  let toastT;
  const api={
    posterEl(item,cls){const d=document.createElement("div");d.className="poster"+(item.kind==="show"?" show":"")+(cls?" "+cls:"");d.style.setProperty("--hue",item.hue);d.innerHTML=`<span class="ttl">${esc(item.title)}</span>${item.year?`<span class="year">${item.year}</span>`:""}`;return d;},
    play(x){const p=x.path||(x.seasons?model.nextUnwatched(x).path:"");show(`pre hook → mpv --fs --save-position-on-quit "${p}" → post hook`);},
    toast:show,
    esc,
  };
  function esc(s){return String(s).replace(/[&<>"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]));}
  function show(t){toast.textContent=t;toast.classList.add("show");clearTimeout(toastT);toastT=setTimeout(()=>toast.classList.remove("show"),2600);}
  // theme
  const sel=document.getElementById("theme");
  Object.keys(themes).forEach(k=>{const o=document.createElement("option");o.value=k;o.textContent=k;sel.appendChild(o);});
  function applyTheme(k){const t=themes[k];const r=document.documentElement.style;
    ["bg","surface","surface2","text","muted","accent","accent2","border"].forEach(n=>r.setProperty("--"+n,t[n]));
    document.documentElement.dataset.mode=t.mode; sel.value=k; try{localStorage.setItem("omv-theme",k)}catch(e){}
    if(current&&current.onTheme) current.onTheme(t);}
  sel.addEventListener("change",()=>applyTheme(sel.value));
  // variants
  const bar=document.getElementById("variants"), thesis=document.getElementById("thesis");
  let current=null, cur=-1;
  function select(i){ if(i===cur) return; if(current){try{current.unmount&&current.unmount()}catch(e){console.error(e)} stage.innerHTML=""; stage.className="stage";}
    cur=i; current=variants[i]; stage.classList.add(current.id); thesis.textContent=current.name+" — "+current.thesis;
    [...bar.children].forEach((b,j)=>b.setAttribute("aria-pressed",j===i));
    try{current.mount(stage,model,api)}catch(e){console.error(e);show("mount error: "+e.message)}
    try{localStorage.setItem("omv-variant",i)}catch(e){} }
  variants.forEach((v,i)=>{const b=document.createElement("button");b.textContent=(i+1)+" "+v.name;b.addEventListener("click",()=>{select(i);stage.focus()});bar.appendChild(b);});
  // keys: [ ] switch variant, \ cycle theme, everything else to the variant
  document.addEventListener("keydown",e=>{
    if(e.target===sel) return;
    if(e.key==="["){select((cur+variants.length-1)%variants.length);e.preventDefault();return}
    if(e.key==="]"){select((cur+1)%variants.length);e.preventDefault();return}
    if(e.key==="\\"){const ks=Object.keys(themes);applyTheme(ks[(ks.indexOf(sel.value)+1)%ks.length]);e.preventDefault();return}
    if(current&&current.onKey&&current.onKey(e)) e.preventDefault();
  });
  // scale stage to viewport
  const vp=document.getElementById("viewport");
  function fit(){const s=vp.clientWidth/1920;stage.style.transform=`scale(${s})`;}
  new ResizeObserver(fit).observe(vp); fit();
  let t0="tokyo-night", v0=0; try{t0=localStorage.getItem("omv-theme")||t0; v0=+localStorage.getItem("omv-variant")||0}catch(e){}
  applyTheme(themes[t0]?t0:Object.keys(themes)[0]); if(variants.length) select(Math.min(v0,variants.length-1)); stage.focus();
})();
