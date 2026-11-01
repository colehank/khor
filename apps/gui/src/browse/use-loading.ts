// Native page loading is an issued lifecycle snapshot. Khor's own errands
// remain separate: completing a borrow is not a successful page load.
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import { inApp, pageState } from "@/api";
import { alive, ended, loading, nativeBusy, nativeReceived, NONE, started, type Loads, type NativeLoads } from "@/browse/loading-rule";
import type { PageNavigationState } from "@/gen/bindings/PageNavigationState";
import { listenNativeEvent } from "@/lib/native-events";

const BEAT_MS = 1_000;
/** The successful line reaches its end briefly before disappearing. */
export const SETTLE_MS = 600;
export type Loading = {
  busy: (id: string) => boolean;
  errand: (id: string, busy: boolean) => void;
  state: (id: string) => PageNavigationState | null;
  share: (id: string) => number | undefined;
};

export function useLoading(ids: readonly string[], current: string): Loading {
  const wantedKey = JSON.stringify([...ids].sort());
  const wanted = useRef(new Set(ids));
  wanted.current = new Set(ids);
  const [errands, setErrands] = useState<Loads>(NONE);
  const [states, setStates] = useState<NativeLoads>(new Map());
  const accepted = useRef<NativeLoads>(states);
  const [finishing, setFinishing] = useState<ReadonlyMap<string, string>>(new Map());
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const lifetime = useRef(0);
  const [ready, setReady] = useState(0);

  const accept = useCallback((state: PageNavigationState, event = true) => {
    if (!wanted.current.has(state.tab)) return;
    const previous = accepted.current.get(state.tab);
    const next = nativeReceived(accepted.current, state);
    if (next === accepted.current) return;
    // Resuming a page that already finished is not a new completion.
    const showFinish = state.phase === "finished" && (event || (previous?.view === state.view && previous?.navigation === state.navigation && nativeBusy(previous)));
    accepted.current = next;
    setStates(next);
    clearTimeout(timers.current.get(state.tab));
    timers.current.delete(state.tab);
    setFinishing(was => {
      const next = new Map(was);
      if (showFinish) next.set(state.tab, state.revision);
      else next.delete(state.tab);
      return next;
    });
    if (showFinish) {
      timers.current.set(state.tab, setTimeout(() => {
        timers.current.delete(state.tab);
        setFinishing(was => {
          if (was.get(state.tab) !== state.revision) return was;
          const next = new Map(was); next.delete(state.tab); return next;
        });
      }, SETTLE_MS));
    }
  }, []);

  useEffect(() => {
    if (!inApp) return;
    const epoch = ++lifetime.current;
    const drop = listenNativeEvent<PageNavigationState>("page:navigation", state => {
      if (epoch === lifetime.current) accept(state);
    }, { onReady: () => { if (epoch === lifetime.current) setReady(epoch); } });
    return () => {
      ++lifetime.current;
      drop();
      for (const timer of timers.current.values()) clearTimeout(timer);
      timers.current.clear();
    };
  }, [accept]);

  // Subscribe before querying. A query that returns after a newer event is
  // rejected by nativeReceived's issued revision, including same-URL reload.
  useEffect(() => {
    if (!inApp || !current || ready !== lifetime.current) return;
    let active = true;
    const epoch = lifetime.current;
    void pageState(current).then(state => {
      if (active && epoch === lifetime.current && state) accept(state, false);
    }).catch(() => {});
    return () => { active = false; };
  }, [current, ready, accept]);

  useLayoutEffect(() => {
    const next = new Map([...accepted.current].filter(([id]) => wanted.current.has(id)));
    if (next.size !== accepted.current.size) { accepted.current = next; setStates(next); }
    for (const [id, timer] of timers.current) if (!wanted.current.has(id)) { clearTimeout(timer); timers.current.delete(id); }
    setFinishing(was => {
      const next = new Map([...was].filter(([id]) => wanted.current.has(id)));
      return next.size === was.size ? was : next;
    });
    setErrands(was => {
      const next = new Map([...was].filter(([id]) => wanted.current.has(id)));
      return next.size === was.size ? was : next;
    });
  }, [wantedKey]);

  const any = errands.size > 0;
  useEffect(() => {
    if (!any) return;
    const timer = setInterval(() => setErrands(was => alive(was, Date.now())), BEAT_MS);
    return () => clearInterval(timer);
  }, [any]);
  const errand = useCallback((id: string, busy: boolean) => {
    if (wanted.current.has(id)) setErrands(was => busy ? started(was, id, Date.now()) : ended(was, id));
  }, []);
  const state = useCallback((id: string) => wanted.current.has(id) ? states.get(id) ?? null : null, [states]);
  const busy = useCallback((id: string) => nativeBusy(state(id)) || loading(errands, id, Date.now()), [state, errands]);
  const share = useCallback((id: string): number | undefined => {
    const value = state(id);
    if (value?.phase === "finished") return finishing.get(id) === value.revision ? 1 : undefined;
    return nativeBusy(value) ? value?.estimate ?? undefined : undefined;
  }, [state, finishing]);
  return { busy, errand, state, share };
}
