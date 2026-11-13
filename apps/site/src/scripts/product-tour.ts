export function mountProductTour() {
  const tour=document.querySelector<HTMLElement>('[data-product-tour]');
  if(!tour)return;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const figures=[...tour.querySelectorAll<HTMLElement>('[data-concept]')];
  const visible=new Set<HTMLElement>();
  const paused=new Set<HTMLElement>();
  const timers=new Map<HTMLElement,ReturnType<typeof setTimeout>[]>();
  const canPlay=(figure:HTMLElement)=>visible.has(figure)&&!paused.has(figure)&&!reduced.matches&&!document.hidden;
  const clear=(figure:HTMLElement)=>{
    for(const timer of timers.get(figure)??[])clearTimeout(timer);
    timers.delete(figure);
    figure.classList.remove('is-running');
  };
  const later=(figure:HTMLElement,callback:()=>void,delay:number)=>{
    timers.set(figure,[...(timers.get(figure)??[]),setTimeout(callback,delay)]);
  };
  const ready=(figure:HTMLElement)=>{
    clear(figure);
    if(figure.dataset.concept==='agent'){
      figure.dataset.phase='ready';
      figure.querySelector('[data-agent-status]')!.textContent='Two results, back to you';
      figure.querySelector('[data-agent-result]')!.textContent='Ready';
      figure.querySelectorAll('[data-worker-state]').forEach(item=>item.textContent='Done');
      figure.querySelector('[data-code-result]')!.textContent='Local dates are preserved in CSV exports';
      figure.querySelector('[data-notes-result]')!.textContent='README explains the date column and CSV format';
    }
  };
  const terminal=tour.querySelector<HTMLElement>('[data-concept="terminal"]')!;
  const selectView=(view:string)=>{
    terminal.dataset.view=view;
    terminal.querySelector('[data-view-status]')!.textContent='Viewing from '+(view==='phone'?'Phone':'Desktop');
    terminal.querySelectorAll<HTMLButtonElement>('[data-view-choice]').forEach(item=>item.setAttribute('aria-pressed',String(item.dataset.viewChoice===view)));
  };
  const play=(figure:HTMLElement)=>{
    ready(figure);
    if(!canPlay(figure))return;
    // Only this visible illustration restarts; there is no continuous JS renderer.
    void figure.offsetWidth;
    figure.classList.add('is-running');
    if(figure.dataset.concept==='agent'){
      const phase=(name:string,text:string)=>{
        figure.dataset.phase=name;
        figure.querySelector('[data-agent-status]')!.textContent=text;
        figure.querySelector('[data-agent-result]')!.textContent=name==='collecting'?'Collecting':'In progress';
        figure.querySelectorAll('[data-worker-state]').forEach(item=>item.textContent=name==='delegating'?'Queued':name==='working'?'Working':'Done');
      };
      figure.querySelector('[data-code-result]')!.textContent='Checking date handling and export tests';
      figure.querySelector('[data-notes-result]')!.textContent='Reading changes and preparing project notes';
      phase('delegating','Khor delegates the work');
      later(figure,()=>phase('working','Agents working in parallel'),1100);
      later(figure,()=>phase('collecting','Bringing results together'),2700);
      later(figure,()=>{
        ready(figure);
        // Keep the actual result readable before the next request begins.
        if(canPlay(figure))later(figure,()=>play(figure),3600);
      },3800);
    }else if(figure===terminal){
      later(figure,()=>{
        if(!canPlay(figure))return;
        selectView(figure.dataset.view==='desktop'?'phone':'desktop');
        play(figure);
      },4600);
    }
  };
  const peer=tour.querySelector<HTMLElement>('[data-concept="peer"]')!;
  peer.querySelectorAll<HTMLButtonElement>('[data-resource-choice]').forEach(button=>button.addEventListener('click',()=>{
    const resource=button.dataset.resourceChoice!;
    peer.dataset.resource=resource;
    peer.querySelectorAll<HTMLElement>('[data-resource-panel]').forEach(panel=>panel.hidden=panel.dataset.resourcePanel!==resource);
    peer.querySelectorAll<HTMLButtonElement>('[data-resource-choice]').forEach(item=>item.setAttribute('aria-pressed',String(item===button)));
    play(peer);
  }));
  selectView(matchMedia('(any-pointer:coarse)').matches?'phone':'desktop');
  terminal.querySelectorAll<HTMLButtonElement>('[data-view-choice]').forEach(button=>button.addEventListener('click',()=>{
    selectView(button.dataset.viewChoice!);
    play(terminal);
  }));
  figures.forEach(figure=>{
    const button=figure.querySelector<HTMLButtonElement>('[data-motion-toggle]')!;
    button.addEventListener('click',()=>{
      if(paused.has(figure)){paused.delete(figure);play(figure);}
      else{paused.add(figure);ready(figure);}
      button.textContent=paused.has(figure)?'Play':'Pause';
      button.setAttribute('aria-label',`${paused.has(figure)?'Play':'Pause'} ${button.dataset.motionToggle} animation`);
    });
  });
  const observer='IntersectionObserver' in window?new IntersectionObserver(entries=>{
    for(const entry of entries){
      const figure=entry.target as HTMLElement;
      const isVisible=entry.isIntersecting&&entry.intersectionRatio>=.35;
      if(isVisible&&!visible.has(figure)){visible.add(figure);play(figure);}
      else if(!isVisible){visible.delete(figure);ready(figure);}
    }
  },{threshold:[0,.35]}):null;
  figures.forEach(figure=>observer?.observe(figure));
  const syncPlayback=()=>figures.forEach(figure=>canPlay(figure)?play(figure):ready(figure));
  reduced.addEventListener('change',syncPlayback);
  document.addEventListener('visibilitychange',syncPlayback);
  window.addEventListener('pageshow',event=>{if(event.persisted)syncPlayback();});
  window.addEventListener('pagehide',event=>{
    figures.forEach(ready);
    if(!event.persisted)observer?.disconnect();
  });
}
