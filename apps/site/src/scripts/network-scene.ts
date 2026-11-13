import {mountNetworkMap} from './network-map';

// The introduction runs once. Mobile then roams only between Wi-Fi and cellular.
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
  const map=mountNetworkMap(figure),reduced=matchMedia('(prefers-reduced-motion: reduce)');
  let phaseIndex=reduced.matches?2:0,position=1,build=reduced.matches?1:0,active=false;
  let remaining:number=phases[phaseIndex].duration,started=0,frame=0,timer:ReturnType<typeof setTimeout>|undefined;
  let movement:{from:number;to:number;fromBuild:number;toBuild:number;duration:number;elapsed:number;started:number}|null=null;
  const draw=()=>map.draw(position,build);
  const setConnection=map.setConnection;
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
  reduced.addEventListener('change',settleReduced);
  syncPhase();draw();
  return {play,pause,destroy(){pause();map.destroy();reduced.removeEventListener('change',settleReduced);}};
}
