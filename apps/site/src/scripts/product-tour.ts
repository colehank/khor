import {devices,sessions,type SessionId} from '../components/concepts/tour-data';

export function mountProductTour() {
  const tour=document.querySelector<HTMLElement>('[data-product-tour]');
  if(!tour)return;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const figures=[...tour.querySelectorAll<HTMLElement>('[data-concept]')];
  const network=tour.querySelector<HTMLElement>('[data-concept="network"]')!;
  const sessionFigure=tour.querySelector<HTMLElement>('[data-concept="sessions"]')!;
  const resources=tour.querySelector<HTMLElement>('[data-concept="resources"]')!;
  const visible=new Set<HTMLElement>();
  const paused=new Set<HTMLElement>();
  const timers=new Map<HTMLElement,ReturnType<typeof setTimeout>>();
  let handoffNext=true;
  const canPlay=(figure:HTMLElement)=>visible.has(figure)&&!paused.has(figure)&&!reduced.matches&&!document.hidden;
  const stop=(figure:HTMLElement)=>{
    clearTimeout(timers.get(figure));timers.delete(figure);
    figure.classList.remove('is-running');
  };
  const selectPath=(path:'direct'|'relay')=>{
    network.dataset.path=path;
    network.querySelector('[data-path-status]')!.textContent=path==='direct'?'Direct connection':'Connected via relay';
    network.querySelectorAll<HTMLButtonElement>('[data-path-choice]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.pathChoice===path)));
  };
  const selectSession=(id:SessionId)=>{
    const session=sessions.find(item=>item.id===id)!;
    sessionFigure.dataset.session=id;
    sessionFigure.querySelector('[data-session-title]')!.textContent=session.name;
    sessionFigure.querySelector('[data-session-host]')!.textContent=devices[session.host].name;
    sessionFigure.querySelectorAll<HTMLElement>('[data-session-panel]').forEach(panel=>panel.hidden=panel.dataset.sessionPanel!==id);
    sessionFigure.querySelectorAll<HTMLButtonElement>('[data-session-choice]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.sessionChoice===id)));
    // The next act continues the actual selected session, not a second example.
    resources.dataset.session=id;
    resources.querySelector('[data-resource-session-title]')!.textContent=session.name;
    resources.querySelector('[data-resource-session-owner]')!.textContent=`${session.type} · ${devices[session.host].name}`;
    resources.querySelectorAll<HTMLElement>('[data-session-mark]').forEach(mark=>mark.hidden=mark.dataset.sessionMark!==id);
  };
  const selectView=(view:'mac'|'phone')=>{
    sessionFigure.dataset.view=view;
    sessionFigure.querySelectorAll<HTMLButtonElement>('[data-view-choice]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.viewChoice===view)));
  };
  const resourceOrder=['web','file','workspace'] as const;
  const selectResource=(resource:typeof resourceOrder[number])=>{
    resources.dataset.resource=resource;
    resources.querySelectorAll<HTMLElement>('[data-resource-panel]').forEach(panel=>panel.hidden=panel.dataset.resourcePanel!==resource);
    resources.querySelectorAll<HTMLButtonElement>('[data-resource-choice]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.resourceChoice===resource)));
  };
  const play=(figure:HTMLElement)=>{
    stop(figure);
    if(!canPlay(figure))return;
    figure.classList.add('is-running');
    timers.set(figure,setTimeout(()=>{
      if(!canPlay(figure))return;
      if(figure===network)selectPath(network.dataset.path==='direct'?'relay':'direct');
      else if(figure===sessionFigure){
        if(handoffNext)selectView(sessionFigure.dataset.view==='mac'?'phone':'mac');
        else{
          const index=sessions.findIndex(item=>item.id===sessionFigure.dataset.session);
          selectSession(sessions[(index+1)%sessions.length].id);
        }
        handoffNext=!handoffNext;
      }else{
        const index=resourceOrder.indexOf(resources.dataset.resource as typeof resourceOrder[number]);
        selectResource(resourceOrder[(index+1)%resourceOrder.length]);
      }
      play(figure);
    },figure===network?5200:figure===sessionFigure?3600:4800));
  };
  network.querySelectorAll<HTMLButtonElement>('[data-path-choice]').forEach(button=>button.addEventListener('click',()=>{
    selectPath(button.dataset.pathChoice as 'direct'|'relay');play(network);
  }));
  sessionFigure.querySelectorAll<HTMLButtonElement>('[data-session-choice]').forEach(button=>button.addEventListener('click',()=>{
    selectSession(button.dataset.sessionChoice as SessionId);handoffNext=true;play(sessionFigure);
  }));
  sessionFigure.querySelectorAll<HTMLButtonElement>('[data-view-choice]').forEach(button=>button.addEventListener('click',()=>{
    selectView(button.dataset.viewChoice as 'mac'|'phone');handoffNext=false;play(sessionFigure);
  }));
  resources.querySelectorAll<HTMLButtonElement>('[data-resource-choice]').forEach(button=>button.addEventListener('click',()=>{
    selectResource(button.dataset.resourceChoice as typeof resourceOrder[number]);play(resources);
  }));
  figures.forEach(figure=>{
    const button=figure.querySelector<HTMLButtonElement>('[data-motion-toggle]')!;
    button.addEventListener('click',()=>{
      if(paused.has(figure)){paused.delete(figure);play(figure);}
      else{paused.add(figure);stop(figure);}
      button.textContent=paused.has(figure)?'Play':'Pause';
      button.setAttribute('aria-label',`${paused.has(figure)?'Play':'Pause'} ${button.dataset.motionToggle} animation`);
    });
  });
  const observer='IntersectionObserver' in window?new IntersectionObserver(entries=>{
    for(const entry of entries){
      const figure=entry.target as HTMLElement;
      const isVisible=entry.isIntersecting&&entry.intersectionRatio>=.35;
      if(isVisible&&!visible.has(figure)){visible.add(figure);play(figure);}
      else if(!isVisible){visible.delete(figure);stop(figure);}
    }
  },{threshold:[0,.35]}):null;
  figures.forEach(figure=>observer?.observe(figure));
  const syncPlayback=()=>figures.forEach(figure=>canPlay(figure)?play(figure):stop(figure));
  reduced.addEventListener('change',syncPlayback);
  document.addEventListener('visibilitychange',syncPlayback);
  window.addEventListener('pageshow',event=>{if(event.persisted)syncPlayback();});
  window.addEventListener('pagehide',event=>{
    figures.forEach(stop);
    if(!event.persisted)observer?.disconnect();
  });
}
