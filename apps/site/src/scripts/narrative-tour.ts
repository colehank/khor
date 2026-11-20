import {mountNetworkScene} from './network-scene';
import {mountCapabilityScene} from './capability-scene';
import {createStoryMorph} from './story-morph';
import {createStoryHandoff} from './tour-motion';

const ease=(t:number)=>{const p=Math.max(0,Math.min(1,t));return p*p*(3-2*p);};

/** The two icon handoffs play within their pinned stages. The middle chapter scrolls normally. */
export function mountNarrativeTour(stage:HTMLElement){
  const opening=stage.parentElement!;
  const openingCopy=stage.querySelector<HTMLElement>('.opening-copy')!;
  const networkSection=stage.querySelector<HTMLElement>('.tour-section')!;
  const networkFigure=networkSection.querySelector<HTMLElement>('[data-concept]')!;
  const network=mountNetworkScene(networkFigure);
  const assembly=stage.querySelector<HTMLElement>('[data-assembly]')!;
  const workFigure=document.querySelector<HTMLElement>('[data-concept="work"]')!;
  const work=mountCapabilityScene(workFigure);
  const closing=document.querySelector<HTMLElement>('[data-story-closing]')!;
  const closingStage=closing.querySelector<HTMLElement>('.story-closing-sticky')!;
  const agentSection=closing.querySelector<HTMLElement>('.tour-section')!;
  const agentFigure=agentSection.querySelector<HTMLElement>('[data-concept]')!;
  const agent=mountCapabilityScene(agentFigure);
  const finalCopy=closing.querySelector<HTMLElement>('[data-story-final-copy]')!;
  const finalBrand=closing.querySelector<HTMLElement>('[data-story-final-brand]')!;
  const finalLayout=closing.querySelector<HTMLElement>('[data-story-final-layout]')!;
  const finalInstallations=closing.querySelector<HTMLElement>('[data-story-final-installations]')!;
  const openingLight=stage.querySelector<SVGSVGElement>('[data-meet-environment] > svg')!;
  const closingLight=document.createElement('div');closingLight.className='meet-environment';closingLight.setAttribute('aria-hidden','true');
  closingLight.innerHTML=openingLight.outerHTML.replaceAll('story-environment-','closing-environment-');closingStage.prepend(closingLight);
  const intro=createStoryMorph(stage,assembly.querySelector<SVGSVGElement>('[data-story-layer="material"] > svg')!,networkFigure.querySelector('[data-net-scene]')!,'unfold-');
  const fold=createStoryMorph(closingStage,finalBrand.querySelector('svg')!,agentFigure.querySelector('[data-net-scene]')!,'fold-');
  const controller=new AbortController(),on={signal:controller.signal};
  let networkPlaying=false,networkExpanded=false,openingVisible=false,workVisible=false,closingVisible=false;
  let closingTop=0,agentHold=0,foldDistance=1,unfoldDistance=1,closingFrame=0,compactClosing=false,agentPlaying=false,lastClosing=NaN;

  const setNetwork=(playing:boolean)=>{
    if(playing===networkPlaying)return;
    networkPlaying=playing;
    if(playing)network.play();else network.pause();
  };
  const syncWork=()=>{
    if(workVisible&&!document.hidden)work.play();else work.pause();
  };
  const syncNetwork=()=>setNetwork(networkExpanded&&openingVisible&&!document.hidden);
  const syncAgent=()=>{
    const playAgent=closingVisible&&lastClosing===0&&!document.hidden;
    if(playAgent!==agentPlaying){
      agentPlaying=playAgent;
      if(playAgent)agent.play();else agent.pause();
    }
  };
  const paintClosing=(p:number)=>{
    lastClosing=p;
    syncAgent();
    // Let the icon clear both text regions before the compact final composition.
    const build=p>=(compactClosing?.92:.45);
    agentSection.classList.toggle('is-current',p<(compactClosing?.6:.45));finalCopy.classList.toggle('is-current',build);finalCopy.inert=!build;
    finalInstallations.classList.toggle('is-current',build);finalInstallations.inert=!build;
    agentFigure.style.opacity='1';finalBrand.style.visibility='';
    // Endpoint handoff only: the live SVG already matches the static artwork.
    finalBrand.style.opacity=p===1?'1':'0';fold.frame(1-p);
    closingLight.style.opacity=String(1-ease(((1-p)-.06)/.58));
  };
  const paintOpening=(p:number)=>{
    if(p<1&&networkExpanded){setNetwork(false);network.reset();intro.measure();}
    networkExpanded=p===1;
    syncNetwork();
    networkSection.classList.toggle('is-current',p>=.45);
    openingCopy.style.opacity=p<.45?'1':'0';
    networkFigure.style.opacity='1';assembly.style.visibility='';
    assembly.style.opacity=p===0?'1':'0';intro.frame(p);
    openingLight.style.opacity=String(1-ease((p-.06)/.58));
  };
  const unfolding=createStoryHandoff(paintOpening),folding=createStoryHandoff(paintClosing);
  const renderClosing=()=>{
    folding.update((scrollY-closingTop-agentHold)/foldDistance,closingVisible&&!document.hidden);
    syncAgent();
  };
  const requestClosing=()=>{
    if(closingFrame||!closingVisible)return;
    closingFrame=requestAnimationFrame(()=>{closingFrame=0;renderClosing();});
  };
  const measureCopy=(host:HTMLElement)=>{
    const copy=host.querySelector<HTMLElement>('.tour-copy')!;
    host.style.setProperty('--tour-copy-height',`${copy.offsetHeight}px`);
  };
  const measure=()=>{
    // Reserve natural document space if a short viewport cannot fit the final
    // composition. It scrolls out with the closing stage, without an inner scroller.
    const stageHeight=closingStage.offsetHeight;
    compactClosing=closingStage.clientWidth<=760;
    const overflow=Math.max(0,finalLayout.offsetHeight-stageHeight);
    closing.parentElement!.style.setProperty('--story-final-overflow',`${Math.ceil(overflow)}px`);
    const networkOverflow=Math.max(0,networkFigure.getBoundingClientRect().bottom-stage.getBoundingClientRect().bottom);
    opening.style.setProperty('--story-network-overflow',`${Math.ceil(networkOverflow)}px`);
    measureCopy(stage);measureCopy(closingStage);
    network.map.measure();work.map.measure();agent.map.measure();
    intro.measure();fold.measure();
    closingTop=closing.getBoundingClientRect().top+scrollY;
    const closingStyle=getComputedStyle(closing);
    agentHold=Number(closingStyle.getPropertyValue('--story-agent-hold'))*stageHeight;
    foldDistance=Number(closingStyle.getPropertyValue('--story-fold-travel'))*stageHeight;
    unfoldDistance=Number(getComputedStyle(opening).getPropertyValue('--story-unfold-travel'));
    if(!Number.isNaN(lastClosing))paintClosing(lastClosing);
    renderClosing();
  };
  /** q is native viewport distance after the complete Meet Khor icon. */
  const render=(q:number,visible:boolean)=>{
    openingVisible=visible;
    unfolding.update(q/unfoldDistance,visible&&!document.hidden);
    syncNetwork();
  };
  const observer=new IntersectionObserver(entries=>{
    for(const entry of entries){
      if(entry.target===workFigure){workVisible=entry.isIntersecting;syncWork();}
      else{closingVisible=entry.isIntersecting;renderClosing();}
    }
  },{threshold:0});
  observer.observe(workFigure);observer.observe(closing);
  const finalResize=new ResizeObserver(measure);finalResize.observe(finalLayout);
  window.addEventListener('scroll',requestClosing,{passive:true,...on});
  document.addEventListener('visibilitychange',()=>{syncWork();renderClosing();},on);
  return {
    measure,render,
    destroy(){controller.abort();observer.disconnect();finalResize.disconnect();cancelAnimationFrame(closingFrame);unfolding.stop();folding.stop();closing.parentElement!.style.removeProperty('--story-final-overflow');opening.style.removeProperty('--story-network-overflow');network.destroy();work.destroy();agent.destroy();intro.destroy();fold.destroy();closingLight.remove();},
  };
}
