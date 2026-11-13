import {networkGeometry} from './network-geometry';

// The introduction runs once. iOS then roams only between Wi-Fi and cellular.
const phases=[
  {id:'native',location:'b',position:1,build:0,connection:'p2p',duration:1600,transition:0},
  {id:'establish',location:'b',position:1,build:1,connection:'relay',duration:2600,transition:2000},
  {id:'connected',location:'b',position:1,build:1,connection:'p2p',duration:1800,transition:0},
  {id:'cellular-arrive',location:'c',position:0,build:1,connection:'relay',duration:3000,transition:1700},
  {id:'cellular',location:'c',position:0,build:1,connection:'p2p',duration:2000,transition:0},
  {id:'wifi-arrive',location:'b',position:1,build:1,connection:'relay',duration:3000,transition:1700},
  {id:'wifi',location:'b',position:1,build:1,connection:'p2p',duration:2000,transition:0},
] as const;

export function mountNetworkScene(figure:HTMLElement) {
  const svg=figure.querySelector<SVGSVGElement>('[data-net-scene]')!;
  const compact=matchMedia('(max-width: 760px)'),reduced=matchMedia('(prefers-reduced-motion: reduce)');
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
  let phaseIndex=reduced.matches?2:0,position=1,build=reduced.matches?1:0,active=false;
  let remaining:number=phases[phaseIndex].duration,started=0,frame=0,timer:ReturnType<typeof setTimeout>|undefined;
  let movement:{from:number;to:number;fromBuild:number;toBuild:number;duration:number;elapsed:number;started:number}|null=null;
  let lastPosition=NaN,lastCompact:boolean|undefined;
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
    connectionLabel.textContent=connection==='p2p'?'P2P':'relay';
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
  const draw=()=>{
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
  const tick=()=>{
    if(!active||!movement)return;
    const elapsed=Math.min(movement.duration,movement.elapsed+performance.now()-movement.started);
    const t=elapsed/movement.duration,progress=t*t*(3-2*t);
    position=movement.from+(movement.to-movement.from)*progress;
    build=movement.fromBuild+(movement.toBuild-movement.fromBuild)*progress;
    draw();
    if(elapsed<movement.duration)frame=requestAnimationFrame(tick);
    else{movement=null;frame=0;figure.dataset.netTravel='false';}
  };
  const syncPhase=()=>{
    const phase=phases[phaseIndex];
    figure.dataset.netPhase=phase.id;figure.dataset.location=phase.location;
    setConnection(phase.connection);
  };
  const schedule=()=>{
    if(!active)return;
    started=performance.now();timer=setTimeout(()=>setPhase(phaseIndex===phases.length-1?3:phaseIndex+1),remaining);
  };
  const setPhase=(index:number)=>{
    clearTimeout(timer);cancelAnimationFrame(frame);phaseIndex=index;syncPhase();
    const phase=phases[index];remaining=phase.duration;
    figure.dataset.netTravel=String(position!==phase.position);
    if(active&&!reduced.matches&&phase.transition&&(position!==phase.position||build!==phase.build)){
      movement={from:position,to:phase.position,fromBuild:build,toBuild:phase.build,duration:phase.transition,elapsed:0,started:performance.now()};tick();
    }else{position=phase.position;build=phase.build;movement=null;figure.dataset.netTravel='false';draw();}
    schedule();
  };
  const play=()=>{
    if(active||reduced.matches)return;
    active=true;if(movement){movement.started=performance.now();tick();}schedule();
  };
  const pause=()=>{
    if(!active)return;
    remaining=Math.max(0,remaining-(performance.now()-started));
    if(movement)movement.elapsed=Math.min(movement.duration,movement.elapsed+performance.now()-movement.started);
    active=false;clearTimeout(timer);cancelAnimationFrame(frame);frame=0;
  };
  const settleReduced=()=>{
    if(!reduced.matches)return;
    pause();if(phaseIndex<3)phaseIndex=2;else if(phases[phaseIndex].connection==='relay')phaseIndex++;
    position=phases[phaseIndex].position;build=1;remaining=phases[phaseIndex].duration;movement=null;
    figure.dataset.netTravel='false';setConnection('p2p');syncPhase();draw();
  };
  compact.addEventListener('change',draw);reduced.addEventListener('change',settleReduced);
  window.addEventListener('resize',layoutLabels,{passive:true});
  syncPhase();draw();
  return {play,pause,destroy(){pause();compact.removeEventListener('change',draw);reduced.removeEventListener('change',settleReduced);window.removeEventListener('resize',layoutLabels);}};
}
