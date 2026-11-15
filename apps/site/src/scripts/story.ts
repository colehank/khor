import {createIconMotion,iconState,iconStops,iconSegments} from './icon-story';
import {mountCopyButtons} from './copy';

const duration=12000,firstHold=2100;
const timeAt=(progress:number)=>firstHold+(duration-firstHold)*progress;
// The glass scene begins without copy; name Khor near the sweep's arrival.
const glass=iconSegments.at(-1)!;
const meetAt=timeAt(glass.motionStart+(glass.motionEnd-glass.motionStart)*.82);
const chapters=[0,...iconStops.slice(0,-1).map(timeAt)];
const captionCues=[...chapters,meetAt];

export function mountStory() {
  const opening=document.querySelector<HTMLElement>('[data-opening]');
  const assembly=opening?.querySelector<HTMLElement>('[data-assembly]');
  if(!opening||!assembly)return;
  const lines=[...opening.querySelectorAll<HTMLElement>('[data-line]')];
  const steps=[...opening.querySelectorAll<HTMLButtonElement>('[data-step]')];
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const tracks=createIconMotion(assembly);
  const controller=new AbortController(),on={signal:controller.signal};
  let animations:Animation[]=[],timer:number|undefined;
  let started=false,inView=false,completed=false,disposed=false,active=-1,captionVisible=true;

  function caption(index:number,visible=true) {
    if(index===active&&visible===captionVisible)return;
    active=index;captionVisible=visible;
    lines.forEach((line,i)=>line.classList.toggle('is-current',i===index&&visible));
    steps.forEach((step,i)=>{
      if(i===index)step.setAttribute('aria-current','step');else step.removeAttribute('aria-current');
    });
    assembly!.dataset.iconScene=['devices','harnesses','mind','together','meet'][index];
  }
  function paint(progress:number) {
    const state=iconState(progress);
    for(const {target,sample} of tracks)for(const [property,value] of Object.entries(sample(state,progress))){
      target.style.setProperty(property.replace(/[A-Z]/g,letter=>`-${letter.toLowerCase()}`),String(value));
    }
  }
  function pause() {
    clearTimeout(timer);
    animations.forEach(animation=>animation.pause());
    opening!.removeAttribute('data-story-visible');
  }
  function finish() {
    if(completed||disposed)return;
    completed=true;
    animations.forEach(animation=>{animation.pause();animation.currentTime=duration;});
    opening!.setAttribute('data-story-complete','');caption(4);sync();
  }
  // Only chapter boundaries need JS. Geometry runs on one browser-owned clock.
  function scheduleCaption() {
    clearTimeout(timer);
    const time=Number(animations[0]?.currentTime??0);
    const index=chapters.findLastIndex(start=>time>=start);
    caption(index,index!==4||time>=meetAt);
    const next=captionCues.find(start=>start>time)??duration;
    timer=window.setTimeout(()=>{
      if(Number(animations[0]?.currentTime??0)>=duration-1)finish();else scheduleCaption();
    },Math.max(1,next-time));
  }
  function sync() {
    clearTimeout(timer);
    const visible=inView&&!document.hidden&&!reduced.matches&&!disposed;
    opening!.toggleAttribute('data-story-visible',visible);
    if(!visible||!started||completed){animations.forEach(animation=>animation.pause());return;}
    animations.forEach(animation=>animation.play());scheduleCaption();
  }
  function configure() {
    pause();
    animations.forEach(animation=>{animation.onfinish=null;animation.cancel();});animations=[];
    if(reduced.matches){
      completed=true;paint(1);caption(4);opening!.setAttribute('data-story-complete','');
      assembly!.dataset.iconMotion='reduced';return;
    }
    paint(0);
    const cues=iconSegments.flatMap(segment=>[segment.start,segment.copyEnd,segment.motionStart,segment.motionEnd,segment.end]);
    const positions=[...new Set([...Array.from({length:129},(_,i)=>i/128),...cues])].sort((a,b)=>a-b);
    for(const [index,{target,sample}] of tracks.entries()){
      const frames=[{...sample(iconState(0),0),offset:0},...positions.map(progress=>({
        ...sample(iconState(progress),progress),offset:timeAt(progress)/duration,
      }))];
      const animation=target.animate(frames,{duration,fill:'both'});
      animation.id=`khor-intro-${index}`;animation.pause();animation.currentTime=completed?duration:0;
      animations.push(animation);
    }
    animations[0].onfinish=finish;
    assembly!.dataset.iconMotion='autoplay';caption(completed?4:0);sync();
  }
  steps.forEach((step,index)=>step.addEventListener('click',()=>{
    if(reduced.matches)return;
    completed=false;started=true;opening!.removeAttribute('data-story-complete');
    animations.forEach(animation=>{animation.pause();animation.currentTime=chapters[index];});
    caption(index,index!==4);sync();
  },on));
  mountCopyButtons();configure();
  requestAnimationFrame(()=>opening.setAttribute('data-caption-ready',''));
  const visibility=new IntersectionObserver(entries=>{
    const entry=entries[0];inView=entry.isIntersecting&&entry.intersectionRatio>=.35;
    if(entry.intersectionRatio>=.75)started=true;
    sync();
  },{threshold:[0,.35,.75]});
  visibility.observe(opening);
  reduced.addEventListener('change',configure,on);
  document.addEventListener('visibilitychange',sync,on);
  window.addEventListener('pageshow',event=>{if(event.persisted)sync();},on);
  window.addEventListener('pagehide',event=>{
    pause();
    if(event.persisted)return;
    disposed=true;controller.abort();visibility.disconnect();
    animations.forEach(animation=>{animation.onfinish=null;animation.cancel();});
  },on);
}
