export function mountDesktopPreview() {
  const preview=document.querySelector<HTMLElement>('[data-desktop-preview]');
  const stage=document.querySelector<HTMLElement>('[data-desktop-stage]');
  const frame=document.querySelector<HTMLIFrameElement>('[data-desktop-demo]');
  const shell=preview?.closest<HTMLElement>('.macos-window');
  const showcase=preview?.closest<HTMLElement>('[data-product-showcase]');
  const hero=preview?.closest<HTMLElement>('[data-product-hero]');
  if(!preview||!stage||!frame||!shell||!showcase||!hero)return;
  const openButton=showcase.querySelector<HTMLButtonElement>('[data-preview-open]')!;
  const closeButton=showcase.querySelector<HTMLButtonElement>('[data-preview-close]')!;
  const backdrop=showcase.querySelector<HTMLElement>('[data-viewer-backdrop]')!;
  const controls=[...showcase.querySelectorAll<HTMLElement>('[data-viewer-controls]')];
  const pan=showcase.querySelector<HTMLInputElement>('[data-preview-pan]')!;
  const compact=matchMedia('(max-width:760px)');
  const controller=new AbortController(),on={signal:controller.signal};
  let started=false,expanded=false,visible=false,idle=0,timer=0,returnFocus:HTMLElement|null=null;
  let oldOverflow='',oldScroll=0;
  const inertSiblings=new Map<HTMLElement,boolean>();
  const measure=()=>{
    if(!expanded)shell.style.setProperty('--desktop-scale',String(preview.clientWidth/stage.offsetWidth));
    const max=Math.max(0,stage.offsetWidth-preview.clientWidth);
    pan.disabled=max<1;pan.value=String(max?preview.scrollLeft/max*100:50);
    showcase.toggleAttribute('data-viewer-pan',expanded&&max>1);
  };
  const resize=new ResizeObserver(measure);resize.observe(preview);measure();
  const start=()=>{
    if(started)return;started=true;
    frame.src=new URL(frame.dataset.src!,location.origin).href;
  };
  const schedule=()=>{
    if(started||compact.matches||!visible)return;
    // Paint the real poster first; phones initialize the live GUI on inspection.
    if('requestIdleCallback' in window)idle=window.requestIdleCallback(start,{timeout:1600});
    else timer=setTimeout(start,240);
  };
  const show=()=>requestAnimationFrame(()=>requestAnimationFrame(()=>preview.setAttribute('data-ready','')));
  const setExpanded=(next:boolean)=>{
    if(next===expanded)return;
    expanded=next;
    if(next){
      returnFocus=document.activeElement instanceof HTMLElement?document.activeElement:openButton;
      oldOverflow=document.documentElement.style.overflow;oldScroll=scrollY;
      document.documentElement.style.overflow='hidden';
      // Keep the same iframe in place: reparenting would reload its document.
      let node:HTMLElement=showcase;
      while(node.parentElement){
        for(const sibling of [...node.parentElement.children])if(sibling!==node&&sibling instanceof HTMLElement){inertSiblings.set(sibling,sibling.inert);sibling.inert=true;}
        node=node.parentElement;if(node===document.body)break;
      }
      showcase.setAttribute('role','dialog');showcase.setAttribute('aria-modal','true');showcase.setAttribute('aria-labelledby','preview-viewer-title');
      start();
    }else{
      inertSiblings.forEach((was,node)=>node.inert=was);inertSiblings.clear();
      document.documentElement.style.overflow=oldOverflow;
      showcase.removeAttribute('role');showcase.removeAttribute('aria-modal');showcase.removeAttribute('aria-labelledby');
    }
    shell.toggleAttribute('data-expanded',next);hero.toggleAttribute('data-viewer-open',next);
    document.body.toggleAttribute('data-product-viewing',next);
    backdrop.hidden=!next;controls.forEach(control=>control.hidden=!next);
    window.dispatchEvent(new CustomEvent('khor-viewer-change',{detail:{open:next}}));
    requestAnimationFrame(()=>{
      if(next){const max=Math.max(0,stage.offsetWidth-preview.clientWidth);preview.scrollLeft=compact.matches?Math.min(max,stage.offsetWidth/4):max/2;closeButton.focus({preventScroll:true});}
      else{preview.scrollLeft=0;scrollTo({top:oldScroll,behavior:'instant'});returnFocus?.focus({preventScroll:true});}
      measure();
    });
  };
  const frameStops=()=>[...frame.contentDocument?.querySelectorAll<HTMLElement>('a[href],button,input,textarea,select,[tabindex]')??[]].filter(el=>el.tabIndex>=0&&!el.matches(':disabled')&&el.getClientRects().length>0);
  const escape=(event:KeyboardEvent)=>{
    if(!expanded)return;
    if(event.key==='Escape'){event.preventDefault();setExpanded(false);return;}
    if(event.key!=='Tab')return;
    const inFrame=event.view===frame.contentWindow;
    const stops=inFrame?frameStops():[];
    if(inFrame&&event.shiftKey&&frame.contentDocument?.activeElement===stops[0]){event.preventDefault();closeButton.focus();}
    else if(inFrame&&!event.shiftKey&&frame.contentDocument?.activeElement===stops.at(-1)){event.preventDefault();(pan.disabled?closeButton:pan).focus();}
    else if(event.target===closeButton&&event.shiftKey){event.preventDefault();if(!pan.disabled)pan.focus();else frameStops().at(-1)?.focus();}
    else if(event.target===pan&&!event.shiftKey){event.preventDefault();closeButton.focus();}
  };
  openButton.addEventListener('click',()=>setExpanded(true),on);
  closeButton.addEventListener('click',()=>setExpanded(false),on);
  backdrop.addEventListener('click',()=>setExpanded(false),on);
  document.addEventListener('keydown',escape,on);
  pan.addEventListener('input',()=>preview.scrollLeft=(stage.offsetWidth-preview.clientWidth)*Number(pan.value)/100,on);
  preview.addEventListener('scroll',()=>{if(expanded)measure();},{passive:true,...on});
  window.addEventListener('message',event=>{
    if(event.origin!==location.origin||event.source!==frame.contentWindow)return;
    if(event.data?.type==='khor-demo-ready')show();
    if(event.data?.type==='khor-demo-download')location.assign(document.querySelector<HTMLAnchorElement>('[data-device-download]')?.href??'/download/');
  },on);
  frame.addEventListener('load',()=>{
    if(frame.contentDocument?.getElementById('root')?.childElementCount)show();
    frame.contentWindow?.addEventListener('keydown',escape,{capture:true,...on});
  },on);
  const visibility=new IntersectionObserver(entries=>{visible=entries[0].isIntersecting;if(visible)schedule();},{rootMargin:'100px'});
  visibility.observe(showcase);compact.addEventListener('change',schedule,on);
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    controller.abort();resize.disconnect();visibility.disconnect();
    if(idle&&'cancelIdleCallback' in window)window.cancelIdleCallback(idle);clearTimeout(timer);
  },{once:true});
}
