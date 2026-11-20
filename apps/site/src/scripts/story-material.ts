const ns='http://www.w3.org/2000/svg';
const make=<K extends keyof SVGElementTagNameMap>(tag:K,attributes:Record<string,string>={})=>{
  const el=document.createElementNS(ns,tag);
  for(const [name,value] of Object.entries(attributes))el.setAttribute(name,value);
  return el;
};
const lerp=(a:number,b:number,t:number)=>a+(b-a)*t;
const number=(el:Element,name:string)=>Number(el.getAttribute(name)??0);
const smooth=(t:number)=>{const p=Math.max(0,Math.min(1,t));return p*p*(3-2*p);};

/** In the moving SVG, feather the pearl shadows with a fixed paint instead of a filter surface. */
function softenGlass(definitions:SVGDefsElement,art:SVGGElement,prefix:string){
  const shadow=make('radialGradient',{id:`${prefix}soft-shadow`});
  for(const [offset,opacity] of [['.55','1'],['.72','.84'],['.86','.35'],['1','0']]){
    shadow.append(make('stop',{offset,'stop-color':'black','stop-opacity':opacity}));
  }
  definitions.append(shadow);
  for(const el of art.querySelectorAll('[filter]')){
    const filter=el.getAttribute('filter')!;
    el.removeAttribute('filter');
    if(filter.endsWith('specular-soft)')){
      // The specular ellipse already has a soft gradient in the same position.
      el.remove();
    }else if(el.tagName==='circle'&&el.getAttribute('fill')==='#000000'){
      el.setAttribute('r',String(number(el,'r')+6));
      el.setAttribute('fill',`url(#${shadow.id})`);
    }
  }
  definitions.querySelectorAll('filter').forEach(el=>el.remove());
}

/** Preserve the canonical contours and let fixed optical detail recede as the glass flattens. */
export function createStoryMaterial(icon:SVGSVGElement,target:SVGGElement,host:SVGElement,prefix:string){
  const sourceRing=icon.querySelector<SVGCircleElement>('circle[stroke*="glass-band"]')!;
  const source=sourceRing.parentElement!;
  const defs=icon.querySelector('defs')!;
  const ids=[...defs.querySelectorAll('[id]')].map(el=>el.id);
  const isolate=(html:string)=>{
    for(const id of ids)html=html.replaceAll(`id="${id}"`,`id="${prefix}${id}"`).replaceAll(`url(#${id})`,`url(#${prefix}${id})`).replaceAll(`href="#${id}"`,`href="#${prefix}${id}"`);
    return html;
  };
  const definitions=make('defs');definitions.innerHTML=isolate(defs.innerHTML);
  const art=make('g',{'data-morph-material':''});
  art.innerHTML=isolate([...source.children].filter(el=>el.tagName!=='defs').map(el=>el.outerHTML).join(''));
  host.append(definitions,art);
  const ring=art.querySelector<SVGCircleElement>('circle[stroke*="glass-band"]')!;
  // The original ring and its reflections travel together in the perimeter.
  const boundaryShadow=ring.previousElementSibling! as SVGCircleElement;
  const boundaryLight=art.querySelector<SVGGElement>('g[clip-path*="annulus"]')!;
  const rims=[...art.querySelectorAll<SVGCircleElement>('circle[stroke*="outer-edge"],circle[stroke*="inner-edge"]')];
  const boundaryEdges=rims.filter(edge=>!edge.hasAttribute('filter'));
  const boundaryBand=ring.cloneNode(false) as SVGCircleElement;
  softenGlass(definitions,art,prefix);
  [boundaryShadow,boundaryLight,...rims].forEach(el=>el.remove());
  // This small circle draws in after the core reaches the title.
  ring.setAttribute('data-morph-mark-ring','');
  ring.setAttribute('stroke','currentColor');
  ring.setAttribute('pathLength','1');ring.setAttribute('stroke-dasharray','1');
  ring.setAttribute('stroke-dashoffset','1');

  const diamond=art.querySelector<SVGPathElement>(':scope > path[stroke*="diamond-edge"]')!;
  const inner=art.querySelector<SVGCircleElement>('circle[stroke*="ring-solid"]')!;
  const pearls=Array.from({length:5},(_,i)=>art.querySelector<SVGCircleElement>(`circle[fill*="pearl-${i}"]`)!);
  const targetRing=target.querySelector<SVGCircleElement>('circle[r="368"]')!;
  const targetDiamond=target.querySelector<SVGPathElement>('path')!;
  const targetInner=target.querySelector<SVGCircleElement>('circle[r="170"]')!;
  const targetPearls=pearls.map(pearl=>[...target.querySelectorAll<SVGCircleElement>('circle')].find(el=>el.getAttribute('cx')===pearl.getAttribute('cx')&&el.getAttribute('cy')===pearl.getAttribute('cy')&&el.parentElement?.getAttribute('fill')==='currentColor')!);
  const markTips=targetPearls.slice(1).map(target=>{
    const el=target.cloneNode(true) as SVGCircleElement,r=number(target,'r');
    el.setAttribute('fill','currentColor');el.setAttribute('r','0');
    el.setAttribute('data-morph-mark-tip','');art.append(el);return {el,r};
  });
  const satellites=pearls.slice(1).map(body=>{
    const shadow=body.previousElementSibling!;
    const light=body.nextElementSibling!;
    const rim=light.nextElementSibling!;
    const halo=art.querySelector(`:scope > circle[cx="${body.getAttribute('cx')}"][cy="${body.getAttribute('cy')}"][fill*="light-wash"]`)!;
    const back=make('g',{'class':'story-pearl-light'}),front=make('g',{'class':'story-pearl-light'});
    const disk=body.cloneNode(false) as SVGCircleElement;
    disk.setAttribute('data-morph-pearl','');
    back.append(halo,shadow);front.append(body,light,rim);
    return {parts:[back,disk,front],disk};
  });

  // Keep the original stacking order. Only the underlying shade and highlights
  // fade; the diamond, inner ring and center remain solid as their geometry moves.
  const primary=new Set<Element>([ring,diamond,inner,pearls[0],...markTips.map(tip=>tip.el)]);
  const details:SVGGElement[]=[];
  let group:SVGGElement|undefined;
  for(const el of [...art.children]){
    if(primary.has(el)){group=undefined;continue;}
    if(!group){group=make('g');details.push(group);el.before(group);}
    group.append(el);
  }
  const strokes=[{el:ring,target:targetRing},{el:diamond,target:targetDiamond},{el:inner,target:targetInner}].map(({el,target})=>({el,from:number(el,'stroke-width'),to:number(target,'stroke-width')}));
  const radii=[{el:inner,target:targetInner},{el:pearls[0],target:targetPearls[0]}].map(({el,target})=>({el,from:number(el,'r'),to:number(target,'r')}));
  for(const gradient of definitions.querySelectorAll('linearGradient,radialGradient')){
    if(!/(diamond-edge|ring-solid|pearl-0)$/.test(gradient.id))continue;
    for(const stop of gradient.querySelectorAll('stop')){
      const color=stop.getAttribute('stop-color')!,alpha=stop.getAttribute('stop-opacity')??'1';
      stop.style.stopColor=`color-mix(in srgb, ${color} calc((1 - var(--material-flat,0))*100%), currentColor)`;
      stop.style.stopOpacity=`calc(${alpha} + (1 - ${alpha})*var(--material-flat,0))`;
    }
  }
  let previousMaterial=NaN,previousSettle=NaN,previousMark=NaN,previousScale=NaN;
  const frame=(p:number,scale:number,sourceScale:number,targetScale:number)=>{
    const material=smooth((p-.06)/.58),settle=smooth((p-.08)/.68),mark=smooth((p-.72)/.14);
    if(scale!==previousScale||material!==previousMaterial){
      for(const {el,from,to} of strokes)el.setAttribute('stroke-width',String(lerp(from*sourceScale,to*targetScale,material)/scale));
      previousScale=scale;
    }
    if(material!==previousMaterial){
      definitions.style.setProperty('--material-flat',String(material));
      for(const el of details)el.setAttribute('opacity',String(1-material));
      previousMaterial=material;
    }
    if(settle!==previousSettle){
      for(const {el,from,to} of radii)el.setAttribute('r',String(lerp(from,to,settle)));
      previousSettle=settle;
    }
    if(mark!==previousMark){
      ring.setAttribute('stroke-dashoffset',String(1-mark));
      for(const {el,r} of markTips)el.setAttribute('r',String(r*mark));
      previousMark=mark;
    }
  };
  return {art,definitions,sourceRing,targetRing,satellites,boundaryShadow,boundaryBand,boundaryLight,boundaryEdges,frame,
    invalidate(){previousScale=NaN;}};
}
