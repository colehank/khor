export function mountPlatformHints() {
  const buttons=[...document.querySelectorAll<HTMLButtonElement>('.platform-option[data-unavailable]')];
  const controller=new AbortController(),on={signal:controller.signal};
  let current:HTMLButtonElement|undefined,reset=0;
  const close=()=>{
    current?.removeAttribute('data-hint-open');current=undefined;
    clearTimeout(reset);
  };
  buttons.forEach(button=>button.addEventListener('click',()=>{
    const opened=button===current;close();
    if(opened)return;
    button.removeAttribute('data-hint-dismissed');
    current=button;button.setAttribute('data-hint-open','');
    reset=window.setTimeout(close,1800);
  },on));
  buttons.forEach(button=>{
    const resetDismissal=()=>button.removeAttribute('data-hint-dismissed');
    button.addEventListener('focus',resetDismissal,on);
    button.addEventListener('pointerenter',resetDismissal,on);
  });
  document.addEventListener('pointerdown',event=>{
    if(current&&!current.contains(event.target as Node))close();
  },on);
  document.addEventListener('keydown',event=>{
    if(event.key!=='Escape')return;
    buttons.filter(button=>button===current||button.matches(':hover,:focus-visible')).forEach(button=>button.setAttribute('data-hint-dismissed',''));
    close();
  },on);
  window.addEventListener('scroll',close,{passive:true,...on});
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    close();controller.abort();
  },{once:true});
}
