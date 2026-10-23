/** Continuous cyclic presentation only; selection and timing belong to the gesture. */
import {useLayoutEffect,useRef,useState} from 'react';
import {stillMotion} from '@/lib/travel';

export function useDeckPosition(index:number,count:number,direction:number,direct?:number,members='') {
  const [position,setPosition]=useState(index);
  const current=useRef(index),target=useRef(index),previous=useRef(index),set=useRef({count,members});
  useLayoutEffect(()=>{
    const reset=count!==set.current.count||members!==set.current.members;
    set.current={count,members};
    if(reset||count<2){current.current=target.current=index;previous.current=index;setPosition(index);return;}
    if(direct!==undefined){current.current=target.current=direct;previous.current=index;setPosition(direct);return;}
    let delta=index-previous.current;
    if(delta>count/2)delta-=count;if(delta<-count/2)delta+=count;
    if(count===2&&Math.abs(delta)===1&&direction)delta=direction;
    previous.current=index;target.current=Math.round(target.current)+delta;
    const from=current.current,to=target.current;
    if(stillMotion()||Math.abs(to-from)<.001){current.current=to;setPosition(to);return;}
    const duration=parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--dur-180'));
    const began=performance.now();let frame=0;
    const tick=(now:number)=>{
      const p=Math.min(1,(now-began)/duration);
      current.current=from+(to-from)*(1-(1-p)**3);setPosition(current.current);
      if(p<1)frame=requestAnimationFrame(tick);
    };
    frame=requestAnimationFrame(tick);return()=>cancelAnimationFrame(frame);
  },[index,count,direct,members]);
  return direct??position;
}
