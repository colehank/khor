import {createStoryMaterial} from './story-material';

const ns='http://www.w3.org/2000/svg';
const make=<K extends keyof SVGElementTagNameMap>(tag:K,attrs:Record<string,string|number>={})=>{
  const el=document.createElementNS(ns,tag);
  for(const [k,v] of Object.entries(attrs))el.setAttribute(k,String(v));
  return el;
};
const clamp=(n:number)=>Math.max(0,Math.min(1,n));
const ease=(a:number,b:number,t:number)=>{const p=clamp((t-a)/(b-a));return p*p*(3-2*p);};
const mix=(a:number,b:number,t:number)=>a+(b-a)*t;
const curve=(a:number,b:number,c:number,d:number,t:number)=>{const s=1-t;return s**3*a+3*s*s*t*b+3*s*t*t*c+t**3*d;};
const departures=[[.8,-.35],[.35,.8],[-.8,.35],[-.35,-.8]] as const;
const arrivals=[[.2,-.5],[.5,.2],[-.2,.5],[-.5,-.2]] as const;
const center=512,radius=368;
const diskPath=(r:number)=>`M${center-r} ${center}a${r} ${r} 0 1 0 ${2*r} 0a${r} ${r} 0 1 0 ${-2*r} 0Z`;

type Axes={a:number;b:number;c:number;d:number;x:number;y:number};
const identity:Axes={a:1,b:0,c:0,d:1,x:center,y:center};
const interpolateAxes=(from:Axes,to:Axes,t:number):Axes=>({
  a:mix(from.a,to.a,t),b:mix(from.b,to.b,t),c:mix(from.c,to.c,t),d:mix(from.d,to.d,t),
  x:mix(from.x,to.x,t),y:mix(from.y,to.y,t),
});

/** A shared ellipse and the original four pearls carry the physical handoff. */
export function createStoryMorph(stage:HTMLElement,icon:SVGSVGElement,graph:SVGSVGElement,prefix:string){
  const layer=make('svg',{'class':'story-morph','aria-hidden':'true'});
  layer.style.visibility='hidden';
  const targetBrand=graph.querySelector<SVGGElement>('[data-net-brand]')!;
  const material=createStoryMaterial(icon,targetBrand,layer,prefix);
  const graphRing=graph.querySelector<SVGPathElement>('[data-net-global]')!;
  const perimeter=make('g',{'data-morph-boundary':''}),ellipse=make('g'),glass=make('g'),shade=make('g');
  layer.prepend(perimeter);perimeter.append(ellipse);ellipse.append(glass);
  const {boundaryShadow:shadow,boundaryBand:band,boundaryLight:reflection,boundaryEdges:edges}=material;
  // The wire and the narrowing glass share the same centerline throughout.
  const wire=make('circle',{cx:center,cy:center,r:radius,fill:'none',stroke:'currentColor','vector-effect':'non-scaling-stroke','data-morph-perimeter':''});
  const reflectionClip=make('clipPath',{id:`${prefix}perimeter-reflections`});
  const reflectionBand=make('path',{'clip-rule':'evenodd'});reflectionClip.append(reflectionBand);
  const mask=make('mask',{id:`${prefix}perimeter-clearance`,maskUnits:'userSpaceOnUse',style:'mask-type:luminance'});
  const white=make('rect',{fill:'white'}),cutouts=make('g'),clearance=make('g');
  cutouts.append(clearance);mask.append(white,cutouts);material.definitions.append(reflectionClip,mask);
  reflection.setAttribute('clip-path',`url(#${reflectionClip.id})`);
  shade.append(shadow);
  glass.append(shade,band,reflection,...edges);ellipse.append(wire);

  const nodeLayer=make('g');
  layer.append(nodeLayer);
  const nodes=['linux','mac','phone','windows'].map((id,i)=>{
    const source=icon.querySelector<SVGCircleElement>(`circle[fill*="pearl-${i+1}"]`)!;
    const target=graph.querySelector<SVGGElement>(`[data-net-device="${id}"]`)!;
    const group=make('g',{'data-morph-device':id}),satellite=material.satellites[i];
    const glyph=target.querySelector<SVGSVGElement>('[data-net-device-face]')!.cloneNode(true) as SVGSVGElement;
    const face=make('g'),cx=source.cx.baseVal.value,cy=source.cy.baseVal.value;
    const faceOrigin=make('g',{transform:`translate(${cx} ${cy})`});
    for(const [k,v] of Object.entries({x:-.5,y:-.5,width:1,height:1}))glyph.setAttribute(k,String(v));
    face.append(glyph);faceOrigin.append(face);
    group.append(...satellite.parts,faceOrigin);nodeLayer.append(group);
    return {source,target,group,disk:satellite.disk,face,cx,cy,r:source.r.baseVal.value,scale:1,
      from:{x:0,y:0,scale:1},to:{x:0,y:0,r:0,size:0,width:0},c1:{x:0,y:0},c2:{x:0,y:0}};
  });
  stage.append(layer);
  const primary=[graphRing,...graph.querySelectorAll<SVGElement>('[data-net-device]'),targetBrand];
  const drawing=[...graph.querySelectorAll<SVGPathElement>('.net-native path,.relation-wire,.relation-access')].map(el=>({el,length:el.getAttribute('pathLength'),dash:el.style.strokeDasharray,offset:el.style.strokeDashoffset,fill:el.style.fill,marker:el.style.markerEnd}));
  const fillLayers=[...graph.querySelectorAll<SVGElement>('.net-surfaces,.relation-access-active,.relation-flow,.relation-signal,.relation-port')];
  const legends=[...graph.querySelectorAll<SVGElement>('[data-net-mark],.net-khor-mark > text,.relation-access-label,.relation-request-label,.relation-router')];
  const captions=legends.filter(el=>el.matches('.net-khor-mark > .net-boundary-name'));
  const otherLegends=legends.filter(el=>!captions.includes(el));
  const badges=[...graph.querySelectorAll<SVGElement>('.relation-badge')];
  let graphScrubbing=false,previousDraw=NaN,previousDetails:boolean|undefined;
  const restoreGraph=()=>{
    const wasHidden=graph.style.visibility==='hidden';
    graph.style.visibility='visible';
    if(!graphScrubbing){
      if(wasHidden)for(const el of captions)el.style.opacity='';
      return;
    }
    graphScrubbing=false;previousDraw=NaN;previousDetails=undefined;
    graph.style.removeProperty('--story-graph-draw');
    primary.forEach(el=>el.style.visibility='');
    for(const {el,length,dash,offset,fill,marker} of drawing){
      if(length===null)el.removeAttribute('pathLength');else el.setAttribute('pathLength',length);
      el.style.strokeDasharray=dash;el.style.strokeDashoffset=offset;el.style.fill=fill;el.style.markerEnd=marker;
    }
    [...fillLayers,...legends].forEach(el=>el.style.visibility='');
    [...otherLegends,...badges].forEach(el=>el.style.clipPath='');
    captions.forEach(el=>{el.style.opacity='';el.style.transition='';});
  };
  const graphProgress=(p:number)=>{
    if(p===1){restoreGraph();return;}
    // During the glass phase the overlay owns all visible geometry.
    if(p<.72){
      if(graph.style.visibility!=='hidden'){
        graph.style.visibility='hidden';
        if(graphScrubbing)legends.forEach(el=>el.style.visibility='hidden');
        for(const el of captions)el.style.opacity='0';
        previousDetails=undefined;
      }
      return;
    }
    const entering=graph.style.visibility!=='visible'||!graphScrubbing;
    if(entering)graph.style.visibility='visible';
    if(!graphScrubbing){
      graphScrubbing=true;
      [...primary,...fillLayers].forEach(el=>el.style.visibility='hidden');
      // In reverse the connections withdraw before their device endpoints move.
      for(const {el} of drawing){
        el.setAttribute('pathLength','1');
        el.style.strokeDasharray='1';el.style.strokeDashoffset='calc(1 - var(--story-graph-draw))';el.style.fill='none';el.style.markerEnd='none';
      }
      for(const el of captions)el.style.transition='opacity var(--dur-120)';
      for(const el of otherLegends)el.style.clipPath='inset(0 calc((1 - var(--story-graph-draw))*100%) 0 0)';
      for(const el of badges)el.style.clipPath='circle(calc(var(--story-graph-draw)*72%) at 50% 50%)';
    }
    const draw=ease(.8,.98,p),showDetails=draw>0;
    if(draw!==previousDraw){previousDraw=draw;graph.style.setProperty('--story-graph-draw',String(draw));}
    if(entering){
      for(const el of captions){el.style.visibility='visible';el.style.opacity='1';}
    }
    if(showDetails!==previousDetails){
      previousDetails=showDetails;
      for(const el of otherLegends)el.style.visibility=showDetails?'visible':'hidden';
    }
  };
  let lastProgress=0,sourceAxes=identity,targetAxes=identity,sourceScale=1,lineWidth=1,strokeOpacity=1;
  let gapCenter={x:0,y:0};
  let brandFrom={x:0,y:0,scale:1,angle:0},brandTo={x:0,y:0,scale:1,angle:0};
  let previousBoundary=NaN,previousWidth=NaN,previousInk=NaN,previousGap=NaN;
  let previousJourney=NaN,previousSurface=NaN,previousGrowth=NaN,previousTravel=NaN;
  const bandWidth=Number(band.getAttribute('stroke-width'));
  const rims=edges.map(el=>({el,radius:el.r.baseVal.value,width:Number(el.getAttribute('stroke-width'))}));
  const measure=()=>{
    const rect=stage.getBoundingClientRect(),gm=graph.getScreenCTM()!,im=material.sourceRing.getScreenCTM()!;
    const graphScale=Math.hypot(gm.a,gm.b),ringPaint=getComputedStyle(graphRing);
    layer.setAttribute('viewBox',`0 0 ${rect.width} ${rect.height}`);
    layer.style.color=getComputedStyle(targetBrand).color;
    strokeOpacity=Number(ringPaint.strokeOpacity);
    for(const el of [mask,white])for(const [key,value] of Object.entries({x:0,y:0,width:rect.width,height:rect.height}))el.setAttribute(key,String(value));
    const gapCopy=graph.querySelector('mask[id$="network-edge-clearance"] .net-khor-gap')!.cloneNode(true) as SVGElement;
    gapCopy.removeAttribute('id'); // The live graph keeps the original mask reference.
    clearance.replaceChildren(gapCopy);
    const gapIcon=graph.querySelector<SVGCircleElement>('[data-net-gap-icon="khor"]')!;
    gapCenter={x:gapIcon.cx.baseVal.value,y:gapIcon.cy.baseVal.value};
    cutouts.setAttribute('transform',`matrix(${gm.a} ${gm.b} ${gm.c} ${gm.d} ${gm.e-rect.x} ${gm.f-rect.y})`);
    const source=new DOMPoint(center,center).matrixTransform(im);
    sourceScale=Math.hypot(im.a,im.b);
    sourceAxes={a:im.a,b:im.b,c:im.c,d:im.d,x:source.x-rect.x,y:source.y-rect.y};
    // KhorNet is an ellipse. Its four cardinal vertices give its exact affine
    // axes, so the ring does not need a new spline for every scroll position.
    const points=graphRing.getAttribute('d')!.match(/-?\d+(?:\.\d+)?/g)!.map(Number);
    const quarter=(points.length-2)/4;
    const vertex=(index:number)=>new DOMPoint(points[index*quarter],points[index*quarter+1]).matrixTransform(gm);
    const [top,right,bottom,left]=[0,1,2,3].map(vertex);
    targetAxes={a:(right.x-left.x)/(2*radius),b:(right.y-left.y)/(2*radius),c:(bottom.x-top.x)/(2*radius),d:(bottom.y-top.y)/(2*radius),
      x:(top.x+bottom.x)/2-rect.x,y:(top.y+bottom.y)/2-rect.y};
    lineWidth=Number.parseFloat(ringPaint.strokeWidth)*graphScale;
    wire.setAttribute('stroke-width',String(lineWidth));
    for(const [index,node] of nodes.entries()){
      const m=node.source.getScreenCTM()!,p=new DOMPoint(node.cx,node.cy).matrixTransform(m),t=node.target.getScreenCTM()!;
      node.from={x:p.x-rect.x,y:p.y-rect.y,scale:Math.hypot(m.a,m.b)};
      const body=node.target.querySelector<SVGCircleElement>(':scope > circle')!,paint=getComputedStyle(body);
      node.to={x:t.e-rect.x,y:t.f-rect.y,r:body.r.baseVal.value*graphScale,size:Number(node.target.querySelector('[data-net-device-face]')!.getAttribute('width'))*graphScale,
        width:Number.parseFloat(paint.strokeWidth)*(paint.vectorEffect==='non-scaling-stroke'?1:graphScale)};
      node.disk.setAttribute('fill',paint.fill);
      node.disk.setAttribute('stroke',paint.stroke);
      node.c1={x:node.from.x+departures[index][0]*radius*sourceScale,y:node.from.y+departures[index][1]*radius*sourceScale};
      node.c2={x:node.to.x+arrivals[index][0]*radius*sourceScale,y:node.to.y+arrivals[index][1]*radius*sourceScale};
    }
    const tm=material.targetRing.getScreenCTM()!,destination=new DOMPoint(center,center).matrixTransform(tm);
    brandFrom={x:sourceAxes.x,y:sourceAxes.y,scale:sourceScale,angle:Math.atan2(im.b,im.a)*180/Math.PI};
    brandTo={x:destination.x-rect.x,y:destination.y-rect.y,scale:Math.hypot(tm.a,tm.b),angle:Math.atan2(tm.b,tm.a)*180/Math.PI};
    previousBoundary=previousWidth=previousInk=previousGap=NaN;
    previousJourney=previousSurface=previousGrowth=previousTravel=NaN;
    material.invalidate();
    frame(lastProgress);
  };
  const frame=(p:number)=>{
    lastProgress=p;graphProgress(p);
    const active=p>0&&p<1;
    const visibility=active?'visible':'hidden';
    if(layer.style.visibility!==visibility)layer.style.visibility=visibility;
    if(!active)return;

    const t=ease(0,.78,p),flat=ease(.14,.68,p),ink=ease(.76,1,flat);
    const {a,b,c,d,x,y}=interpolateAxes(sourceAxes,targetAxes,t);
    if(t!==previousBoundary){
      previousBoundary=t;
      ellipse.setAttribute('transform',`matrix(${a} ${b} ${c} ${d} ${x-center*(a+c)} ${y-center*(b+d)})`);
    }
    if(ink!==previousInk){
      previousInk=ink;
      glass.setAttribute('opacity',String(1-ink));
      wire.setAttribute('opacity',String(ink*strokeOpacity));
    }
    if(ink<1){
      const width=mix(bandWidth*sourceScale,lineWidth,flat)/Math.sqrt(Math.abs(a*d-b*c));
      if(width!==previousWidth){
        previousWidth=width;
        band.setAttribute('stroke-width',String(width));
        reflectionBand.setAttribute('d',diskPath(radius+width/2)+diskPath(radius-width/2));
        for(const rim of rims){
          rim.el.setAttribute('r',String(mix(radius,rim.radius,width/bandWidth)));
          rim.el.setAttribute('stroke-width',String(rim.width*(1-flat)));
        }
        shade.setAttribute('opacity',String(1-flat));
        reflection.setAttribute('opacity',String(1-flat));
      }
    }

    // Keep the large mask out of the glass phase. The caption's cutout opens
    // before the text appears, using the same measured geometry as the graph.
    const gap=ease(.5,.68,p);
    if(gap!==previousGap){
      previousGap=gap;
      if(gap===0)perimeter.removeAttribute('mask');
      else{
        if(!perimeter.hasAttribute('mask'))perimeter.setAttribute('mask',`url(#${mask.id})`);
        clearance.setAttribute('transform',`translate(${gapCenter.x} ${gapCenter.y}) scale(${gap}) translate(${-gapCenter.x} ${-gapCenter.y})`);
      }
    }

    const growth=ease(.28,.62,p),surface=ease(.08,.58,p),journey=ease(.12,.8,p);
    if(surface!==previousSurface)nodeLayer.style.setProperty('--story-pearl-light',String(1-surface));
    if(journey!==previousJourney||surface!==previousSurface||growth!==previousGrowth){
      for(const node of nodes){
        if(journey!==previousJourney){
          node.scale=mix(node.r*node.from.scale,node.to.r,journey)/node.r;
          const x=curve(node.from.x,node.c1.x,node.c2.x,node.to.x,journey),y=curve(node.from.y,node.c1.y,node.c2.y,node.to.y,journey);
          node.group.setAttribute('transform',`translate(${x} ${y}) scale(${node.scale}) translate(${-node.cx} ${-node.cy})`);
        }
        if(journey!==previousJourney||surface!==previousSurface)node.disk.setAttribute('stroke-width',String(node.to.width*surface/node.scale));
        if(journey!==previousJourney||growth!==previousGrowth)node.face.setAttribute('transform',`scale(${node.to.size*growth/node.scale})`);
      }
      previousJourney=journey;previousSurface=surface;previousGrowth=growth;
    }

    const travel=ease(.12,.72,p),scale=Math.exp(mix(Math.log(brandFrom.scale),Math.log(brandTo.scale),travel));
    if(travel!==previousTravel){
      previousTravel=travel;
      const coreX=mix(brandFrom.x,brandTo.x,travel),coreY=mix(brandFrom.y,brandTo.y,travel);
      material.art.setAttribute('transform',`translate(${coreX} ${coreY}) rotate(${mix(brandFrom.angle,brandTo.angle,travel)}) scale(${scale}) translate(${-center} ${-center})`);
    }
    material.frame(p,scale,brandFrom.scale,brandTo.scale);
  };
  return {measure,frame,destroy(){restoreGraph();layer.remove();}};
}
