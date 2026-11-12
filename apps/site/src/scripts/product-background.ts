// A transparent iframe cannot sample its parent's backdrop. Give the real
// glass controls the same Fog underneath them, before the window's tint.
// Reuse the existing canvas; never create a second renderer or read pixels.
export function mountPreviewBackground(frame:HTMLIFrameElement,hero:HTMLElement) {
  const doc=frame.contentDocument!,ground=hero.querySelector<HTMLElement>('[data-fog]')!;
  const poster=ground.querySelector<HTMLImageElement>('.fog-fallback')!;
  const layer=doc.createElement('div'),surface=doc.createElement('div');
  const image=doc.createElement('img'),video=doc.createElement('video'),shade=doc.createElement('div');
  layer.className='site-fog-backdrop';layer.setAttribute('aria-hidden','true');
  surface.className='site-fog-surface';shade.className='site-fog-shade';
  image.src=poster.currentSrc||poster.src;image.alt='';
  video.muted=true;video.playsInline=true;video.disablePictureInPicture=true;
  surface.append(image,video,shade);layer.append(surface);doc.body.prepend(layer);
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const controller=new AbortController(),on={signal:controller.signal};
  let visible=false,source:HTMLCanvasElement|null=null,stream:MediaStream|undefined;
  const stop=()=>{
    stream?.getTracks().forEach(track=>track.stop());stream=undefined;source=null;
    video.pause();video.srcObject=null;delete layer.dataset.live;
  };
  const sync=()=>{
    const canvas=ground.querySelector('canvas');
    if(!visible||document.hidden||reduced.matches||!canvas?.captureStream){stop();return;}
    if(source===canvas)return;
    stop();source=canvas;
    try{
      const current=canvas.captureStream(15);stream=current;video.srcObject=current;
      void video.play().then(()=>{if(stream===current)layer.dataset.live='';}).catch(()=>{if(stream===current)stop();});
    }catch{stop();}
  };
  const measure=()=>{
    const rect=frame.getBoundingClientRect(),world=ground.getBoundingClientRect();
    const scale=rect.width/frame.offsetWidth;if(!scale)return;
    Object.assign(surface.style,{width:`${world.width/scale}px`,height:`${world.height/scale}px`,left:`${(world.left-rect.left)/scale}px`,top:`${(world.top-rect.top)/scale}px`,filter:`blur(${parseFloat(getComputedStyle(hero).getPropertyValue('--gap-pane'))/scale}px)`});
    const fade=getComputedStyle(hero,'::after');
    shade.style.height=`${parseFloat(fade.height)/scale}px`;shade.style.backgroundImage=fade.backgroundImage;
  };
  const visibility=new IntersectionObserver(entries=>{visible=entries[0].isIntersecting;sync();});visibility.observe(frame);
  const canvasChanges=new MutationObserver(sync);canvasChanges.observe(ground,{childList:true});
  reduced.addEventListener('change',sync,on);document.addEventListener('visibilitychange',sync,on);
  measure();
  return {ready:image.decode().catch(()=>{}),measure,dispose:()=>{stop();controller.abort();visibility.disconnect();canvasChanges.disconnect();layer.remove();}};
}
