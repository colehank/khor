import type {TaskEndpoint,TaskMark,TaskPhase} from '../components/concepts/capability-data';
const clamp=(value:number)=>Math.max(0,Math.min(1,value));

// Transport stays legible at the shorter cadence. The remaining time shows the
// destination working; final delivery includes a longer hold for the result.
export function taskTransport(phase:TaskPhase,elapsed:number) {
  const departure=80,arrival=Math.min(900,phase.duration*.52);
  const t=clamp((elapsed-departure)/(arrival-departure));
  return {arrival,delivered:elapsed>=arrival,position:t*t*(3-2*t),
    opacity:Math.min(clamp((elapsed-departure)/80),clamp((arrival+120-elapsed)/120))};
}

export function taskState(phases:TaskPhase[],index:number,elapsed:number) {
  const delegated=phases[0].id==='request';
  const state={accepted:!delegated,prepared:!delegated,reviewing:false,revised:false,preview:false,collected:false,result:false,
    run:0,progress:0,receipts:{} as Partial<Record<TaskEndpoint,TaskMark>>};
  if(!delegated)state.receipts.mac='agent';
  // Keep delivery dependencies even when several implementation steps are
  // condensed into one visual beat. In-flight work never activates its target.
  for(let i=0;i<=index;i++){
    const phase=phases[i],time=i<index?phase.duration:Math.max(0,Math.min(elapsed,phase.duration));
    const transport=taskTransport(phase,time);
    if(transport.delivered){
      let receipt:TaskMark|undefined;
      switch(phase.arrival){
        case 'request':state.accepted=true;break;
        case 'plan':if(state.accepted){state.prepared=true;receipt='agent';}break;
        case 'train':if(state.prepared){state.run=1;state.progress=0;receipt='terminal';}break;
        case 'refine':if(state.run===1&&state.progress===1){state.reviewing=true;receipt=delegated?'agent':'edit';}break;
        case 'update':if(state.revised){state.run=2;state.progress=0;receipt='terminal';}break;
        case 'preview':if(state.run===2&&state.progress===1){state.preview=true;receipt='web';}break;
        case 'collect':if(state.preview){state.collected=true;receipt='files';}break;
        case 'result':if(state.collected){state.result=true;receipt='web';}break;
      }
      if(receipt)state.receipts[phase.flow.to]=receipt;
    }
    if(phase.arrival==='refine'&&state.reviewing&&time===phase.duration){state.revised=true;state.receipts.windows='files';}
    if(phase.run===state.run)state.progress=clamp((time-transport.arrival)/(phase.duration-transport.arrival));
  }
  return {...state,complete:!!phases[index].complete&&(delegated?state.result:state.preview)};
}
