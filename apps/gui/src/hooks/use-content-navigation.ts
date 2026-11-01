import { readCommitAddress } from '@/lib/content-commit';
/** The last viewing position belongs to the viewer, independently of the
 * node's open document collection. Wide and narrow layouts use this state. */
import { useEffect, useRef, useState } from 'react';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
import { directoryOf } from '@/lib/paths';
export type ContentSource = 'workspace' | 'browser';
export type ContentScene = 'closed' | 'picker' | 'document' | 'overview' | 'sessions';
export type ContentScroll = { top: number; left: number };
export type ContentPlace = {
  scene: ContentScene; key: string | null; source: ContentSource;
  scope: { device: string; root: string } | null; filter: string; query: string; address: string; exit: string | null;
};
export const contentPickerPosition=(key:string|null,p:ContentPlace)=>JSON.stringify(['picker',key,p.source,p.scope,p.query]);
type ContentCheckpoint = { place: ContentPlace; trail: ContentPlace[] };
export type ContentNavigationState = ContentPlace & {
  from: ContentScene; trail: ContentPlace[]; recent: string | null; documentTrail: ContentPlace[];
  scrolls: Record<string, ContentScroll>;
  browseBefore: string | null;
  /** The pill resumes content; tab/session overviews are separate routes. */
  lastContent: ContentCheckpoint | null;
  tabPlaces: Record<string, ContentCheckpoint>;
};
export const emptyContentPlace = (): ContentPlace => ({scene:'picker',key:null,source:'workspace',scope:null,filter:'',query:'',address:'',exit:null});
const empty = (): ContentNavigationState => ({...emptyContentPlace(),from:'picker',trail:[],recent:null,documentTrail:[],scrolls:{},browseBefore:null,lastContent:null,tabPlaces:{}});
const place = ({scene,key,source,scope,filter,query,address,exit}: ContentPlace): ContentPlace => ({scene,key,source,scope,filter,query,address,exit});
const NAV_KEY = 'khor.content.navigation.v1', EDITOR_KEY = 'khor.content.editors.v1';
const scenes: ContentScene[] = ['closed','picker','document','overview','sessions'];
const isPlace = (value: unknown): value is ContentPlace => {
  if (!value || typeof value !== 'object') return false;
  const p = value as ContentPlace;
  return scenes.includes(p.scene) && (p.source === 'workspace' || p.source === 'browser') &&
    (p.key === null || typeof p.key === 'string') && ['filter','query','address'].every(k => typeof p[k as keyof ContentPlace] === 'string') &&
    (p.exit === null || typeof p.exit === 'string') && (p.scope === null || typeof p.scope?.device === 'string' && typeof p.scope?.root === 'string');
};
const isContent = (p: ContentPlace) => p.scene === 'picker' || p.scene === 'document';
const checkpoint = (s: ContentNavigationState): ContentCheckpoint => ({place:place(s),trail:s.trail});
function readCheckpoint(s: ContentNavigationState, trail: ContentPlace[]): ContentCheckpoint {
  if (isContent(s)) return {place:place(s),trail};
  const kept=s.lastContent;
  if (kept && isPlace(kept.place) && isContent(kept.place)) {
    return {place:place(kept.place),trail:Array.isArray(kept.trail)?kept.trail.filter(isPlace).slice(-64):[]};
  }
  // Migrate old selectors without restoring them as content. Old sessions()
  // discarded the trail, so retain the known source/directory/drafts there.
  for(let i=trail.length-1;i>=0;i--)if(isContent(trail[i]))return {place:trail[i],trail:trail.slice(0,i)};
  return {place:{...place(s),scene:'picker',key:null,filter:''},trail:[]};
}
const read = (key: string): Record<string, unknown> => {
  try { const value = JSON.parse(localStorage.getItem(key) || '{}'); return value && typeof value === 'object' && !Array.isArray(value) ? value : {}; } catch { return {}; }
};
const save = (key: string, value: unknown) => { try { localStorage.setItem(key, JSON.stringify(value)); } catch { /* viewer storage is optional */ } };
function readNavigation(): Record<string, ContentNavigationState> {
  return Object.fromEntries(Object.entries(read(NAV_KEY)).filter(([,s]) => isPlace(s)).map(([session,value]) => {
    const s = value as ContentNavigationState;
    const trail=Array.isArray(s.trail)?s.trail.filter(isPlace).slice(-64):[];
    const scrolls = Object.fromEntries(Object.entries(s.scrolls || {}).filter(([,v]) => v && Number.isFinite(v.top) && Number.isFinite(v.left)));
    return [session,{...empty(),...place(s),from:scenes.includes(s.from)?s.from:'picker',recent:typeof s.recent==='string'?s.recent:null,
      browseBefore:typeof s.browseBefore==='string'?s.browseBefore:null,
      lastContent:readCheckpoint(s,trail),tabPlaces:Object.fromEntries(Object.entries(s.tabPlaces||{}).filter(([,v])=>v&&isPlace(v.place)&&isContent(v.place)).map(([key,v])=>[key,{place:place(v.place),trail:Array.isArray(v.trail)?v.trail.filter(isPlace).slice(-64):[]}])),trail,documentTrail:Array.isArray(s.documentTrail)?s.documentTrail.filter(isPlace).slice(-64):[],scrolls}];
  }));
}
export function enterContent(was: ContentNavigationState, patch: Partial<ContentPlace>): ContentNavigationState {
  const next = {...was,...patch};
  if (was.scene !== 'closed' && (next.scene !== was.scene || next.scene === 'document' && next.key !== was.key)) {
    next.trail = [...was.trail,place(was)].slice(-64); next.from = was.scene;
  }
  if (next.scene === 'document') { next.recent=next.key; next.documentTrail=next.trail; }
  return next;
}
/** Draft text persists; editing/focus and pending machine prompts never do. */
export type ContentEditor = {address?:string;editing?:boolean;exitPrompt?:number};
export type ContentNavigation = {
 editor:(key:string)=>ContentEditor;edit:(key:string,patch:ContentEditor)=>void;
 get:(session:string)=>ContentNavigationState;has:(session:string)=>boolean;
 update:(session:string,patch:Partial<ContentPlace>)=>void;document:(tab:ContentTab|null)=>void;select:(tab:ContentTab)=>void;
 resume:(session:string)=>void;browse:(session:string,scope?:{device:string;root:string})=>void;
 add:(session:string,scope?:{device:string;root:string})=>ContentTab;
 returnToPicker:(session:string)=>void;
 overview:(session:string)=>void;back:(session:string)=>boolean;dismiss:(session:string)=>void;closePicker:(session:string)=>boolean;
 sessions:(session:string)=>void;inspect:(session:string)=>void;
 scroll:(session:string,key:string,value:ContentScroll)=>void;scrollOf:(session:string,key:string)=>ContentScroll|undefined;
 browseBefore:(session:string,key:string|null)=>void;forTab:(session:string,key:string)=>ContentNavigation;
};
export function useContentNavigation(content: SessionContent): ContentNavigation {
  const [editors,setEditors] = useState<Record<string,ContentEditor>>(() => Object.fromEntries(Object.entries(read(EDITOR_KEY)).filter(([,v]) => v && typeof v === 'object' && typeof (v as ContentEditor).address === 'string').map(([key,v]) => [key,{address:(v as ContentEditor).address}])));
  const currentEditors=useRef(editors); currentEditors.current=editors;
  const editor=(key:string)=>currentEditors.current[key]??{};
  const edit=(key:string,patch:ContentEditor)=>{
    const next={...currentEditors.current,[key]:{...editor(key),...patch}};
    currentEditors.current=next;setEditors(next);
    save(EDITOR_KEY,Object.fromEntries(Object.entries(next).filter(([,v])=>v.address!==undefined).map(([key,v])=>[key,{address:v.address}])));
  };
  const [states,setStates] = useState<Record<string,ContentNavigationState>>(readNavigation);
  const pendingResume=useRef(new Set<string>()), current=useRef(states); current.current=states;
  const get=(session:string)=>current.current[session]??empty();
  const write=(session:string,run:(s:ContentNavigationState)=>ContentNavigationState)=>{
    const before=get(session),after=run(before);if(after===before&&current.current[session])return;
    const lastContent=isContent(after)?checkpoint(after):current.current[session]&&isContent(before)?checkpoint(before):after.lastContent;
    const tabPlaces={...after.tabPlaces};
    if(isContent(before)&&before.key)tabPlaces[before.key]=checkpoint(before);
    if(isContent(after)&&after.key)tabPlaces[after.key]=checkpoint(after);
    const next={...current.current,[session]:{...after,lastContent,tabPlaces}};current.current=next;setStates(next);save(NAV_KEY,next);
  };
  const update=(session:string,patch:Partial<ContentPlace>)=>{pendingResume.current.delete(session);write(session,s=>enterContent(s,patch));};
  const valid=(session:string,key:string|null)=>content.allTabs.some(t=>t.session===session&&content.handle(t)===key);
  const tabState=(session:string,key:string):ContentNavigationState=>{
    const s=get(session);if(s.key===key&&isContent(s))return s;
    const kept=s.tabPlaces[key],tab=content.allTabs.find(t=>t.session===session&&content.handle(t)===key);
    const start={...empty(),...place(s),key,scene:tab?.view==='workspace'?'picker' as const:'document' as const,filter:'',trail:[]};
    if(tab?.view==='workspace'&&tab.url)Object.assign(start,{source:'workspace',scope:{device:tab.device,root:tab.url}});
    return kept?{...start,...kept.place,key,trail:kept.trail}:start;
  };
  const select=(tab:ContentTab)=>{
    pendingResume.current.delete(tab.session);
    const key=content.handle(tab);let target=tabState(tab.session,key);
    // Browsing the source does not turn an existing file/page back into a
    // starting tab. Explicit tab selection returns to its document, keeping
    // the picker in the Back trail and the viewer's latest browse checkpoint.
    if((tab.view||'workspace')!=='workspace'&&target.scene==='picker')
      target=enterContent(target,{scene:'document',source:tab.view==='page'?'browser':target.source});
    write(tab.session,s=>({...s,...place(target),trail:target.trail,from:target.from,recent:key,documentTrail:target.documentTrail}));
  };
  const document=(tab:ContentTab|null)=>{
    if(!tab)return;
    if((tab.view||'workspace')==='workspace'){select(tab);return;}
    pendingResume.current.delete(tab.session);
    const key=content.handle(tab),target=tabState(tab.session,key),before=get(tab.session);
    // A new/revisited tab opened from a source list returns to that exact
    // directory/history location. The source route now belongs to its key.
    const start=before.scene==='picker'&&before.key!==key
      ? {...target,...place(before),key,trail:before.trail}:target;
    const next=enterContent(start,{scene:'document',key,source:tab.view==='page'?'browser':start.source});
    const from=contentPickerPosition(before.key,before),to=contentPickerPosition(key,before);
    const scrolls=start===target?{}:Object.fromEntries(Object.entries(before.scrolls)
      .filter(([name])=>name.startsWith(from+' ')).map(([name,value])=>[to+name.slice(from.length),value]));
    write(tab.session,s=>({...s,...place(next),trail:next.trail,from:next.from,recent:key,documentTrail:next.trail,scrolls:{...s.scrolls,...scrolls}}));
  };
  const add=(session:string,scope?:{device:string;root:string})=>{
    pendingResume.current.delete(session);
    const before=get(session),tab=content.newTab(session,scope?.device||before.scope?.device||'');
    write(session,s=>({...s,...emptyContentPlace(),key:content.handle(tab),scope:scope||before.scope,source:scope?'workspace':before.source,exit:before.exit,trail:[],from:'picker',recent:content.handle(tab)}));
    return tab;
  };
  const resume=(session:string)=>{
    if(!content.ready){pendingResume.current.add(session);return;}
    pendingResume.current.delete(session);
    const remembered=current.current[session],kept=remembered?.lastContent;
    const usable=(p:ContentPlace)=>isContent(p)&&valid(session,p.key);
    if(remembered&&usable(remembered))return;
    let target=kept?.place,trail=[...(kept?.trail||remembered?.trail||[])];
    if(!target||!usable(target)){target=undefined;while(trail.length){const p=trail.pop()!;if(usable(p)){target=p;break;}}}
    if(target){
      write(session,s=>({...s,...target!,trail,from:trail.at(-1)?.scene||'picker'}));
      const tab=content.allTabs.find(t=>t.session===session&&content.handle(t)===target!.key);
      if(tab&&content.active(session)?.id!==tab.id)void content.focus(tab.id).catch(()=>{});
      return;
    }
    const tab=content.active(session);if(tab){select(tab);return;}
    // With no tab yet, restore the initial browsing location.
    if(remembered&&(isContent(remembered)?remembered.scene==='picker':kept?.place.scene==='picker')){
      const old=isContent(remembered)?place(remembered):kept!.place;
      add(session);update(session,{...old,key:get(session).key,scene:'picker'});return;
    }
    add(session);
  };
  useEffect(()=>{if(content.ready)for(const session of [...pendingResume.current])resume(session);},[content.ready]);
  const browse=(session:string,scope?:{device:string;root:string})=>{
    const s=get(session);let key=s.key;
    if(!valid(session,key)){const tab=content.active(session);if(tab){select(tab);key=content.handle(tab);}else{add(session,scope);return;}}
    update(session,{scene:'picker',key,...(scope?{scope,source:'workspace',query:''}:{})});
  };
  /** Outer Back replaces this tab's document with its starting page.
   * Browser history and selector Back keep their independent meanings. */
  const returnToPicker=(session:string):void=>{
    pendingResume.current.delete(session);
    const s=get(session);
    const tab=content.allTabs.find(t=>t.session===session&&content.handle(t)===s.key)||content.active(session);
    if(!tab)return;
    const key=content.handle(tab),owned=tabState(session,key),kept=s.tabPlaces[key];
    const source=[place(owned),...[...owned.trail].reverse(),...[...owned.documentTrail].reverse(),
      ...(kept?[kept.place,...[...kept.trail].reverse()]:[]),
      ...(s.lastContent?[s.lastContent.place,...[...s.lastContent.trail].reverse()]:[])]
      .find(p=>p.scene==='picker');
    const fallback:ContentPlace={...emptyContentPlace(),key,
      ...(tab.view==='page'?{source:'browser',exit:tab.device}:
        {scope:{device:tab.device,root:tab.view==='commit'?readCommitAddress(tab.url)?.root||'':tab.view==='file'?directoryOf(tab.url):tab.url}})};
    const picker:ContentPlace={...(source||fallback),scene:'picker',key};
    const device=picker.source==='workspace'?picker.scope?.device||tab.device:picker.exit||tab.device;
    const root=picker.source==='workspace'?picker.scope?.root||'':'';
    // point() publishes its optimistic target synchronously and serializes
    // persistence. A new choice in this same tick therefore fills this same
    // starting tab, and its write cannot be overtaken by the reset.
    void content.point(tab.id,device,root,'workspace','').catch(()=>{});
    const from=contentPickerPosition(source?source.key:key,source||fallback),to=contentPickerPosition(key,picker);
    write(session,s=>{
      const tabPlaces={...s.tabPlaces};delete tabPlaces[key];delete tabPlaces[tab.id];
      const scrolls={...s.scrolls,...Object.fromEntries(Object.entries(s.scrolls)
        .filter(([name])=>name.startsWith(from+' ')).map(([name,value])=>[to+name.slice(from.length),value]))};
      return {...s,...picker,tabPlaces,scrolls,trail:[],documentTrail:[],from:'picker',recent:key,browseBefore:null};
    });
    if(editor(key).editing||editor(key).exitPrompt!==undefined)edit(key,{editing:false,exitPrompt:undefined});
  };
  const back=(session:string):boolean=>{
    pendingResume.current.delete(session);
    const s=get(session),trail=[...s.trail];let prev:ContentPlace|undefined;
    const active=content.active(session),activeKey=active?content.handle(active):null;
    const owner=valid(session,s.key)?s.key:valid(session,activeKey)?activeKey:null;
    let scrolls=s.scrolls;
    while(trail.length){
      const entry=trail.pop()!;
      if(entry.scene==='closed'||entry.scene==='document'&&!valid(session,entry.key))continue;
      if(entry.scene==='picker'&&!valid(session,entry.key)){
        // Old singleton source routes had no tab key. The current host
        // renders only real tabs, so returning to that route must keep
        // the current tab's identity instead of opening an empty sheet.
        if(!owner)continue;
        prev={...entry,key:owner};
        const from=contentPickerPosition(entry.key,entry),to=contentPickerPosition(owner,entry);
        scrolls={...scrolls,...Object.fromEntries(Object.entries(scrolls)
          .filter(([name])=>name.startsWith(from+' ')).map(([name,value])=>[to+name.slice(from.length),value]))};
      }else prev=entry;
      break;
    }
    if(!prev)return false;
    const next=prev;write(session,s=>({...s,...next,scrolls,trail,from:trail.at(-1)?.scene||'picker',...(next.scene==='document'?{recent:next.key,documentTrail:trail}:{})}));
    const tab=next.scene==='document'?content.allTabs.find(t=>t.session===session&&content.handle(t)===next.key):null;
    if(tab)void content.focus(tab.id).catch(()=>{});
    return true;
  };
  const dismiss=(session:string)=>{
    pendingResume.current.delete(session);
    write(session,s=>({...s,scene:'closed',key:null,trail:[],from:'picker'}));
  };
  /** Closing a starting page closes that actual tab, selecting its neighbour. */
  const closePicker=(session:string):boolean=>{
    const peers=content.allTabs.filter(t=>t.session===session),key=get(session).key;
    const at=peers.findIndex(t=>content.handle(t)===key),tab=peers[at];
    if(!tab){dismiss(session);return false;}
    if(tab.pinned)return true;
    const next=peers.filter(t=>t.id!==tab.id)[Math.min(at,peers.length-2)];
    void content.close(tab.id).catch(()=>{});
    if(next){select(next);return true;}dismiss(session);return false;
  };
  const overview=(session:string)=>get(session).scene==='overview'?back(session):update(session,{scene:'overview',filter:''});
  const sessions=(session:string)=>{pendingResume.current.delete(session);write(session,s=>({...s,scene:'sessions',filter:'',trail:[],from:'picker'}));};
  const inspect=(session:string)=>{pendingResume.current.delete(session);write(session,s=>({...s,scene:'overview',filter:'',trail:[{...place(s),scene:'sessions'}],from:'sessions'}));};
  const browseBefore=(session:string,key:string|null)=>write(session,s=>({...s,browseBefore:key}));
  const scrollOf=(session:string,key:string)=>get(session).scrolls[key];
  const scroll=(session:string,key:string,value:ContentScroll)=>write(session,s=>{
    const before=s.scrolls[key];if(before?.top===value.top&&before?.left===value.left)return s;
    return {...s,scrolls:{...s.scrolls,[key]:{top:Math.max(0,value.top),left:Math.max(0,value.left)}}};
  });
  const forTab=(session:string,key:string):ContentNavigation=>({...api,
    get:id=>id===session?tabState(session,key):get(id),
    update:(id,patch)=>{
      if(id!==session||get(session).key===key&&isContent(get(session))){update(id,patch);return;}
      write(session,s=>{const next=enterContent(tabState(session,key),patch);return {...s,tabPlaces:{...s.tabPlaces,[key]:checkpoint(next)}};});
    },
  });
  const api:ContentNavigation={editor,edit,get,has:(session:string)=>!!current.current[session]||pendingResume.current.has(session),update,document,select,resume,browse,add,returnToPicker,overview,back,dismiss,closePicker,sessions,inspect,scroll,scrollOf,browseBefore,forTab};
  return api;
}
