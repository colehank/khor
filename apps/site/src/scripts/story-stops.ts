/** Native reading stops live in document space, outside the sticky artwork.
 * The browser owns touch momentum; no wheel/touch handlers or scroll correction.
 */
export function createStoryStops(opening:HTMLElement,poses:number[],reduced:MediaQueryList){
  const root=document.documentElement;
  const work=document.querySelector<HTMLElement>('#work')!;
  const closing=document.querySelector<HTMLElement>('[data-story-closing]')!;
  const points=(host:HTMLElement,count:number)=>Array.from({length:count},()=>{
    const point=document.createElement('span');
    point.className='story-snap-point';point.setAttribute('aria-hidden','true');
    host.append(point);return point;
  });
  const introPoints=points(opening,poses.length+1);
  const workPoints=points(work,1);
  const closingPoints=points(closing,2);
  const viewport=window.visualViewport;
  const syncEnabled=()=>{
    root.toggleAttribute('data-story-snapping',!reduced.matches&&(!viewport||viewport.scale<=1));
  };
  viewport?.addEventListener('resize',syncEnabled,{passive:true});
  const value=(style:CSSStyleDeclaration,name:string)=>Number(style.getPropertyValue(name));
  const place=(markers:HTMLElement[],offsets:number[],padding:number)=>markers.forEach((point,index)=>{
    // Root scroll-padding protects anchor links from the fixed navigation.
    // Offset these invisible markers so a stop still frames the full viewport.
    point.style.top=`${offsets[index]+padding}px`;
  });
  return {
    measure(){
      syncEnabled();
      if(reduced.matches)return;
      const height=opening.querySelector<HTMLElement>('.opening-sticky')!.offsetHeight;
      const style=getComputedStyle(opening),end=getComputedStyle(closing);
      const travel=value(style,'--story-intro-travel')*height;
      const hold=value(style,'--story-intro-hold')*height;
      const unfold=value(style,'--story-unfold-travel')*height;
      const padding=parseFloat(getComputedStyle(root).scrollPaddingTop)||0;
      const networkStart=hold+travel+unfold;
      place(introPoints,[...poses.map((pose,index)=>index===0?0:hold+pose*travel),networkStart],padding);
      place(workPoints,[0],padding);
      const closingHeight=closing.querySelector<HTMLElement>('.story-closing-sticky')!.offsetHeight;
      const agentHold=value(end,'--story-agent-hold')*closingHeight;
      const buildStart=agentHold+value(end,'--story-fold-travel')*closingHeight;
      place(closingPoints,[0,buildStart],padding);
    },
    destroy(){
      root.removeAttribute('data-story-snapping');
      viewport?.removeEventListener('resize',syncEnabled);
      [...introPoints,...workPoints,...closingPoints].forEach(point=>point.remove());
    },
  };
}
