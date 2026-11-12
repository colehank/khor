import {createIconMotion,iconState,iconStops,clamp,ease,type MotionTrack} from './icon-story';

type ScrollTimelineConstructor=new(options:{source:Element;axis:string})=>AnimationTimeline;
type ScrollOptions=KeyframeAnimationOptions&{rangeStart:string;rangeEnd:string};

export function mountStory() {
  const opening=document.querySelector<HTMLElement>('[data-opening]');
  const assembly=document.querySelector<HTMLElement>('[data-assembly]');
  const sticky=opening?.querySelector<HTMLElement>('.opening-sticky');
  if(!opening||!assembly||!sticky)return;
  const lines=[...opening.querySelectorAll<HTMLElement>('[data-line]')];
  const steps=[...opening.querySelectorAll<HTMLButtonElement>('[data-step]')];
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const tracks=createIconMotion(assembly);
  const environment=opening.querySelector<HTMLElement>('[data-meet-environment]')!;
  const cue=opening.querySelector<HTMLElement>('.scroll-cue')!;
  const cueProgress=cue.querySelector<HTMLElement>('.scroll-stem-progress')!;
  const controls=opening.querySelector<HTMLElement>('.story-steps')!;
  const controller=new AbortController(),on={signal:controller.signal};
  const Timeline=(window as Window&{ScrollTimeline?:ScrollTimelineConstructor}).ScrollTimeline;
  let animations:Animation[]=[],native=false,scheduled=false,disposed=false;
  let topOpening=0,openingTravel=1,introProgress=0,exitProgress=.1,active=-1,lastProgress=NaN;
  let geometry='',inView=true;
  let previous=new WeakMap<MotionTrack['target'],Record<string,string>>();
  const track=(target:MotionTrack['target'],sample:MotionTrack['sample'])=>tracks.push({target,sample});
  lines.forEach((line,index)=>track(line,state=>{
    const delta=state.phase-index;
    return {opacity:1-ease(Math.abs(delta),.12,.5),transform:`translateY(calc(${-Math.max(-1,Math.min(1,delta))} * var(--gap-pane)))`};
  }));
  track(cue,state=>({opacity:1-ease(state.phase,0,.5)}));
  track(cueProgress,(_,progress)=>({transform:`scaleY(${introProgress?clamp(1+progress/introProgress):1})`}));
  track(controls,(_,progress)=>({opacity:1-ease(progress,.92,1)}));
  track(environment,(state,progress)=>({opacity:(.35*clamp(state.light/.55)+.65*ease(state.light,0,.55))*(1-ease(progress,1,1+exitProgress))}));

  function paint(progress:number) {
    const state=iconState(progress);
    for(const {target,sample} of tracks){
      const values=previous.get(target)??{};
      for(const [property,value] of Object.entries(sample(state,progress))){
        const text=String(value);
        if(values[property]===text)continue;
        target.style.setProperty(property.replace(/[A-Z]/g,letter=>`-${letter.toLowerCase()}`),text);
        values[property]=text;
      }
      previous.set(target,values);
    }
  }
  function render() {
    scheduled=false;
    if(disposed||!inView)return;
    const distance=scrollY-topOpening-openingTravel*introProgress;
    const progress=distance>=openingTravel-1&&distance<=openingTravel?1:Math.max(-introProgress,distance/openingTravel);
    const next=Math.round(iconState(progress).phase);
    if(next!==active){
      active=next;
      assembly!.dataset.iconScene=['devices','harnesses','mind','together','meet'][active];
      steps.forEach((step,index)=>{
        if(index===active)step.setAttribute('aria-current','step');else step.removeAttribute('aria-current');
      });
    }
    if(!native&&progress!==lastProgress){
      if(reduced.matches){
        paint(iconStops[active]);
        // Reduced motion is a static chapter, including static text.
        lines.forEach((line,index)=>{line.style.opacity=String(index===active?1:0);line.style.transform='none';});
      }else paint(progress);
      lastProgress=progress;
    }
  }
  function requestRender() {
    if(scheduled||disposed||!inView)return;
    scheduled=true;requestAnimationFrame(render);
  }
  function bindTimeline() {
    animations.forEach(animation=>animation.cancel());animations=[];
    native=false;lastProgress=NaN;previous=new WeakMap();
    if(Timeline&&!reduced.matches&&CSS.supports('animation-range','0px 1px')){
      try{
        const timeline=new Timeline({source:document.documentElement,axis:'block'});
        const end=1+exitProgress;
        const positions=[...new Set([-introProgress,...Array.from({length:129},(_,i)=>i/128),...iconStops,...Array.from({length:17},(_,i)=>1+exitProgress*i/16)])].sort((a,b)=>a-b);
        const samples=positions.map(progress=>({progress,state:iconState(progress)}));
        for(const {target,sample} of tracks){
          const frames=samples.map(({progress,state})=>({...sample(state,progress),offset:(progress+introProgress)/(end+introProgress)}));
          const options:ScrollOptions={timeline,fill:'both',rangeStart:`${topOpening}px`,rangeEnd:`${topOpening+openingTravel*(end+introProgress)}px`};
          const animation=target.animate(frames,options);animations.push(animation);
          if(animation.timeline!==timeline)throw new Error('Scroll timeline is unavailable');
        }
        native=true;
      }catch{animations.forEach(animation=>animation.cancel());animations=[];}
    }
    assembly!.dataset.iconMotion=reduced.matches?'reduced':native?'native':'fallback';
    requestRender();
  }
  function measure() {
    // CSS and JS share the stable sticky height. Mobile toolbar/keyboard
    // changes must not remap a stationary scroll position via innerHeight.
    const top=opening!.getBoundingClientRect().top+scrollY;
    const intro=Number(getComputedStyle(opening!).getPropertyValue('--story-intro-hold'))*sticky!.offsetHeight;
    const travel=Math.max(1,opening!.offsetHeight-sticky!.offsetHeight-intro);
    const exit=sticky!.offsetHeight*.7/travel;
    const key=[top,travel,intro,exit,reduced.matches].join(':');
    if(key===geometry)return;
    geometry=key;topOpening=top;openingTravel=travel;introProgress=intro/travel;exitProgress=exit;bindTimeline();
  }
  steps.forEach((step,index)=>step.addEventListener('click',()=>{
    // Round forward so the final bookmark reaches the exact canonical pose.
    const distance=index===0?0:openingTravel*(introProgress+iconStops[index]);
    scrollTo({top:Math.ceil(topOpening+distance),behavior:reduced.matches?'instant':'smooth'});
  },on));
  document.querySelectorAll<HTMLButtonElement>('[data-copy]').forEach(button=>button.addEventListener('click',async()=>{
    const value=document.getElementById(button.dataset.copy!)?.textContent??'';
    try{await navigator.clipboard.writeText(value);button.textContent='已复制';}
    catch{button.textContent='复制未成功';}
    setTimeout(()=>{if(button.isConnected)button.textContent='复制';},1800);
  },on));
  const visibility=new IntersectionObserver(entries=>{
    inView=entries[0].isIntersecting;if(inView)requestRender();
  });
  visibility.observe(opening);
  const resize=new ResizeObserver(measure);resize.observe(opening);resize.observe(sticky);
  const hero=document.querySelector<HTMLElement>('[data-product-hero]');if(hero)resize.observe(hero);
  window.addEventListener('scroll',requestRender,{passive:true,...on});
  reduced.addEventListener('change',measure,on);measure();
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    disposed=true;controller.abort();resize.disconnect();visibility.disconnect();
    animations.forEach(animation=>animation.cancel());
  },on);
}
