// One cadence for the network's roaming and the fixed capability diagrams.
const travel=1700,settle=1300,hold=2000;
export const tourMotion={travel,settle,hold,turn:travel+settle+hold,cycle:(travel+settle+hold)*2} as const;
// With x control points at 1/3 and 2/3, this is the same smoothstep as below.
export const tourEasing='cubic-bezier(.333333,0,.666667,1)';
export const tourProgress=(t:number)=>t*t*(3-2*t);

/** Scroll chooses the destination. Once triggered, time carries the handoff there. */
export function createStoryHandoff(paint:(progress:number)=>void){
  let target:0|1|undefined,progress=NaN,frame=0;
  const stop=()=>{cancelAnimationFrame(frame);frame=0;};
  const render=(value:number)=>{
    if(value===progress)return;
    progress=value;
    paint(value);
  };
  return {
    update(offset:number,visible:boolean){
      // Leave a small gap between thresholds so touch jitter cannot reverse it.
      const next=offset>=.16?1:offset<=.06?0:target??0;
      if(target===undefined||!visible||offset<-.65){
        stop();
        target=next;
        render(next);
        return;
      }
      if(next===target)return;
      target=next;
      stop();
      // Jump links and large scroll jumps land on a complete scene.
      if(offset>=1||progress===next){render(next);return;}
      const from=progress,start=performance.now(),duration=800*Math.abs(next-from);
      const tick=(now:number)=>{
        const time=Math.min(1,Math.max(0,(now-start)/duration));
        render(time===1?next:from+(next-from)*tourProgress(time));
        frame=time===1?0:requestAnimationFrame(tick);
      };
      frame=requestAnimationFrame(tick);
    },
    stop,
  };
}
