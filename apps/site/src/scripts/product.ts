export function mountDesktopPreview() {
  const preview=document.querySelector<HTMLElement>('[data-desktop-preview]');
  const stage=document.querySelector<HTMLElement>('[data-desktop-stage]');
  const frame=document.querySelector<HTMLIFrameElement>('[data-desktop-demo]');
  const shell=preview?.closest<HTMLElement>('.macos-window');
  if(!preview||!stage||!frame||!shell)return;
  // Preserve the complete desktop shell instead of switching to mobile.
  // The containing window's corner shares the same scale as its controls.
  const measure=()=>shell.style.setProperty('--desktop-scale',String(preview.clientWidth/stage.offsetWidth));
  const resize=new ResizeObserver(measure);
  measure();
  resize.observe(preview);
  const show=()=>{
    // WebKit can report the intended innerWidth while actually laying the
    // document out at the zoomed width. Check its real layout before
    // replacing the full-size poster with an interactive frame.
    const layoutWidth=frame.contentDocument?.documentElement.clientWidth;
    if(layoutWidth&&Math.abs(layoutWidth-stage.offsetWidth)>2)stage.dataset.scaleMode='transform';
    requestAnimationFrame(()=>requestAnimationFrame(()=>preview.setAttribute('data-ready','')));
  };
  window.addEventListener('message',event=>{
    if(event.origin!==location.origin||event.source!==frame.contentWindow)return;
    if(event.data?.type==='khor-demo-ready')show();
    if(event.data?.type==='khor-demo-download')location.assign(document.querySelector<HTMLAnchorElement>('[data-device-download]')?.href??'/download/');
  });
  // The same-origin DOM is also a readiness receipt after a cached load.
  frame.addEventListener('load',()=>{
    if(frame.contentDocument?.getElementById('root')?.childElementCount)show();
  });
  frame.src=new URL(frame.dataset.src!,location.origin).href;
  window.addEventListener('pagehide',event=>{if(!event.persisted)resize.disconnect();},{once:true});
}
