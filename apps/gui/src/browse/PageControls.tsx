/** Native page history and reload, shared by the main and session decks. */
import {useEffect,useState} from 'react';
import {pageStep,pageSteps,onPageWord,inApp} from '@/api';
import {listenNativeEvent} from '@/lib/native-events';
import type {PageNavigationState} from '@/gen/bindings/PageNavigationState';
import {GlassItem} from '@/components/Glass';
import {IconArrow,IconReload} from '@/components/icons';
import {gui} from '@/gen/catalog';
export type PageSteps={back:boolean;forward:boolean}|null;
const unavailable={back:false,forward:false};
/** Subscribe before the snapshot so initial attach and retained-page resume
 * cannot lose a history event. Late queries never overwrite newer events. */
export function usePageSteps(tab:string,enabled=true):PageSteps {
  const [current,setCurrent]=useState<{tab:string;steps:PageSteps}|null>(null);
  useEffect(()=>{
    if(!enabled)return;
    let alive=true,version=0,pending=0;
    const accept=(steps:PageSteps)=>setCurrent(was=>was?.tab===tab&&was.steps?.back===steps?.back&&was.steps?.forward===steps?.forward?was:{tab,steps});
    const refresh=()=>{
      const request=++version;++pending;
      void pageSteps(tab).then(steps=>{if(alive&&version===request)accept(steps);}).catch(()=>{}).finally(()=>{--pending;});
    };
    const drop=onPageWord('page:steps',tab,text=>{
      if(!/^[01]{2}$/.test(text))return;
      ++version;accept({back:text[0]==='1',forward:text[1]==='1'});
    },refresh);
    const load=inApp?listenNativeEvent<PageNavigationState>('page:navigation',state=>{if(state.tab===tab&&(state.phase==='finished'||state.phase==='settled'))refresh();}):()=>{};
    const went=onPageWord('page:went',tab,refresh);
    const visible=()=>{if(document.visibilityState==='visible')refresh();};
    document.addEventListener('visibilitychange',visible);
    // Same-document SPA history may not produce a load event. Query only
    // the actually presented native page; never pile up requests, poll
    // hidden tabs, or redraw unchanged history. This matches iOS's cadence.
    const timer=inApp?setInterval(()=>{
      if(!pending&&document.visibilityState!=='hidden'&&document.querySelector(`[data-native-page-tab="${CSS.escape(tab)}"][data-native-page-visible="true"]`))refresh();
    },400):undefined;
    return()=>{alive=false;++version;clearInterval(timer);drop();load();went();document.removeEventListener('visibilitychange',visible);};
  },[tab,enabled]);
  return enabled&&current?.tab===tab?current.steps:unavailable;
}
const step=(tab:string,way:'back'|'forward'|'reload')=>{void pageStep(tab,way).catch(()=>{});};
export function PageHistory({tab,steps}:{tab:string;steps:PageSteps}) {
  return <span data-deck-history className="flex items-center">
    <GlassItem aria-label={gui.page_back} disabled={!steps?.back} aria-disabled={!steps?.back||undefined} data-page-step="back" className="aria-disabled:pointer-events-none aria-disabled:text-tertiary" onClick={()=>step(tab,'back')}><IconArrow to="left"/></GlassItem>
    {steps?.forward===true&&<GlassItem aria-label={gui.page_forward} data-page-step="forward" onClick={()=>step(tab,'forward')}><IconArrow to="right"/></GlassItem>}
  </span>;
}
export function PageReload({tab,busy=false}:{tab:string;busy?:boolean}) {
  return <GlassItem aria-label={gui.page_reload} aria-busy={busy||undefined} data-page-step="reload" onClick={()=>step(tab,'reload')}><IconReload/></GlassItem>;
}
