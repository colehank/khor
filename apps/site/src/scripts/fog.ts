import {sitePath} from '../paths';
import {fogOptions} from './fog-options';

// Runtime fields of the vendored Vanta Fog 0.5.24 build.
type FogEffect={
  destroy:()=>void;
  animationLoop:()=>void;
  isOnScreen:()=>boolean;
  afterRender?:()=>void;
  req:number;
  prevNow:number;
  width:number;
  height:number;
  scale:number;
  renderer?:{domElement:HTMLCanvasElement;setPixelRatio:(ratio:number)=>void;dispose:()=>void};
  uniforms:{iResolution:{value:{x:number;y:number;set:(width:number,height:number)=>void}}};
};
type FogWindow=Window&{VANTA?:{FOG:(options:typeof fogOptions&{el:HTMLElement})=>FogEffect}};
const load=(src:string)=>new Promise<void>((resolve,reject)=>{
  const script=document.createElement('script');script.src=src;script.async=true;
  script.onload=()=>resolve();script.onerror=()=>reject(new Error('Could not load background'));
  document.head.append(script);
});

export function mountFog(){
  const hero=document.querySelector<HTMLElement>('[data-product-hero]');
  const ground=document.querySelector<HTMLElement>('[data-fog]');
  const header=document.querySelector<HTMLElement>('.story-nav');
  if(!hero||!ground)return;
  const controller=new AbortController(),on={signal:controller.signal};
  let effect:FogEffect|undefined,disposed=false,visible=false,started=false,running=false,contextLost=false;
  let heroBottom=0,navCutoff=0;
  const nav=()=>{
    header?.toggleAttribute('data-scrolled',scrollY>0);
    header?.toggleAttribute('data-over-hero',heroBottom-scrollY>navCutoff);
  };
  const measureNav=()=>{
    heroBottom=hero.getBoundingClientRect().bottom+scrollY;
    navCutoff=Math.max(header?.offsetHeight??0,innerHeight*.25);
    nav();
  };
  const resize=new ResizeObserver(measureNav);resize.observe(hero);
  window.addEventListener('resize',measureNav,on);
  window.addEventListener('scroll',nav,{passive:true,...on});measureNav();

  const sync=()=>{
    const shouldRun=visible&&!document.hidden&&!disposed&&!contextLost;
    if(!effect||running===shouldRun)return;
    running=shouldRun;
    if(running){effect.prevNow=0;effect.animationLoop();}
    else cancelAnimationFrame(effect.req);
  };
  const start=async()=>{
    if(started||!visible||document.hidden)return;
    started=true;
    try{
      await load(sitePath('/vendor/three-r134.min.js'));
      if(disposed)return;
      await load(sitePath('/vendor/vanta-fog-0.5.24.min.js'));
      if(disposed)return;
      const fog=(window as FogWindow).VANTA!.FOG({...fogOptions,el:ground});
      const renderer=fog.renderer,canvas=renderer?.domElement;
      if(!renderer||!canvas?.isConnected){renderer?.dispose();return;}
      effect=fog;
      cancelAnimationFrame(fog.req);
      const setPixelRatio=renderer.setPixelRatio.bind(renderer);
      // Bound fragment work on high-DPI monitors. Keep shader coordinates in
      // the original preset's space, independent of the backing resolution.
      const coarse=matchMedia('(any-pointer:coarse)');
      renderer.setPixelRatio=requested=>{
        const pixelLimit=coarse.matches?240_000:420_000;
        setPixelRatio(Math.min(requested,Math.sqrt(pixelLimit/(fog.width*fog.height))));
      };
      renderer.setPixelRatio(devicePixelRatio/fog.scale);
      let nextDraw=0;
      fog.isOnScreen=()=>{
        const now=performance.now();
        if(now+2<nextDraw)return false;
        // Retain the shader time and speed while drawing soft Fog at 30 Hz.
        nextDraw=Math.max(now,nextDraw)+1000/30;
        const resolution=fog.uniforms.iResolution.value;
        const width=canvas.width/devicePixelRatio,height=canvas.height/devicePixelRatio;
        if(resolution.x!==width||resolution.y!==height)resolution.set(width,height);
        return true;
      };
      fog.afterRender=()=>{if(!ground.hasAttribute('data-fog-ready'))ground.dataset.fogReady='';};
      canvas.addEventListener('webglcontextlost',event=>{
        event.preventDefault();contextLost=true;delete ground.dataset.fogReady;sync();
      },on);
      canvas.addEventListener('webglcontextrestored',()=>{
        contextLost=false;sync();
      },on);
      sync();
    }catch{/* The captured frame of this same preset remains visible. */}
  };
  const update=()=>{sync();void start();};
  const observer=new IntersectionObserver(([entry])=>{visible=entry.isIntersecting;update();});
  observer.observe(ground);document.addEventListener('visibilitychange',update,on);
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    disposed=true;controller.abort();resize.disconnect();observer.disconnect();
    const renderer=effect?.renderer;
    effect?.destroy();
    // The vendor releases the canvas but delegates renderer disposal to its host.
    renderer?.dispose();
  },on);
}
