/** Session-owned starting pages and documents share one durable collection. The small
 * optimistic queue is transport state, never a separate preview collection. */
import { useCallback, useEffect, useRef, useState } from "react";
import { closeTab, fetchTabs, focusTab, goTab, openTab, pinTab, reorderTabs, type TabsOpen } from "@/api";

import {usePageIcons} from '@/hooks/use-page-icons';

import type { ContentView } from '@/lib/content';
export type { ContentView } from '@/lib/content';
export type ContentTab = TabsOpen["tabs"][number] & { view: ContentView | null };
export const CONTENT_KIND = "content";
type WorkspaceDocumentRequest = {
  session:string;source?:string|null;device:string;address:string;view:'file'|'commit';
  line?:number|null;keep?:boolean;title?:string;
};
const EMPTY: TabsOpen = { tabs: [], active: "" };
const HANDLE_KEY = "khor.content.handles";
type State = { open: TabsOpen; opening: ContentTab[]; handles: Record<string, string>; pointing: Record<string, Partial<ContentTab>> };
function readHandles(): Record<string, string> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(HANDLE_KEY) || '{}');
    return value && typeof value === 'object' && !Array.isArray(value)
      ? Object.fromEntries(Object.entries(value).filter((entry): entry is [string,string] => typeof entry[1] === 'string')) : {};
  } catch { return {}; }
}
// Browser observations normalize host casing/default ports and the empty path.
// Keep draft spelling separate; files and page query/hash remain distinct.
const pageIdentity = (url: string) => {
  try { return new URL(url).href; } catch { return url; }
};
const same = (tab: ContentTab, session: string, device: string, url: string, view: ContentView) =>
  tab.session === session && tab.device === device && (view === 'page' ? pageIdentity(tab.url) === pageIdentity(url) : tab.url === url) && (tab.view || 'workspace') === view;
export function useSessionContent(watch: boolean) {
  const pageIcons = usePageIcons();
  const [state, setState] = useState<State>(() => ({ open: EMPTY, opening: [], handles: readHandles(), pointing: {} }));
  const [lines, setLines] = useState<Record<string, { line: number; at: number }>>({});
  const [ready, setReady] = useState(false), [error, setError] = useState<string | null>(null);
  const [maximized, setMaximized] = useState<string | null>(null);
  const current = useRef(state.open), opening = useRef(state.opening), handles = useRef(state.handles);
  const mounted = useRef(true), queue = useRef<Promise<unknown>>(Promise.resolve());
  const writing = useRef(0), epoch = useRef(0), actionFailed = useRef(false);
  const chosen = useRef(new Map<string, string>()), jobs = useRef(new Map<string, Promise<string>>());
  const aliases = useRef(new Map(Object.entries(state.handles).map(([id, handle]) => [handle, id])));
  const previews = useRef(new Map<string, string>());
  const pointing = useRef(state.pointing), versions = useRef(new Map<string, number>());
  const publish = () => setState({ open: current.current, opening: opening.current, handles: handles.current, pointing: pointing.current });
  const resolve = (id: string) => aliases.current.get(id) || id;
  const handle = (tab: ContentTab) => handles.current[tab.id] || tab.id;
  const take = useCallback((value: TabsOpen, wrote = false) => {
    if (!mounted.current) return;
    const before = current.current, active = value.tabs.find(t => t.id === value.active);
    const changed = before.active !== value.active || before.tabs.find(t => t.id === before.active)?.touched !== active?.touched;
    // A successful open can arrive through polling or another write after
    // its direct response was lost. Reconcile by request, never blank URL.
    for(const tab of value.tabs){
      if(!tab.request||!opening.current.some(t=>t.id===tab.request))continue;
      aliases.current.set(tab.request,tab.id);
      handles.current={...handles.current,[tab.id]:tab.request};
      opening.current=opening.current.filter(t=>t.id!==tab.request);
    }
    current.current = value;
    if (active?.session) {
      const selection = chosen.current.get(active.session);
      const awaiting = opening.current.some(t => t.id === selection);
      // A queued acknowledgement must never take focus from a newer action.
      if (!selection || !wrote && changed && !awaiting) chosen.current.set(active.session, active.id);
    }
    handles.current = Object.fromEntries(Object.entries(handles.current).filter(([id]) => value.tabs.some(t => t.id === id)));
    try { localStorage.setItem(HANDLE_KEY, JSON.stringify(handles.current)); } catch { /* device storage may be full */ }
    setState({ open: value, opening: opening.current, handles: handles.current, pointing: pointing.current });
    setReady(true);
    if (wrote) actionFailed.current = false;
    if (!actionFailed.current) setError(null);
  }, []);
  useEffect(() => {
    mounted.current = true; let live = true;
    const read = () => {
      if (writing.current) return;
      const turn = epoch.current;
      void fetchTabs(CONTENT_KIND).then(value => {
        if (live && !writing.current && turn === epoch.current) take(value);
      }, e => { if (live && !actionFailed.current) setError(String(e)); });
    };
    read(); const timer = watch ? window.setInterval(read, 2000) : undefined;
    return () => { live = false; mounted.current = false; clearInterval(timer); };
  }, [watch, take]);
  const send = useCallback(<T,>(run: () => Promise<{ value: TabsOpen; result: T }>): Promise<T> => {
    writing.current++; epoch.current++;
    const task = queue.current.then(run).then(({ value, result }) => { take(value, true); return result; });
    queue.current = task.catch(() => {});
    return task.catch(e => {
      if (mounted.current) { actionFailed.current = true; setError(String(e instanceof Error ? e.message : e)); }
      throw e;
    }).finally(() => { writing.current--; });
  }, [take]);
  const fail = (message: string) => { actionFailed.current = true; setError(message); };
  const persist = (id: string, after?: string): Promise<string> => {
    const pending = jobs.current.get(id); if (pending) return pending;
    const tab = opening.current.find(t => t.id === id);
    if (!tab) return Promise.resolve(resolve(id));
    const task = send(async () => {
      const value = await fetchTabs(CONTENT_KIND);
      const assigned = aliases.current.get(id);
      const existing = (value.tabs as ContentTab[]).find(t => assigned ? t.id === assigned : tab.view !== 'workspace' && same(t, tab.session, tab.device, tab.url, tab.view || 'file'));
      const saved = existing?.id ?? await openTab(CONTENT_KIND, tab.device, tab.url, tab.session, after ? resolve(after) : undefined, tab.view || 'file', id);
      aliases.current.set(id, saved);
      handles.current = { ...handles.current, [saved]: id };
      if (tab.title && existing?.title !== tab.title) await goTab(CONTENT_KIND, saved, tab.device, tab.url, tab.title, tab.view || 'file');
      const focused = await focusTab(CONTENT_KIND, saved);
      opening.current = opening.current.filter(t => t.id !== id);
      return { result: saved, value: focused };
    }).finally(() => jobs.current.delete(id));
    jobs.current.set(id, task); return task;
  };
  const focus = (id: string): Promise<void> => {
    const tab = opening.current.find(t => t.id === id) || current.current.tabs.find(t => t.id === resolve(id));
    if (tab) { chosen.current.set(tab.session, handle(tab as ContentTab)); publish(); }
    if (opening.current.some(t => t.id === id)) return persist(id).then(() => {});
    return send(async () => ({ result: undefined, value: await focusTab(CONTENT_KIND, resolve(id)) }));
  };
  const open = (session: string, device: string, path: string, view: ContentView, line?: number | null, after?: string, title = ''): ContentTab => {
    // + always creates a distinct starting tab. Address deduplication is
    // reserved for links opened from outside an existing content tab.
    const known = view === 'workspace' ? undefined : [...current.current.tabs as ContentTab[], ...opening.current].map(t=>({...t,...pointing.current[handle(t)]})).find(t => same(t, session, device, path, view));
    const tab = known || { id: `opening:${globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`}`, session, device, url: path, title, view, pinned: false, touched: Date.now() };
    chosen.current.set(session, handle(tab));
    if (!known) opening.current = [...opening.current, tab];
    publish();
    if (opening.current.some(t => t.id === tab.id)) void persist(tab.id, after).catch(() => {});
    else void focus(tab.id).catch(() => {});
    if (line != null) setLines(all => ({ ...all, [handle(tab)]: { line, at: Date.now() } }));
    return tab;
  };
  const add = (session: string, device: string, path: string, view: ContentView = 'file', after?: string, line?: number | null) => {
    const tab = open(session, device, path, view, line, after); return persist(tab.id, after);
  };
  const point = (id: string, device: string, path: string, view: ContentView, title = '', line?: number | null) => {
    const tab = opening.current.find(t=>t.id===id) || current.current.tabs.find(t=>t.id===resolve(id));
    if (!tab) return Promise.reject(new Error('Content tab was closed'));
    const key=handle(tab as ContentTab), version=(versions.current.get(key)||0)+1;
    if(view!=='file'&&view!=='commit'&&previews.current.get(tab.session)===key)previews.current.delete(tab.session);
    versions.current.set(key,version);
    pointing.current={...pointing.current,[key]:{device,url:path,view,title}};publish();
    if(line!=null)setLines(all=>({...all,[key]:{line,at:Date.now()}}));
    return persist(id).then(saved=>send(async()=>({result:undefined,value:await goTab(CONTENT_KIND,saved,device,path,title,view)}))).finally(()=>{
      if(versions.current.get(key)===version){const next={...pointing.current};delete next[key];pointing.current=next;if(mounted.current)publish();}
    });
  };
  /** Choose a document from workspace/history, preserving already opened tabs. */
  const openFromPicker = (session: string, key: string | null, device: string, path: string, view: 'file'|'page', line?: number|null): ContentTab => {
    // Read live optimistic targets: two choices may arrive before React renders
    // or the node acknowledges the first conversion of a blank starting page.
    const tab = [...current.current.tabs as ContentTab[], ...opening.current]
      .map(t=>({...t,...pointing.current[handle(t)]}))
      .find(t=>t.session===session&&(handle(t)===key||t.id===key));
    if(tab&&(tab.view||'workspace')==='workspace'){
      const next={...tab,device,url:path,view,title:''};
      void point(tab.id,device,path,view,'',line).catch(()=>{});
      return next;
    }
    return open(session,device,path,view,line,tab?.id);
  };
  const liveTabs = () => [...current.current.tabs as ContentTab[], ...opening.current]
    .map(tab => ({...tab, ...pointing.current[handle(tab)]}));
  const isPreview = (tab: ContentTab) => previews.current.get(tab.session) === handle(tab);
  const keepDocument = (id: string) => {
    const tab = liveTabs().find(tab => tab.id === resolve(id) || handle(tab) === id);
    if (tab && isPreview(tab)) { previews.current.delete(tab.session); publish(); }
  };
  /** A workspace keeps its source tab and one replaceable, clean document per session.
   * Preview ownership is local to this viewer; persisted tabs reopen as kept documents. */
  const openWorkspaceDocument = ({session,source=null,device,address,view,line,keep=false,title=''}:WorkspaceDocumentRequest):ContentTab => {
    const tabs = liveTabs();
    const known = tabs.find(tab => same(tab,session,device,address,view));
    const preview = tabs.find(tab => tab.session === session && isPreview(tab) && !tab.pinned);
    let tab: ContentTab;
    if (known) {
      tab = open(session,device,address,view,line);
    } else if (preview) {
      tab = {...preview,device,url:address,view,title};
      chosen.current.set(session,handle(tab));
      void point(tab.id,device,address,view,title,line).catch(()=>{});
      void focus(tab.id).catch(()=>{});
    } else {
      tab = open(session,device,address,view,line,source || undefined,title);
      previews.current.set(session,handle(tab));
    }
    if (keep) keepDocument(tab.id);
    publish();
    return tab;
  };
  /** Native page observations can arrive after outer Back reset the tab.
   * Check the optimistic target now and its destination version at write
   * time so an old page cannot rename a starting tab or undo its reset. */
  const pageObservation=(id:string)=>{
    const tab=opening.current.find(t=>t.id===id)||current.current.tabs.find(t=>t.id===resolve(id));
    if(!tab)return null;
    const key=handle(tab as ContentTab),live={...tab,...pointing.current[key]};
    return live.view==='page'?{key,version:versions.current.get(key)||0}:null;
  };
  const rename = (id: string, title: string) => {
    const observation=pageObservation(id);if(!observation)return Promise.resolve();
    return persist(id).then(saved => send(async () => {
      const tab = current.current.tabs.find(t => t.id === saved);
      const currentVersion=versions.current.get(observation.key)||0;
      return { result: undefined, value: tab?.view==='page'&&currentVersion===observation.version ? await goTab(CONTENT_KIND, saved, tab.device, tab.url, title) : current.current };
    }));
  };
  const follow = (id: string, path: string) => {
    const observation=pageObservation(id);if(!observation)return Promise.resolve();
    return persist(id).then(saved => send(async () => {
      const tab = current.current.tabs.find(t => t.id === saved);
      const currentVersion=versions.current.get(observation.key)||0;
      return { result: undefined, value: tab?.view==='page'&&currentVersion===observation.version ? await goTab(CONTENT_KIND, saved, tab.device, path, tab.title) : current.current };
    }));
  };
  const pin = (id: string, on: boolean) => { if(on)keepDocument(id); return persist(id).then(saved => send(async () => ({ result: undefined, value: await pinTab(CONTENT_KIND, saved, on) }))); };
  const close = (id: string): Promise<void> => {
    // Closing during the first write closes the assigned node id afterwards.
    return persist(id).then(saved => send(async () => {
      const tab = current.current.tabs.find(t => t.id === saved);
      const peers = current.current.tabs.filter(t => t.session === tab?.session);
      const at = peers.findIndex(t => t.id === saved);
      let value = await closeTab(CONTENT_KIND, saved);
      if (tab?.session && [saved, handles.current[saved]].includes(chosen.current.get(tab.session) || '')) {
        const next = peers.filter(t => t.id !== saved)[Math.min(at, peers.length - 2)];
        if (next && value.tabs.some(t => t.id === next.id)) {
          chosen.current.set(tab.session, handles.current[next.id] || next.id);
          value = await focusTab(CONTENT_KIND, next.id);
        } else chosen.current.delete(tab.session);
      }
      return { result: undefined, value };
    }));
  };
  const tabs = [...state.open.tabs as ContentTab[], ...state.opening].map(t=>({...t,...state.pointing[handle(t)]}));
  const active = (session: string | null) => {
    const peers = tabs.filter(t => t.session === session), selected = chosen.current.get(session || '');
    return peers.find(t => t.id === selected || handle(t) === selected) ?? peers.reduce<ContentTab | undefined>((a, b) => !a || b.touched > a.touched ? b : a, undefined);
  };
  const reorder = async (session: string, ids: string[]) => {
    // Resolve optimistic handles before entering the serialized mutation queue.
    const saved = await Promise.all(ids.filter(id => !id.startsWith('browse:')).map(id => persist(id)));
    return send(async () => ({result:undefined,value:await reorderTabs(CONTENT_KIND,session,saved)}));
  };
  const ensure = (session: string, _device: string) => { const tab = active(session); return tab ? focus(tab.id).then(() => resolve(tab.id)) : Promise.resolve(undefined); };
  return { newTab:(session:string,device='')=>open(session,device,'','workspace'), pageIcon: pageIcons.iconOf, recordPageIcon: pageIcons.record, tabs, allTabs: tabs, handle, open, lines, ready, error, fail, ensure, reorder, add, point, openFromPicker, openWorkspaceDocument, isPreview, keepDocument, rename, follow, focus, pin, close, active, maximized, setMaximized };
}
export type SessionContent = ReturnType<typeof useSessionContent>;
