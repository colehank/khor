import {tourEasing,tourMotion} from './tour-motion';

const leg=tourMotion.travel+tourMotion.settle;
export const capabilityTiming={request:0,call:leg,reply:leg*2,delivery:leg*3,agentCycle:leg*4} as const;
type Point=[number,Omit<Keyframe,'offset'>];
const frames=(points:Point[],cycle:number):Keyframe[]=>{
  const merged=new Map<number,Omit<Keyframe,'offset'>>();
  for(const [time,value] of points)merged.set(time,{...merged.get(time),...value});
  return [...merged].sort(([a],[b])=>a-b).map(([time,value])=>({offset:time/cycle,easing:tourEasing,...value}));
};
const flow=(start:number,cycle:number,reverse=false)=>{
  const from=reverse?-1:.12,to=reverse?.12:-1,end=start+tourMotion.travel;
  return frames([[0,{strokeDashoffset:from,opacity:0}],[start,{strokeDashoffset:from,opacity:0}],
    [start+150,{opacity:1}],[end-150,{opacity:1}],[end,{strokeDashoffset:to,opacity:0}],
    [cycle,{strokeDashoffset:to,opacity:0}]],cycle);
};
const pulse=(times:number[],cycle:number,base=.22)=>frames([
  [0,{opacity:base}],...times.flatMap((time):Point[]=>[
    [time,{opacity:base}],[time+480,{opacity:.85}],[time+960,{opacity:base}],
  ]),[cycle,{opacity:base}],
],cycle);
const busy=(start:number,end:number,cycle:number,base:number,active:number)=>{
  // Fit whole gentle pulses to the busy interval, settling exactly on delivery.
  const count=Math.max(1,Math.round((end-start)/960)),beat=(end-start)/count;
  const points:Point[]=[[0,{opacity:base}],[start,{opacity:base}]];
  for(let i=0;i<count;i++)points.push(
    [start+(i+.5)*beat,{opacity:active}],[start+(i+1)*beat,{opacity:base}],
  );
  points.push([cycle,{opacity:base}]);return frames(points,cycle);
};
const light=(start:number,end:number,cycle:number,ink:string,accent:string)=>frames([
  [0,{color:ink}],[start,{color:ink}],[start+240,{color:accent}],
  [end,{color:accent}],[end+240,{color:ink}],[cycle,{color:ink}],
],cycle);

// Browser-owned timelines: no frame loop or DOM changes during playback.
// A reply cannot begin before the calls arrive and the agents light up.
export function mountCapabilityMotion(figure:HTMLElement,agent:boolean){
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const cycle=agent?capabilityTiming.agentCycle:tourMotion.turn;
  let animations:Animation[]=[];
  const animate=(selector:string,keyframes:Keyframe[])=>{
    for(const node of figure.querySelectorAll<SVGElement>(selector)){
      const animation=node.animate(keyframes,{duration:cycle,iterations:Infinity});
      animation.pause();animation.currentTime=0;animations.push(animation);
    }
  };
  const create=()=>{
    const style=getComputedStyle(figure),ink=style.getPropertyValue('--secondary').trim(),accent=style.getPropertyValue('--tour-accent').trim();
    for(const id of ['mac','linux']){
      const start=agent?capabilityTiming.call:0;
      const arrival=start+tourMotion.travel;
      animate(`[data-relation-flow="${agent?'call':'access'}"][data-relation-route="${id}"]`,flow(start,cycle,!agent));
      animate(`.relation-status.relation-${id}`,pulse([agent?arrival:start],cycle,0));
      animate(`[data-relation-session="${id}"]`,light(agent?arrival:start,agent?capabilityTiming.reply:start+960,cycle,ink,accent));
      if(agent)animate(`[data-relation-flow="reply"][data-relation-route="${id}"]`,flow(capabilityTiming.reply,cycle,true));
    }
    if(agent){
      animate('[data-relation-flow="request"]',flow(capabilityTiming.request,cycle));
      animate('[data-relation-flow="delivery"]',flow(capabilityTiming.delivery,cycle));
      const received=capabilityTiming.request+tourMotion.travel,delivered=capabilityTiming.delivery+tourMotion.travel;
      animate('.relation-router',busy(received,delivered,cycle,.22,.85));
      animate('[data-net-brand] > svg',busy(received,delivered,cycle,1,.6));
      animate('.relation-user-receipt',pulse([capabilityTiming.delivery+tourMotion.travel],cycle,0));
    }else animate('.relation-access-receipt',pulse([tourMotion.travel],cycle,0));
  };
  const clear=()=>{animations.forEach(animation=>animation.cancel());animations=[];};
  const settle=()=>{if(reduced.matches)clear();};
  reduced.addEventListener('change',settle);
  return {
    play(){if(reduced.matches)return;if(!animations.length)create();animations.forEach(animation=>animation.play());},
    pause(){animations.forEach(animation=>animation.pause());},
    destroy(){clear();reduced.removeEventListener('change',settle);},
  };
}
