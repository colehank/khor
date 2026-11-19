import {sitePath} from '../paths';
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
  let effect:FogEffect|undefined,disposed=false;
  const nav=()=>{
    if(!header)return;
    header.toggleAttribute('data-scrolled',window.scrollY>0);
    header.toggleAttribute('data-over-hero',hero.getBoundingClientRect().bottom>Math.max(header.offsetHeight,innerHeight*.25));
  };
  window.addEventListener('scroll',nav,{passive:true});nav();
  const start=async()=>{
    try{
      await load(sitePath('/vendor/three-r134.min.js'));
      if(disposed)return;
      await load(sitePath('/vendor/vanta-fog-0.5.24.min.js'));
      if(disposed)return;
      effect=(window as FogWindow).VANTA!.FOG({...fogOptions,el:ground});
      if(ground.querySelector('canvas'))ground.dataset.fogReady='';
    }catch{/* The captured frame of this same preset remains visible. */}
  };
  void start();
  window.addEventListener('pagehide',event=>{
    if(event.persisted)return;
    disposed=true;effect?.destroy();window.removeEventListener('scroll',nav);
  },{once:true});
}
