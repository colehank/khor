import {agentPhases,workPhases,taskTiming} from '../components/concepts/capability-data';
import {capabilityGeometry} from './capability-geometry';
import {mountNetworkMap} from './network-map';
import {taskState,taskTransport} from './task-state';
import type {TaskDevice,TaskEndpoint,TaskSession} from '../components/concepts/capability-data';

export function mountCapabilityScene(figure:HTMLElement) {
  const isAgent=figure.dataset.concept==='agent',phases=isAgent?agentPhases:workPhases;
  const map=mountNetworkMap(figure),compact=matchMedia('(max-width: 760px)'),reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const find=<T extends Element=SVGElement>(selector:string)=>figure.querySelector<T>(selector)!;
  const path=find<SVGPathElement>('[data-task-route]'),packet=find('[data-task-packet]');
  const sessions=[...figure.querySelectorAll<SVGElement>('[data-task-session]')].map(element=>({
    element,device:element.dataset.taskSession as TaskSession,
    progress:element.querySelector('[data-task-progress]')!,cut:find(`[data-task-session-cut="${element.dataset.taskSession}"]`),
  }));
  const receipts=[...figure.querySelectorAll<SVGElement>('[data-task-receipt]')];
  const user=find('[data-task-user]'),userCut=find('[data-task-user-cut]'),caption=find<HTMLElement>('[data-task-caption]');
  let index=reduced.matches?phases.length-1:0,active=false,remaining=phases[index].duration,started=0,frame=0,length=0;
  let geometry=capabilityGeometry(compact.matches),timer:ReturnType<typeof setTimeout>|undefined,lastReceipts='';
  const translate=(element:Element,x:number,y:number)=>element.setAttribute('transform',`translate(${x} ${y})`);
  const elapsedNow=()=>Math.min(phases[index].duration,phases[index].duration-remaining+(active?performance.now()-started:0));
  const showState=(elapsed:number)=>{
    const state=taskState(phases,index,elapsed);
    const attributes={taskReady:String(state.webReady),taskAccepted:String(state.accepted),taskComplete:String(state.complete),
      taskFile:String(state.fileReady),taskFileOnServer:String(state.fileOnServer),taskResult:String(state.result)};
    for(const [key,value] of Object.entries(attributes))if(figure.dataset[key]!==value)figure.dataset[key]=value;
    for(const {element,device,progress,cut} of sessions){
      const session=state.sessions[device],status=!session.started?'idle':session.progress===1?'done':'running';
      if(element.dataset.sessionState!==status)element.dataset.sessionState=status;
      cut.style.opacity=session.started?'1':'0';
      progress.setAttribute('stroke-dashoffset',String(1-session.progress));
    }
    const receiptState=JSON.stringify(state.receipts);
    if(receiptState!==lastReceipts){
      lastReceipts=receiptState;
      for(const badge of receipts){
        const payload=state.receipts[badge.dataset.taskReceipt as TaskEndpoint];
        badge.dataset.hasReceipt=String(!!payload);
        for(const mark of badge.querySelectorAll<SVGElement>('[data-receipt-payload]'))mark.dataset.visible=String(mark.dataset.receiptPayload===payload);
        const label=badge.querySelector('[data-task-resource-label]');
        if(label)label.textContent=payload==='web'?'Web':'File';
      }
    }
  };
  const showUser=(elapsed:number)=>{
    const phase=phases[index],{nodes}=geometry,p=nodes[phase.user==='outside'?'you':phase.user];
    let x=p.x,y=p.y+(isAgent?0:70);
    if(phase.id==='handoff'&&!reduced.matches){
      const t=Math.max(0,Math.min(1,(elapsed-taskTiming.departure)/taskTiming.handoff)),u=t*t*(3-2*t),from=nodes.windows,to=nodes.phone;
      // Move the access marker around the phone, not through its device glyph.
      x=(1-u)*(1-u)*from.x+2*(1-u)*u*(Math.min(from.x,to.x)-100)+u*u*to.x;
      y=from.y+70+(to.y-from.y)*u;
    }
    translate(user,x,y);userCut.setAttribute('x',String(x-36));userCut.setAttribute('y',String(y+(isAgent?-16:-21)));
  };
  const draw=(elapsed:number)=>{
    showState(elapsed);showUser(elapsed);
    const transport=taskTransport(phases[index],elapsed);
    if(length){const position=path.getPointAtLength(length*transport.position);translate(packet,position.x,position.y);}
    packet.setAttribute('opacity',String(reduced.matches?0:transport.opacity));
  };
  const layout=(elapsed:number)=>{
    const phase=phases[index];geometry=capabilityGeometry(compact.matches,phase.flow);
    const {nodes}=geometry;
    for(const {element,device,cut} of sessions){
      translate(element.parentElement!,nodes[device].x,nodes[device].y);
      cut.setAttribute('x',String(nodes[device].x-70));cut.setAttribute('y',String(nodes[device].y+49));
    }
    for(const badge of receipts){const device=badge.dataset.taskReceipt as TaskEndpoint;translate(badge.parentElement!,nodes[device].x,nodes[device].y);}
    if(isAgent){
      find('[data-task-user-link]').setAttribute('d',geometry.userPath);
      const ring=find('[data-task-router-ring]');ring.setAttribute('cx',String(nodes.khor.x));ring.setAttribute('cy',String(nodes.khor.y));
    }
    path.setAttribute('d',geometry.path||'M0 0');length=geometry.path?path.getTotalLength():0;draw(elapsed);
  };
  const render=()=>{
    const phase=phases[index],focus=reduced.matches?[isAgent?'you':'phone']:phase.flow?[phase.flow.from,phase.flow.to]:['phone'];
    Object.assign(figure.dataset,{taskStage:phase.id,taskRoute:String(!!phase.flow&&!reduced.matches),taskUser:phase.user,taskRouting:String(focus.includes('khor'))});
    if(caption.textContent!==phase.caption)caption.textContent=phase.caption;
    for(const node of figure.querySelectorAll<SVGElement>('[data-net-device]'))node.dataset.taskFocus=String(focus.includes(node.dataset.netDevice as TaskDevice));
    for(const mark of figure.querySelectorAll<SVGElement>('[data-task-payload]'))mark.dataset.visible=String(mark.dataset.taskPayload===phase.packet);
    layout(reduced.matches?phase.duration:phase.duration-remaining);
  };
  const tick=()=>{
    frame=0;if(!active)return;
    const phase=phases[index],elapsed=elapsedNow();draw(elapsed);
    const movingUntil=Math.max(taskTransport(phase,0).arrival+taskTiming.fade,phase.session?phase.duration-taskTiming.outputHold:0,phase.id==='handoff'?taskTiming.departure+taskTiming.handoff:0);
    if(elapsed<movingUntil)frame=requestAnimationFrame(tick);
  };
  const schedule=()=>{
    started=performance.now();
    timer=setTimeout(()=>{cancelAnimationFrame(frame);index=(index+1)%phases.length;remaining=phases[index].duration;render();if(active){schedule();tick();}},remaining);
  };
  const play=()=>{if(active||reduced.matches)return;active=true;schedule();tick();};
  const pause=()=>{if(!active)return;remaining=Math.max(0,remaining-(performance.now()-started));active=false;clearTimeout(timer);cancelAnimationFrame(frame);frame=0;};
  const preferencesChanged=()=>{pause();index=reduced.matches?phases.length-1:0;remaining=phases[index].duration;render();};
  const resize=()=>layout(reduced.matches?phases[index].duration:elapsedNow());
  compact.addEventListener('change',resize);reduced.addEventListener('change',preferencesChanged);render();
  return {play,pause,destroy(){pause();map.destroy();compact.removeEventListener('change',resize);reduced.removeEventListener('change',preferencesChanged);}};
}
