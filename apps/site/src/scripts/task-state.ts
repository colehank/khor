import type {TaskDevice,TaskPacket,TaskPhase} from '../components/concepts/capability-data';

const clamp=(value:number)=>Math.max(0,Math.min(1,value));

// The packet and its destination share this clock. Arrival leaves time to read
// the new state before the next action, and is stable across pause/resume.
export function taskTransport(phase:TaskPhase,elapsed:number) {
  const departure=200,arrival=phase.duration*.62;
  const t=clamp((elapsed-departure)/(arrival-departure));
  return {arrival,delivered:!!phase.packet&&elapsed>=arrival,position:t*t*(3-2*t),
    opacity:phase.packet?Math.min(clamp((elapsed-departure)/120),clamp((arrival+160-elapsed)/160)):0};
}

export function taskState(phases:TaskPhase[],index:number,elapsed:number) {
  const delegated=phases[0].id==='request';
  const state={accepted:!delegated,prepared:false,project:false,started:false,preview:false,report:false,result:false,
    progress:0,receipts:{} as Partial<Record<TaskDevice,TaskPacket>>};
  // Replay a handful of completed actions rather than resetting all badges on
  // every stage. A destination can only acquire something that has arrived.
  for(let i=0;i<=index;i++){
    const phase=phases[i],time=i<index?phase.duration:Math.max(0,Math.min(elapsed,phase.duration));
    const transport=taskTransport(phase,time),arrival=phase.arrival;
    if(phase.codex&&state.accepted&&time>=phase.duration)state.prepared=true;
    if(arrival&&transport.delivered){
      let receipt:TaskPacket|undefined;
      switch(arrival.effect){
        case 'request':state.accepted=true;break;
        case 'project':if(state.prepared){state.project=true;receipt='files';}break;
        case 'start':if(state.project){state.started=true;receipt='terminal';}break;
        case 'terminal':if(state.started)receipt='terminal';break;
        case 'preview':if(state.progress===1){state.preview=true;receipt='web';}break;
        case 'report':if(state.preview){state.report=true;receipt='files';}break;
        case 'result':if(state.report){state.result=true;receipt='check';}break;
      }
      if(receipt)state.receipts[arrival.device]=receipt;
    }
    if(state.started){
      const start=arrival?.effect==='start'?transport.arrival:0;
      const t=clamp((time-start)/(phase.duration-start));
      state.progress=phase.progress[0]+(phase.progress[1]-phase.progress[0])*t;
    }
  }
  return {...state,complete:!!phases[index].complete&&state.report&&(!delegated||state.result)};
}
