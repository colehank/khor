import {networkGeometry} from './network-geometry';

const phases = [
  {id:'local',location:'a',position:0,route:'direct',duration:3600},
  {id:'relay',location:'cellular',position:1,route:'relay',duration:4600},
  {id:'direct',location:'cellular',position:1,route:'direct',duration:3600},
  {id:'remote',location:'b',position:2,route:'direct',duration:4600},
  {id:'return',location:'cellular',position:1,route:'relay',duration:4000},
] as const;

export function mountNetworkScene(figure:HTMLElement) {
  const svg=figure.querySelector<SVGSVGElement>('[data-net-scene]')!;
  const compact=matchMedia('(max-width: 760px)');
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const device=(id:string)=>svg.querySelector<SVGGElement>(`[data-net-device="${id}"]`)!;
  const nodes={linux:device('linux'),mac:device('mac'),windows:device('windows'),relay:device('relay'),phone:device('phone')};
  const connections=new Map([...svg.querySelectorAll<SVGPathElement>('[data-net-connection]')].map(path=>[path.dataset.netConnection!,path]));
  const areas=new Map([...svg.querySelectorAll<SVGGElement>('[data-net-area]')].map(area=>[area.dataset.netArea!,{group:area,path:area.querySelector('path')!,label:area.querySelector('text')!}]));
  let phaseIndex=0, position=0, relayAmount=0, active=false, timer:ReturnType<typeof setTimeout>|undefined;
  let remaining:number=phases[0].duration, started=0, frame=0;
  let movement:{from:number;to:number;fromRelay:number;toRelay:number;elapsed:number;started:number}|null=null;
  const moveDuration=1350;
  const draw=()=>{
    const geometry=networkGeometry(compact.matches,position,relayAmount);
    svg.setAttribute('viewBox',geometry.box);
    for(const id of ['linux','mac','windows','relay','phone'] as const){
      const p=geometry.nodes[id];nodes[id].setAttribute('transform',`translate(${p.x} ${p.y})`);
    }
    for(const [id,shape] of Object.entries(geometry.areas)){
      const area=areas.get(id)!;
      area.path.setAttribute('d',shape.d);area.group.style.opacity=String(shape.opacity);
      area.group.style.setProperty('--net-local',String(shape.strength));
      area.group.setAttribute('aria-hidden',String(shape.opacity<.01));
      area.label.setAttribute('x',String(shape.label.x));area.label.setAttribute('y',String(shape.label.y));
      area.label.setAttribute('text-anchor',shape.label.anchor);
    }
    for(const [id,shape] of Object.entries(geometry.connections)){
      const path=connections.get(id)!;path.setAttribute('d',shape.d);path.style.opacity=String(shape.opacity);
    }
  };
  const tick=()=>{
    if(!active||!movement)return;
    const elapsed=Math.min(moveDuration,movement.elapsed+performance.now()-movement.started);
    const t=elapsed/moveDuration,progress=t*t*(3-2*t);
    position=movement.from+(movement.to-movement.from)*progress;
    relayAmount=movement.fromRelay+(movement.toRelay-movement.fromRelay)*progress;
    draw();
    if(elapsed<moveDuration)frame=requestAnimationFrame(tick);
    else{position=movement.to;relayAmount=movement.toRelay;movement=null;frame=0;}
  };
  const schedule=()=>{
    if(!active)return;
    started=performance.now();
    timer=setTimeout(()=>setPhase((phaseIndex+1)%phases.length),remaining);
  };
  const setPhase=(index:number)=>{
    clearTimeout(timer);cancelAnimationFrame(frame);
    phaseIndex=index;
    const phase=phases[index];
    remaining=phase.duration;
    figure.dataset.netPhase=phase.id;figure.dataset.route=phase.route;figure.dataset.location=phase.location;
    const nextRelay=phase.route==='relay'?1:0;
    if(active&&!reduced.matches&&(position!==phase.position||relayAmount!==nextRelay)){
      movement={from:position,to:phase.position,fromRelay:relayAmount,toRelay:nextRelay,elapsed:0,started:performance.now()};tick();
    }else{position=phase.position;relayAmount=nextRelay;movement=null;draw();}
    schedule();
  };
  const play=()=>{
    if(active||reduced.matches)return;
    active=true;
    if(movement){movement.started=performance.now();tick();}
    schedule();
  };
  const pause=()=>{
    if(!active)return;
    remaining=Math.max(0,remaining-(performance.now()-started));
    if(movement)movement.elapsed=Math.min(moveDuration,movement.elapsed+performance.now()-movement.started);
    active=false;clearTimeout(timer);cancelAnimationFrame(frame);frame=0;
  };
  const settleReduced=()=>{
    if(reduced.matches){pause();position=phases[phaseIndex].position;relayAmount=phases[phaseIndex].route==='relay'?1:0;movement=null;draw();}
  };
  compact.addEventListener('change',draw);
  reduced.addEventListener('change',settleReduced);
  draw();
  return {play,pause,destroy(){pause();compact.removeEventListener('change',draw);reduced.removeEventListener('change',settleReduced);}};
}
