import {createIconMotion,iconStops,iconScrollScreens} from './icon-story';
const clamp = (value:number) => Math.min(1,Math.max(0,value));
const ramp = (value:number,from:number,to:number) => {
  const t=clamp((value-from)/(to-from));
  return t*t*(3-2*t);
};

export function mountStory() {
  const opening=document.querySelector<HTMLElement>('[data-opening]');
  const assembly=document.querySelector<HTMLElement>('[data-assembly]');
  if(!opening||!assembly)return;
  const lines=[...document.querySelectorAll<HTMLElement>('[data-line]')];
  const steps=[...document.querySelectorAll<HTMLButtonElement>('[data-step]')];
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const updateIcon=createIconMotion(assembly);
  const controller=new AbortController();
  const on={signal:controller.signal};
  let scheduled=false,disposed=false,topOpening=0,openingTravel=1;
  opening.style.setProperty('--opening-screens',String(iconScrollScreens+1));

  function render() {
    scheduled=false;
    if(disposed)return;
    const progress=clamp((scrollY-topOpening)/openingTravel);
    const state=updateIcon(progress,reduced.matches);
    const active=Math.round(state.copyPhase);
    lines.forEach((line,index)=>{
      const distance=Math.abs(state.copyPhase-index);
      line.style.opacity=String(reduced.matches?Number(index===active):1-ramp(distance,.25,.75));
      const shift=clamp(index-state.copyPhase)-clamp(state.copyPhase-index);
      line.style.transform=reduced.matches?'none':`translateY(calc(${shift} * var(--story-title) * 2.4))`;
    });
    steps.forEach((step,index)=>{
      if(index===active)step.setAttribute('aria-current','step');
      else step.removeAttribute('aria-current');
    });
    const exit=ramp(scrollY,topOpening+openingTravel,topOpening+openingTravel+innerHeight*.7);
    document.documentElement.style.setProperty('--meet-world',String(ramp(state.color,0,.55)*(1-exit)));
    opening!.style.setProperty('--story-controls-opacity',String(1-ramp(progress,.9,1)));
  }
  function requestRender() {
    if(scheduled)return;
    scheduled=true;
    requestAnimationFrame(render);
  }
  function measure() {
    topOpening=opening!.getBoundingClientRect().top+scrollY;
    openingTravel=Math.max(1,opening!.offsetHeight-innerHeight);
    requestRender();
  }
  steps.forEach((step,index)=>step.addEventListener('click',()=>{
    scrollTo({top:topOpening+openingTravel*iconStops[index],behavior:reduced.matches?'instant':'smooth'});
  },on));
  document.querySelectorAll<HTMLButtonElement>('[data-copy]').forEach(button=>button.addEventListener('click',async()=>{
    const value=document.getElementById(button.dataset.copy!)?.textContent??'';
    try{await navigator.clipboard.writeText(value);button.textContent='已复制';}
    catch{button.textContent='复制未成功';}
    setTimeout(()=>{if(button.isConnected)button.textContent='复制';},1800);
  },on));
  window.addEventListener('scroll',requestRender,{passive:true,...on});
  window.addEventListener('resize',measure,on);
  reduced.addEventListener('change',measure,on);
  const resize=new ResizeObserver(measure);
  resize.observe(opening);
  measure();
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    disposed=true;controller.abort();resize.disconnect();
  },on);
}
