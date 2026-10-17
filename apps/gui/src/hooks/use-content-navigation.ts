/** The last viewing position belongs to the viewer, independently of the
 * node's open document collection. Wide and narrow layouts use this state. */
import { useEffect, useRef, useState } from 'react';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
export type ContentSource = 'workspace' | 'browser';
export type ContentScene = 'picker' | 'document' | 'overview' | 'sessions';
export type ContentScroll = { top: number; left: number };
export type ContentPlace = {
  scene: ContentScene; key: string | null; source: ContentSource;
  scope: { device: string; root: string } | null; filter: string; query: string; address: string; exit: string | null;
};
export type ContentNavigationState = ContentPlace & {
  from: ContentScene; trail: ContentPlace[]; recent: string | null; documentTrail: ContentPlace[];
  scrolls: Record<string, ContentScroll>;
  browseBefore: string | null;
};
export const emptyContentPlace = (): ContentPlace => ({scene:'picker',key:null,source:'workspace',scope:null,filter:'',query:'',address:'',exit:null});
const empty = (): ContentNavigationState => ({...emptyContentPlace(),from:'picker',trail:[],recent:null,documentTrail:[],scrolls:{},browseBefore:null});
const place = ({scene,key,source,scope,filter,query,address,exit}: ContentPlace): ContentPlace => ({scene,key,source,scope,filter,query,address,exit});
const NAV_KEY = 'khor.content.navigation.v1', EDITOR_KEY = 'khor.content.editors.v1';
const scenes: ContentScene[] = ['picker','document','overview','sessions'];
const isPlace = (value: unknown): value is ContentPlace => {
  if (!value || typeof value !== 'object') return false;
  const p = value as ContentPlace;
  return scenes.includes(p.scene) && (p.source === 'workspace' || p.source === 'browser') &&
    (p.key === null || typeof p.key === 'string') && ['filter','query','address'].every(k => typeof p[k as keyof ContentPlace] === 'string') &&
    (p.exit === null || typeof p.exit === 'string') && (p.scope === null || typeof p.scope?.device === 'string' && typeof p.scope?.root === 'string');
};
const read = (key: string): Record<string, unknown> => {
  try { const value = JSON.parse(localStorage.getItem(key) || '{}'); return value && typeof value === 'object' && !Array.isArray(value) ? value : {}; } catch { return {}; }
};
const save = (key: string, value: unknown) => { try { localStorage.setItem(key, JSON.stringify(value)); } catch { /* viewer storage is optional */ } };
function readNavigation(): Record<string, ContentNavigationState> {
  return Object.fromEntries(Object.entries(read(NAV_KEY)).filter(([,s]) => isPlace(s)).map(([session,value]) => {
    const s = value as ContentNavigationState;
    const scrolls = Object.fromEntries(Object.entries(s.scrolls || {}).filter(([,v]) => v && Number.isFinite(v.top) && Number.isFinite(v.left)));
    return [session,{...empty(),...place(s),from:scenes.includes(s.from)?s.from:'picker',recent:typeof s.recent==='string'?s.recent:null,
      browseBefore:typeof s.browseBefore==='string'?s.browseBefore:null,
      trail:Array.isArray(s.trail)?s.trail.filter(isPlace).slice(-64):[],documentTrail:Array.isArray(s.documentTrail)?s.documentTrail.filter(isPlace).slice(-64):[],scrolls}];
  }));
}
export function enterContent(was: ContentNavigationState, patch: Partial<ContentPlace>): ContentNavigationState {
  const next = {...was,...patch};
  if (next.scene !== was.scene || next.scene === 'document' && next.key !== was.key) {
    next.trail = [...was.trail,place(was)].slice(-64); next.from = was.scene;
  }
  if (next.scene === 'document') { next.recent=next.key; next.documentTrail=next.trail; }
  return next;
}
/** Draft text persists; editing/focus and pending machine prompts never do. */
export type ContentEditor = {address?:string;editing?:boolean;exitPrompt?:number};
export function useContentNavigation(content: SessionContent) {
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
    const next={...current.current,[session]:after};current.current=next;setStates(next);save(NAV_KEY,next);
  };
  const update=(session:string,patch:Partial<ContentPlace>)=>{pendingResume.current.delete(session);write(session,s=>enterContent(s,patch));};
  const document=(tab:ContentTab)=>update(tab.session,(tab.view||'workspace')==='workspace'?{scene:'picker',source:'workspace',scope:tab.url?{device:tab.device,root:tab.url}:null}:{scene:'document',key:content.handle(tab)});
  const valid=(session:string,key:string|null)=>content.allTabs.some(t=>t.session===session&&content.handle(t)===key);
  const resume=(session:string)=>{
    if(!content.ready){pendingResume.current.add(session);return;}
    pendingResume.current.delete(session);
    const remembered=current.current[session];
    if(remembered){
      if(remembered.scene!=='document'||valid(session,remembered.key))return;
      const trail=[...remembered.trail];let previous:ContentPlace|undefined;
      while(trail.length){const entry=trail.pop()!;if(entry.scene!=='document'||valid(session,entry.key)){previous=entry;break;}}
      write(session,s=>({...s,...(previous||{scene:'picker' as const,key:null}),trail,from:trail.at(-1)?.scene||'picker'}));return;
    }
    const tab=content.active(session);
    write(session,s=>tab?enterContent(tab.view==='page'?{...s,source:'browser'}:s,{scene:'document',key:content.handle(tab)}):s);
  };
  useEffect(()=>{if(content.ready)for(const session of [...pendingResume.current])resume(session);},[content.ready]);
  const browse=(session:string,scope?:{device:string;root:string})=>update(session,{scene:'picker',...(scope?{scope,source:'workspace',query:''}:{})});
  const back=(session:string):boolean=>{
    pendingResume.current.delete(session);
    const s=get(session),trail=[...s.trail];let prev:ContentPlace|undefined;
    while(trail.length){const entry=trail.pop()!;if(entry.scene!=='document'||valid(session,entry.key)){prev=entry;break;}}
    if(!prev)return false;
    const next=prev;write(session,s=>({...s,...next,trail,from:trail.at(-1)?.scene||'picker',...(next.scene==='document'?{recent:next.key,documentTrail:trail}:{})}));
    const tab=next.scene==='document'?content.allTabs.find(t=>t.session===session&&content.handle(t)===next.key):null;
    if(tab)void content.focus(tab.id).catch(()=>{});
    return true;
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
  return {editor,edit,get,has:(session:string)=>!!current.current[session]||pendingResume.current.has(session),update,document,resume,browse,overview,back,sessions,inspect,scroll,scrollOf,browseBefore};
}
export type ContentNavigation = ReturnType<typeof useContentNavigation>;
