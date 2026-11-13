export function mountProductTour() {
  const tour=document.querySelector<HTMLElement>('[data-product-tour]');
  if(!tour)return;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const timers=new Map<HTMLElement,ReturnType<typeof setTimeout>[]>();
  const clear=(figure:HTMLElement)=>{
    for(const timer of timers.get(figure)??[])clearTimeout(timer);
    timers.delete(figure);figure.classList.remove('is-running');
  };
  const later=(figure:HTMLElement,callback:()=>void,delay:number)=>{
    timers.set(figure,[...(timers.get(figure)??[]),setTimeout(callback,delay)]);
  };
  const ready=(figure:HTMLElement)=>{
    clear(figure);
    if(figure.dataset.concept==='agent'){
      figure.dataset.phase='ready';
      figure.querySelector('[data-agent-status]')!.textContent='Two results, together';
      figure.querySelector('[data-agent-result]')!.textContent='Together, back to you';
    }
  };
  const play=(figure:HTMLElement)=>{
    ready(figure);
    if(reduced.matches)return;
    // A style flush restarts only this small SVG flow on an explicit replay.
    void figure.offsetWidth;
    figure.classList.add('is-running');
    if(figure.dataset.concept==='agent'){
      const phase=(name:string,text:string)=>{
        figure.dataset.phase=name;
        figure.querySelector('[data-agent-status]')!.textContent=text;
        figure.querySelector('[data-agent-result]')!.textContent=name==='collecting'?'Collecting results':'Results return here';
      };
      phase('delegating','Khor delegates the work');
      later(figure,()=>phase('working','Two agents, working in parallel'),1000);
      later(figure,()=>phase('collecting','Bringing the results together'),1900);
      later(figure,()=>ready(figure),3000);
    }else later(figure,()=>clear(figure),3500);
  };
  const peer=tour.querySelector<HTMLElement>('[data-concept="peer"]')!;
  peer.querySelectorAll<HTMLButtonElement>('[data-resource-choice]').forEach(button=>button.addEventListener('click',()=>{
    const resource=button.dataset.resourceChoice!;
    peer.dataset.resource=resource;
    peer.querySelector('[data-peer-title]')!.textContent=resource==='file'?'A file on your Mac':'Your localhost web app';
    peer.querySelectorAll<HTMLButtonElement>('[data-resource-choice]').forEach(item=>item.setAttribute('aria-pressed',String(item===button)));
    play(peer);
  }));
  const agent=tour.querySelector<HTMLElement>('[data-concept="agent"]')!;
  agent.querySelector('[data-replay-agent]')!.addEventListener('click',()=>play(agent));
  const terminal=tour.querySelector<HTMLElement>('[data-concept="terminal"]')!;
  const selectView=(view:string,animate:boolean)=>{
    terminal.dataset.view=view;
    terminal.querySelector('[data-view-status]')!.textContent='Viewing from '+(view==='phone'?'Phone':'Desktop');
    terminal.querySelectorAll<HTMLButtonElement>('[data-view-choice]').forEach(item=>item.setAttribute('aria-pressed',String(item.dataset.viewChoice===view)));
    if(animate)play(terminal);
  };
  selectView(matchMedia('(any-pointer:coarse)').matches?'phone':'desktop',false);
  terminal.querySelectorAll<HTMLButtonElement>('[data-view-choice]').forEach(button=>button.addEventListener('click',()=>selectView(button.dataset.viewChoice!,true)));
  const figures=[...tour.querySelectorAll<HTMLElement>('[data-concept]')];
  const seen=new WeakSet<Element>();
  const observer='IntersectionObserver' in window?new IntersectionObserver(entries=>{
    for(const entry of entries){
      const figure=entry.target as HTMLElement;
      if(entry.isIntersecting){
        if(!seen.has(figure)){seen.add(figure);play(figure);}
      }else ready(figure);
    }
  },{threshold:.35}):null;
  figures.forEach(figure=>observer?.observe(figure));
  reduced.addEventListener('change',()=>{if(reduced.matches)figures.forEach(ready);});
  document.addEventListener('visibilitychange',()=>{if(document.hidden)figures.forEach(ready);});
  window.addEventListener('pagehide',event=>{
    figures.forEach(ready);
    if(!event.persisted)observer?.disconnect();
  });
}
