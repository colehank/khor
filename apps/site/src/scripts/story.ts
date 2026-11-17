import {createIconMotion,iconState,iconStops,iconSegments,clamp,ease,type MotionTrack} from './icon-story';
import {mountCopyButtons} from './copy';
import {mountNarrativeTour} from './narrative-tour';
import {createStoryStops} from './story-stops';

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
  const readingPoses=[0,...iconSegments.map(segment=>segment.motionEnd)];
  const stops=createStoryStops(opening,readingPoses,reduced);
  const tracks=createIconMotion(assembly);
  const tour=mountNarrativeTour(sticky);
  const openingCopy=opening.querySelector<HTMLElement>('.opening-copy')!;
  const cue=opening.querySelector<HTMLElement>('.scroll-cue')!;
  const cueProgress=cue.querySelector<HTMLElement>('.scroll-stem-progress')!;
  const controls=opening.querySelector<HTMLElement>('.story-steps')!;
  const controller=new AbortController(),on={signal:controller.signal};
  const Timeline=(window as Window&{ScrollTimeline?:ScrollTimelineConstructor}).ScrollTimeline;
  let animations:Animation[]=[],native=false,scheduled=false,disposed=false;
  let topOpening=0,openingTravel=1,introProgress=0,active=-1,lastProgress=NaN;
  let geometry='',inView=true;
  let previous=new WeakMap<MotionTrack['target'],Record<string,string>>();
  const track=(target:MotionTrack['target'],sample:MotionTrack['sample'])=>tracks.push({target,sample});
  track(cue,state=>({opacity:1-ease(state.copyPhase,0,.5)}));
  track(cueProgress,(_,progress)=>({transform:`scaleY(${introProgress?clamp(1+progress/introProgress):1})`}));
  track(controls,(_,progress)=>({opacity:1-ease(progress,.92,1)}));

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
    if(disposed)return;
    if(reduced.matches){
      if(lastProgress!==1){paint(1);assembly!.dataset.iconScene='meet';lastProgress=1;}
      tour.render(0,true,true);openingCopy.style.opacity='1';
      return;
    }
    if(!inView)return;
    const distance=scrollY-topOpening-openingTravel*introProgress;
    const progress=distance>=openingTravel-1&&distance<=openingTravel?1:Math.max(-introProgress,distance/openingTravel);
    const q=(scrollY-topOpening-openingTravel*(1+introProgress))/sticky!.offsetHeight;
    openingCopy.style.opacity=tour.render(q,inView,false)?'1':'0';
    // Introduce the next sentence as its geometry starts. Its short fade is
    // time-based, so pausing a scroll never leaves two half-visible headlines.
    // Bookmarks round to the next CSS pixel; that must not select another chapter.
    const copyPosition=progress-1/openingTravel;
    const next=copyPosition<=0?0:Math.min(lines.length-1,iconStops.findLastIndex(stop=>copyPosition>stop)+1);
    if(next!==active){
      const first=active<0;
      active=next;
      lines.forEach((line,index)=>line.classList.toggle('is-current',index===active));
      if(first)requestAnimationFrame(()=>opening!.setAttribute('data-caption-ready',''));
      assembly!.dataset.iconScene=['devices','harnesses','mind','together','meet'][active];
      steps.forEach((step,index)=>{
        if(index===active)step.setAttribute('aria-current','step');else step.removeAttribute('aria-current');
      });
    }
    if(!native&&progress!==lastProgress){
      paint(progress);
      lastProgress=progress;
    }
  }
  function requestRender() {
    if(scheduled||disposed||(!inView&&!reduced.matches))return;
    scheduled=true;requestAnimationFrame(render);
  }
  function bindTimeline() {
    animations.forEach(animation=>animation.cancel());animations=[];
    native=false;lastProgress=NaN;previous=new WeakMap();
    if(Timeline&&!reduced.matches&&CSS.supports('animation-range','0px 1px')){
      try{
        const timeline=new Timeline({source:document.documentElement,axis:'block'});
        const end=1;
        const cues=iconSegments.flatMap(segment=>[segment.start,segment.copyEnd,segment.motionStart,segment.motionEnd,segment.end]);
        const positions=[...new Set([-introProgress,...Array.from({length:129},(_,i)=>i/128),...cues])].sort((a,b)=>a-b);
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
    const intro=reduced.matches?0:Number(getComputedStyle(opening!).getPropertyValue('--story-intro-hold'))*sticky!.offsetHeight;
    const travel=reduced.matches?1:Number(getComputedStyle(opening!).getPropertyValue('--story-intro-travel'))*sticky!.offsetHeight;
    tour.measure();
    stops.measure();
    const key=[top,travel,intro,reduced.matches].join(':');
    if(key===geometry)return;
    geometry=key;topOpening=top;openingTravel=travel;introProgress=intro/travel;bindTimeline();
  }
  steps.forEach((step,index)=>step.addEventListener('click',()=>{
    // Round forward so the final bookmark reaches the exact canonical pose.
    const distance=index===0?0:openingTravel*(introProgress+readingPoses[index]);
    scrollTo({top:Math.ceil(topOpening+distance),behavior:reduced.matches?'instant':'smooth'});
  },on));
  const navigateHash=(initial=false)=>{
    if(location.hash!=='#connect'||reduced.matches)return;
    const unfold=Number(getComputedStyle(opening!).getPropertyValue('--story-unfold-travel'));
    scrollTo({top:Math.round(topOpening+openingTravel*(1+introProgress)+unfold*sticky!.offsetHeight),behavior:initial?'instant':'smooth'});
  };
  mountCopyButtons();
  const visibility=new IntersectionObserver(entries=>{
    inView=entries[0].isIntersecting;if(inView)requestRender();else tour.pause();
  });
  visibility.observe(opening);
  const resize=new ResizeObserver(measure);resize.observe(opening);resize.observe(sticky);
  const hero=document.querySelector<HTMLElement>('[data-product-hero]');if(hero)resize.observe(hero);
  window.addEventListener('scroll',requestRender,{passive:true,...on});
  window.addEventListener('resize',measure,on);window.addEventListener('khor:localechange',measure,on);window.addEventListener('khor:themechange',measure,on);
  window.addEventListener('hashchange',()=>navigateHash(),on);
  document.addEventListener('visibilitychange',()=>{if(document.hidden)tour.pause();else requestRender();},on);
  reduced.addEventListener('change',measure,on);measure();
  requestAnimationFrame(()=>navigateHash(true));
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    disposed=true;controller.abort();resize.disconnect();visibility.disconnect();
    tour.destroy();
    stops.destroy();
    animations.forEach(animation=>animation.cancel());
  },on);
}
