const ns='http://www.w3.org/2000/svg';
const make=(tag:string,attributes:Record<string,string>={})=>{
  const el=document.createElementNS(ns,tag);
  for(const [name,value] of Object.entries(attributes))el.setAttribute(name,value);
  return el;
};
const lerp=(a:number,b:number,t:number)=>a+(b-a)*t;
const number=(el:Element,name:string)=>Number(el.getAttribute(name)??0);
const smooth=(t:number)=>{const p=Math.max(0,Math.min(1,t));return p*p*(3-2*p);};

/** One set of canonical shapes: the material changes in place, never crossfades. */
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
  // The panel contains a second, blurred edge. Only the outer contour is the
  // persistent diamond; selecting the first descendant masks it with the panel.
  const diamond=art.querySelector<SVGPathElement>(':scope > path[stroke*="diamond-edge"]')!;
  const inner=art.querySelector<SVGCircleElement>('circle[stroke*="ring-solid"]')!;
  const pearls=Array.from({length:5},(_,i)=>art.querySelector<SVGCircleElement>(`circle[fill*="pearl-${i}"]`)!);
  const targetRing=target.querySelector<SVGCircleElement>('circle[r="368"]')!;
  const targetDiamond=target.querySelector<SVGPathElement>('path')!;
  const targetInner=target.querySelector<SVGCircleElement>('circle[r="170"]')!;
  const targetPearls=pearls.map(pearl=>[...target.querySelectorAll<SVGCircleElement>('circle')].find(el=>el.getAttribute('cx')===pearl.getAttribute('cx')&&el.getAttribute('cy')===pearl.getAttribute('cy')&&el.parentElement?.getAttribute('fill')==='currentColor')!);
  const satellites=pearls.slice(1).map((body,index)=>{
    // One physical set of outer pearls: move these parts out of the core.
    // On the return, the machines themselves become the final icon's pearls.
    const shadow=body.previousElementSibling! as SVGElement;
    const light=body.nextElementSibling! as SVGElement;
    const rim=light.nextElementSibling! as SVGElement;
    const halo=art.querySelector<SVGCircleElement>(`:scope > circle[cx="${body.getAttribute('cx')}"][cy="${body.getAttribute('cy')}"][fill*="light-wash"]`)!;
    const parts=[halo,shadow,body,light,rim];
    parts.forEach(el=>el.remove());
    const paint=definitions.querySelector(`[id$="pearl-${index+1}"]`)!.cloneNode(true) as SVGGradientElement;
    paint.id=`${prefix}device-pearl-${index}`;definitions.append(paint);body.setAttribute('fill',`url(#${paint.id})`);
    const stops=[...paint.querySelectorAll('stop')].map(el=>({el,color:el.getAttribute('stop-color')!}));
    const highlights=[...light.querySelectorAll('ellipse')].map(el=>({el,rx:number(el,'rx'),ry:number(el,'ry')}));
    const shadowRadius=number(shadow,'r'),haloRadius=number(halo,'r'),rimWidth=number(rim,'stroke-width');
    body.setAttribute('data-morph-pearl',String(index+1));
    return {body,parts,frame(flat:number,fill:string,color:string,stroke:string,width:number){
      body.setAttribute('fill',flat===1?fill:`url(#${paint.id})`);
      for(const {el,color:sourceColor} of stops)el.style.stopColor=`color-mix(in srgb, ${sourceColor} ${(1-flat)*100}%, ${color})`;
      for(const {el,rx,ry} of highlights){el.setAttribute('rx',String(rx*(1-flat)));el.setAttribute('ry',String(ry*(1-flat)));}
      halo.setAttribute('r',String(haloRadius*(1-flat)));
      shadow.setAttribute('r',String(shadowRadius*(1-flat)));rim.setAttribute('stroke-width',String(rimWidth*(1-flat)));
      body.setAttribute('stroke',stroke);body.setAttribute('stroke-width',String(width));
    }};
  });
  const primary=new Set<Element>([ring,diamond,inner,pearls[0]]);
  const strokes=[{el:ring,target:targetRing},{el:diamond,target:targetDiamond},{el:inner,target:targetInner}].map(({el,target})=>({el,from:number(el,'stroke-width'),to:number(target,'stroke-width')}));
  const radii=[{el:inner,target:targetInner},{el:pearls[0],target:targetPearls[0]}].map(({el,target})=>({el,from:number(el,'r'),to:number(target,'r')}));

  // Lighting flattens on the existing paint servers as the glass loses depth.
  // No silhouette layer loses opacity; the same stops become a single solid ink.
  for(const gradient of definitions.querySelectorAll('linearGradient,radialGradient')){
    if(!/(glass-band|diamond-edge|ring-solid|pearl-[0-4])$/.test(gradient.id))continue;
    for(const stop of gradient.querySelectorAll('stop')){
      const color=stop.getAttribute('stop-color')!,alpha=stop.getAttribute('stop-opacity')??'1';
      (stop as SVGStopElement).style.stopColor=`color-mix(in srgb, ${color} calc((1 - var(--material-flat,0))*100%), currentColor)`;
      (stop as SVGStopElement).style.stopOpacity=`calc(${alpha} + (1 - ${alpha})*var(--material-flat,0))`;
    }
  }
  const mask=make('mask',{id:`${prefix}diamond-volume`,maskUnits:'userSpaceOnUse',x:'0',y:'0',width:'1024',height:'1024',style:'mask-type:luminance'});
  mask.append(make('rect',{width:'1024',height:'1024',fill:'white'}));
  const hole=make('circle',{cx:'512',cy:'512',r:'0',fill:'black'});mask.append(hole);definitions.append(mask);
  const panel=art.querySelector<SVGGElement>('g[clip-path*="diamond"]')!;
  panel.setAttribute('mask',`url(#${mask.id})`);
  const decorations=[...art.children].filter(el=>!primary.has(el)).map(el=>{
    if(el.tagName==='path')el.setAttribute('mask',`url(#${mask.id})`);
    const circles=el.tagName==='circle'?[el]:[];
    const lights=el.tagName==='g'?[...el.querySelectorAll('ellipse,circle')]:[];
    return {el,circles:circles.map(el=>({el,r:number(el,'r'),width:number(el,'stroke-width')})),lights:lights.map(el=>({el,rx:number(el,'rx'),ry:number(el,'ry'),r:number(el,'r')}))};
  });
  const filters=[...definitions.querySelectorAll('feGaussianBlur')].map(el=>({el,blur:number(el,'stdDeviation')}));
  const outerEdges=[...art.querySelectorAll<SVGCircleElement>('circle[stroke*="outer-edge"],circle[stroke*="inner-edge"]')].map(el=>({el,r:number(el,'r'),outside:number(el,'r')>number(ring,'r')}));
  const annulus=definitions.querySelector<SVGPathElement>('[id$="annulus"] path')!;
  let previous=NaN;
  const frame=(p:number,scale:number,sourceScale:number,targetScale:number)=>{
    // The first part visibly compresses the original volume, while the whole
    // icon is already traveling. Small-symbol weights arrive continuously.
    const material=smooth((p-.06)/.58),settle=smooth((p-.08)/.68);
    definitions.style.setProperty('--material-flat',String(material));
    for(const {el,from,to} of strokes)el.setAttribute('stroke-width',String(lerp(from*sourceScale,to*targetScale,material)/scale));
    for(const {el,from,to} of radii)el.setAttribute('r',String(lerp(from,to,settle)));
    const width=number(ring,'stroke-width'),outside=368+width/2,inside=Math.max(0,368-width/2);
    const round=(r:number)=>`M${512+r} 512 a${r} ${r} 0 1 0 ${-2*r} 0 a${r} ${r} 0 1 0 ${2*r} 0`;
    annulus.setAttribute('d',`${round(outside)} ${round(inside)}`);
    hole.setAttribute('r',String(368*material));
    if(material!==previous){
      previous=material;
      for(const {circles,lights} of decorations){
        for(const {el,r,width} of circles){if(width)el.setAttribute('stroke-width',String(width*(1-material)));else el.setAttribute('r',String(r*(1-material)));}
        for(const {el,rx,ry,r} of lights){if(rx){el.setAttribute('rx',String(rx*(1-material)));el.setAttribute('ry',String(ry*(1-material)));}else el.setAttribute('r',String(r*(1-material)));}
      }
      for(const {el,blur} of filters)el.setAttribute('stdDeviation',String(blur*(1-material)));
    }
    for(const {el,r,outside:outer} of outerEdges)el.setAttribute('r',String(lerp(r,368+(outer?1:-1)*width/2,material)));
  };
  return {art,definitions,sourceRing,targetRing,satellites,frame};
}
