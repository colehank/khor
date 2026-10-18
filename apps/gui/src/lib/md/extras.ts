// The parts of the renderer that load on first need (lane md-render):
// formulas, emoji short codes, and the file page's HTML and anchors.
//
// **Loaded by what the text holds, not by which page is open.** A
// conversation that never writes `$$` never fetches KaTeX; one that
// does fetches it once, and every block drawn after that has it. A block
// already drawn without it is drawn again once — the version below is
// part of what each block is memoised on, so the arrival is a change and
// nothing else is.
//
// The file page waits for what it needs before it draws (`ready`): a
// README is read whole and not streamed, and drawing it once as source
// and again as HTML is a flash with nothing to gain. The conversation
// does not wait — a turn's words are worth more now than its formulas.
import { useEffect, useSyncExternalStore } from "react";

import type { Needs } from "@/lib/md/blocks";

type Math = typeof import("@/lib/md/math");
type File = typeof import("@/lib/md/file");
type Emoji = typeof import("remark-emoji")["default"];

export type Loaded = { math?: Math; file?: File; emoji?: Emoji };

const loaded: Loaded = {};
const loading = new Set<keyof Loaded>();
/** Chunks that would not load. The text is drawn without them rather
    than waited on for ever; the next text that needs one tries again. */
const failed = new Set<keyof Loaded>();
let version = 0;
const listeners = new Set<() => void>();

function load<K extends keyof Loaded>(key: K, get: () => Promise<Loaded[K]>) {
  if (loaded[key] !== undefined || loading.has(key)) return;
  loading.add(key);
  failed.delete(key);
  void get().then(
    (m) => {
      loaded[key] = m;
      bump();
    },
    () => {
      loading.delete(key);
      failed.add(key);
      bump();
    },
  );
}

function bump() {
  version += 1;
  for (const l of listeners) l();
}

const subscribe = (l: () => void) => {
  listeners.add(l);
  return () => listeners.delete(l);
};

/** Starts loading what `needs` names. The file page calls it while the
    source is still showing, so the first switch to the preview finds its
    chunks here and draws — and keeps its place (`FilePage`'s `placeAt`)
    — on the first frame. */
export function prepare(needs: Needs, file: boolean) {
  if (needs.math) load("math", () => import("@/lib/md/math"));
  if (needs.emoji) load("emoji", () => import("remark-emoji").then((m) => m.default));
  if (file) load("file", () => import("@/lib/md/file"));
}

/** What is loaded now, as a version that changes when more is; asks for
    what `needs` names and is not loaded yet. `ready` is whether all of
    it is here. */
export function useExtras(needs: Needs, file: boolean): { loaded: Loaded; version: number; ready: boolean } {
  const v = useSyncExternalStore(subscribe, () => version);
  useEffect(() => prepare(needs, file), [needs.math, needs.emoji, file]);
  const here = (k: keyof Loaded) => loaded[k] !== undefined || failed.has(k);
  const ready = (!needs.math || here("math")) && (!needs.emoji || here("emoji")) && (!file || here("file"));
  return { loaded, version: v, ready };
}
