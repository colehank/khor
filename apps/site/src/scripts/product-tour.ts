import {mountNetworkScene} from './network-scene';
import {mountCapabilityScene} from './capability-scene';

export function mountProductTour() {
  const tour=document.querySelector<HTMLElement>('[data-product-tour]');
  if(!tour)return;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const figures=[...tour.querySelectorAll<HTMLElement>('[data-concept]')];
  const scenes=new Map(figures.map(figure=>[figure,figure.dataset.concept==='network'?mountNetworkScene(figure):mountCapabilityScene(figure)]));
  const visible=new Set<HTMLElement>();
  const stop=(figure:HTMLElement)=>{figure.classList.remove('is-running');scenes.get(figure)!.pause();};
  const sync=(figure:HTMLElement)=>{
    if(visible.has(figure)&&!reduced.matches&&!document.hidden){figure.classList.add('is-running');scenes.get(figure)!.play();}
    else stop(figure);
  };
  const observer=new IntersectionObserver(entries=>{
    for(const entry of entries){
      const figure=entry.target as HTMLElement;
      if(entry.isIntersecting&&entry.intersectionRatio>=.35)visible.add(figure);else visible.delete(figure);
      sync(figure);
    }
  },{threshold:[0,.35]});
  figures.forEach(figure=>observer.observe(figure));
  const syncPlayback=()=>figures.forEach(sync);
  reduced.addEventListener('change',syncPlayback);
  document.addEventListener('visibilitychange',syncPlayback);
  window.addEventListener('pageshow',event=>{if(event.persisted)syncPlayback();});
  window.addEventListener('pagehide',event=>{
    figures.forEach(stop);
    if(!event.persisted){
      observer.disconnect();scenes.forEach(scene=>scene.destroy());
      reduced.removeEventListener('change',syncPlayback);document.removeEventListener('visibilitychange',syncPlayback);
    }
  });
}
