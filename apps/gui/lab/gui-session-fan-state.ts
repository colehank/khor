/** Lab-only direct selection. Coordinates are relative to the actual contact. */
export type FanPoint={x:number;y:number};
export type FanState={phase:'idle'|'pending'|'fan'|'overview';lane:'fan'|'overview'|null;held:boolean;origin:FanPoint;point:FanPoint;ids:string[];hot:number|null;armed:boolean};
export type FanResult={kind:'none'|'tap'|'overview'}|{kind:'focus';id:string};
export function recentCandidates(history:readonly string[],known:readonly string[],active:string){
  const valid=new Set(known);
  return [...new Set(history)].filter(id=>id!==active&&valid.has(id)&&!id.startsWith('new-tab:')).slice(0,3);
}
export function leftThumbFan(touch:number,changed:(state:FanState)=>void){
  const neutral=touch*.55,upThreshold=touch*1.1;
  const idle=():FanState=>({phase:'idle',lane:null,held:false,origin:{x:0,y:0},point:{x:0,y:0},ids:[],hot:null,armed:false});
  let state=idle(),travelled=false;
  const read=()=>({...state,origin:{...state.origin},point:{...state.point},ids:[...state.ids]});
  const publish=()=>changed(read());
  const update=()=>{
    const dx=state.point.x-state.origin.x,up=state.origin.y-state.point.y,distance=Math.hypot(dx,up);
    travelled ||= distance>neutral;
    state.hot=null;
    if(distance<=neutral){state.armed=false;state.phase=state.lane==='overview'?'overview':state.held?'fan':'pending';return;}
    if(!state.lane){
      // A clear vertical start owns overview, before or after a hold.
      // The gap between vertical and fan starts is deliberately unclaimed.
      if(up>=neutral&&Math.abs(dx)<=up*.25)state.lane='overview';
      else if(state.held&&dx<=-neutral*.5&&up>=-touch*.45&&up<=-dx*2.25)state.lane='fan';
    }
    if(state.lane==='overview'){
      state.phase='overview';
      state.armed=up>=(state.armed?upThreshold*.8:upThreshold)&&Math.abs(dx)<=Math.max(touch,up*.65);
      return;
    }
    state.phase=state.held?'fan':'pending';state.armed=false;
    if(state.lane!=='fan'||dx>-neutral*.5||up<-touch*.45||!state.ids.length)return;
    const angle=Math.max(0,Math.min(66,Math.atan2(up,-dx)*180/Math.PI));
    const slot=Math.min(state.ids.length-1,Math.floor(angle/66*state.ids.length));
    state.hot=slot;
  };
  let lastHot:number|null=null;
  return {
    read,
    start(origin:FanPoint,ids:readonly string[]){state={...idle(),phase:'pending',origin:{...origin},point:{...origin},ids:[...ids].slice(0,3)};travelled=false;lastHot=null;publish();},
    hold(){if(state.phase==='idle')return;state.held=true;update();lastHot=state.hot;publish();},
    move(point:FanPoint){
      if(state.phase==='idle')return;state.point={...point};update();
      if(state.hot!==null&&lastHot!==null&&state.hot!==lastHot){
        const angle=Math.atan2(state.origin.y-point.y,state.origin.x-point.x)*180/Math.PI;
        const boundary=66/state.ids.length*(state.hot>lastHot?state.hot:lastHot);
        if(Math.abs(angle-boundary)<3)state.hot=lastHot;
      }
      lastHot=state.hot;publish();
    },
    cancel(){state=idle();lastHot=null;publish();},
    release():FanResult{
      if(state.phase==='idle')return {kind:'none'};
      const before=read();let result:FanResult={kind:'none'};
      if(before.lane==='overview'&&before.armed)result={kind:'overview'};
      else if(before.held&&before.hot!==null)result={kind:'focus',id:before.ids[before.hot]};
      else if(!before.held&&!travelled&&before.lane===null)result={kind:'tap'};
      state=idle();lastHot=null;publish();return result;
    },
  };
}
