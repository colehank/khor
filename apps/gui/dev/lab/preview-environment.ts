import {paintWindowChoice} from '../../src/lib/window-surface';
import {holdFoot} from '../../src/lib/viewport';
import {wearTheme} from '../../src/theme';
import {setConfig} from '../../src/api';
import {readPreview,type PreviewState} from './preview-state';
import './preview-environment.css';

export function installPreviewEnvironment(initial: unknown, runtime: {tree:string;revision:string;mode:string}) {
  if(runtime.mode!=='frontend')throw new Error('Browser environments require frontend Lab');
  const root=document.documentElement;
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
    if(event.origin!==location.origin||event.source!==parent||parent===window||event.data?.type!=='khor.lab.environment')return;
    try{apply(readPreview(event.data.value));}catch(error){console.error('Lab environment rejected',String(error));}
  };
  addEventListener('message',receive);
  parent.postMessage({type:'khor.lab.ready',tree:runtime.tree,revision:runtime.revision,pointer:matchMedia('(pointer: coarse)').matches?'touch':'mouse'},location.origin);
  addEventListener('pagehide',()=>{removeEventListener('message',receive);background.remove();},{once:true});
}
