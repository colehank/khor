import type {SessionRow} from '../../src/gen/bindings/SessionRow';
import type {ActivityEvent} from '../../src/gen/bindings/ActivityEvent';

/** Data-only driver: all presentation, suppression and gestures remain production code. */
export function noticeFixture(options:{normal:boolean;burst:boolean}, environment:{now:()=>number;active:()=>string|null;enabled:(row:SessionRow)=>boolean}) {
  let readyAt:number|null=null,round=-1,serial=0,manual=false;
  let rows:SessionRow[]=[];
  let pending:{burst:boolean;skip:boolean}|null=null;
  const events=new Map<string,ActivityEvent>();
  const key=(row:SessionRow)=>JSON.stringify([row.execution_home,row.id]);
  const emit=(burst:boolean)=>{
    const eligible=rows.filter(row=>row.id!==environment.active()&&environment.enabled(row)&&row.execution_home);
    const targets=burst?eligible:eligible.slice(0,1);
    for(const row of targets)events.set(key(row),{id:`lab-notice:${++serial}`,at_ms:environment.now(),
      kind:burst?'turn_completed':(['needs_input','turn_completed','failed'] as const)[(serial-1)%3],resolved:false,text:null,duration_ms:null,exit:null});
    return targets.map(row=>row.title);
  };
  return {
    get ready(){return readyAt!==null&&rows.length>0;},
    trigger(burst:boolean){
      manual=true;
      if(pending)pending.burst=burst;else pending={burst,skip:true};
      const eligible=rows.filter(row=>row.id!==environment.active()&&environment.enabled(row)&&row.execution_home);
      return (burst?eligible:eligible.slice(0,1)).map(row=>row.title);
    },
    snapshot(next:SessionRow[]):SessionRow[]{
      rows=next;
      const now=environment.now();
      // The first real host snapshot is always quiet, even after a slow page import.
      if(readyAt===null){readyAt=now;return next;}
      // Allow the App to consume a fresh resume baseline before the requested event.
      if(pending){if(pending.skip)pending.skip=false;else{emit(pending.burst);pending=null;}}
      if(!manual&&(options.normal||options.burst)&&now-readyAt>=2000){
        const current=Math.floor((now-readyAt-2000)/12000);
        if(current>round){round=current;emit(options.burst);}
      }
      return next.map(row=>({...row,activity:events.has(key(row))?[events.get(key(row))!]:row.activity}));
    },
  };
}
