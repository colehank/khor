/** One contact owns the gesture: up opens tabs; a held offset selects a repeat gear. */
import {DECK_JOYSTICK_HOLD_MS,DECK_JOYSTICK_REPEAT_MS} from './tempo';
export type DeckJoystickTuning={holdMs:number;neutral:number;reach:number;slowMs:number;mediumMs:number;fastMs:number};
export const DECK_JOYSTICK_DEFAULTS:Readonly<DeckJoystickTuning>=Object.freeze({holdMs:DECK_JOYSTICK_HOLD_MS,neutral:10,reach:56,slowMs:DECK_JOYSTICK_REPEAT_MS[0],mediumMs:DECK_JOYSTICK_REPEAT_MS[1],fastMs:DECK_JOYSTICK_REPEAT_MS[2]});
export type DeckJoystickState={phase:'idle'|'pending'|'swipe'|'pull'|'steering';preview:number;dx:number;dy:number;direction:number;interval:number;overview:boolean;tier:number;pulse:number};
export type DeckJoystickClock={now:()=>number;request:(f:()=>void)=>number;cancel:(id:number)=>void};
export function deckJoystick(count:number,initial:number,changed:(s:DeckJoystickState)=>void,commit:(index:number)=>void,overview:()=>void,
  clock:DeckJoystickClock={now:()=>performance.now(),request:f=>requestAnimationFrame(f),cancel:id=>cancelAnimationFrame(id)},getTuning=()=>DECK_JOYSTICK_DEFAULTS){
  const wrap=(i:number)=>count?((i%count)+count)%count:0;
  const idle=():DeckJoystickState=>({phase:'idle',preview:wrap(initial),dx:0,dy:0,direction:0,interval:DECK_JOYSTICK_DEFAULTS.slowMs,overview:false,tier:0,pulse:0});
  let s=idle(),tuning={...getTuning()},origin={x:0,y:0},began=0,last=0,progress=0,frame=0;
  const publish=()=>changed({...s});
  const step=(direction:number)=>{if(count>1){s.preview=wrap(s.preview+direction);s.pulse++;}};
  const cancel=()=>{clock.cancel(frame);frame=0;progress=0;s=idle();publish();};
  const tick=()=>{
    frame=0;const now=clock.now(),elapsed=Math.min(64,now-last);last=now;
    if(s.phase==='pending'&&now-began>=tuning.holdMs){s.phase='steering';publish();}
    if(s.phase==='steering'&&s.direction&&!s.overview&&count>1){
      progress+=elapsed/s.interval;
      if(progress>=1){step(s.direction);progress%=1;publish();}
    }
    if(s.phase!=='idle')frame=clock.request(tick);
  };
  return {
    read:()=>({...s}),cancel,restartReading:()=>{progress=0;last=clock.now();},dispose:()=>{clock.cancel(frame);frame=0;},
    start(x:number,y:number){cancel();tuning={...getTuning()};origin={x,y};began=last=clock.now();s.phase='pending';publish();frame=clock.request(tick);},
    move(x:number,y:number){
      if(s.phase==='idle')return;
      const dx=x-origin.x,dy=y-origin.y;s.dx=dx;s.dy=dy;
      if(s.phase==='pending'){
        if(Math.hypot(dx,dy)<8)return;
        if(Math.abs(dy)>=Math.abs(dx)){
          if(dy>=0){cancel();return;}
          s.phase='pull';
        }else s.phase='swipe';
      }
      if(s.phase==='swipe'){
        // The shared band hands this contact to the existing direct-swipe
        // model. This recognizer never commits a second swipe selection.
        publish();return;
      }
      // Up is a visible target just above the pill. Hysteresis lets the
      // finger return to horizontal control without flickering at its edge.
      s.overview=dy<-(s.overview?32:48);
      if(s.phase==='pull'){publish();return;}
      const direction=s.overview||count<2||Math.abs(dx)<=tuning.neutral?0:Math.sign(dx);
      const gain=Math.min(1,Math.max(0,(Math.abs(dx)-tuning.neutral)/(tuning.reach-tuning.neutral)));
      // Stable gears make each chevron count predict one repeat rhythm.
      // A small overlap keeps finger tremor from changing gears repeatedly.
      let tier=direction?Math.min(3,1+Math.floor(gain*3)):0;
      if(direction&&direction===s.direction){
        tier=s.tier;
        while(tier<3&&gain>=tier/3+.05)tier++;
        while(tier>1&&gain<(tier-1)/3-.05)tier--;
      }
      s.tier=tier;
      s.interval=[tuning.slowMs,tuning.slowMs,tuning.mediumMs,tuning.fastMs][tier];
      if(direction!==s.direction){
        progress=0;last=clock.now();
        if(direction)step(direction); // First candidate responds immediately.
      }
      s.direction=direction;publish();
    },
    release(){
      const before={...s};cancel();
      if(before.phase==='pending')return 'tap';
      if((before.phase==='steering'||before.phase==='pull')&&before.overview){overview();return 'overview';}
      if(before.phase!=='idle'&&before.preview!==wrap(initial)){commit(before.preview);return 'commit';}
      return 'none';
    },
  };
}
