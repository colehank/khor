import {fogOptions} from './fog-options';
type FogEffect={destroy:()=>void};
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
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  let effect:FogEffect|undefined,disposed=false,generation=0,libraries:Promise<void>|undefined;
  const nav=()=>header?.toggleAttribute('data-over-hero',hero.getBoundingClientRect().bottom>Math.max(header.offsetHeight,innerHeight*.25));
  window.addEventListener('scroll',nav,{passive:true});nav();
  const start=async()=>{
    const current=++generation;
    if(reduced.matches||disposed)return;
    try{
      libraries??=load('/vendor/three-r134.min.js').then(()=>load('/vendor/vanta-fog-0.5.24.min.js'));
      await libraries;
      if(disposed||reduced.matches||current!==generation)return;
      effect=(window as FogWindow).VANTA!.FOG({...fogOptions,el:ground});
      if(ground.querySelector('canvas'))ground.dataset.fogReady='';
    }catch{/* The captured frame of this same preset remains visible. */}
  };
  const change=()=>{effect?.destroy();effect=undefined;delete ground.dataset.fogReady;void start();};
  reduced.addEventListener('change',change);void start();
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    disposed=true;effect?.destroy();window.removeEventListener('scroll',nav);reduced.removeEventListener('change',change);
  },{once:true});
}
