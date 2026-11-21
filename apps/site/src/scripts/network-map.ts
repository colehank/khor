import {networkGeometry,networkNodeRadius,networkViewBox} from './network-geometry';
import {translate as localize} from '../i18n';

// Shared rendering only; each scene owns its playback and task state.
export function mountNetworkMap(figure:HTMLElement) {
  const svg=figure.querySelector<SVGSVGElement>('[data-net-scene]')!;
  const compact=matchMedia('(max-width: 760px)');
  const device=(id:string)=>svg.querySelector<SVGGElement>(`[data-net-device="${id}"]`)!;
  const nodes={mac:device('mac'),linux:device('linux'),windows:device('windows'),phone:device('phone')};
  const globalPath=svg.querySelector<SVGPathElement>('[data-net-global]')!;
  const globalMark=svg.querySelector<SVGGElement>('[data-net-brand]')!;
  const globalLabelPath=svg.querySelector<SVGPathElement>('[data-net-label-path="khor"]')!;
  const globalSubtitlePath=svg.querySelector<SVGPathElement>('[data-net-label-path="khor-subtitle"]')!;
  const connectionLabel=svg.querySelector<SVGTSpanElement>('[data-net-connection-label]')!;
  const brandOffset=Number(connectionLabel.parentElement!.getAttribute('startOffset'));
  const subtitleText=svg.querySelector<SVGTextPathElement>('.net-handoff-status textPath')!;
  const gap=(id:string)=>svg.querySelector<SVGPathElement>(`[data-net-gap="${id}"]`)!;
  const gapIcon=(id:string)=>svg.querySelector<SVGCircleElement>(`[data-net-gap-icon="${id}"]`)!;
  const areas=new Map([...svg.querySelectorAll<SVGGElement>('[data-net-area]')].map(area=>[area.dataset.netArea!,{
    group:area,path:area.querySelector('path')!,mark:svg.querySelector<SVGGElement>(`[data-net-mark="${area.dataset.netArea}"]`)!,
    icon:svg.querySelector<SVGGElement>(`[data-net-mark="${area.dataset.netArea}"] [data-net-icon]`)!,
    labelPath:svg.querySelector<SVGPathElement>(`[data-net-label-path="${area.dataset.netArea}"]`)!,
    gapGroup:svg.querySelector<SVGGElement>(`[data-net-gap-group="${area.dataset.netArea}"]`)!,
  }]));
  const iconGaps=[...svg.querySelectorAll<SVGCircleElement>('[data-net-gap-icon]')].map(circle=>({circle,radius:Number(circle.getAttribute('r'))}));
  const layoutListeners=new Set<()=>void>();
  let lastPosition=NaN,lastCompact:boolean|undefined,lastBuild=1,markScale=1,textScale=1;
  const attr=(node:Element,key:string,value:string)=>{if(node.getAttribute(key)!==value)node.setAttribute(key,value);};
  const translate=(node:Element,p:{x:number;y:number})=>attr(node,'transform',`translate(${p.x} ${p.y})`);
  const orientMark=(node:Element,p:{x:number;y:number;rotation:number})=>attr(node,'transform',`translate(${p.x} ${p.y}) rotate(${p.rotation})`);
  const sizeFace=(face:SVGSVGElement,size:number)=>{
    for(const [key,value] of Object.entries({x:-size/2,y:-size/2,width:size,height:size}))attr(face,key,String(value));
  };
  const fitGap=(id:string,text:SVGTextElement,start:number)=>{
    const inset=8*textScale;
    attr(gap(id),'stroke-dasharray',`0 ${start-inset} ${text.getComputedTextLength()+inset*2} 10000`);
    const clearance=svg.querySelector(`[data-net-text-clearance="${id}"]`)!;
    // A single bounding rectangle around curved text erases unrelated edges
    // in its empty corners. Follow the glyphs instead, with the same padding.
    const boxes=Array.from({length:text.getNumberOfChars()},(_,index)=>text.getExtentOfChar(index)).filter(box=>box.width&&box.height);
    while(clearance.children.length<boxes.length)clearance.append(document.createElementNS('http://www.w3.org/2000/svg','rect'));
    while(clearance.children.length>boxes.length)clearance.lastElementChild!.remove();
    boxes.forEach((box,index)=>{
      const rect=clearance.children[index];
      for(const [key,value] of Object.entries({x:box.x-inset,y:box.y-4*textScale,width:box.width+inset*2,height:box.height+8*textScale}))attr(rect,key,String(value));
    });
  };
  const fitKhorGaps=()=>{
    fitGap('khor',connectionLabel.closest('text')!,Number(connectionLabel.parentElement!.getAttribute('startOffset')));
    fitGap('khor-subtitle',svg.querySelector<SVGTextElement>('.net-handoff-status')!,Number(subtitleText.getAttribute('startOffset')));
  };
  const fitWired=(reference:ReturnType<typeof networkGeometry>)=>{
    if(svg.dataset.netInfrastructure==='false')return;
    const area=areas.get('a')!,text=area.mark.querySelector<SVGTextElement>('text')!;
    const samples=(d:string)=>{
      const path=document.createElementNS('http://www.w3.org/2000/svg','path');path.setAttribute('d',d);
      const length=path.getTotalLength(),steps=Math.ceil(length/textScale);
      return Array.from({length:steps+1},(_,i)=>path.getPointAtLength(length*i/steps));
    };
    const poses=Array.from({length:9},(_,index)=>networkGeometry(compact.matches,index/8));
    const boundaries=[...samples(reference.khorNet.d),...poses.filter((_,index)=>index%2===0).flatMap(pose=>samples(pose.areas.b.d))];
    const cellular=samples(reference.areas.c.d);
    const radius=networkNodeRadius*Math.sqrt(markScale);
    const devices=[reference.nodes.mac,reference.nodes.linux,reference.nodes.windows,...poses.map(pose=>pose.nodes.phone)];
    // Reserve the complete neighboring label, including its rotated icon.
    // Text-only bounds missed the cable icon colliding with Cellular's last
    // glyphs on compact Chinese layouts.
    const iconBox=(icon:SVGGElement)=>{
      const box=icon.getBBox(),matrix=svg.getScreenCTM()!.inverse().multiply(icon.getScreenCTM()!);
      const points=[[box.x,box.y],[box.x+box.width,box.y],[box.x,box.y+box.height],[box.x+box.width,box.y+box.height]].map(([x,y])=>new DOMPoint(x,y).matrixTransform(matrix));
      const xs=points.map(p=>p.x),ys=points.map(p=>p.y);
      return new DOMRect(Math.min(...xs),Math.min(...ys),Math.max(...xs)-Math.min(...xs),Math.max(...ys)-Math.min(...ys));
    };
    const others=['b','c'].flatMap(id=>{
      const other=areas.get(id)!,label=other.mark.querySelector<SVGTextElement>('text')!;
      return [iconBox(other.icon),...Array.from({length:label.getNumberOfChars()},(_,i)=>label.getExtentOfChar(i)).filter(b=>b.width&&b.height)];
    });
    const distance=(b:DOMRect,p:{x:number;y:number})=>Math.hypot(Math.max(b.x-p.x,0,p.x-b.x-b.width),Math.max(b.y-p.y,0,p.y-b.y-b.height));
    const separation=(a:DOMRect,b:DOMRect)=>Math.hypot(Math.max(a.x-b.x-b.width,b.x-a.x-a.width,0),Math.max(a.y-b.y-b.height,b.y-a.y-a.height,0));
    const place=(shape:typeof reference.areas.a,baseline:string)=>{
      orientMark(area.icon,shape.mark);attr(area.labelPath,'d',shape.labelPath);attr(text,'dominant-baseline',baseline);
    };
    const score=()=>{
      const boxes=[...Array.from({length:text.getNumberOfChars()},(_,i)=>text.getExtentOfChar(i)).filter(b=>b.width&&b.height),iconBox(area.icon)];
      let clearance=Infinity;
      for(const box of boxes){
        for(const node of devices)clearance=Math.min(clearance,distance(box,node)-radius-4*textScale);
        for(const point of boundaries)clearance=Math.min(clearance,distance(box,point)-5*textScale);
        for(const point of cellular)clearance=Math.min(clearance,distance(box,point)-2*textScale);
        for(const other of others)clearance=Math.min(clearance,separation(box,other)-12*textScale);
      }
      return clearance;
    };
    // Measure the real text at the displayed scale, once per layout. A short
    // viewport can magnify SVG text far beyond the device-size breakpoint.
    let best={shape:reference.areas.a,baseline:'auto',score:-Infinity};
    const angles=compact.matches?[.7,.73,.76,.79,.82,.85,.88,.91,.94,.97,.67,.64,.61,.58,.55,.52,.49]:[-.3,-.27,-.24,-.21,-.18,-.15,-.12,-.09,-.06];
    outer:for(const baseline of ['auto','central'])for(const angle of angles){
      const shape=networkGeometry(compact.matches,1,Math.PI*angle).areas.a;place(shape,baseline);
      const candidate=score();if(candidate>best.score)best={shape,baseline,score:candidate};
      if(candidate>=0)break outer;
    }
    place(best.shape,best.baseline);
    attr(gap('a'),'d',best.shape.labelPath);attr(gapIcon('a'),'cx',String(best.shape.mark.x));attr(gapIcon('a'),'cy',String(best.shape.mark.y));
    fitGap('a',text,24*markScale);
  };
  const setConnection=(connection:'p2p'|'relay')=>{
    figure.dataset.netConnection=connection;
    connectionLabel.textContent=svg.dataset.netInfrastructure==='false'?'':connection==='p2p'?'P2P':localize('relay','中继');
    fitKhorGaps();
  };
  const layoutLabels=()=>{
    // Use one reference pose per viewport. Roaming never moves a label or icon.
    const reference=networkGeometry(compact.matches,1);
    const placeGap=(id:string,shape:{mark:{x:number;y:number};labelPath:string})=>{
      attr(gap(id),'d',shape.labelPath);
      attr(gapIcon(id),'cx',String(shape.mark.x));attr(gapIcon(id),'cy',String(shape.mark.y));
    };
    for(const [id,shape] of Object.entries(reference.areas)){
      const area=areas.get(id)!;
      orientMark(area.icon,shape.mark);attr(area.labelPath,'d',shape.labelPath);placeGap(id,shape);
      const text=area.mark.querySelector<SVGTextElement>('text')!;
      attr(text,'dominant-baseline',id==='c'?'central':'auto');
      attr(text.querySelector('textPath')!,'startOffset',String(24*markScale));
      fitGap(id,text,24*markScale);
    }
    fitWired(reference);
    orientMark(globalMark,reference.khorNet.mark);
    attr(globalLabelPath,'d',reference.khorNet.labelPath);
    attr(globalSubtitlePath,'d',reference.khorNet.subtitlePath);
    placeGap('khor',reference.khorNet);attr(gap('khor-subtitle'),'d',reference.khorNet.subtitlePath);
    fitKhorGaps();
  };
  const syncMetrics=()=>{
    const width=svg.getBoundingClientRect().width;
    if(width<=0)return;
    const inverseScale=1/(svg.getScreenCTM()?.a??width/svg.viewBox.baseVal.width);
    // Cap compensation in both layouts so a narrow or short canvas still
    // shrinks the labels, icons and clearances along with the map.
    textScale=Math.min(compact.matches?1.32:2,inverseScale);
    markScale=Math.max(.9,Math.min(compact.matches?1.1:1.3,inverseScale));
    figure.style.setProperty('--net-text-scale',String(textScale));
    for(const node of Object.values(nodes)){
      attr(node.querySelector('circle')!,'r',String(networkNodeRadius*Math.sqrt(markScale)));
      sizeFace(node.querySelector<SVGSVGElement>('[data-net-device-face]')!,44*markScale);
    }
    for(const {icon} of areas.values()){
      sizeFace(icon.querySelector('svg')!,28*markScale);
    }
    sizeFace(globalMark.querySelector('svg')!,32*markScale);
    for(const {circle,radius} of iconGaps)attr(circle,'r',String(radius*markScale));
    attr(connectionLabel.parentElement!,'startOffset',String(brandOffset*markScale));
    attr(subtitleText,'startOffset',String(26*markScale));
    layoutLabels();layoutListeners.forEach(listener=>listener());
  };
  const draw=(position=1,build=1)=>{
    lastBuild=build;
    figure.style.setProperty('--net-build',String(build));
    if(position===lastPosition&&compact.matches===lastCompact)return;
    const geometry=networkGeometry(compact.matches,position);
    attr(svg,'viewBox',networkViewBox(compact.matches,svg.dataset.netExternalUser==='true'));
    if(compact.matches!==lastCompact)syncMetrics();
    lastPosition=position;lastCompact=compact.matches;
    for(const id of ['mac','linux','windows','phone'] as const)translate(nodes[id],geometry.nodes[id]);
    for(const [id,shape] of Object.entries(geometry.areas)){
      const area=areas.get(id)!;
      attr(area.path,'d',shape.d);area.group.style.opacity=String(shape.opacity);
      area.mark.style.opacity=String(shape.opacity);
      // Keep the caption's gap fully cut until the caption is gone.
      area.gapGroup.style.opacity=shape.opacity>0?'1':'0';
      attr(area.group,'aria-hidden',String(shape.opacity<.01));
    }
    attr(globalPath,'d',geometry.khorNet.d);
  };
  const relayout=()=>{lastCompact=undefined;draw(Number.isNaN(lastPosition)?1:lastPosition,lastBuild);};
  compact.addEventListener('change',relayout);
  const resize=new ResizeObserver(syncMetrics);resize.observe(svg);
  const language=()=>{setConnection(figure.dataset.netConnection==='relay'?'relay':'p2p');syncMetrics();};
  window.addEventListener('khor:localechange',language);
  setConnection('p2p');draw();
  return {draw,setConnection,measure:syncMetrics,get position(){return Number.isNaN(lastPosition)?1:lastPosition;},get markScale(){return markScale;},get textScale(){return textScale;},
    onLayout(listener:()=>void){layoutListeners.add(listener);return ()=>layoutListeners.delete(listener);},
    destroy(){resize.disconnect();layoutListeners.clear();compact.removeEventListener('change',relayout);window.removeEventListener('khor:localechange',language);}};
}
