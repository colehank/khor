import {createStoryMaterial} from './story-material';

const ns='http://www.w3.org/2000/svg';
const make=(tag:string,attrs:Record<string,string|number>={})=>{const el=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attrs))el.setAttribute(k,String(v));return el;};
const clamp=(n:number)=>Math.max(0,Math.min(1,n));
const ease=(a:number,b:number,t:number)=>{const p=clamp((t-a)/(b-a));return p*p*(3-2*p);};
const mix=(a:number,b:number,t:number)=>a+(b-a)*t;
const curve=(a:number,b:number,c:number,d:number,t:number)=>{const s=1-t;return s**3*a+3*s*s*t*b+3*s*t*t*c+t**3*d;};
const departures=[[.8,-.35],[.35,.8],[-.8,.35],[-.35,-.8]] as const;
const arrivals=[[.2,-.5],[.5,.2],[-.2,.5],[-.5,-.2]] as const;
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

/** One large perimeter; the four devices are the icon's outer pearls. */
export function createStoryMorph(stage:HTMLElement,icon:SVGSVGElement,graph:SVGSVGElement,prefix:string){
  const layer=make('svg',{'class':'story-morph','aria-hidden':'true'});layer.style.visibility='hidden';
  const targetBrand=graph.querySelector<SVGGElement>('[data-net-brand]')!;
  const material=createStoryMaterial(icon,targetBrand,layer,prefix);
  const graphRing=graph.querySelector<SVGPathElement>('[data-net-global]')!;
  // This owns the source icon's outer ring throughout the handoff. The core
  // carries only its diamond, inner circle and center, never a duplicate band.
  const perimeter=make('g',{'data-morph-boundary':''});layer.prepend(perimeter);
  const shadow=make('path');
  for(const attr of material.boundaryShadow.attributes)if(!['cx','cy','r'].includes(attr.name))shadow.setAttribute(attr.name,attr.value);
  perimeter.append(shadow);
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
  // Reparent the actual reflections, rather than retaining a lit ring in both
  // the shrinking core and the expanding boundary.
  const reflectedLight=material.boundaryLight;
  reflectedLight.removeAttribute('clip-path');
  const reflections=make('g'),reflectionMask=make('mask',{id:`${prefix}perimeter-reflections`,maskUnits:'userSpaceOnUse',style:'mask-type:alpha'});
  const reflectionBand=make('path',{fill:'none',stroke:'white'});reflectionMask.append(reflectionBand);material.definitions.append(reflectionMask);
  reflections.setAttribute('mask',`url(#${reflectionMask.id})`);reflections.append(reflectedLight);bands[0].path.after(reflections);
  const lightShapes=[...reflectedLight.querySelectorAll<SVGElement>('ellipse,circle')].map(el=>({el,rx:Number(el.getAttribute('rx')),ry:Number(el.getAttribute('ry')),r:Number(el.getAttribute('r'))}));
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
      from:{x:0,y:0,scale:1},to:{x:0,y:0,r:0,size:0,fill:'',color:'',stroke:'',width:0},c1:{x:0,y:0},c2:{x:0,y:0}};
  });
  stage.append(layer);
  const primary=[graphRing,...graph.querySelectorAll<SVGElement>('[data-net-device]'),targetBrand];
  const drawing=[...graph.querySelectorAll<SVGPathElement>('.net-native path,.relation-wire,.relation-access')].map(el=>({el,length:el.getAttribute('pathLength'),dash:el.style.strokeDasharray,offset:el.style.strokeDashoffset,fill:el.style.fill,marker:el.style.markerEnd}));
  const fillLayers=[...graph.querySelectorAll<SVGElement>('.net-surfaces,.relation-access-active,.relation-flow,.relation-signal,.relation-port')];
  const legends=[...graph.querySelectorAll<SVGElement>('[data-net-mark],.net-khor-mark > text,.relation-access-label,.relation-request-label,.relation-router')];
  const captions=new Set(graph.querySelectorAll<SVGElement>('.net-khor-mark > .net-boundary-name'));
  const badges=[...graph.querySelectorAll<SVGElement>('.relation-badge')];
  let previousLayerState='',previousDraw=NaN,lastProgress=0;
  const restoreGraph=()=>{
    graph.style.visibility='visible';primary.forEach(el=>el.style.visibility='');
    for(const {el,length,dash,offset,fill,marker} of drawing){if(length===null)el.removeAttribute('pathLength');else el.setAttribute('pathLength',length);el.style.strokeDasharray=dash;el.style.strokeDashoffset=offset;el.style.fill=fill;el.style.markerEnd=marker;}
    [...fillLayers,...legends].forEach(el=>el.style.visibility='');[...legends,...badges].forEach(el=>el.style.clipPath='');
    captions.forEach(el=>{el.style.opacity='';el.style.transition='';});
  };
  const graphProgress=(p:number)=>{
    const draw=ease(.8,.98,p),captionVisible=p>=.72;
    const layerState=p===1?'complete':p===0?'hidden':captionVisible?'caption':'outline';
    if(layerState===previousLayerState&&draw===previousDraw)return;
    previousLayerState=layerState;previousDraw=draw;
    if(p===1){restoreGraph();return;}
    graph.style.visibility=p===0?'hidden':'visible';primary.forEach(el=>el.style.visibility='hidden');
    // Connections withdraw first on the way back. Nodes begin moving only once
    // these paths are gone, so their endpoints never detach from their hosts.
    const dashOffset=String(1-draw);
    for(const {el} of drawing){el.setAttribute('pathLength','1');el.style.strokeDasharray='1';el.style.strokeDashoffset=dashOffset;el.style.fill='none';el.style.markerEnd='none';}
    fillLayers.forEach(el=>el.style.visibility='hidden');
    for(const el of legends){
      if(captions.has(el)){
        el.style.visibility=captionVisible?'visible':'hidden';el.style.clipPath='';
        el.style.transition='opacity var(--dur-120)';el.style.opacity=captionVisible?'1':'0';
      }else{
        el.style.visibility=draw>0?'visible':'hidden';el.style.clipPath=`inset(0 ${(1-draw)*100}% 0 0)`;
      }
    }
    badges.forEach(el=>el.style.clipPath=`circle(${draw*72}% at 50% 50%)`);
  };
  let pathParts:string[]=[],to:number[]=[],lineWidth=1,sourceScale=1,strokeOpacity=1;
  let gapCenter={x:0,y:0};
  let brandFrom={x:0,y:0,scale:1,angle:0},brandTo={x:0,y:0,scale:1,angle:0};
  const measure=()=>{
    const rect=stage.getBoundingClientRect(),gm=graph.getScreenCTM()!,im=material.sourceRing.getScreenCTM()!;
    layer.setAttribute('viewBox',`0 0 ${rect.width} ${rect.height}`);
    layer.style.color=getComputedStyle(targetBrand).color;
    strokeOpacity=Number(getComputedStyle(graphRing).strokeOpacity);
    for(const el of [mask,white,reflectionMask])for(const [key,value] of Object.entries({x:0,y:0,width:rect.width,height:rect.height}))el.setAttribute(key,String(value));
    const gapCopy=graph.querySelector('mask[id$="network-edge-clearance"] .net-khor-gap')!.cloneNode(true) as SVGElement;
    gapCopy.removeAttribute('id'); // The live graph owns the ID referenced by its outer-edge mask.
    clearance.replaceChildren(gapCopy);
    const gapIcon=graph.querySelector<SVGCircleElement>('[data-net-gap-icon="khor"]')!;gapCenter={x:gapIcon.cx.baseVal.value,y:gapIcon.cy.baseVal.value};
    cutouts.setAttribute('transform',`matrix(${gm.a} ${gm.b} ${gm.c} ${gm.d} ${gm.e-rect.x} ${gm.f-rect.y})`);
    const c=new DOMPoint(512,512).matrixTransform(im);
    sourceScale=Math.hypot(im.a,im.b);
    const template=graphRing.getAttribute('d')!;
    pathParts=template.split(/-?\d+(?:\.\d+)?/g);
    to=nums(template).map((v,i)=>i%2?v*gm.d+gm.f-rect.y:v*gm.a+gm.e-rect.x);lineWidth=1.5*gm.a;
    for(const band of bands){
      band.from=nums(circle(c.x-rect.x,c.y-rect.y,band.radius*sourceScale));
    }
    for(const [index,node] of nodes.entries()){
      const m=node.source.getScreenCTM()!,p=new DOMPoint(node.source.cx.baseVal.value,node.source.cy.baseVal.value).matrixTransform(m),t=node.target.getScreenCTM()!;
      node.from={x:p.x-rect.x,y:p.y-rect.y,scale:Math.hypot(m.a,m.b)};
      const paint=getComputedStyle(node.target.querySelector(':scope > circle')!);
      node.to={x:t.e-rect.x,y:t.f-rect.y,r:node.target.querySelector<SVGCircleElement>(':scope > circle')!.r.baseVal.value*gm.a,size:Number(node.target.querySelector('[data-net-device-face]')!.getAttribute('width'))*gm.a,
        fill:paint.fill,color:paintColor(paint.fill),stroke:paint.stroke,width:Number.parseFloat(paint.strokeWidth)*(paint.vectorEffect==='non-scaling-stroke'?1:gm.a)};
      const radius=368*sourceScale;
      node.c1={x:node.from.x+departures[index][0]*radius,y:node.from.y+departures[index][1]*radius};
      node.c2={x:node.to.x+arrivals[index][0]*radius,y:node.to.y+arrivals[index][1]*radius};
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
    // The single source band expands, then loses its thickness. There is no
    // original full-size ring left behind to separate from or mask against.
    const t=ease(0,.78,p),flat=ease(.14,.68,p);
    // Keep the glass's theme-aware reflections while it has volume.
    // Its green wire paint arrives only after most of that thickness is gone;
    // using the width progress directly made an opaque olive ribbon in daylight.
    const ink=ease(.76,1,flat);
    material.definitions.style.setProperty('--perimeter-ink',String(ink));
    const bandWidth=mix(bands[0].width*sourceScale,lineWidth,flat);
    const outline=to.map((value,i)=>mix(bands[0].from[i],value,t));
    if(ink<1){
      // The shared 48-segment path keeps its four cardinal anchors. They define
      // the source circle's affine transform, including the final ellipse tilt.
      const a=(outline[72]-outline[216])/736,b=(outline[73]-outline[217])/736;
      const c=(outline[144]-outline[0])/736,d=(outline[145]-outline[1])/736;
      const x=(outline[0]+outline[144])/2,y=(outline[1]+outline[145])/2;
      const transform=`matrix(${a} ${b} ${c} ${d} ${x-512*(a+c)} ${y-512*(b+d)})`;
      for(const band of bands)band.paint.setAttribute('gradientTransform',transform);
      reflectedLight.setAttribute('transform',transform);
    }
    // Rims follow the narrowing band so highlights stay on the glass.
    const rimScale=bandWidth/(bands[0].width*sourceScale);
    let bandPath='';
    for(const [index,band] of bands.entries()){
      let path=pathParts[0];
      for(let i=0;i<outline.length;i++){
        path+=String(outline[i]+(band.from[i]-bands[0].from[i])*rimScale)+pathParts[i+1];
      }
      band.path.setAttribute('d',path);
      if(index===0)bandPath=path;
      band.path.setAttribute('stroke-width',String(index===0?bandWidth:band.width*sourceScale*(1-flat)));
      band.path.setAttribute('stroke-opacity',String(index===0?mix(1,strokeOpacity,flat):band.alpha));
    }
    reflectionBand.setAttribute('d',bandPath);reflectionBand.setAttribute('stroke-width',String(bandWidth));
    shadow.setAttribute('d',bandPath);
    shadow.setAttribute('stroke-width',String(Number(material.boundaryShadow.getAttribute('stroke-width'))*sourceScale*(1-flat)));
    shadow.setAttribute('transform',`translate(${(Number(material.boundaryShadow.getAttribute('cx'))-512)*sourceScale*(1-flat)} ${(Number(material.boundaryShadow.getAttribute('cy'))-512)*sourceScale*(1-flat)})`);
    for(const {el,rx,ry,r} of lightShapes){
      if(rx){el.setAttribute('rx',String(rx*(1-flat)));el.setAttribute('ry',String(ry*(1-flat)));}
      else el.setAttribute('r',String(r*(1-flat)));
    }
    // The opening grows geometrically, then the caption reveals into it.
    // No caption is visible while a line still occupies its reserved space.
    const gap=ease(.5,.68,p);
    clearance.setAttribute('transform',`translate(${gapCenter.x} ${gapCenter.y}) scale(${gap}) translate(${-gapCenter.x} ${-gapCenter.y})`);
    const growth=ease(.28,.62,p),surface=ease(.08,.58,p);
    const journey=ease(.12,.8,p);
    nodes.forEach(node=>{
      // Depart around the ring's exterior, then arrive from the device's outer
      // quadrant. Reversing these curves keeps pearls clear of the growing core.
      const scale=mix(node.r*node.from.scale,node.to.r,journey)/node.r;
      const x=curve(node.from.x,node.c1.x,node.c2.x,node.to.x,journey),y=curve(node.from.y,node.c1.y,node.c2.y,node.to.y,journey);
      node.group.setAttribute('transform',`translate(${x} ${y}) scale(${scale}) translate(${-node.cx} ${-node.cy})`);
      node.satellite.frame(surface,node.to.fill,node.to.color,node.to.stroke,node.to.width*surface/scale);
      const size=node.to.size*growth/scale;for(const [k,v] of Object.entries({x:node.cx-size/2,y:node.cy-size/2,width:size,height:size}))node.glyph.setAttribute(k,String(v));
    });
    const travel=ease(.12,.72,p),scale=Math.exp(mix(Math.log(brandFrom.scale),Math.log(brandTo.scale),travel));
    const coreX=mix(brandFrom.x,brandTo.x,travel),coreY=mix(brandFrom.y,brandTo.y,travel);
    material.art.setAttribute('transform',`translate(${coreX} ${coreY}) rotate(${mix(brandFrom.angle,brandTo.angle,travel)}) scale(${scale}) translate(-512 -512)`);
    material.frame(p,scale,brandFrom.scale,brandTo.scale);
  };
  return {measure,frame,destroy(){restoreGraph();layer.remove();}};
}
