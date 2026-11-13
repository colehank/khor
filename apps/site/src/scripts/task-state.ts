import type {TaskEndpoint,TaskPacket,TaskPhase} from '../components/concepts/capability-data';
import {taskTiming} from '../components/concepts/capability-data';
const clamp=(value:number)=>Math.max(0,Math.min(1,value));

export function taskTransport(phase:TaskPhase,elapsed:number) {
  if(!phase.flow)return {arrival:0,delivered:false,position:0,opacity:0};
  const {departure,transfer,fade}=taskTiming,arrival=Math.min(departure+transfer,phase.duration-fade);
  const t=clamp((elapsed-departure)/(arrival-departure));
  return {arrival,delivered:elapsed>=arrival,position:t*t*(3-2*t),
    opacity:Math.min(clamp((elapsed-departure)/fade),clamp((arrival+fade-elapsed)/fade))};
}

export function taskState(phases:TaskPhase[],index:number,elapsed:number) {
  const state={accepted:phases[0].id!=='delegate',fileReady:false,fileNotified:false,serverRequested:false,fileOnServer:false,
    webReady:false,webNotified:false,webRequested:false,result:false,
    sessions:{mac:{started:false,progress:0},linux:{started:false,progress:0}},
    receipts:{} as Partial<Record<TaskEndpoint,TaskPacket>>};
  // A request starts a session on its host. Sessions never travel; neither the
  // laptop nor the phone becomes an execution host. File and Web keep theirs.
  for(let i=0;i<=index;i++){
    const phase=phases[i],time=i<index?phase.duration:Math.max(0,Math.min(elapsed,phase.duration));
    const transport=taskTransport(phase,time);
    if(transport.delivered&&phase.flow){
      let receipt:TaskPacket|undefined;
      switch(phase.arrival){
        case 'accept':state.accepted=true;break;
        case 'claude':if(state.accepted)state.sessions.mac.started=true;break;
        case 'file-notice':if(state.fileReady){state.fileNotified=true;receipt='check';}break;
        case 'server-request':if(state.fileNotified)state.serverRequested=true;break;
        case 'file':if(state.serverRequested&&state.fileReady){state.fileOnServer=true;state.sessions.linux.started=true;receipt='files';}break;
        case 'web-notice':if(state.webReady){state.webNotified=true;receipt='link';}break;
        case 'web-request':if(state.webNotified)state.webRequested=true;break;
        case 'page':if(state.webRequested&&state.webReady){state.result=true;receipt='web';}break;
        case 'deliver':if(state.webNotified){state.result=true;receipt='link';}break;
      }
      if(receipt)state.receipts[phase.flow.to]=receipt;
    }
    if(phase.session&&state.sessions[phase.session].started){
      const session=state.sessions[phase.session];
      session.progress=clamp((time-transport.arrival)/(phase.duration-taskTiming.outputHold-transport.arrival));
      if(session.progress===1){
        if(phase.session==='mac'){state.fileReady=true;state.receipts.mac='files';}
        else{state.webReady=true;state.receipts.linux='web';}
      }
    }
  }
  return {...state,complete:!!phases[index].complete&&state.result};
}
