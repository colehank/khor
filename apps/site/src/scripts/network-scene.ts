type Location = 'a'|'cellular'|'b';
type Point = {x:number;y:number;scale:number};
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
  const nodeRadius=nodes.mac.querySelector<SVGCircleElement>('.net-device-body')!.r.baseVal.value;
  const wires=new Map([...svg.querySelectorAll<SVGPathElement>('[data-net-wire]')].map(path=>[path.dataset.netWire!,path]));
  const copies=[...svg.querySelectorAll<SVGPathElement>('[data-net-copy]')];
  let phaseIndex=0, position=0, active=false, timer:ReturnType<typeof setTimeout>|undefined;
  let remaining:number=phases[0].duration, started=0, frame=0;
  let movement:{from:number;to:number;elapsed:number;started:number}|null=null;
  const moveDuration=950;
  const layout=()=>compact.matches?{
    box:'0 0 500 600',a:[8,155,222,420],b:[322,155,170,420],
    linux:{x:66,y:265,scale:1},mac:{x:169,y:265,scale:1},windows:{x:407,y:265,scale:1},relay:{x:275,y:65,scale:1},phoneY:475,stops:[169,275,407],
  }:{
    box:'0 0 1000 500',a:[20,140,400,330],b:[650,140,330,330],
    linux:{x:120,y:240,scale:1},mac:{x:310,y:240,scale:1},windows:{x:820,y:240,scale:1},relay:{x:530,y:55,scale:1},phoneY:380,stops:[310,530,820],
  };
  const transform=(node:SVGGElement,p:Point)=>{
    node.setAttribute('transform',`translate(${p.x} ${p.y}) scale(${p.scale})`);
    node.style.setProperty('--net-device-scale',String(p.scale));
  };
  const draw=()=>{
    const l=layout();
    svg.setAttribute('viewBox',l.box);
    for(const id of ['a','b'] as const){
      const area=svg.querySelector<SVGRectElement>(`[data-net-area="${id}"]`)!;
      ['x','y','width','height'].forEach((attr,i)=>area.setAttribute(attr,String(l[id][i])));
      const label=svg.querySelector<SVGTextElement>(`[data-net-area-label="${id}"]`)!;
      label.setAttribute('x',String(l[id][0]+(compact.matches?20:28)));label.setAttribute('y',String(l[id][1]+37));
    }
    for(const id of ['linux','mac','windows','relay'] as const)transform(nodes[id],l[id]);
    const segment=Math.min(1,Math.floor(position));
    const x=l.stops[segment]+(l.stops[segment+1]-l.stops[segment])*(position-segment);
    transform(nodes.phone,{x,y:l.phoneY,scale:1});
    const start=l.mac.x+nodeRadius, y=l.mac.y, top=l.phoneY-nodeRadius;
    const approach=Math.min(1,position);
    const directEnd=x+nodeRadius*(1-2*approach);
    const outer=Math.min(l.b[0]+l.b[2]-12,l.windows.x+112*l.windows.scale);
    const paths:Record<string,string>={
      lan:`M${l.linux.x+nodeRadius} ${y} H${l.mac.x-nodeRadius}`,
      backbone:`M${start} ${y} H${l.windows.x-nodeRadius}`,
      nearby:`M${l.windows.x+nodeRadius} ${y} C${outer} ${y} ${outer} ${l.phoneY} ${x+nodeRadius} ${l.phoneY}`,
      direct:`M${start} ${y} C${start+(compact.matches?6:30)} ${y} ${directEnd-50*approach} ${l.phoneY} ${directEnd} ${l.phoneY}`,
      'relay-in':`M${start} ${y} C${start+64} ${y} ${l.relay.x-96} ${l.relay.y} ${l.relay.x-25} ${l.relay.y}`,
      'relay-out':`M${l.relay.x} ${l.relay.y+25} C${l.relay.x} ${l.relay.y+130} ${x} ${top-46} ${x} ${top}`,
    };
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
