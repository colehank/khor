import {createStoryMaterial} from './story-material';

const ns='http://www.w3.org/2000/svg';
const make=(tag:string,attrs:Record<string,string|number>={})=>{const el=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attrs))el.setAttribute(k,String(v));return el;};
const clamp=(n:number)=>Math.max(0,Math.min(1,n));
const ease=(a:number,b:number,t:number)=>{const p=clamp((t-a)/(b-a));return p*p*(3-2*p);};
const mix=(a:number,b:number,t:number)=>a+(b-a)*t;
const nums=(d:string)=>[...d.matchAll(/-?\d+(?:\.\d+)?/g)].map(m=>Number(m[0]));
const paintColor=(paint:string)=>{
  const id=paint.match(/url\(["']?[^#]*#([^"')]+)["']?\)/)?.[1];
  if(!id)return paint;
  const stops=[...document.getElementById(id)!.querySelectorAll('stop')];
  return `color-mix(in srgb, ${getComputedStyle(stops[0]).stopColor} 50%, ${getComputedStyle(stops.at(-1)!).stopColor})`;
};
function circle(x:number,y:number,r:number){
  const step=Math.PI*2/48;let d=`M ${x} ${y-r}`;
  for(let i=0;i<48;i++){const a=-Math.PI/2+i*step,b=a+step;d+=` C ${x+r*Math.cos(a)-r*Math.sin(a)*step/3} ${y+r*Math.sin(a)+r*Math.cos(a)*step/3} ${x+r*Math.cos(b)+r*Math.sin(b)*step/3} ${y+r*Math.sin(b)-r*Math.cos(b)*step/3} ${x+r*Math.cos(b)} ${y+r*Math.sin(b)}`;}return d+' Z';
}

/** Only the perimeter is copied; the four devices are the icon's outer pearls. */
export function createStoryMorph(stage:HTMLElement,icon:SVGSVGElement,graph:SVGSVGElement,prefix:string){
  const layer=make('svg',{'class':'story-morph','aria-hidden':'true'});layer.style.visibility='hidden';
  const targetBrand=graph.querySelector<SVGGElement>('[data-net-brand]')!;
  const material=createStoryMaterial(icon,targetBrand,layer,prefix);
  const graphRing=graph.querySelector<SVGPathElement>('[data-net-global]')!;
  // Copy the whole closed glass band, not a new line traced around its outline.
  const perimeter=make('g',{'data-morph-boundary':''});layer.prepend(perimeter);
  const bands=[['glass-band',368,30,1],['outer-edge',382.5,2.2,1],['inner-edge',353.5,1.1,.864]].map(([name,radius,width,alpha],i)=>{
    const paint=material.definitions.querySelector(`[id$="${name}"]`)!.cloneNode(true) as SVGGradientElement;
    paint.id=`${prefix}perimeter-${name}`;
    for(const stop of paint.querySelectorAll('stop')){
      const color=stop.getAttribute('stop-color')!,opacity=stop.getAttribute('stop-opacity')??'1';
      stop.style.stopColor=`color-mix(in srgb, ${color} calc((1 - var(--perimeter-ink,0))*100%), currentColor)`;
      stop.style.stopOpacity=`calc(${opacity} + (1 - ${opacity})*var(--perimeter-ink,0))`;
    }
    material.definitions.append(paint);
    const path=make('path',{'class':'story-morph-ring',...(i===0?{'data-morph-perimeter':''}:{})});path.style.stroke=`url(#${paint.id})`;perimeter.append(path);
    return {path,paint,radius:Number(radius),width:Number(width),alpha:Number(alpha),from:[] as number[]};
  });
  const mask=make('mask',{id:`${prefix}perimeter-clearance`,maskUnits:'userSpaceOnUse',style:'mask-type:luminance'}),white=make('rect',{fill:'white'}),cutouts=make('g'),clearance=make('g');
  cutouts.append(clearance);
  mask.append(white,cutouts);layer.querySelector('defs')!.append(mask);layer.style.setProperty('--net-build','1');
  perimeter.setAttribute('mask',`url(#${mask.id})`);
  const nodes=['linux','mac','phone','windows'].map((id,i)=>{
    const source=icon.querySelector<SVGCircleElement>(`circle[fill*="pearl-${i+1}"]`)!;
    const target=graph.querySelector<SVGGElement>(`[data-net-device="${id}"]`)!;
    const group=make('g',{'data-morph-device':id}),satellite=material.satellites[i];
    const glyph=target.querySelector<SVGSVGElement>('[data-net-device-face]')!.cloneNode(true) as SVGSVGElement;
    group.append(...satellite.parts,glyph);layer.append(group);
    return {source,target,group,satellite,glyph,cx:source.cx.baseVal.value,cy:source.cy.baseVal.value,r:source.r.baseVal.value,
      from:{x:0,y:0,scale:1},to:{x:0,y:0,r:0,size:0,fill:'',color:'',stroke:'',width:0}};
  });
  stage.append(layer);
  const primary=[graphRing,...graph.querySelectorAll<SVGElement>('[data-net-device]'),targetBrand];
  const drawing=[...graph.querySelectorAll<SVGPathElement>('.net-native path,.relation-wire,.relation-access')].map(el=>({el,length:el.getAttribute('pathLength'),dash:el.style.strokeDasharray,offset:el.style.strokeDashoffset,fill:el.style.fill,marker:el.style.markerEnd}));
  const fillLayers=[...graph.querySelectorAll<SVGElement>('.net-surfaces,.relation-access-active,.relation-flow,.relation-signal,.relation-port')];
  const legends=[...graph.querySelectorAll<SVGElement>('[data-net-mark],.net-khor-mark > text,.relation-access-label,.relation-request-label,.relation-router')];
  const captions=new Set(graph.querySelectorAll<SVGElement>('.net-khor-mark > .net-boundary-name'));
  const badges=[...graph.querySelectorAll<SVGElement>('.relation-badge')];
  let previousGraph=NaN,lastProgress=0;
  const restoreGraph=()=>{
    graph.style.visibility='visible';primary.forEach(el=>el.style.visibility='');
    for(const {el,length,dash,offset,fill,marker} of drawing){if(length===null)el.removeAttribute('pathLength');else el.setAttribute('pathLength',length);el.style.strokeDasharray=dash;el.style.strokeDashoffset=offset;el.style.fill=fill;el.style.markerEnd=marker;}
    [...fillLayers,...legends].forEach(el=>el.style.visibility='');[...legends,...badges].forEach(el=>el.style.clipPath='');
    captions.forEach(el=>{el.style.opacity='';el.style.transition='';});
  };
  const graphProgress=(p:number)=>{
    if(p===previousGraph)return;previousGraph=p;
    if(p===1){restoreGraph();return;}
    graph.style.visibility=p===0?'hidden':'visible';primary.forEach(el=>el.style.visibility='hidden');
    // Connections withdraw first on the way back. Nodes begin moving only once
    // these paths are gone, so their endpoints never detach from their hosts.
    const draw=ease(.8,.98,p);
    for(const {el} of drawing){el.setAttribute('pathLength','1');el.style.strokeDasharray='1';el.style.strokeDashoffset=String(1-draw);el.style.fill='none';el.style.markerEnd='none';}
    fillLayers.forEach(el=>el.style.visibility='hidden');
    for(const el of legends){
      if(captions.has(el)){
        const visible=p>=.72;
        el.style.visibility=visible?'visible':'hidden';el.style.clipPath='';
        el.style.transition='opacity var(--dur-120)';el.style.opacity=visible?'1':'0';
      }else{
        el.style.visibility=draw>0?'visible':'hidden';el.style.clipPath=`inset(0 ${(1-draw)*100}% 0 0)`;
      }
    }
    badges.forEach(el=>el.style.clipPath=`circle(${draw*72}% at 50% 50%)`);
  };
  let template='',to:number[]=[],lineWidth=1,sourceScale=1,strokeOpacity=1;
  let gapCenter={x:0,y:0};
  let brandFrom={x:0,y:0,scale:1,angle:0},brandTo={x:0,y:0,scale:1,angle:0};
  const measure=()=>{
    const rect=stage.getBoundingClientRect(),gm=graph.getScreenCTM()!,im=material.sourceRing.getScreenCTM()!;
    layer.setAttribute('viewBox',`0 0 ${rect.width} ${rect.height}`);
    layer.style.color=getComputedStyle(targetBrand).color;
    strokeOpacity=Number(getComputedStyle(graphRing).strokeOpacity);
    for(const el of [mask,white])for(const [key,value] of Object.entries({x:0,y:0,width:rect.width,height:rect.height}))el.setAttribute(key,String(value));
    clearance.innerHTML=graph.querySelector('mask[id$="network-edge-clearance"] .net-khor-gap')!.outerHTML;
    const gapIcon=graph.querySelector<SVGCircleElement>('[data-net-gap-icon="khor"]')!;gapCenter={x:gapIcon.cx.baseVal.value,y:gapIcon.cy.baseVal.value};
    cutouts.setAttribute('transform',`matrix(${gm.a} ${gm.b} ${gm.c} ${gm.d} ${gm.e-rect.x} ${gm.f-rect.y})`);
    const c=new DOMPoint(512,512).matrixTransform(im);
    sourceScale=Math.hypot(im.a,im.b);template=graphRing.getAttribute('d')!;
    to=nums(template).map((v,i)=>i%2?v*gm.d+gm.f-rect.y:v*gm.a+gm.e-rect.x);lineWidth=1.5*gm.a;
    for(const band of bands){
      band.from=nums(circle(c.x-rect.x,c.y-rect.y,band.radius*sourceScale));
    }
    for(const node of nodes){
      const m=node.source.getScreenCTM()!,p=new DOMPoint(node.source.cx.baseVal.value,node.source.cy.baseVal.value).matrixTransform(m),t=node.target.getScreenCTM()!;
      node.from={x:p.x-rect.x,y:p.y-rect.y,scale:Math.hypot(m.a,m.b)};
      const paint=getComputedStyle(node.target.querySelector(':scope > circle')!);
      node.to={x:t.e-rect.x,y:t.f-rect.y,r:node.target.querySelector<SVGCircleElement>(':scope > circle')!.r.baseVal.value*gm.a,size:Number(node.target.querySelector('[data-net-device-face]')!.getAttribute('width'))*gm.a,
        fill:paint.fill,color:paintColor(paint.fill),stroke:paint.stroke,width:Number.parseFloat(paint.strokeWidth)*(paint.vectorEffect==='non-scaling-stroke'?1:gm.a)};
      node.group.style.color=getComputedStyle(node.target).color;
    }
    const tm=material.targetRing.getScreenCTM()!,center=new DOMPoint(512,512).matrixTransform(tm);
    brandFrom={x:c.x-rect.x,y:c.y-rect.y,scale:Math.hypot(im.a,im.b),angle:Math.atan2(im.b,im.a)*180/Math.PI};
    brandTo={x:center.x-rect.x,y:center.y-rect.y,scale:Math.hypot(tm.a,tm.b),angle:Math.atan2(tm.b,tm.a)*180/Math.PI};
    frame(lastProgress);
  };
  const frame=(p:number)=>{
    lastProgress=p;graphProgress(p);
    if(p<=0||p>=1){layer.style.visibility='hidden';return;}
    layer.style.visibility='visible';
    const t=ease(.04,.78,p),flat=1-Math.pow(1-clamp(p/.58),3);
    // Keep the copied glass's theme-aware reflections while it has volume.
    // Its green wire paint arrives only after most of that thickness is gone;
    // using the width progress directly made an opaque olive ribbon in daylight.
    const ink=ease(.76,1,flat);
    material.definitions.style.setProperty('--perimeter-ink',String(ink));
    const bandWidth=mix(bands[0].width*sourceScale,lineWidth,flat);
    const outline=to.map((value,i)=>mix(bands[0].from[i],value,t));
    if(ink<1){
      // Carry the same diagonal reflection across the expanding glass. Keeping
      // the original icon's tiny paint bounds clamped the outer arc to one stop.
      let left=Infinity,top=Infinity,right=-Infinity,bottom=-Infinity;
      for(let i=0;i<outline.length;i+=2){left=Math.min(left,outline[i]);right=Math.max(right,outline[i]);top=Math.min(top,outline[i+1]);bottom=Math.max(bottom,outline[i+1]);}
      const sx=(right-left)/736,sy=(bottom-top)/736;
      const transform=`matrix(${sx} 0 0 ${sy} ${(left+right)/2-512*sx} ${(top+bottom)/2-512*sy})`;
      for(const band of bands)band.paint.setAttribute('gradientTransform',transform);
    }
    for(const [index,band] of bands.entries()){
      // Rims stay on the narrowing band. Their distance must follow its
      // thickness, otherwise the highlights become two stray dotted rings.
      const rimScale=bandWidth/(bands[0].width*sourceScale);
      let i=0;band.path.setAttribute('d',template.replace(/-?\d+(?:\.\d+)?/g,()=>{const n=i++;return String(outline[n]+(band.from[n]-bands[0].from[n])*rimScale);}));
      band.path.setAttribute('stroke-width',String(index===0?bandWidth:band.width*sourceScale*(1-flat)));
      band.path.setAttribute('stroke-opacity',String(index===0?mix(1,strokeOpacity,flat):band.alpha));
    }
    // The opening grows geometrically, then the caption reveals into it.
    // No caption is visible while a line still occupies its reserved space.
    const gap=ease(.5,.68,p);
    clearance.setAttribute('transform',`translate(${gapCenter.x} ${gapCenter.y}) scale(${gap}) translate(${-gapCenter.x} ${-gapCenter.y})`);
    const growth=ease(.28,.62,p),surface=ease(.08,.58,p);
    nodes.forEach(node=>{
      const scale=mix(node.r*node.from.scale,node.to.r,t)/node.r;
      node.group.setAttribute('transform',`translate(${mix(node.from.x,node.to.x,t)} ${mix(node.from.y,node.to.y,t)}) scale(${scale}) translate(${-node.cx} ${-node.cy})`);
      node.satellite.frame(surface,node.to.fill,node.to.color,node.to.stroke,node.to.width*surface/scale);
      const size=node.to.size*growth/scale;for(const [k,v] of Object.entries({x:node.cx-size/2,y:node.cy-size/2,width:size,height:size}))node.glyph.setAttribute(k,String(v));
    });
    const travel=ease(0,.72,p),scale=Math.exp(mix(Math.log(brandFrom.scale),Math.log(brandTo.scale),travel));
    material.art.setAttribute('transform',`translate(${mix(brandFrom.x,brandTo.x,travel)} ${mix(brandFrom.y,brandTo.y,travel)}) rotate(${mix(brandFrom.angle,brandTo.angle,travel)}) scale(${scale}) translate(-512 -512)`);
    material.frame(p,scale,brandFrom.scale,brandTo.scale);
  };
  return {measure,frame,showStatic(){previousGraph=NaN;graphProgress(1);layer.style.visibility='hidden';},destroy(){restoreGraph();layer.remove();}};
}
