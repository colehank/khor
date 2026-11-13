export function mountProductTour() {
  const tour=document.querySelector<HTMLElement>('[data-product-tour]');
  if(!tour)return;
  const preview=tour.querySelector<HTMLElement>('[data-terminal-preview]');
  const tabs=[...tour.querySelectorAll<HTMLButtonElement>('[data-terminal-device]')];
  const panels=[...tour.querySelectorAll<HTMLElement>('[data-terminal-panel]')];
  const select=(device:string)=>{
    if(preview)preview.dataset.terminalView=device;
    tabs.forEach(tab=>{const active=tab.dataset.terminalDevice===device;tab.setAttribute('aria-selected',String(active));tab.tabIndex=active?0:-1;});
    panels.forEach(panel=>panel.hidden=panel.dataset.terminalPanel!==device);
  };
  select(matchMedia('(any-pointer:coarse)').matches?'phone':'desktop');
  tabs.forEach((tab,index)=>{
    tab.addEventListener('click',()=>select(tab.dataset.terminalDevice!));
    tab.addEventListener('keydown',event=>{
      const next=event.key==='Home'?0:event.key==='End'?tabs.length-1:event.key==='ArrowRight'?(index+1)%tabs.length:event.key==='ArrowLeft'?(index+tabs.length-1)%tabs.length:-1;
      if(next<0)return;event.preventDefault();select(tabs[next].dataset.terminalDevice!);tabs[next].focus();
    });
  });
  if(matchMedia('(prefers-reduced-motion: reduce)').matches||!('IntersectionObserver' in window))return;
  tour.setAttribute('data-tour-motion','');
  const observer=new IntersectionObserver(entries=>{
    for(const entry of entries){
      if(!entry.isIntersecting)continue;
      entry.target.classList.add('is-in-view');
      observer.unobserve(entry.target);
    }
  },{threshold:.12});
  tour.querySelectorAll('[data-tour-scene]').forEach(scene=>observer.observe(scene));
  window.addEventListener('pagehide',event=>{if(!event.persisted)observer.disconnect();},{once:true});
}
