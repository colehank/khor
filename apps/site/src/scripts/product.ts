export function mountDesktopPreview() {
  const preview=document.querySelector<HTMLElement>('[data-desktop-preview]');
  const stage=document.querySelector<HTMLElement>('[data-desktop-stage]');
  const frame=document.querySelector<HTMLIFrameElement>('[data-desktop-demo]');
  if(!preview||!stage||!frame)return;
  // Preserve the complete desktop shell instead of switching to mobile.
  const resize=new ResizeObserver(()=>stage.style.setProperty('--desktop-scale',String(preview.clientWidth/stage.offsetWidth)));
  resize.observe(preview);
  const url=new URL(frame.dataset.src!,location.origin);
  url.searchParams.set('theme',matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light');
  frame.src=url.href;
  window.addEventListener('message',event=>{
    if(event.origin!==location.origin||event.source!==frame.contentWindow)return;
    if(event.data?.type==='khor-demo-ready')preview.dataset.ready='';
    if(event.data?.type==='khor-demo-download')location.assign(document.querySelector<HTMLAnchorElement>('[data-device-download]')?.href??'/download/');
  });
  window.addEventListener('pagehide',event=>{if(!event.persisted)resize.disconnect();},{once:true});
}
