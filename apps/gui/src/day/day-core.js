/**
 * The machine card's 24-hour block — **`day-core.js` moved across, not
 * written again.**
 *
 * Every drawing function below is the code from
 * `/private/tmp/khor-machine-face/card-whole.html` (`KhorDay`, lines
 * 1062–1804), which is itself `v5-onepanel.html`'s panel lifted whole.
 * It is not a picture of a design: it is the design, already running,
 * already argued over. The face used to hold a React re-implementation
 * written from the spec's *description* of it, and every difference
 * between the two came out of that gap — the ridge drawn with the
 * vertical-hair fill its own author had measured and thrown away, the
 * parallel coordinates with no axes, geometry off by whatever the
 * prose did not pin. 「artifacts 已经写好代码画了,为什么还要自己画一遍」.
 *
 * So React owns the box and the module owns everything inside it. There
 * is no second implementation to keep in step, and 「it looks like the
 * prototype」 stops being a thing to verify item by item: it is the same
 * code.
 *
 * # The four things that did change, and nothing else
 *
 * 1. **Where the numbers come from.** The original generated a day; this
 *    takes `CpuDay`'s five series. `toTracks` is that adapter and it is
 *    the whole of it — the drawing functions' signatures are untouched.
 * 2. **Where the words come from.** No Chinese lives in this repo's
 *    code; every literal the original wrote inline is now a field on
 *    `WORDS`, filled from `crates/catalog/zh.toml`. (It was `W` for a
 *    day, and that name collided with the drawing functions' width
 *    parameter — see the declaration.)
 * 3. **Where the clock comes from.** The original's fixture pinned
 *    「now」 at 20:00 and could write its hour ticks as constants. A real
 *    window starts wherever the machine was last heard from, so
 *    `iOfHour` and `hhmm` read the window this mount was given. **The
 *    tick constants themselves are untouched** — `TICKS_W` is still
 *    every third hour on the clock and `TICKS_N` every sixth.
 * 4. **The ramp is the eight the ledger settled on** (`[3, 8, 20, 45,
 *    70, 80, 90]`). The prototype's *stylesheet* already declares eight
 *    steps `--r1..--r8`; its `BANDS` array is the half that had not
 *    caught up. turing's video memory spends a whole day between 74.6
 *    and 89.6 — fifteen points inside one old band — so the cut at 80
 *    is the one that stops that row being a slab.
 *
 * Plus attributes: the gates address rows, cells and marks by
 * `data-day-*`, and those are set on the nodes the original code makes.
 * Not one line of geometry moved to put them there.
 *
 * # What a bucket is drawn as
 *
 * **Adjacent buckets of the same step merge into one rect** (`runs`), so
 * a row is a handful of segments rather than 288 of them. This is the
 * original's own arrangement and it is not a saving: off the integer
 * grid every rect edge is antialiased, and a row of equal shades comes
 * out combed. `px()` rounds each boundary to a whole pixel and
 * neighbours share it exactly.
 */
/* eslint-disable */

/** How many buckets a day is, per the answer being drawn. */
let N = 288;
/** Every word this drawing says, from the catalog — see the head.
 *
 *  ⚠️ **Not `W`, and the name is the fix.** It was `W` until
 *  2026-09-04, and the three drawing functions take the figure's pixel
 *  width in a parameter of that same name (`drawHeat(svg, T, W,
 *  narrow)` — the prototype's own signature, kept). So inside all three
 *  every `W.something` read a property off **a number**: the gap label,
 *  the shade legend and the strain legend came out `undefined`, and
 *  `W.peakAt(…)` in 叠放 threw `TypeError: W.peakAt is not a function`
 *  and took the whole card's subtree down with it.
 *
 *  The collision is the transplant's, not the prototype's: over there
 *  the words were Chinese literals in the drawing, so there was no
 *  second `W` to shadow. It was found by **looking at a screenshot of
 *  real data** — `tsc`, seven gates and the smoke run were all green
 *  through it, because a shadowed name is well-typed, the numbers and
 *  positions everything measured were right, and nothing asked whether
 *  the words were there. */
let WORDS = {};
/** The window: when the oldest bucket opens, and how wide one is. */
let T0 = 0;
let BUCKET_MS = 300000;
let BUCKET_MIN = 5;
/** Clock hour → bucket index, on the window this mount was given. The
    fixture's `((H-20+24)%24)*12` said the same thing about a day that
    always ended at 20:00. */
const iOfHour = (H) => {
  const d = new Date(T0);
  d.setMinutes(0, 0, 0);
  for (let k = 0; k < 26; k += 1) {
    if (d.getHours() === H && d.getTime() >= T0) return Math.round((d.getTime() - T0) / BUCKET_MS);
    d.setHours(d.getHours() + 1);
  }
  return -1;
};
const hhmm = (i) => {
  const d = new Date(T0 + i * BUCKET_MS);
  return String(d.getHours()).padStart(2, '0') + ':' + String(d.getMinutes()).padStart(2, '0');
};

/* ══════════════════════════════════════════════════════════════════
   轨（track）—— 三种画法共用的中间层。
   一条轨要么是一个序列，要么是一个折叠起来的卡组（最忙的卡 / 最闲的卡）。
   ══════════════════════════════════════════════════════════════════ */
function tracks(series, expanded){
  const out=[];
  for(const s of series){
    if(s.kind==='group'){
      if(expanded || s.cards.length===1){
        s.cards.forEach(c=>out.push({kind:'one', name:c.name, short:c.short,
          v:c.v, strain:s.strain}));
      } else {
        const hi=[], lo=[];
        for(let i=0;i<N;i++){
          const vs = s.cards.map(c=>c.v[i]).filter(v=>v!=null);
          if(!vs.length){ hi.push(null); lo.push(null); }
          else { hi.push(Math.max(...vs)); lo.push(Math.min(...vs)); }
        }
        out.push({kind:'range', name:s.name+' ×'+s.cards.length,
          short:s.name+'×'+s.cards.length, hi, lo, n:s.cards.length, strain:s.strain});
      }
    } else out.push({kind:'one', key:s.key, name:s.name, short:s.short, v:s.v, strain:s.strain});
  }
  return out;
}
const topOf = (t,i) => t.kind==='range' ? t.hi[i] : t.v[i];   // 这一轨在这一桶的「上沿」
const botOf = (t,i) => t.kind==='range' ? t.lo[i] : t.v[i];
function peakOf(t){ let pi=-1, pv=-1;
  for(let i=0;i<N;i++){ const v=topOf(t,i); if(v!=null && v>pv){pv=v; pi=i;} }
  return pi<0 ? null : {i:pi, v:pv}; }
const cardCount = s => s.reduce((m,x)=>x.kind==='group'?Math.max(m,x.cards.length):m,1);

/* ══ 通用几何 / SVG ══════════════════════════════════════════════ */
const NS='http://www.w3.org/2000/svg';
const el=(p,t,a)=>{const n=document.createElementNS(NS,t);
  for(const k in a) n.setAttribute(k,a[k]); p.appendChild(n); return n;};
const txt=(p,a,s)=>{const n=el(p,'text',a); n.textContent=s; return n;};
const TICKS_W=[21,0,3,6,9,12,15,18], TICKS_N=[0,6,12,18];
let GAPMARK = true;

// 连续有值 / 连续空缺的段落。空缺永远不桥接。
function segsBy(has){
  const out=[]; let s=-1;
  for(let i=0;i<=N;i++){
    const ok = i<N && has(i);
    if(ok && s<0) s=i;
    if(!ok && s>=0){ out.push([s,i]); s=-1; }
  }
  return out;
}
const dataSegs = t => segsBy(i=>topOf(t,i)!=null);
const gapSegs  = t => segsBy(i=>topOf(t,i)==null);

/* 阶梯剖面：一个桶是一个 5 分钟的平台，不是两点之间可以插值的一个点。
   用阶梯而不是折线，「空缺不桥接」就成了结构性的，不是靠记得断开。 */
function stepPath(get, a, b, X, y0, mapH, close){
  let d = close ? 'M'+X(a).toFixed(2)+' '+y0.toFixed(2) : '';
  for(let i=a;i<b;i++){
    const y = (y0 - mapH(get(i))).toFixed(2);
    d += (d===''?'M':' L') + X(i).toFixed(2)+' '+y + ' L'+X(i+1).toFixed(2)+' '+y;
  }
  return close ? d+' L'+X(b).toFixed(2)+' '+y0.toFixed(2)+' Z' : d;
}
// hi 去、lo 回，围出一条带（多卡折叠时「最忙」和「最闲」之间的那一条）
function bandPath(t, a, b, X, y0, mapH){
  let d='';
  for(let i=a;i<b;i++){ const y=(y0-mapH(t.hi[i])).toFixed(2);
    d += (d===''?'M':' L')+X(i).toFixed(2)+' '+y+' L'+X(i+1).toFixed(2)+' '+y; }
  for(let i=b-1;i>=a;i--){ const y=(y0-mapH(t.lo[i])).toFixed(2);
    d += ' L'+X(i+1).toFixed(2)+' '+y+' L'+X(i).toFixed(2)+' '+y; }
  return d+' Z';
}

const BANDS=[3,8,20,45,70,80,90];             // 90 = the product's strain threshold
const bandOf = v => { let k=0; while(k<BANDS.length && v>=BANDS[k]) k++; return k; };
const shadeCls = (t,v) => (bandOf(v)===BANDS.length && t.strain) ? 'st' : 'r'+(bandOf(v)+1);

// 把连续同键的桶并成一段（省元素，也免掉相邻同色格之间的抗锯齿细缝）
function runs(keyAt){
  const out=[]; let s=0, k=null;
  for(let i=0;i<=N;i++){
    const kk = i<N ? keyAt(i) : null;
    if(kk!==k){ if(k!==null) out.push([s,i,k]); s=i; k=kk; }
  }
  return out;
}

/* ══════════════════════════════════════════════════════════════════
   读法一 · 热力带（默认）—— L16 Matrix Heat 骨架
   明度 = 占用率，行 = 指标，列 = 时刻。一竖列同时读五个指标。
   多卡折叠：上厚条 = 最忙的卡，下薄条 = 最闲的卡。两条不同色 = 各卡不一致。
   ══════════════════════════════════════════════════════════════════ */
function drawHeat(svg, T, W, narrow, fill){
  const GL = narrow?46:56, GR = narrow?42:58;
  const CH = narrow?13:11, HB = Math.round(CH*0.64);
  const TOP = 5, AXIS = narrow?18:31;
  const X0=GL, X1=W-GR, PW=X1-X0, slot=PW/N;
  // **余量放在行距上,不放在带的厚度上。** `CH` 是那条带的厚度,而带是
  // **明度编码的载体** —— 同一支灰在一条更胖的带上读起来就是另一个数,
  // 所以它是这三张图里唯一不许动的尺寸。`ROW` 只是两行之间隔多远,
  // 把它拉开改变的是呼吸,不是任何一个读数。
  const ROW0 = narrow?17:14;
  const nat = TOP + T.length*ROW0 + AXIS;
  const H = fill > nat ? fill : nat;
  const ROW = (H - TOP - AXIS) / T.length;
  svg.setAttribute('viewBox',`0 0 ${W} ${H}`); svg.setAttribute('height',H);
  // 取整:带的上下沿是水平边,落在小数位上抗锯齿会把每一行的边啃出深浅
  // —— 和横向那条「段边界对齐到整数像素」是同一个理由,只是换了个轴。
  const rowY = r => TOP + Math.round(r*ROW);
  // 段边界对齐到整数 CSS 像素：相邻段共用同一个取整结果，所以既不留缝也不重叠。
  // 不对齐的话每一段的边都落在小数位上，抗锯齿会把边缘啃出深浅不一的灰。
  const px = i => Math.round(X0 + i*slot);

  T.forEach((t,r)=>{
    const y = rowY(r);
    txt(svg,{x:X0-8,y:y+CH/2+3,'font-size':narrow?8:8.5,'font-weight':700,
      'text-anchor':'end',class:'lab fade',style:`animation-delay:${r*.05}s`}, t.name);

    const g = el(svg,'g',{class:'wipe',style:`animation-delay:${r*.06}s`,'data-day-row':t.key||t.name});
    if(t.kind==='range'){
      // 键里同时装上沿和下沿：两者同档时并成整格，不同档时才劈成两条
      for(const [a,b,k] of runs(i=> t.hi[i]==null ? null
            : bandOf(t.hi[i])*8 + bandOf(t.lo[i]))){
        const x=px(a), w=Math.max(1, px(b)-px(a));
        const bh=Math.floor(k/8), bl=k%8;
        if(bh===bl) el(g,'rect',{x,y,width:w,height:CH,class:shadeCls(t,t.hi[a]),'data-day-cell':'','data-day-shade':shadeCls(t,t.hi[a])});
        else{
          el(g,'rect',{x,y,width:w,height:HB,class:shadeCls(t,t.hi[a])});
          el(g,'rect',{x,y:y+HB,width:w,height:CH-HB,class:shadeCls(t,t.lo[a])});
        }
      }
      /* 卡组这一行要画一条底边。理由是量出来的：最闲的卡贴近 0 时，
         下面那薄条是最浅的一档 #D8D7D1，和纸 #F0EFEB 差得太少 ——
         没有底边的话，「上厚条黑 + 下薄条几乎是纸」会被读成
         「一条画细了的单值行」，而不是「有四张卡，其中三张是空的」。 */
      dataSegs(t).forEach(([a,b])=>
        el(g,'line',{x1:px(a),y1:y+CH,x2:px(b),y2:y+CH,'stroke-width':.7,class:'hair'}));
    } else {
      for(const [a,b,k] of runs(i=> t.v[i]==null ? null : bandOf(t.v[i])))
        el(g,'rect',{x:px(a),y,width:Math.max(1,px(b)-px(a)),height:CH,
          class:(k===BANDS.length&&t.strain)?'st':'r'+(k+1),
          'data-day-cell':'','data-day-shade':(k===BANDS.length&&t.strain)?'strain':String(k),
          'data-day-from':a,'data-day-to':b});
    }

    if(GAPMARK) gapSegs(t).forEach(([a,b])=>{
      if(b-a<3) return;
      el(svg,'rect',{x:px(a),y,width:Math.max(1,px(b)-px(a)),height:CH,fill:'none',
        'stroke-width':.8,'stroke-dasharray':'2 2.5',class:'hair fade'});
      if(b-a>=22 && !narrow)
        txt(svg,{x:(px(a)+px(b))/2,y:y+CH/2+2.5,'font-size':6.5,'font-weight':600,
          'text-anchor':'middle',class:'fnt fade'},WORDS.gap);
    });

    const pk = peakOf(t);
    if(pk){
      const bw=Math.max(2.4,slot);
      el(svg,'rect',{x:X0+pk.i*slot-(bw-slot)/2,y:y-2,width:bw,height:CH+4,rx:1.5,
        fill:'none','stroke-width':1,'stroke-dasharray':'2 2',class:'inkline fade',
        style:'animation-delay:.9s'});
      txt(svg,{x:X1+8,y:y+CH/2+4,'font-size':narrow?10:10.5,'font-weight':800,
        class:((t.strain&&pk.v>=90)?'sf':'ink')+' fade',style:'animation-delay:.95s',
        'data-day-peak':t.key||t.name}, Math.round(pk.v));
      txt(svg,{x:X1+8+(narrow?19:20),y:y+CH/2+4,'font-size':7,'font-weight':600,
        class:'fnt fade',style:'animation-delay:.95s'},'%');
    }
  });

  const yb = rowY(T.length)+1;
  timeAxis(svg, X0, X1, slot, yb, narrow);
  if(!narrow){
    const LG=BANDS.map((b,k)=>['r'+(k+1), k===0?'<'+b:BANDS[k-1]+'–'+b])
      .concat([['r'+(BANDS.length+1), BANDS[BANDS.length-1]+'+']]);
    const ly = yb+22; let lx = X0;
    txt(svg,{x:X0-8,y:ly+7,'font-size':7,'font-weight':600,'text-anchor':'end',
      class:'fnt fade',style:'animation-delay:1s'},WORDS.shade);
    LG.forEach(([c,s],k)=>{
      el(svg,'rect',{x:lx,y:ly,width:7,height:7,rx:1.5,class:c+' fade',
        style:`animation-delay:${1+k*.03}s`});
      txt(svg,{x:lx+10,y:ly+7,'font-size':7,'font-weight':600,class:'fnt fade',
        style:`animation-delay:${1+k*.03}s`},s);
      lx += 10 + s.length*4.8 + 13;
    });
    el(svg,'rect',{x:lx,y:ly,width:7,height:7,rx:1.5,class:'st fade',
      style:'animation-delay:1.2s'});
    txt(svg,{x:lx+10,y:ly+7,'font-size':7,'font-weight':600,class:'fnt fade',
      style:'animation-delay:1.2s'},WORDS.strainLegend);
    if(T.some(t=>t.kind==='range'))
      txt(svg,{x:X1,y:ly+7,'font-size':7,'font-weight':600,'text-anchor':'end',
        class:'fnt fade',style:'animation-delay:1.25s'},WORDS.cards||'');
  }
  return {
    top: TOP-3, bot: yb, X0, X1, slot,
    dotAt: (t,i) => { const v=topOf(t,i); return v==null?null:null; }   // 热力带不用点
  };
}

/* ══════════════════════════════════════════════════════════════════
   读法二 · 山脊 —— L19 Ridgeline 骨架
   每条指标一条自己的地板，高度 = 占用率。面积是**实心的**，
   不是一桶一根竖发丝：实测那种画法在 288 桶 / 非整数桶宽下，
   逐列墨量的变异系数是 12.5–21.5%，而数据本身只有 3.6% —— 它凭空
   造出 4.7~6 倍于数据的明暗抖动，dpr=2 时最重。见 README。
   ══════════════════════════════════════════════════════════════════ */
function drawRidge(svg, T, W, narrow, fill){
  const n=T.length, dense = n>6;
  const GL = narrow?42:48, GR = narrow?34:38;
  const SP0 = dense ? (narrow?16:14) : (narrow?22:20);
  const AMP = dense ? (narrow?20:18) : (narrow?26:24);
  // AXIS 是**最后一笔墨到图底还剩多少**,三张图的这个数是一组、要一起改:
  // 热力带 31(时间轴 + 明度图例)、山脊 15(只有时间轴)、叠放 31(轴名 + 计数)。
  // 它们凑在一起是为了让三张图**画到同一条底线上** —— 高度一样只是容器一样,
  // 墨停在不同的地方,读者看到的仍然是一块空地(2026-09-04)。
  const TOP = 8, AXIS = 15;
  const X0=GL, X1=W-GR, PW=X1-X0, slot=PW/N;
  const rowOf = k => (n-1)-k;                 // 最重要的排最前排（最下面）
  // **余量放在错层间距上,不放在振幅上。** `AMP` 是 100% 对应多少像素——
  // 撑大它,同一条脊的起伏看起来就更剧烈,那是在改这张图说的话。`SP` 是
  // 前后两排隔多远,拉开它只是让后排从前排背后多露出来一点。
  //
  // **只有一条脊时没有「排与排之间」可分**,那时余量对半放在上下(居中):
  // 把一条脊拉长只能靠 AMP,而 AMP 不许动。
  const natBot = TOP + AMP + (n-1)*SP0, nat = natBot + AXIS;
  const H = fill > nat ? fill : nat;
  const room = H - AXIS - TOP - AMP;
  const SP  = n > 1 ? room/(n-1) : SP0;
  const PADY = n > 1 ? 0 : room/2;
  const baseY = r => TOP + PADY + AMP + r*SP;
  const bot = baseY(n-1);
  svg.setAttribute('viewBox',`0 0 ${W} ${H}`); svg.setAttribute('height',H);
  const X = i => X0 + i*slot, mapH = v => v/100*AMP;

  const order = T.map((t,k)=>({t,k,r:rowOf(k)})).sort((a,b)=>a.r-b.r);
  // 后排先画、前排后画：纸色 slab 压住后排（L19 的遮挡手法）
  order.forEach(({t,k,r})=>{
    const y0 = baseY(r);
    const g = el(svg,'g',{class:'wipe',style:`animation-delay:${r*.06}s`});
    dataSegs(t).forEach(([a,b])=>{
      el(g,'path',{d:stepPath(i=>topOf(t,i),a,b,X,y0,mapH,true),fill:'var(--card)'});
      if(t.kind==='range'){
        el(g,'path',{d:bandPath(t,a,b,X,y0,mapH),class:'areaLo'});        // 最忙~最闲之间
        el(g,'path',{d:stepPath(i=>t.lo[i],a,b,X,y0,mapH,true),class:'area'});
      } else {
        el(g,'path',{d:stepPath(i=>t.v[i],a,b,X,y0,mapH,true),class:'area'});
      }
      el(g,'line',{x1:X(a),y1:y0,x2:X(b),y2:y0,'stroke-width':.7,class:'hair'});
    });
    // 纹理换成整点刻度：24 根，一根 = 一小时，是**数得清也说得出**的单位。
    // 原来那 288 根竖发丝数不清，只会和像素网格打架。
    if(!dense) for(let i=0;i<N;i+=12){
      if(topOf(t,i)==null) continue;
      el(g,'line',{x1:X(i),y1:y0,x2:X(i),y2:y0-3.5,'stroke-width':.6,class:'hair'});
    }
    if(GAPMARK) gapSegs(t).forEach(([a,b])=>{
      if(b-a<3) return;
      el(g,'line',{x1:X(a),y1:y0,x2:X(b),y2:y0,'stroke-width':.7,class:'hair',
        'stroke-dasharray':'1.5 3'});
      if(b-a>=20 && !narrow && !dense)
        txt(g,{x:(X(a)+X(b))/2,y:y0-3,'font-size':6.5,'font-weight':600,
          'text-anchor':'middle',class:'fnt'},WORDS.gap);
    });
    txt(svg,{x:X0-7,y:y0-2,'font-size':narrow?8:8.5,'font-weight':700,
      'text-anchor':'end',class:'lab fade',style:`animation-delay:${r*.05}s`,
      'data-day-row':t.key||t.name}, t.name);
  });

  // 脊线统一在最上层再描一遍：被前排压住的那条也读得出来。
  // 只给后排垫纸色描边 —— 给最前排也垫，那 1.8px 纸色会啃掉它自己的面积边缘。
  order.forEach(({t,k,r})=>{
    const y0=baseY(r);
    const g = el(svg,'g',{class:'wipe',style:`animation-delay:${.08+r*.06}s`});
    dataSegs(t).forEach(([a,b])=>{
      const d = stepPath(i=>topOf(t,i),a,b,X,y0,mapH,false);
      if(r<n-1) el(g,'path',{d,fill:'none','stroke-width':1.8,class:'halo',opacity:.9});
      el(g,'path',{d,fill:'none','stroke-width':r===n-1?1.1:.9,
        class:'k'+(Math.min(k,4)+1)});
      if(t.kind==='range')
        el(g,'path',{d:stepPath(i=>t.lo[i],a,b,X,y0,mapH,false),fill:'none',
          'stroke-width':.7,class:'k'+(Math.min(k,4)+1),opacity:.55,
          'stroke-dasharray':'2.5 2'});
      if(t.strain){
        segsBy(i=>{const v=topOf(t,i); return v!=null && v>=90;}).forEach(([p,q])=>{
          if(p<a||q>b) return;
          el(g,'path',{d:stepPath(i=>topOf(t,i),p,q,X,y0,mapH,false),fill:'none',
            'stroke-width':1.9,class:'sk','stroke-linecap':'round'});
        });
      }
    });
    const pk = peakOf(t);
    if(pk){
      const x=X(pk.i)+slot/2, y=y0-mapH(pk.v), strained=t.strain&&pk.v>=90;
      el(svg,'circle',{cx:x,cy:y,r:2.4,class:(strained?'sf':'f'+(Math.min(k,4)+1))+' fade',
        style:`animation-delay:${.85+r*.04}s`});
      const right = x < X1-44;
      txt(svg,{x:right?x+6:x-6,y:y+2.5,'font-size':narrow?8.5:9,'font-weight':800,
        'text-anchor':right?'start':'end',class:(strained?'sf':'ink')+' fade',
        style:`paint-order:stroke;stroke:var(--card);stroke-width:3.4px;animation-delay:${.9+r*.04}s`},
        Math.round(pk.v)+'%');
    }
  });

  timeAxis(svg, X0, X1, slot, bot, narrow);
  el(svg,'line',{x1:X0-4,y1:baseY(n-1)-AMP,x2:X0,y2:baseY(n-1)-AMP,
    'stroke-width':.8,class:'hair fade'});
  return {
    top: TOP-4, bot, X0, X1, slot,
    dotAt: (t,i)=>{ const v=topOf(t,i); if(v==null) return null;
      const k=T.indexOf(t); return {x:X(i)+slot/2, y:baseY(rowOf(k))-mapH(v)}; }
  };
}

/* ══════════════════════════════════════════════════════════════════
   读法三 · 平行坐标 —— L20 骨架
   一条发丝 = 一个 5 分钟时刻，一根轴 = 一个指标。时间被压掉，
   换来的是跨指标关系成为图形本身。轴上上下划动可以刷选。
   多卡展开时它是唯一**高度一点不涨**的读法：加卡只是加轴。
   ══════════════════════════════════════════════════════════════════ */
function drawPara(svg, T, W, narrow, fill){
  const n=T.length;
  // **余量放在轴的长度上。** 这张图的纵向是每根轴的 0..100,把轴拉长
  // 不改变任何一个读数落在轴上的比例——一条 15% 的线在更长的轴上还是
  // 15%,只是分得开一点。这是三张图里最便宜的一处:它本来就是最高的那张,
  // 要补的那点最少。
  // 31 = 轴名 + 它下面那行计数到图底的距离。**和另外两张图的同一个数是一组**
  // —— 见 `drawRidge` 里那一段。
  const FOOT = 31;
  const BOT0 = narrow?122:116, nat = BOT0 + FOOT;
  const H = fill > nat ? fill : nat;
  const PAD = narrow?26:44, TOP=14, BOT=H-FOOT;
  const AX = Array.from({length:n},(_,k)=> n===1 ? W/2 : PAD + k*(W-2*PAD)/(n-1));
  const mapY = v => BOT - v/100*(BOT-TOP);
  svg.setAttribute('viewBox',`0 0 ${W} ${H}`); svg.setAttribute('height',H);
  const short = n>6;

  el(svg,'line',{x1:AX[0],y1:mapY(90),x2:AX[n-1],y2:mapY(90),'stroke-width':.8,
    'stroke-dasharray':'3 3',class:'sk fade',opacity:.7});
  txt(svg,{x:AX[0]-6,y:mapY(90)+3,'font-size':7,'font-weight':700,'text-anchor':'end',
    class:'sf fade'},'90');

  T.forEach((t,k)=>{
    const x=AX[k];
    el(svg,'line',{x1:x,y1:TOP-6,x2:x,y2:BOT+6,'stroke-width':1,class:'hair fade',
      style:`animation-delay:${k*.04}s`});
    [TOP-6,BOT+6].forEach(yy=>el(svg,'line',{x1:x-4,y1:yy,x2:x+4,y2:yy,
      'stroke-width':1,class:'hair fade'}));
    if(!short) [0,50].forEach(gv=>{
      el(svg,'line',{x1:x-3,y1:mapY(gv),x2:x+3,y2:mapY(gv),'stroke-width':.7,class:'hair fade'});
      txt(svg,{x:x-6,y:mapY(gv)+3,'font-size':6.5,'font-weight':600,'text-anchor':'end',
        class:'fnt fade'},String(gv));
    });
    txt(svg,{x,y:BOT+21,'font-size':short?7.5:(narrow?8:8.5),'font-weight':800,
      'text-anchor':'middle',class:'lab fade',style:`animation-delay:${k*.04}s`,
      'data-day-row':t.key||t.name}, short ? t.short : t.name);
    const got = (t.kind==='range'?t.hi:t.v).filter(v=>v!=null).length;
    txt(svg,{x,y:BOT+30,'font-size':6.5,'font-weight':600,'text-anchor':'middle',
      class:'fnt fade',style:`animation-delay:${.08+k*.04}s`}, got+'/'+N);
  });

  /* 折叠的卡组：这根轴上，每个时刻额外画一小段从「最闲的卡」到「最忙的卡」的竖线。
     各卡一致时它是一个点，一张满三张闲时它是一整条 —— 这一列的「毛」有多长，
     就是各卡分歧有多大。折线本身走「最忙的卡」。 */
  T.forEach((t,k)=>{
    if(t.kind!=='range') return;
    const g=el(svg,'g',{class:'fade',style:'animation-delay:.5s'});
    for(let i=0;i<N;i++){
      if(t.hi[i]==null) continue;
      if(t.hi[i]-t.lo[i] < 1.5) continue;
      el(g,'line',{x1:AX[k],y1:mapY(t.hi[i]),x2:AX[k],y2:mapY(t.lo[i]),
        'stroke-width':2.6,class:'thr',opacity:.10});
    }
  });

  const lines=[];
  const gl=el(svg,'g',{});
  for(let i=0;i<N;i++){
    const ys = T.map(t=>{const v=topOf(t,i); return v==null?null:mapY(v);});
    if(ys.every(y=>y==null)) continue;
    let d='', open=false;
    for(let k=0;k<n;k++){
      if(ys[k]==null){ open=false; continue; }
      if(!open){ d+=` M${AX[k]} ${ys[k].toFixed(2)}`; open=true; continue; }
      const mx=(AX[k-1]+AX[k])/2;
      d+=` C${mx} ${ys[k-1].toFixed(2)} ${mx} ${ys[k].toFixed(2)} ${AX[k]} ${ys[k].toFixed(2)}`;
    }
    const fresh=i/N;
    const p=el(gl,'path',{d,fill:'none','stroke-width':.6,
      class:(fresh>.875?'rec':'thr')+' draw', opacity:(.14+fresh*.28).toFixed(3),
      pathLength:1, style:`animation-delay:${(fresh*.45).toFixed(3)}s;animation-duration:.65s`});
    lines.push({i,p,base:{cls:p.getAttribute('class'),op:p.getAttribute('opacity'),w:'0.6'}});
  }

  const peakY={};
  T.forEach((t,k)=>{
    const pk=peakOf(t); if(!pk) return;
    const strained=t.strain&&pk.v>=90;
    el(svg,'circle',{cx:AX[k],cy:mapY(pk.v),r:2.8,class:(strained?'sf':'ink')+' fade',
      style:`animation-delay:${.85+k*.04}s`});
    const left = k>=n-1 && n>1;
    txt(svg,{x:AX[k]+(left?-8:8),y:mapY(pk.v)+3.5,'font-size':short?8:(narrow?8.5:9),
      'font-weight':800,'text-anchor':left?'end':'start',
      class:(strained?'sf':'ink')+' fade',
      style:`paint-order:stroke;stroke:var(--card);stroke-width:3.4px;animation-delay:${.9+k*.04}s`},
      short?Math.round(pk.v)+'%':WORDS.peakAt(Math.round(pk.v)+'%',hhmm(pk.i)));
    peakY[k]=mapY(pk.v);
  });

  let last=N-1; while(last>=0 && T.every(t=>topOf(t,last)==null)) last--;
  if(last>=0){
    const rec=lines.find(l=>l.i===last);
    if(rec){
      rec.p.setAttribute('stroke-width','2'); rec.p.setAttribute('opacity','1');
      rec.p.setAttribute('class','inkline draw');
      rec.base={cls:'inkline draw',op:'1',w:'2'};
      const lv=topOf(T[n-1],last);
      if(lv!=null){
        const y=mapY(lv), clash = peakY[n-1]!=null && Math.abs(peakY[n-1]-y)<13;
        txt(svg,{x:AX[n-1]+(n>1?-8:8),y:clash?y+14:y-8,'font-size':8.5,'font-weight':800,
          'text-anchor':n>1?'end':'start',class:'ink fade',
          style:'paint-order:stroke;stroke:var(--card);stroke-width:3.4px;animation-delay:1s'},
          WORDS.now);
      }
    }
  }
  return { top:TOP-8, bot:BOT, AX, mapY, lines, T,
    dotAt:(t,i)=>{const v=topOf(t,i); if(v==null) return null;
      return {x:AX[T.indexOf(t)], y:mapY(v)};} };
}

function timeAxis(svg, X0, X1, slot, yb, narrow){
  (narrow?TICKS_N:TICKS_W).forEach(Hh=>{
    const x=X0+iOfHour(Hh)*slot;
    if(x > X1-(narrow?36:30)) return;               // 别撞上右端的「现在」
    el(svg,'line',{x1:x,y1:yb,x2:x,y2:yb+4,'stroke-width':.7,class:'hair fade','data-day-hour':''});
    txt(svg,{x,y:yb+13,'font-size':narrow?7:7.5,'font-weight':600,'text-anchor':'middle',
      class:'fnt fade'},String(Hh).padStart(2,'0')+':00');
  });
  txt(svg,{x:X1,y:yb+13,'font-size':narrow?7:7.5,'font-weight':700,'text-anchor':'end',
    class:'lab fade'},WORDS.now);
}

/* ══ 读数条：三种读法共用同一句话，所以切换视图时它一个字都不变 ══ */
function readout(T, i){
  if(i==null) return null;
  return '<b>'+hhmm(i)+'</b> &nbsp; ' + T.map(t=>{
    if(t.kind==='range'){
      if(t.hi[i]==null) return '<span class="gone">'+t.name+' '+WORDS.gap+'</span>';
      const hi=Math.round(t.hi[i]), lo=Math.round(t.lo[i]);
      const sp = t.strain && t.hi[i]>=90 ? 'sp' : 'b';
      return t.name+' '+(WORDS.busiest||'')+' <'+sp+'>'+hi+'%</'+sp+'> / '+(WORDS.idlest||'')+' <b>'+lo+'%</b>';
    }
    if(t.v[i]==null) return '<span class="gone">'+t.name+' '+WORDS.gap+'</span>';
    const v=Math.round(t.v[i]);
    return t.name+(t.strain&&t.v[i]>=90 ? ' <span class="sp">'+v+'%</span>'
                                        : ' <b>'+v+'%</b>');
  }).join(' &nbsp; ');
}

/** The three readings, as the words this face calls them. */
const viewsOf = () => [['heat', WORDS.heat], ['ridge', WORDS.ridge], ['para', WORDS.para]];

/**
 * `CpuDay`'s five series as the tracks the drawing functions eat.
 *
 * **A series nobody recorded is not a row.** `some` and not `length`: a
 * node can send 288 holes, and a row of holes under a name reads as a
 * reading this machine failed to take rather than one it does not take.
 *
 * Only 内存 and 磁盘 carry `strain`. A machine at 95% CPU is a machine
 * doing its job, and a card holding 43 GiB is the normal state of a box
 * in use — 显存 was on this list until 2026-09-04 and came off it: the
 * consequence of a full card is the same kind as a full disk but the
 * *unit* is not, 90% of 24G leaves 2.4G and 90% of 80G leaves 8G, and
 * this band has percentages and no bytes.
 */
export function toTracks(day, words) {
  const five = [
    ['cpu', words.cpu, day.buckets, false],
    ['gpu', words.gpu, day.gpu, false],
    ['mem', words.mem, day.mem, true],
    ['disk', words.disk, day.disk, true],
    ['vram', words.vram, day.vram, false],
  ];
  return five
    .filter(([, , v]) => Array.isArray(v) && v.some((x) => x !== null))
    .map(([key, name, v, strain]) => ({ kind: 'one', key, name, short: name, v, strain }));
}

/**
 * Hang a 24-hour panel in `host` and keep it drawn.
 *
 * The gallery's own chrome — a heading, a caption, the plate naming the
 * source layout — does not come across: the card already has a heading
 * and the two ends of its window. What is here is the original's three
 * parts, in its order: the figure, the readout under it, and the three
 * words that choose a reading.
 *
 * Returns `{ update, destroy }`. React calls `update` when a new answer
 * lands and `destroy` on unmount; **nothing else about this drawing is
 * React's business**, which is the point.
 */
export function mount(host, opts) {
  const st = { view: 'heat', cur: null, pin: false, W: 0, H: 0, geo: null, T: [], series: [], first: true };
  host.classList.add('dayblk');
  host.innerHTML =
    '<div class="ch"><svg preserveAspectRatio="xMinYMin meet" data-day-band></svg></div>' +
    '<div class="ro" data-day-at></div>' +
    '<div class="sw" data-day-views></div>';
  const svg = host.querySelector('svg');
  const ro = host.querySelector('.ro');
  const sw = host.querySelector('.sw');
  const ch = host.querySelector('.ch');
  // **Buttons, not the original's `<i>` with one delegated handler.**
  // The one thing this face asks of every control is a finger-sized
  // target, and the gate that measures it looks at the element carrying
  // the press. Ten-pixel words in a gallery need no such thing.
  const drawSwitch = () => {
    sw.innerHTML = viewsOf()
      .map(
        ([v, label], k) =>
          (k ? '<s>·</s>' : '') +
          '<button type="button" data-day-view-pick="' + v + '" data-on="' + (v === st.view) +
          '" aria-pressed="' + (v === st.view) + '">' + label + '</button>',
      )
      .join('');
  };
  // The original showed a sentence here when nothing was under the
  // cursor — 「把鼠标横着扫过去,点一下钉住」. **Not brought over**: this
  // repo does not write instructions for its own controls, and a line
  // that is always there is not a readout.
  const legendOf = () => '';

  const render = () => {
    const w = Math.round(ch.clientWidth);
    if (w < 60) return;
    // **How tall the drawing is allowed to be, and is expected to fill.**
    //
    // The three readings are not naturally the same height — 92, 108 and
    // 153 for four rows — and the block they live in has one height, so
    // two of them used to end well above its floor. 2026-09-04:
    // 「24小时趋势图,不要满足最高高度,而导致部分图下面空掉了。可以全部
    // 图画到等高。不然视觉一致性很差」.
    //
    // So the box owns the height (`.ch` is the flexible one in the
    // column) and each drawing is told what to fill. **What each one
    // puts the slack into is its own decision and is written where that
    // decision lives** — none of them scales: a drawing stretched to
    // fit is a drawing whose numbers moved.
    const h = Math.round(ch.clientHeight);
    st.W = w;
    st.H = h;
    const narrow = w < 520;
    const T = tracks(st.series, false);
    svg.innerHTML = '';
    st.T = T;
    st.geo = st.view === 'heat' ? drawHeat(svg, T, w, narrow, h)
           : st.view === 'ridge' ? drawRidge(svg, T, w, narrow, h)
           :                       drawPara(svg, T, w, narrow, h);
    svg.setAttribute('data-day-view', st.view);
    attach(T);
    paint();
  };

  let cur=null, dots=[], hitZones=[];
  function attach(T){
    const G=st.geo;
    if(st.view==='para'){
      // 平行坐标没有时间横轴：刷轴代替游标
      attachBrush(T,G);
      return;
    }
    cur = el(svg,'line',{x1:G.X0,y1:G.top,x2:G.X0,y2:G.bot,'stroke-width':1,
      class:'inkline',opacity:0,'pointer-events':'none'});
    dots = T.map(()=>el(svg,'circle',{r:2.6,class:'ink',opacity:0,'pointer-events':'none'}));
    const hit = el(svg,'rect',{x:G.X0,y:G.top,width:G.X1-G.X0,height:G.bot-G.top,
      fill:'transparent',style:'cursor:crosshair','data-day-hit':''});
    const at = ev => {
      const r=svg.getBoundingClientRect();
      const px=((ev.touches?ev.touches[0].clientX:ev.clientX)-r.left)/r.width*st.W;
      return Math.max(0,Math.min(N-1,Math.floor((px-G.X0)/G.slot)));
    };
    hit.addEventListener('mousemove',ev=>{ if(!st.pin){ st.cur=at(ev); paint(); }});
    hit.addEventListener('mouseleave',()=>{ if(!st.pin){ st.cur=null; paint(); }});
    hit.addEventListener('click',ev=>{ st.pin=!st.pin; st.cur=st.pin?at(ev):null; paint(); });
    hit.addEventListener('touchstart',ev=>{st.pin=true; st.cur=at(ev); paint(); ev.preventDefault();});
    hit.addEventListener('touchmove', ev=>{st.cur=at(ev); paint(); ev.preventDefault();});
  }

  function attachBrush(T,G){
    const HALF=8;
    const box = el(svg,'rect',{x:0,y:0,width:0,height:0,fill:'none','stroke-width':1,
      'stroke-dasharray':'3 2',class:'inkline',opacity:0,'pointer-events':'none'});
    const reset = () => { G.lines.forEach(l=>{
        l.p.setAttribute('class',l.base.cls); l.p.setAttribute('opacity',l.base.op);
        l.p.setAttribute('stroke-width',l.base.w); });
      box.setAttribute('opacity',0); ro.textContent = legendOf(T); };
    T.forEach((t,k)=>{
      const hit=el(svg,'rect',{x:G.AX[k]-14,y:G.top,width:28,height:G.bot-G.top+14,
        fill:'transparent',style:'cursor:ns-resize'});
      const at=ev=>{const r=svg.getBoundingClientRect();
        const py=((ev.touches?ev.touches[0].clientY:ev.clientY)-r.top)/r.height
          *svg.viewBox.baseVal.height;
        return Math.max(0,Math.min(100,(G.bot-py)/(G.bot-G.top-6)*100));};
      const brush = val => {
        const lo=Math.max(0,val-HALF), hi=Math.min(100,val+HALF), idx=[];
        G.lines.forEach(l=>{
          const v=topOf(t,l.i), on = v!=null && v>=lo && v<=hi;
          l.p.setAttribute('class', on?'inkline draw':l.base.cls);
          l.p.setAttribute('opacity', on?'.9':'.05');
          l.p.setAttribute('stroke-width', on?'.9':l.base.w);
          if(on) idx.push(l.i);
        });
        box.setAttribute('x',G.AX[k]-9); box.setAttribute('y',G.mapY(hi));
        box.setAttribute('width',18);
        box.setAttribute('height',Math.max(2,G.mapY(lo)-G.mapY(hi)));
        box.setAttribute('opacity',1);
        if(!idx.length){ ro.innerHTML='<span class="gone">'
          +WORDS.brushEmpty(t.name, Math.round(lo)+'–'+Math.round(hi)+'%')+'</span>'; return; }
        const med=a=>{const b=a.slice().sort((x,y)=>x-y);return b[Math.floor(b.length/2)]};
        const others = T.map((o,oi)=>{ if(oi===k) return null;
          const vs=idx.map(i=>topOf(o,i)).filter(v=>v!=null);
          return vs.length ? WORDS.brushMedian(o.name, '<b>'+Math.round(med(vs))+'%</b>') : WORDS.brushNone(o.name);
        }).filter(Boolean);
        ro.innerHTML=WORDS.brushHead('<b>'+t.name+' '+Math.round(lo)+'–'+Math.round(hi)+'%</b>',
          idx.length, (idx.length*BUCKET_MIN/60).toFixed(1))+others.join(' &nbsp; ');
      };
      hit.addEventListener('mousemove',ev=>brush(at(ev)));
      hit.addEventListener('mouseleave',reset);
      hit.addEventListener('touchstart',ev=>{brush(at(ev)); ev.preventDefault();});
      hit.addEventListener('touchmove', ev=>{brush(at(ev)); ev.preventDefault();});
    });
    reset();
  }

  function paint(){
    const T=st.T, G=st.geo;
    if(st.view==='para'){ if(st.cur==null) ro.textContent=legendOf(T); return; }
    if(st.cur==null){
      cur.setAttribute('opacity',0); dots.forEach(d=>d.setAttribute('opacity',0));
      ro.textContent = legendOf(T); return;
    }
    const x=G.X0+(st.cur+.5)*G.slot;
    cur.setAttribute('x1',x); cur.setAttribute('x2',x); cur.setAttribute('opacity',1);
    T.forEach((t,k)=>{
      const p = G.dotAt ? G.dotAt(t,st.cur) : null;
      if(!p || !dots[k]){ if(dots[k]) dots[k].setAttribute('opacity',0); return; }
      dots[k].setAttribute('cx',p.x); dots[k].setAttribute('cy',p.y);
      dots[k].setAttribute('opacity',1);
    });
    ro.innerHTML = readout(T, st.cur);
  }


  drawSwitch();
  sw.addEventListener('click', (ev) => {
    const b = ev.target.closest('button[data-day-view-pick]');
    if (!b) return;
    st.view = b.dataset.dayViewPick;
    //   **This one is an arrival**, so it plays the entrance — see
    //   `nofx` in the stylesheet and in `update` below.
    host.classList.remove('nofx');
    drawSwitch();
    render();                                    // st.cur / st.pin kept
  });
  const ro_ = new ResizeObserver(() => {
    const w = Math.round(ch.clientWidth);
    // **Height too, now that the drawing fills it.** Watching only the
    // width was right while every reading picked its own height; it is
    // not now, and a box that grew without the drawing following would
    // put the empty strip straight back.
    const h = Math.round(ch.clientHeight);
    if (w >= 60 && (w !== st.W || h !== st.H)) render();
  });
  ro_.observe(ch);
  return {
    update(day, words) {
      WORDS = words;
      N = Math.max(1, day.buckets.length);
      BUCKET_MS = day.bucket_ms;
      BUCKET_MIN = day.bucket_ms / 60000;
      T0 = day.now_ms - day.bucket_ms * N;
      st.series = toTracks(day, words);
      if (st.cur !== null && st.cur >= N) st.cur = N - 1;
      //   **Only the first answer is an arrival.** Every one after it
      //   is the same figure with a newer number in it, and replaying
      //   the entrance for those left the drawing fading in more often
      //   than it stood still — see `nofx` in the stylesheet.
      if (!st.first) host.classList.add('nofx');
      st.first = false;
      drawSwitch();
      st.W = 0;                                  // force a redraw at the same width
      render();
    },
    destroy() {
      ro_.disconnect();
      host.innerHTML = '';
      host.classList.remove('dayblk');
    },
  };
}
