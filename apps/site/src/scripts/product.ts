import {mountPreviewBackground} from './product-background';
import {sitePath} from '../paths';
import {renderProbe,disablePreviewGlass} from './render-probe';

export function mountDesktopPreview() {
  const preview=document.querySelector<HTMLElement>('[data-desktop-preview]');
  const stage=document.querySelector<HTMLElement>('[data-desktop-stage]');
  const frame=document.querySelector<HTMLIFrameElement>('[data-desktop-demo]');
  const shell=preview?.closest<HTMLElement>('.macos-window');
  const hero=preview?.closest<HTMLElement>('[data-product-hero]');
  if(!preview||!stage||!frame||!shell||!hero)return;
  const probe=renderProbe();
  if(probe==='no-glass')disablePreviewGlass(document);
  const controller=new AbortController(),on={signal:controller.signal};
  let started=false,visible=false,idle=0,timer=0;
  let background:ReturnType<typeof mountPreviewBackground>|undefined;
  const measure=()=>{shell.style.setProperty('--desktop-scale',String(preview.clientWidth/stage.offsetWidth));background?.measure();};
  const resize=new ResizeObserver(measure);resize.observe(preview);resize.observe(hero);measure();
  const start=()=>{
    idle=0;timer=0;
    if(started||!visible)return;
    started=true;frame.src=new URL(frame.dataset.src!,location.origin).href;
  };
  const schedule=()=>{
    if(probe==='poster'||started||idle||timer||!visible)return;
    // Paint the real poster first, then activate the inline GUI on any device.
    if('requestIdleCallback' in window)idle=window.requestIdleCallback(start,{timeout:1600});
    else timer=setTimeout(start,240);
  };
  const show=async()=>{
    if(probe==='no-glass')disablePreviewGlass(frame.contentDocument!);
    if(probe!=='no-backdrop'){
      background??=mountPreviewBackground(frame,hero);
      await background.ready;
    }
    requestAnimationFrame(()=>requestAnimationFrame(()=>preview.setAttribute('data-ready','')));
  };
  window.addEventListener('message',event=>{
    if(event.origin!==location.origin||event.source!==frame.contentWindow)return;
    if(event.data?.type==='khor-demo-ready')show();
    if(event.data?.type==='khor-demo-download')location.assign(document.querySelector<HTMLAnchorElement>('[data-device-download]')?.href??sitePath('/download/'));
  },on);
  frame.addEventListener('load',()=>{
    if(frame.contentDocument?.getElementById('root')?.childElementCount)show();
  },on);
  const visibility=new IntersectionObserver(entries=>{visible=entries[0].isIntersecting;if(visible)schedule();},{rootMargin:'100px'});
  visibility.observe(preview);
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    controller.abort();resize.disconnect();visibility.disconnect();background?.dispose();
    if(idle&&'cancelIdleCallback' in window)window.cancelIdleCallback(idle);clearTimeout(timer);
  },{once:true});
}
