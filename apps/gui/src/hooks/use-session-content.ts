/** Session-owned documents are tabs as soon as they are opened. The small
 * optimistic queue is transport state, never a separate preview collection. */
import { useCallback, useEffect, useRef, useState } from "react";
import { closeTab, fetchTabs, focusTab, goTab, openTab, pinTab, type TabsOpen } from "@/api";

export type ContentView = "workspace" | "file" | "page";
export type ContentTab = TabsOpen["tabs"][number] & { view: ContentView | null };
export const CONTENT_KIND = "content";
const EMPTY: TabsOpen = { tabs: [], active: "" };
const HANDLE_KEY = "khor.content.handles";
type State = { open: TabsOpen; opening: ContentTab[]; handles: Record<string, string> };
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
  const [state, setState] = useState<State>(() => ({ open: EMPTY, opening: [], handles: readHandles() }));
  const [lines, setLines] = useState<Record<string, { line: number; at: number }>>({});
  const [ready, setReady] = useState(false), [error, setError] = useState<string | null>(null);
  const [maximized, setMaximized] = useState<string | null>(null);
  const current = useRef(state.open), opening = useRef(state.opening), handles = useRef(state.handles);
  const mounted = useRef(true), queue = useRef<Promise<unknown>>(Promise.resolve());
  const writing = useRef(0), epoch = useRef(0), actionFailed = useRef(false);
  const chosen = useRef(new Map<string, string>()), jobs = useRef(new Map<string, Promise<string>>());
  const aliases = useRef(new Map(Object.entries(state.handles).map(([id, handle]) => [handle, id])));
  const publish = () => setState({ open: current.current, opening: opening.current, handles: handles.current });
  const resolve = (id: string) => aliases.current.get(id) || id;
  const handle = (tab: ContentTab) => handles.current[tab.id] || tab.id;
  const take = useCallback((value: TabsOpen, wrote = false) => {
    if (!mounted.current) return;
    const before = current.current, active = value.tabs.find(t => t.id === value.active);
    const changed = before.active !== value.active || before.tabs.find(t => t.id === before.active)?.touched !== active?.touched;
    current.current = value;
    if (active?.session) {
      const selection = chosen.current.get(active.session);
      const awaiting = opening.current.some(t => t.id === selection);
      // A queued acknowledgement must never take focus from a newer action.
      if (!selection || !wrote && changed && !awaiting) chosen.current.set(active.session, active.id);
    }
    handles.current = Object.fromEntries(Object.entries(handles.current).filter(([id]) => value.tabs.some(t => t.id === id)));
    try { localStorage.setItem(HANDLE_KEY, JSON.stringify(handles.current)); } catch { /* device storage may be full */ }
    setState({ open: value, opening: opening.current, handles: handles.current });
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
      const existing = (value.tabs as ContentTab[]).find(t => same(t, tab.session, tab.device, tab.url, tab.view || 'file'));
      const saved = existing?.id ?? await openTab(CONTENT_KIND, tab.device, tab.url, tab.session, after ? resolve(after) : undefined, tab.view || 'file');
      const focused = await focusTab(CONTENT_KIND, saved);
      aliases.current.set(id, saved);
      handles.current = { ...handles.current, [saved]: id };
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
  const open = (session: string, device: string, path: string, view: ContentView, line?: number | null, after?: string): ContentTab => {
    // The picker itself is navigation and must never allocate an empty tab.
    if (view === 'workspace') throw new Error('Workspace browsing is navigation, not a document tab');
    const known = [...current.current.tabs as ContentTab[], ...opening.current].find(t => same(t, session, device, path, view));
    const tab = known || { id: `opening:${globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`}`, session, device, url: path, title: '', view, pinned: false, touched: Date.now() };
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
  const point = (id: string, device: string, path: string, view: ContentView, title = '', line?: number | null) => persist(id).then(saved => send(async () => {
    if (line != null) setLines(all => ({ ...all, [handles.current[saved] || saved]: { line, at: Date.now() } }));
    return { result: undefined, value: await goTab(CONTENT_KIND, saved, device, path, title, view) };
  }));
  const rename = (id: string, title: string) => persist(id).then(saved => send(async () => {
    const tab = current.current.tabs.find(t => t.id === saved);
    return { result: undefined, value: tab ? await goTab(CONTENT_KIND, saved, tab.device, tab.url, title) : current.current };
  }));
  const follow = (id: string, path: string) => persist(id).then(saved => send(async () => {
    const tab = current.current.tabs.find(t => t.id === saved);
    return { result: undefined, value: tab ? await goTab(CONTENT_KIND, saved, tab.device, path, tab.title) : current.current };
  }));
  const pin = (id: string, on: boolean) => persist(id).then(saved => send(async () => ({ result: undefined, value: await pinTab(CONTENT_KIND, saved, on) })));
  const close = (id: string): Promise<void> => {
    // Closing during the first write closes the assigned node id afterwards.
    const failed = opening.current.find(t => t.id === id);
    if (failed && !jobs.current.has(id)) {
      // The node may have accepted open even if its response/focus failed.
      // Reconcile before removing the optimistic row; never leave an orphan.
      return send(async () => {
        const value=await fetchTabs(CONTENT_KIND);
        const saved=(value.tabs as ContentTab[]).find(t=>same(t,failed.session,failed.device,failed.url,failed.view||'file'));
        const next=saved?await closeTab(CONTENT_KIND,saved.id):value;
        opening.current=opening.current.filter(t=>t.id!==id);
        if(chosen.current.get(failed.session)===id)chosen.current.delete(failed.session);
        return {result:undefined,value:next};
      });
    }
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
  const tabs = [...state.open.tabs as ContentTab[], ...state.opening].filter(t => t.view === 'file' || t.view === 'page');
  const active = (session: string | null) => {
    const peers = tabs.filter(t => t.session === session), selected = chosen.current.get(session || '');
    return peers.find(t => t.id === selected || handle(t) === selected) ?? peers.reduce<ContentTab | undefined>((a, b) => !a || b.touched > a.touched ? b : a, undefined);
  };
  const ensure = (session: string, _device: string) => { const tab = active(session); return tab ? focus(tab.id).then(() => resolve(tab.id)) : Promise.resolve(undefined); };
  return { tabs, allTabs: tabs, handle, open, lines, ready, error, fail, ensure, add, point, rename, follow, focus, pin, close, active, maximized, setMaximized };
}
export type SessionContent = ReturnType<typeof useSessionContent>;
