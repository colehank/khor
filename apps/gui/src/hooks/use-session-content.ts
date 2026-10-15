/** Session-owned content uses the node's tab table, including CLI writes.
 * Open/collapse and maximization remain viewer layout; addresses and handles
 * belong to the same persistent table the browsing landing uses. */
import { useCallback, useEffect, useRef, useState } from "react";
import { closeTab, fetchTabs, focusTab, goTab, openTab, pinTab, type TabsOpen } from "@/api";

export type ContentView = "workspace" | "file" | "page";
export type ContentTab = TabsOpen["tabs"][number] & { view: ContentView | null };
export const CONTENT_KIND = "content";
const EMPTY: TabsOpen = { tabs: [], active: "" };
export function useSessionContent(watch: boolean) {
  const [open, setOpen] = useState<TabsOpen>(EMPTY);
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
  const take = useCallback((value: TabsOpen, wrote = false) => {
    if (!mounted.current) return;
    current.current = value;
    const active = value.tabs.find(t => t.id === value.active);
    if (active?.session) chosen.current.set(active.session, active.id);
    setOpen(value);
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
  const send = useCallback(<T,>(run: () => Promise<{ value: TabsOpen; result: T }>): Promise<T> => {
    writing.current++;
    epoch.current++;
    const task = queue.current.then(run).then(({ value, result }) => { take(value, true); return result; });
    queue.current = task.catch(() => {});
    return task.catch(e => { if (mounted.current) { actionFailed.current = true; setError(String(e instanceof Error ? e.message : e)); } throw e; }).finally(() => { writing.current--; });
  }, [take]);
  const fail = (message: string) => { actionFailed.current = true; setError(message); };
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
  const point = (id: string, device: string, path: string, view: ContentView, title = "", line?: number | null) => send(async () => {
    const value = await goTab(CONTENT_KIND, id, device, path, title, view);
    if (line != null) setLines(all => ({ ...all, [id]: { line, at: Date.now() } }));
    return { result: undefined, value };
  });
  const rename = (id: string, title: string) => send(async () => {
    const tab = current.current.tabs.find(t => t.id === id);
    if (!tab) return { result: undefined, value: current.current };
    return { result: undefined, value: await goTab(CONTENT_KIND, id, tab.device, tab.url, title) };
  });
  const follow = (id: string, path: string) => send(async () => {
    const tab = current.current.tabs.find(t => t.id === id);
    return { result: undefined, value: tab ? await goTab(CONTENT_KIND, id, tab.device, path, tab.title) : current.current };
  });
  const focus = (id: string) => send(async () => ({ result: undefined, value: await focusTab(CONTENT_KIND, id) }));
  const pin = (id: string, on: boolean) => send(async () => ({ result: undefined, value: await pinTab(CONTENT_KIND, id, on) }));
  const close = (id: string) => send(async () => {
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
  const tabs = open.tabs as ContentTab[];
  const active = (session: string | null) => {
    const peers = tabs.filter(t => t.session === session);
    return peers.find(t => t.id === chosen.current.get(session || "")) ?? peers.reduce<ContentTab | undefined>((a, b) => !a || b.touched > a.touched ? b : a, undefined);
  };
  return { tabs, lines, ready, error, fail, ensure, add, point, rename, follow, focus, pin, close, active, maximized, setMaximized };
}
export type SessionContent = ReturnType<typeof useSessionContent>;
