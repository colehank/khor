/** Session-owned content uses the node's tab table, including CLI writes.
 * Open/collapse and maximization remain viewer layout; addresses and handles
 * belong to the same persistent table the browsing landing uses. */
import { useCallback, useEffect, useRef, useState } from "react";
import { closeTab, fetchTabs, focusTab, goTab, openTab, pinTab, type TabsOpen } from "@/api";

export type ContentView = "workspace" | "file" | "page";
export type ContentTab = TabsOpen["tabs"][number] & { view: ContentView | null };
export const CONTENT_KIND = "content";
const EMPTY: TabsOpen = { tabs: [], active: "" };
type Promotion = { preview: string; saved: string };
type ContentState = { open: TabsOpen; previews: Record<string, ContentTab>; handles: Record<string, string> };
export function useSessionContent(watch: boolean) {
  const [state, setState] = useState<ContentState>({ open: EMPTY, previews: {}, handles: {} });
  const { open } = state;
  const [lines, setLines] = useState<Record<string, { line: number; at: number }>>({});
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [maximized, setMaximized] = useState<string | null>(null);
  const current = useRef(open);
  const mounted = useRef(true);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const writing = useRef(0);
  const epoch = useRef(0);
  const actionFailed = useRef(false);
  const chosen = useRef(new Map<string, string>());
  const previews = useRef(state.previews);
  previews.current = state.previews;
  const promotions = useRef(new Map<string, Promise<string>>());
  const savedIds = useRef(new Map<string, string>());
  const take = useCallback((value: TabsOpen, wrote = false, promotion?: Promotion) => {
    if (!mounted.current) return;
    const before = current.current;
    current.current = value;
    const active = value.tabs.find(t => t.id === value.active);
    const changed = before.active !== value.active || before.tabs.find(t => t.id === before.active)?.touched !== active?.touched;
    const watchingPreview = !!active?.session && chosen.current.get(active.session) === previews.current[active.session]?.id;
    const preservePreview = watchingPreview && (promotion ? chosen.current.get(active!.session) !== promotion.preview : !wrote && !changed);
    if (active?.session && !preservePreview) chosen.current.set(active.session, active.id);
    if (promotion) {
      savedIds.current.set(promotion.preview, promotion.saved);
      const next = { ...previews.current };
      for (const [session, tab] of Object.entries(next)) if (tab.id === promotion.preview) delete next[session];
      previews.current = next;
    }
    for (const [key, id] of savedIds.current) if (!value.tabs.some(t => t.id === id)) savedIds.current.delete(key);
    setState(was => {
      const next = { ...was.previews };
      if (promotion) for (const [session, tab] of Object.entries(next)) if (tab.id === promotion.preview) delete next[session];
      const handles = Object.fromEntries(Object.entries(was.handles).filter(([id]) => value.tabs.some(t => t.id === id)));
      if (promotion) handles[promotion.saved] = promotion.preview;
      return { open: value, previews: next, handles };
    });
    setReady(true);
    if (wrote) actionFailed.current = false;
    if (!actionFailed.current) setError(null);
  }, []);
  useEffect(() => {
    mounted.current = true;
    let live = true;
    const read = () => {
      if (writing.current) return;
      const turn = epoch.current;
      void fetchTabs(CONTENT_KIND).then(value => {
        if (live && !writing.current && turn === epoch.current) take(value);
      }, e => { if (live && !actionFailed.current) setError(String(e)); });
    };
    read();
    const timer = watch ? window.setInterval(read, 2000) : undefined;
    return () => { live = false; mounted.current = false; clearInterval(timer); };
  }, [watch, take]);
  const send = useCallback(<T,>(run: () => Promise<{ value: TabsOpen; result: T; promotion?: Promotion }>): Promise<T> => {
    writing.current++;
    epoch.current++;
    const task = queue.current.then(run).then(({ value, result, promotion }) => { take(value, true, promotion); return result; });
    queue.current = task.catch(() => {});
    return task.catch(e => { if (mounted.current) { actionFailed.current = true; setError(String(e instanceof Error ? e.message : e)); } throw e; }).finally(() => { writing.current--; });
  }, [take]);
  const fail = (message: string) => { actionFailed.current = true; setError(message); };
  const findPreview = (id: string) => Object.values(previews.current).find(t => t.id === id);
  const patchPreview = (id: string, patch: Partial<ContentTab>) => {
    setState(was => {
      const tab = Object.values(was.previews).find(t => t.id === id);
      return tab ? { ...was, previews: { ...was.previews, [tab.session]: { ...tab, ...patch } } } : was;
    });
    const tab = findPreview(id);
    if (tab) previews.current = { ...previews.current, [tab.session]: { ...tab, ...patch } };
  };
  /** A glance is viewer state. It never calls tabs_open or appears in CLI ls. */
  const peek = (session: string, device: string, path: string, view: ContentView, line?: number | null): ContentTab => {
    const saved = current.current.tabs.find(t => t.session === session && t.device === device && t.url === path && (t as ContentTab).view === view) as ContentTab | undefined;
    const previous = previews.current[session];
    const tab: ContentTab = saved ?? (previous?.device === device && previous.url === path && previous.view === view ? previous : {
      id: `preview:${crypto.randomUUID()}`, session, device, url: path, title: "", view, pinned: false, touched: Date.now(),
    });
    if (!saved) {
      chosen.current.set(session, tab.id);
      previews.current = { ...previews.current, [session]: tab };
      setState(was => ({ ...was, previews: { ...was.previews, [session]: tab } }));
    } else void focus(saved.id).catch(() => {});
    if (line != null) setLines(all => ({ ...all, [tab.id]: { line, at: Date.now() } }));
    return tab;
  };
  /** Promote atomically with a stable render/native-page handle. Editing and
   * selection survive the node assigning its persistent id. */
  const keep = (id: string): Promise<string> => {
    const saved = savedIds.current.get(id);
    if (saved) return Promise.resolve(saved);
    const pending = promotions.current.get(id);
    if (pending) return pending;
    const tab = findPreview(id);
    if (!tab) return Promise.resolve(id);
    const task = send(async () => {
      const value = await fetchTabs(CONTENT_KIND);
      const existing = value.tabs.find(t => t.session === tab.session && t.device === tab.device && t.url === tab.url && (t as ContentTab).view === tab.view);
      const saved = existing?.id ?? await openTab(CONTENT_KIND, tab.device, tab.url, tab.session, undefined, tab.view || 'file');
      return { result: saved, value: await focusTab(CONTENT_KIND, saved), promotion: { preview: id, saved } };
    }).finally(() => promotions.current.delete(id));
    promotions.current.set(id, task);
    return task;
  };
  const ensure = (session: string, device: string) => send(async () => {
    const value = await fetchTabs(CONTENT_KIND);
    const peers = value.tabs.filter(t => t.session === session);
    const existing = peers.reduce<typeof peers[number] | undefined>((a, b) => !a || b.touched > a.touched ? b : a, undefined);
    if (existing) return { result: existing.id, value: await focusTab(CONTENT_KIND, existing.id) };
    const result = await openTab(CONTENT_KIND, device, "", session, undefined, "workspace");
    return { result, value: await fetchTabs(CONTENT_KIND) };
  });
  const add = (session: string, device: string, path: string, view: ContentView = "workspace", after?: string, line?: number | null) => send(async () => {
    const result = await openTab(CONTENT_KIND, device, path, session, after, view);
    if (line != null) setLines(all => ({ ...all, [result]: { line, at: Date.now() } }));
    return { result, value: await fetchTabs(CONTENT_KIND) };
  });
  const point = (id: string, device: string, path: string, view: ContentView, title = "", line?: number | null) => {
    id = savedIds.current.get(id) || id;
    if (findPreview(id)) {
      patchPreview(id, { device, url: path, view, title });
      if (line != null) setLines(all => ({ ...all, [id]: { line, at: Date.now() } }));
      return Promise.resolve();
    }
    return send(async () => {
      const value = await goTab(CONTENT_KIND, id, device, path, title, view);
      if (line != null) setLines(all => ({ ...all, [id]: { line, at: Date.now() } }));
      return { result: undefined, value };
    });
  };
  const rename = (id: string, title: string) => {
    id = savedIds.current.get(id) || id;
    if (findPreview(id)) { patchPreview(id, { title }); return Promise.resolve(); }
    return send(async () => {
      const tab = current.current.tabs.find(t => t.id === id);
      if (!tab) return { result: undefined, value: current.current };
      return { result: undefined, value: await goTab(CONTENT_KIND, id, tab.device, tab.url, title) };
    });
  };
  const follow = (id: string, path: string) => {
    id = savedIds.current.get(id) || id;
    if (findPreview(id)) { patchPreview(id, { url: path }); return Promise.resolve(); }
    return send(async () => {
      const tab = current.current.tabs.find(t => t.id === id);
      return { result: undefined, value: tab ? await goTab(CONTENT_KIND, id, tab.device, path, tab.title) : current.current };
    });
  };
  const focus = (id: string) => {
    id = savedIds.current.get(id) || id;
    const preview = findPreview(id);
    if (preview) { chosen.current.set(preview.session, id); setState(was => ({ ...was })); return Promise.resolve(); }
    return send(async () => ({ result: undefined, value: await focusTab(CONTENT_KIND, id) }));
  };
  const pin = (id: string, on: boolean) => send(async () => ({ result: undefined, value: await pinTab(CONTENT_KIND, id, on) }));
  const close = (id: string): Promise<void> => {
    const pending = promotions.current.get(id);
    if (pending) return pending.then(saved => close(saved));
    id = savedIds.current.get(id) || id;
    const preview = findPreview(id);
    if (preview) {
      const next = { ...previews.current }; delete next[preview.session]; previews.current = next;
      setState(was => { const next = { ...was.previews }; if (next[preview.session]?.id === id) delete next[preview.session]; return { ...was, previews: next }; });
      return Promise.resolve();
    }
    return send(async () => {
      const tab = current.current.tabs.find(t => t.id === id);
      const peers = current.current.tabs.filter(t => t.session === tab?.session);
      const at = peers.findIndex(t => t.id === id);
      let value = await closeTab(CONTENT_KIND, id);
      if (tab?.session && chosen.current.get(tab.session) === id) {
        const next = peers.filter(t => t.id !== id)[Math.min(at, peers.length - 2)];
        if (next && value.tabs.some(t => t.id === next.id)) value = await focusTab(CONTENT_KIND, next.id);
        else chosen.current.delete(tab.session);
      }
      return { result: undefined, value };
    });
  };
  const tabs = open.tabs as ContentTab[];
  const active = (session: string | null) => {
    const peers = [...tabs, ...Object.values(state.previews)].filter(t => t.session === session);
    return peers.find(t => t.id === chosen.current.get(session || "")) ?? peers.reduce<ContentTab | undefined>((a, b) => !a || b.touched > a.touched ? b : a, undefined);
  };
  const allTabs = [...tabs, ...Object.values(state.previews)];
  const handle = (tab: ContentTab) => state.handles[tab.id] || tab.id;
  const isPreview = (tab: ContentTab) => state.previews[tab.session]?.id === tab.id;
  return { tabs, allTabs, handle, isPreview, previews: state.previews, peek, keep, lines, ready, error, fail, ensure, add, point, rename, follow, focus, pin, close, active, maximized, setMaximized };
}
export type SessionContent = ReturnType<typeof useSessionContent>;
