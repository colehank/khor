import {networkGeometry} from './network-geometry';

// Shared rendering only; each scene owns its playback and task state.
export function mountNetworkMap(figure:HTMLElement) {
  const svg=figure.querySelector<SVGSVGElement>('[data-net-scene]')!;
  const compact=matchMedia('(max-width: 760px)');
  const device=(id:string)=>svg.querySelector<SVGGElement>(`[data-net-device="${id}"]`)!;
  const nodes={mac:device('mac'),linux:device('linux'),windows:device('windows'),phone:device('phone')};
  const globalPath=svg.querySelector<SVGPathElement>('[data-net-global]')!;
  const globalFill=svg.querySelector<SVGPathElement>('[data-net-global-fill]')!;
  const globalMark=svg.querySelector<SVGGElement>('[data-net-brand]')!;
  const globalLabelPath=svg.querySelector<SVGPathElement>('[data-net-label-path="khor"]')!;
  const globalSubtitlePath=svg.querySelector<SVGPathElement>('[data-net-label-path="khor-subtitle"]')!;
  const connectionLabel=svg.querySelector<SVGTSpanElement>('[data-net-connection-label]')!;
  const gap=(id:string)=>svg.querySelector<SVGPathElement>(`[data-net-gap="${id}"]`)!;
  const gapIcon=(id:string)=>svg.querySelector<SVGCircleElement>(`[data-net-gap-icon="${id}"]`)!;
  const areas=new Map([...svg.querySelectorAll<SVGGElement>('[data-net-area]')].map(area=>[area.dataset.netArea!,{
    group:area,path:area.querySelector('path')!,mark:svg.querySelector<SVGGElement>(`[data-net-mark="${area.dataset.netArea}"]`)!,
    fill:svg.querySelector<SVGPathElement>(`[data-net-area-fill="${area.dataset.netArea}"]`)!,
    icon:svg.querySelector<SVGGElement>(`[data-net-mark="${area.dataset.netArea}"] [data-net-icon]`)!,
    labelPath:svg.querySelector<SVGPathElement>(`[data-net-label-path="${area.dataset.netArea}"]`)!,
    gapGroup:svg.querySelector<SVGGElement>(`[data-net-gap-group="${area.dataset.netArea}"]`)!,
  }]));
  let lastPosition=NaN,lastCompact:boolean|undefined,lastBuild=1;
  const attr=(node:Element,key:string,value:string)=>{if(node.getAttribute(key)!==value)node.setAttribute(key,value);};
  const translate=(node:Element,p:{x:number;y:number})=>attr(node,'transform',`translate(${p.x} ${p.y})`);
  const fitGap=(id:string,text:SVGTextElement,start:number)=>{
    const inset=8;
    attr(gap(id),'stroke-dasharray',`0 ${start-inset} ${text.getComputedTextLength()+inset*2} 10000`);
  };
  const fitKhorGaps=()=>{
    fitGap('khor',connectionLabel.closest('text')!,26);
    fitGap('khor-subtitle',svg.querySelector<SVGTextElement>('.net-handoff-status')!,26);
  };
  const setConnection=(connection:'p2p'|'relay')=>{
    figure.dataset.netConnection=connection;
    connectionLabel.textContent=svg.dataset.netInfrastructure==='false'?'':connection==='p2p'?'P2P':'relay';
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
      translate(area.icon,shape.mark);attr(area.labelPath,'d',shape.labelPath);placeGap(id,shape);
      const text=area.mark.querySelector<SVGTextElement>('text')!;
      fitGap(id,text,24);
    }
    translate(globalMark,reference.khorNet.mark);
    attr(globalLabelPath,'d',reference.khorNet.labelPath);
    attr(globalSubtitlePath,'d',reference.khorNet.subtitlePath);
    placeGap('khor',reference.khorNet);attr(gap('khor-subtitle'),'d',reference.khorNet.subtitlePath);
    fitKhorGaps();
  };
  const draw=(position=1,build=1)=>{
    lastBuild=build;
    figure.style.setProperty('--net-build',String(build));
    if(position===lastPosition&&compact.matches===lastCompact)return;
    const geometry=networkGeometry(compact.matches,position);
    attr(svg,'viewBox',geometry.box);
    if(compact.matches!==lastCompact)layoutLabels();
    lastPosition=position;lastCompact=compact.matches;
    for(const id of ['mac','linux','windows','phone'] as const)translate(nodes[id],geometry.nodes[id]);
    for(const [id,shape] of Object.entries(geometry.areas)){
      const area=areas.get(id)!;
      attr(area.path,'d',shape.d);area.group.style.opacity=String(shape.opacity);
      attr(area.fill,'d',shape.d);area.fill.style.opacity=String(shape.opacity);
      area.mark.style.opacity=String(shape.opacity);area.gapGroup.style.opacity=String(shape.opacity);
      attr(area.group,'aria-hidden',String(shape.opacity<.01));
    }
    attr(globalPath,'d',geometry.khorNet.d);attr(globalFill,'d',geometry.khorNet.d);
  };
  const relayout=()=>{lastCompact=undefined;draw(Number.isNaN(lastPosition)?1:lastPosition,lastBuild);};
  compact.addEventListener('change',relayout);
  window.addEventListener('resize',layoutLabels,{passive:true});
  setConnection('p2p');draw();
  return {draw,setConnection,destroy(){compact.removeEventListener('change',relayout);window.removeEventListener('resize',layoutLabels);}};
}
