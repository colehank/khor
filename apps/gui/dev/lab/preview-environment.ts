import {paintWindowChoice} from '../../src/lib/window-surface';
import {holdFoot} from '../../src/lib/viewport';
import {wearTheme} from '../../src/theme';
import {setConfig} from '../../src/api';
import {readPreview,type PreviewState} from './preview-state';
import './preview-environment.css';

export function installPreviewEnvironment(initial: unknown, runtime: {tree:string;revision:string;mode:string;mutable?:boolean}) {
  if(runtime.mode!=='frontend')throw new Error('Browser environments require frontend Lab');
  const root=document.documentElement;
  // Local hot development embeds the real Lab Vite origin rather than
  // re-compiling its modules through Storybook's different Vite config.
  // Immutable review stays same-origin. Only a mutable loopback Lab may
  // accept its actual loopback parent as a controller.
  let controllerOrigin=location.origin;
  if(runtime.mutable&&parent!==window&&document.referrer){
    const controller=new URL(document.referrer);
    const loopback=(url:URL)=>['127.0.0.1','localhost','[::1]'].includes(url.hostname)&&url.protocol==='http:';
    if(loopback(controller)&&loopback(new URL(location.href)))controllerOrigin=controller.origin;
  }
  const background=document.createElement('div');background.dataset.labEnvironmentBackground='';background.setAttribute('aria-hidden','true');document.body.prepend(background);
  let theme: PreviewState['theme']|undefined;
  const apply=(state:PreviewState)=>{
    wearTheme(state.theme);
    if(theme!==state.theme){theme=state.theme;void setConfig('theme',state.theme).catch(error=>console.error('Lab theme update failed',String(error)));}
    const mobile=state.platform!=='desktop';
    paintWindowChoice({material:mobile?'solid':state.material,opacity:mobile?100:state.opacity,outline:!mobile},state.platform==='desktop'?'macos':state.platform);
    root.toggleAttribute('data-ios',state.platform==='ios');
    root.style.setProperty('--safe-top',state.safeTop+'px');
    root.style.setProperty('--safe-bottom',(state.keyboard?0:state.safeBottom)+'px');
    holdFoot(mobile&&state.keyboard?state.keyboard:null);
    background.dataset.background=state.background;
    background.dataset.frost=String(!mobile&&state.material==='frost');
    root.dataset.labEnvironment=JSON.stringify(state);
  };
  apply(readPreview(initial));
  const receive=(event:MessageEvent)=>{
    if(event.origin!==controllerOrigin||event.source!==parent||parent===window||event.data?.type!=='khor.lab.environment')return;
    try{apply(readPreview(event.data.value));}catch(error){console.error('Lab environment rejected',String(error));}
  };
  addEventListener('message',receive);
  let panel=false;
  const panels=new MutationObserver(()=>{const next=!!document.querySelector('[data-attachment-picker]');if(next===panel)return;panel=next;parent.postMessage({type:'khor.lab.keyboard-panel',active:panel},controllerOrigin);});
  panels.observe(document.body,{childList:true,subtree:true});
  parent.postMessage({type:'khor.lab.ready',tree:runtime.tree,revision:runtime.revision,pointer:matchMedia('(pointer: coarse)').matches?'touch':'mouse'},controllerOrigin);
  addEventListener('pagehide',()=>{removeEventListener('message',receive);background.remove();panels.disconnect();},{once:true});
}
