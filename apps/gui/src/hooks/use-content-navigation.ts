/** Viewer navigation is per session. Browsing is a place, not an empty
 * persistent tab. The document key survives preview promotion. */
import { useState } from 'react';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
export type ContentSource = 'workspace' | 'browser';
export type ContentScene = 'picker' | 'document' | 'overview';
export type ContentNavigationState = {
  scene: ContentScene; from: Exclude<ContentScene, 'overview'>; key: string | null;
  source: ContentSource; scope: { device: string; root: string } | null; filter: string;
};
const empty = (): ContentNavigationState => ({scene:'picker',from:'picker',key:null,source:'workspace',scope:null,filter:''});
export function useContentNavigation(content: SessionContent) {
  const [states, setStates] = useState<Record<string, ContentNavigationState>>({});
  const get = (session: string): ContentNavigationState => states[session] ?? empty();
  const update = (session: string, patch: Partial<ContentNavigationState>) => setStates(was => ({...was,[session]:{...(was[session]??empty()),...patch}}));
  const document = (tab: ContentTab) => {
    if ((tab.view || 'workspace') === 'workspace') update(tab.session,{scene:'picker',source:'workspace',scope:tab.url?{device:tab.device,root:tab.url}:null});
    else update(tab.session,{scene:'document',key:content.handle(tab)});
  };
  const resume = (session: string) => {
    if (states[session]) return;
    const tab=content.active(session);
    if(tab) document(tab); else update(session,{});
  };
  const browse = (session: string, scope?: {device:string;root:string}) => update(session,{scene:'picker',...(scope?{scope,source:'workspace' as const}:{})});
  const overview = (session: string) => {
    const s=get(session);
    update(session,s.scene==='overview'?{scene:s.from}:{scene:'overview',from:s.scene,filter:''});
  };
  return {get,update,document,resume,browse,overview};
}
export type ContentNavigation = ReturnType<typeof useContentNavigation>;
