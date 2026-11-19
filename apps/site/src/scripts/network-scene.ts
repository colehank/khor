import {mountNetworkMap} from './network-map';
import {tourMotion,tourProgress} from './tour-motion';

// Mobile roams only between Wi-Fi and cellular after the icon has unfolded.
const phases=[
  {id:'connected',location:'b',position:1,build:1,connection:'p2p',duration:1800,transition:0},
  {id:'cellular-arrive',location:'c',position:0,build:1,connection:'relay',duration:tourMotion.travel+tourMotion.settle,transition:tourMotion.travel},
  {id:'cellular',location:'c',position:0,build:1,connection:'p2p',duration:tourMotion.hold,transition:0},
  {id:'wifi-arrive',location:'b',position:1,build:1,connection:'relay',duration:tourMotion.travel+tourMotion.settle,transition:tourMotion.travel},
  {id:'wifi',location:'b',position:1,build:1,connection:'p2p',duration:tourMotion.hold,transition:0},
] as const;

export function mountNetworkScene(figure:HTMLElement) {
  const map=mountNetworkMap(figure);
  let phaseIndex=0,position=1,build=1,active=false;
  let remaining:number=phases[phaseIndex].duration,started=0,frame=0,timer:ReturnType<typeof setTimeout>|undefined;
  let movement:{from:number;to:number;fromBuild:number;toBuild:number;duration:number;elapsed:number;started:number}|null=null;
  const draw=()=>map.draw(position,build);
  const setConnection=map.setConnection;
  const tick=()=>{
    if(!active||!movement)return;
    const elapsed=Math.min(movement.duration,movement.elapsed+performance.now()-movement.started);
    const progress=tourProgress(elapsed/movement.duration);
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
    started=performance.now();timer=setTimeout(()=>setPhase(phaseIndex===phases.length-1?1:phaseIndex+1),remaining);
  };
  const setPhase=(index:number)=>{
    clearTimeout(timer);cancelAnimationFrame(frame);phaseIndex=index;syncPhase();
    const phase=phases[index];remaining=phase.duration;
    figure.dataset.netTravel=String(position!==phase.position);
    if(active&&phase.transition&&(position!==phase.position||build!==phase.build)){
      movement={from:position,to:phase.position,fromBuild:build,toBuild:phase.build,duration:phase.transition,elapsed:0,started:performance.now()};tick();
    }else{position=phase.position;build=phase.build;movement=null;figure.dataset.netTravel='false';draw();}
    schedule();
  };
  const play=()=>{
    if(active)return;
    active=true;figure.classList.add('is-running');if(movement){movement.started=performance.now();tick();}schedule();
  };
  const pause=()=>{
    if(!active)return;
    remaining=Math.max(0,remaining-(performance.now()-started));
    if(movement)movement.elapsed=Math.min(movement.duration,movement.elapsed+performance.now()-movement.started);
    active=false;figure.classList.remove('is-running');clearTimeout(timer);cancelAnimationFrame(frame);frame=0;
  };
  syncPhase();draw();
  return {map,play,pause,reset(){pause();setPhase(0);},destroy(){pause();map.destroy();}};
}
