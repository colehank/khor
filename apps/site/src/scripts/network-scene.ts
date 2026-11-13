import {networkGeometry} from './network-geometry';

type Location = 'a'|'cellular'|'b';
const phases = [
  {id:'local',location:'a',position:0,route:'direct',duration:3200,status:'Same network, direct connection',detail:'iPhone reaches macOS over Wi-Fi A'},
  {id:'relay',location:'cellular',position:1,route:'relay',duration:4600,status:'Connected through Khor relay',detail:'A direct connection is being tried in parallel'},
  {id:'direct',location:'cellular',position:1,route:'direct',duration:3200,status:'Now connected directly',detail:'Cellular to macOS — the relay steps aside'},
  {id:'remote',location:'b',position:2,route:'direct',duration:4200,status:'Another network, the same devices',detail:'On Wi-Fi B, iPhone still reaches macOS'},
  {id:'return',location:'cellular',position:1,route:'relay',duration:3800,status:'Back on cellular',detail:'Relay traffic and direct attempts run together'},
] as const;

export function mountNetworkScene(figure:HTMLElement) {
  const svg=figure.querySelector<SVGSVGElement>('[data-net-scene]')!;
  const compact=matchMedia('(max-width: 760px)');
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const device=(id:string)=>svg.querySelector<SVGGElement>(`[data-net-device="${id}"]`)!;
  const nodes={linux:device('linux'),mac:device('mac'),windows:device('windows'),relay:device('relay'),phone:device('phone')};
  const wires=new Map([...svg.querySelectorAll<SVGPathElement>('[data-net-wire]')].map(path=>[path.dataset.netWire!,path]));
  const copies=[...svg.querySelectorAll<SVGPathElement>('[data-net-copy]')];
  const areas=new Map([...svg.querySelectorAll<SVGGElement>('[data-net-area]')].map(area=>[area.dataset.netArea!,{group:area,path:area.querySelector('path')!,label:area.querySelector('text')!}]));
  let phaseIndex=0, position=0, active=false, timer:ReturnType<typeof setTimeout>|undefined;
  let remaining:number=phases[0].duration, started=0, frame=0;
  let movement:{from:number;to:number;elapsed:number;started:number}|null=null;
  const moveDuration=950;
  const draw=()=>{
    const geometry=networkGeometry(compact.matches,position);
    svg.setAttribute('viewBox',geometry.box);
    for(const id of ['linux','mac','windows','relay','phone'] as const){
      const p=geometry.nodes[id];nodes[id].setAttribute('transform',`translate(${p.x} ${p.y})`);
    }
    for(const [id,shape] of Object.entries(geometry.areas)){
      const area=areas.get(id)!;
      area.path.setAttribute('d',shape.d);area.group.style.opacity=String(shape.opacity);
      area.group.setAttribute('aria-hidden',String(shape.opacity<.01));
      area.label.setAttribute('x',String(shape.label.x));area.label.setAttribute('y',String(shape.label.y));
    }
    const paths:Record<string,string>=geometry.routes;
    for(const [id,d] of Object.entries(paths))wires.get(id)!.setAttribute('d',d);
    copies.forEach(path=>path.setAttribute('d',paths[path.dataset.netCopy!]));
  };
  const tick=()=>{
    if(!active||!movement)return;
    const elapsed=Math.min(moveDuration,movement.elapsed+performance.now()-movement.started);
    const progress=1-Math.pow(1-elapsed/moveDuration,3);
    position=movement.from+(movement.to-movement.from)*progress;
    draw();
    if(elapsed<moveDuration)frame=requestAnimationFrame(tick);
    else{position=movement.to;movement=null;frame=0;}
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
    figure.querySelector('[data-net-status]')!.textContent=phase.status;
    figure.querySelector('[data-net-detail]')!.textContent=phase.detail;
    figure.querySelectorAll<HTMLButtonElement>('[data-net-location]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.netLocation===phase.location)));
    if(active&&!reduced.matches&&position!==phase.position){
      movement={from:position,to:phase.position,elapsed:0,started:performance.now()};tick();
    }else{position=phase.position;movement=null;draw();}
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
  const choose=(event:Event)=>{
    const location=(event.currentTarget as HTMLButtonElement).dataset.netLocation as Location;
    setPhase(location==='a'?0:location==='cellular'?1:3);
  };
  const buttons=[...figure.querySelectorAll<HTMLButtonElement>('[data-net-location]')];
  buttons.forEach(button=>button.addEventListener('click',choose));
  const settleReduced=()=>{if(reduced.matches){pause();position=phases[phaseIndex].position;movement=null;draw();}};
  compact.addEventListener('change',draw);
  reduced.addEventListener('change',settleReduced);
  draw();
  return {play,pause,destroy(){pause();compact.removeEventListener('change',draw);reduced.removeEventListener('change',settleReduced);buttons.forEach(button=>button.removeEventListener('click',choose));}};
}
