/** Per-session navigation. Viewing history is distinct from the node's
 * kept tabs; promotion never changes a document's stable display handle. */
import { useEffect, useRef, useState } from 'react';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
export type ContentSource = 'workspace' | 'browser';
export type ContentScene = 'picker' | 'document' | 'overview' | 'sessions';
export type ContentPlace = {
  scene: ContentScene; key: string | null; source: ContentSource;
  scope: { device: string; root: string } | null; filter: string; query: string; address:string;exit:string|null;
};
export type ContentNavigationState = ContentPlace & {
  from: ContentScene; trail: ContentPlace[]; recent: string | null; documentTrail: ContentPlace[];
};
export const emptyContentPlace = (): ContentPlace => ({scene:'picker',key:null,source:'workspace',scope:null,filter:'',query:'',address:'',exit:null});
const empty = (): ContentNavigationState => ({...emptyContentPlace(),from:'picker',trail:[],recent:null,documentTrail:[]});
const place = ({scene,key,source,scope,filter,query,address,exit}: ContentPlace): ContentPlace => ({scene,key,source,scope,filter,query,address,exit});
export function enterContent(was: ContentNavigationState, patch: Partial<ContentPlace>): ContentNavigationState {
  const next={...was,...patch};
  if(next.scene!==was.scene || next.scene==='document'&&next.key!==was.key) {
    next.trail=[...was.trail,place(was)]; next.from=was.scene;
  }
  if(next.scene==='document') {next.recent=next.key;next.documentTrail=next.trail;}
  return next;
}
export function useContentNavigation(content: SessionContent) {
  const [states,setStates]=useState<Record<string,ContentNavigationState>>({});
  const pendingResume=useRef(new Set<string>());
  const current=useRef(states);current.current=states;
  const get=(session:string)=>current.current[session]??empty();
  const write=(session:string,run:(s:ContentNavigationState)=>ContentNavigationState)=>{
    const next={...current.current,[session]:run(get(session))};current.current=next;setStates(next);
  };
  const update=(session:string,patch:Partial<ContentPlace>)=>{pendingResume.current.delete(session);write(session,s=>enterContent(s,patch));};
  const document=(tab:ContentTab)=>update(tab.session,(tab.view||'workspace')==='workspace'?{scene:'picker',source:'workspace',scope:tab.url?{device:tab.device,root:tab.url}:null}:{scene:'document',key:content.handle(tab)});
  const valid=(session:string,key:string|null)=>content.allTabs.some(t=>t.session===session&&content.handle(t)===key);
  const resume=(session:string)=>{if(!content.ready){pendingResume.current.add(session);return;}pendingResume.current.delete(session);write(session,s=>{
    const tab=content.active(session),key=valid(session,s.recent)?s.recent:tab&&(tab.view||'workspace')!=='workspace'?content.handle(tab):null;
    if(!key)return {...s,scene:'picker',trail:[],from:'picker',...(tab&&(tab.view||'workspace')==='workspace'?{source:'workspace',scope:tab.url?{device:tab.device,root:tab.url}:null}:{})};
    const trail=s.recent===key?s.documentTrail:[emptyContentPlace()];
    return {...s,scene:'document',key,recent:key,trail,documentTrail:trail,from:trail.at(-1)?.scene||'picker'};
  });};
  useEffect(()=>{if(content.ready)for(const session of [...pendingResume.current])resume(session);},[content.ready]);
  const browse=(session:string,scope?:{device:string;root:string})=>update(session,{scene:'picker',...(scope?{scope,source:'workspace',query:''}:{})});
  const back=(session:string):boolean=>{
    pendingResume.current.delete(session);
    const s=get(session),trail=[...s.trail];let prev:ContentPlace|undefined;
    while(trail.length){const entry=trail.pop()!;if(entry.scene!=='document'||valid(session,entry.key)){prev=entry;break;}}
    if(!prev)return false;
    const next=prev;write(session,s=>({...s,...next,trail,from:trail.at(-1)?.scene||'picker',...(next.scene==='document'?{recent:next.key,documentTrail:trail}:{})}));
    const tab=next.scene==='document'?content.allTabs.find(t=>t.session===session&&content.handle(t)===next.key):null;
    if(tab&&!content.isPreview(tab))void content.focus(tab.id).catch(()=>{});
    return true;
  };
  const overview=(session:string)=>get(session).scene==='overview'?back(session):update(session,{scene:'overview',filter:''});
  const sessions=(session:string)=>{pendingResume.current.delete(session);write(session,s=>({...s,scene:'sessions',trail:[],from:'picker'}));};
  const inspect=(session:string)=>{pendingResume.current.delete(session);write(session,s=>({...s,scene:'overview',filter:'',trail:[{...place(s),scene:'sessions'}],from:'sessions'}));};
  return {get,has:(session:string)=>!!current.current[session]||pendingResume.current.has(session),update,document,resume,browse,overview,back,sessions,inspect};
}
export type ContentNavigation = ReturnType<typeof useContentNavigation>;
