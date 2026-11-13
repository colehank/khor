import {agentPhases,workPhases} from '../components/concepts/capability-data';
import {capabilityGeometry} from './capability-geometry';
import {mountNetworkMap} from './network-map';

export function mountCapabilityScene(figure:HTMLElement) {
  const isAgent=figure.dataset.concept==='agent',phases=isAgent?agentPhases:workPhases;
  const map=mountNetworkMap(figure),compact=matchMedia('(max-width: 760px)'),reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const find=<T extends Element=SVGElement>(selector:string)=>figure.querySelector<T>(selector)!;
  const path=find<SVGPathElement>('[data-task-route]'),packet=find('[data-task-packet]');
  const progress=find('[data-task-progress]'),server=find('[data-task-server]'),runtime=find('[data-task-runtime]');
  const user=find('[data-task-user]'),userCut=find('[data-task-user-cut]'),runtimeCut=find('[data-task-runtime-cut]');
  const caption=find<HTMLElement>('[data-task-caption]'),detail=find<HTMLElement>('[data-task-detail]');
  let index=reduced.matches?phases.length-1:0,active=false,remaining=phases[index].duration,started=0,frame=0,length=0;
  let timer:ReturnType<typeof setTimeout>|undefined;
  const translate=(element:Element,x:number,y:number)=>element.setAttribute('transform',`translate(${x} ${y})`);
  const showProgress=(value:number)=>{
    server.style.opacity=value>0?'1':'0';runtimeCut.style.opacity=value>0?'1':'0';
    progress.setAttribute('stroke-dashoffset',String(1-value));
    figure.dataset.taskReady=String(value>=.999);
    const label=value>=.999?'8 passed':value<.5?'Building':'Testing';
    if(runtime.textContent!==label)runtime.textContent=label;
  };
  const layout=()=>{
    const phase=phases[index],g=capabilityGeometry(compact.matches,phase.route),p=g.nodes[phase.user];
    translate(server,g.nodes.linux.x,g.nodes.linux.y);translate(find('[data-task-workstation]'),g.nodes.mac.x,g.nodes.mac.y);translate(find('[data-task-mobile]'),g.nodes.phone.x,g.nodes.phone.y);
    translate(user,p.x,p.y+70);userCut.setAttribute('x',String(p.x-36));userCut.setAttribute('y',String(p.y+49));
    runtimeCut.setAttribute('x',String(g.nodes.linux.x-70));runtimeCut.setAttribute('y',String(g.nodes.linux.y+49));
    path.setAttribute('d',g.path||'M0 0');length=g.path?path.getTotalLength():0;
    packet.setAttribute('opacity','0');
  };
  const render=()=>{
    const phase=phases[index];
    Object.assign(figure.dataset,{taskStage:phase.id,taskComplete:String(!!phase.complete),taskCodex:String(!!phase.codex),taskRoute:String(!!phase.route),taskUser:phase.user,taskPreview:String(phase.complete||(!isAgent&&['preview','report'].includes(phase.id))||(isAgent&&phase.id==='deliver'))});
    caption.textContent=phase.caption;detail.textContent=phase.detail;
    for(const node of figure.querySelectorAll<SVGElement>('[data-net-device]'))node.dataset.taskFocus=String(phase.focus.includes(node.dataset.netDevice as typeof phase.user));
    for(const mark of figure.querySelectorAll<SVGElement>('[data-task-payload]'))mark.dataset.visible=String(mark.dataset.taskPayload===phase.packet);
    layout();showProgress(phase.progress[0]);
  };
  const tick=()=>{
    frame=0;if(!active)return;
    const phase=phases[index],elapsed=Math.min(phase.duration,phase.duration-remaining+performance.now()-started);
    const t=elapsed/phase.duration;
    showProgress(phase.progress[0]+(phase.progress[1]-phase.progress[0])*t);
    if(phase.packet&&length){
      // One bounded, lightweight moving symbol. Only its transform changes.
      const travel=Math.max(0,Math.min(1,(elapsed-350)/(phase.duration-900)));
      const position=path.getPointAtLength(length*(travel*travel*(3-2*travel)));
      translate(packet,position.x,position.y);
      packet.setAttribute('opacity',String(Math.min(1,Math.max(0,elapsed/250),Math.max(0,(phase.duration-elapsed)/300))));
    }
    if(phase.packet||phase.progress[0]!==phase.progress[1])frame=requestAnimationFrame(tick);
  };
  const schedule=()=>{
    started=performance.now();
    timer=setTimeout(()=>{cancelAnimationFrame(frame);index=(index+1)%phases.length;remaining=phases[index].duration;render();if(active){schedule();tick();}},remaining);
  };
  const play=()=>{if(active||reduced.matches)return;active=true;schedule();tick();};
  const pause=()=>{if(!active)return;remaining=Math.max(0,remaining-(performance.now()-started));active=false;clearTimeout(timer);cancelAnimationFrame(frame);frame=0;};
  const settle=()=>{if(!reduced.matches)return;pause();index=phases.length-1;remaining=phases[index].duration;render();};
  compact.addEventListener('change',layout);reduced.addEventListener('change',settle);render();
  return {play,pause,destroy(){pause();map.destroy();compact.removeEventListener('change',layout);reduced.removeEventListener('change',settle);}};
}
