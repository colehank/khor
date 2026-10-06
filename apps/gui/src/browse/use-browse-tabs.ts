// What this screen currently has open on one browsing landing.
//
// **The shell's, not one landing's** (docs/DESIGN.md 浏览型外壳): a tab
// is a machine and a place on it either way, and the store, the
// validation and the closing rules are the same whether the place is a
// directory or a page. Only the `kind` differs, and it differs in one
// place — the key.
//
// # It is the node's list now (docs/SHELL.md 第 2 条, 2026-09-20)
//
// ⚠️ **Tombstone: this file held the whole store.** It parsed
// `localStorage`, minted ids, decided which tab took the front after a
// close, and wrote back on every change — about a hundred lines of
// judgment, all of it correct and none of it reachable from anywhere
// but this window.
//
// The ruling that moved it is 「CLI 等价：标签页列表是本机状态，CLI 至少
// 要能列、能开、能关」. A list only the webview can read is a list
// `khor tabs` cannot, and **CLI 与 GUI 等价** is one of this repo's
// three standing rules — the tauri command is a thin shell over the
// same function the terminal calls, never a second implementation. So
// every one of those judgments is in `khor_node::tabs` and this file
// keeps none of them:
//
//   - **ids** are minted there, because two callers minting is two id
//     shapes and the first collision is a tab the window opened that
//     `khor tabs close` cannot name;
//   - **which tab takes the front** after a close is there, so the
//     terminal and the window cannot disagree about what you are
//     looking at;
//   - **an unreadable store** reads as nothing open there, once.
//
// What is left here is a cache of the node's answer and a hook's worth
// of plumbing. **Every write sends and then takes the node's answer as
// the new truth** — no local optimism: a refused write must show as the
// list not moving, which is the truth, rather than as a row appearing
// and then vanishing a round trip later (docs/UX.md 「做了但没变化」和
// 「失败」长不一样的脸).
//
// # An empty list is an empty list
//
// ⚠️ 墓碑 (the redesign, 2026-10-02): **a landing with no tabs used to
// get one** — minted here whenever the list read empty, for the
// new-tab page that was always on screen. The browser takes the
// session page's tab logic now, and a page nobody opened is a row
// nobody asked for: every list that read empty for a moment (the last
// page closed, a landing remounted) grew an empty page in the node's
// store, and a scratch node collected seven. Nothing is minted on this
// side any more; a tab is made by ＋ or by arriving with a place
// (`add`, `open`), and a landing with none draws its empty state
// (`empty`).
//
// # It has to keep up with the other process
//
// ⚠️ **This read the list once, on mount, and that was a regression the
// day the store moved** (2026-09-21, caught installing `362ea88`:
// 「CLI 开的标签, 正在跑的窗口里看不见」— four `khor tabs open` on the
// machine, `khor tabs ls` listing five, the window's column showing
// one). Reading once is exactly right for a store only this window can
// write, and exactly wrong for a file on the disk.
//
// So it polls, **on the beat the sessions list already uses for the
// same reason** (`App.tsx`'s `POLL_MS`): a landing whose rows another
// process can change is a landing that asks again. The two directions
// are not symmetric and both had to be checked — writes from here
// landed all along, because every one of them goes through the node
// and takes its answer back.
import { useCallback, useEffect, useRef, useState } from "react";

import { closeTab, fetchTabs, focusTab, goTab, openTab, pinTab, type TabsOpen } from "@/api";

/** One tab: a machine and a place on it. The shape the node keeps,
    renamed for the callers that were written against it. */
export type BrowseTab = {
  /** This screen's handle for the tab. **Minted by the node**; never
      shown, never sent to another machine. Two tabs on one machine and
      one path are two tabs, so the identity cannot be derived from what
      the tab points at. */
  id: string;
  /** The machine's device id, or `null` on a tab that has not been
      pointed anywhere yet — the fresh-tab state, which is a real state
      and not a missing one.
      **`null` here and `""` on the wire**: the node has no nulls in
      this table, and this layer keeps the shape the faces already
      branch on. */
  device: string | null;
  /** Where to open, in this kind's own spelling; `""` asks for that
      kind's starting point on that machine. */
  path: string;
  /** What it was last called, for the column to draw before anything
      has been loaded (第 2 条:「页面内容可以在点到那枚时才加载」). */
  title: string;
  /** Put at the top by a person (docs/SHELL.md 第二轮). The node keeps
      pinned tabs first and refuses to close one — this side only
      draws what it was told. */
  pinned: boolean;
  /** The session it was opened from, `""` where none is known — the
      附栏's browser seat lands on that session's tabs (UX-37 §1). */
  session: string;
};

const shape = (open: TabsOpen): BrowseTab[] =>
  open.tabs.map((t) => ({
    id: t.id,
    device: t.device === "" ? null : t.device,
    path: t.url,
    title: t.title,
    pinned: t.pinned,
    // An older node's row has none.
    session: t.session ?? "",
  }));

export type BrowseTabs = {
  tabs: BrowseTab[];
  /** The open one. Never undefined: with nothing open it is the
      stand-in (`ready` false). */
  current: BrowseTab;
  /** The node has answered and nothing is open: the landing's empty
      state, not a frame to wait through. */
  empty: boolean;
  /**
   * Whether `current` is a real tab yet.
   *
   * ⚠️ **A face that draws against the stand-in gets torn down when the
   * real one lands**, because the id changes and everything keyed by
   * the tab remounts — the address box included, with whatever was
   * being typed into it. That is not theoretical: it is what
   * `npm run tabs` caught the day the list moved to the node
   * (「the kept page completed from what is remembered」 timed out,
   * because the box had been re-created under the keystrokes).
   *
   * The list read to be empty in about a frame before; it is a round
   * trip now, so the frame is long enough to type into.
   */
  ready: boolean;
  focus: (id: string) => void;
  /** A fresh tab, focused — the 「+」 at the foot of the column. */
  add: () => void;
  /** Points the **open** tab somewhere. What a pin, a recent and the
      address bar all do: you are already looking at the tab, so a new
      one would be a tab you did not ask for (Chrome's own behaviour on
      a new-tab page's shortcuts). */
  goTo: (device: string, path: string, title?: string) => void;
  /** Opens a **new** tab, focused. What arriving from somewhere else
      does — the machine card's 借它的网 — because there the tab you are
      looking at is not part of the request. */
  open: (device: string, path: string) => void;
  /** Points the open tab at nothing: taking the machine off the address
      bar. **Not `close` and not `goTo("")`** — the tab is still the tab
      the person is looking at, it has simply stopped naming a place. */
  clear: () => void;
  /** **Answers**, where the rest fire and forget: the row menu reads a
      rejection to stay open with the node's sentence (`RowMenu`) — a
      pinned tab is refused by name, and that refusal is the whole of
      「关不掉之前要先取消置顶」 on this side. */
  close: (id: string) => Promise<void>;
  /** 置顶 / 取消置顶 (docs/SHELL.md 第二轮). Answers, for `close`'s reason. */
  pin: (id: string, on: boolean) => Promise<void>;
};

/**
 * A tab that exists only on this side, for the frame between asking the
 * node for the list and being told.
 *
 * **Never written anywhere**, and its id is deliberately not one the
 * node would mint (`?` is not in its alphabet): a caller that tried to
 * close or focus it is refused by name rather than closing somebody
 * else's tab.
 *
 * ⚠️ **Nothing may be *drawn against* it** — see `ready`. It stands in
 * so `current` is never undefined, not so a face can render it.
 */
const PENDING: BrowseTab = { id: "?", device: null, path: "", title: "", pinned: false, session: "" };

/**
 * How often the list is re-read while the landing that draws it is on
 * screen.
 *
 * **`App.tsx`'s `POLL_MS`, said again rather than imported**, because
 * what it means here is 「a row another process can change」 and not
 * 「the session heartbeat」 — the two happen to be the same number and
 * are not the same fact. Away from the browsing landing nothing is
 * polled at all: the column is not on screen, and the mount-time read
 * is what a person coming back to it sees.
 */
const WATCH_MS = 2000;

export function useBrowseTabs(kind: string, watch = false): BrowseTabs {
  const [tabs, setTabs] = useState<BrowseTab[]>([]);
  const [active, setActive] = useState("");
  /**
   * The id the *last render* is standing on, for the two verbs that act
   * on 「the open tab」 without being told which.
   *
   * ⚠️ **They read it through `setActive(id => …)` at first, and that is
   * a side effect inside a state updater.** React calls those twice in
   * development on purpose, so every 「go here」 was sent twice; and an
   * updater is not a place a caller may act from at all — this repo has
   * the same judgment written down about effects that must be
   * idempotent (`use-list-width`'s `show`). A ref is what 「the current
   * value, read outside render」 actually is.
   */
  const front = useRef("");
  front.current = active;
  /**
   * Whether the first read has come back. Until it has, an empty list
   * is a list still in flight, not an empty landing.
   */
  const [settled, setSettled] = useState(false);

  /**
   * How many writes are in the air.
   *
   * **A poll that landed between a write and its answer would paint the
   * list as it was before the press**, and the press would read as
   * having done nothing until the next tick undid that. So the beat
   * stands still while this side is mid-sentence; the write's own
   * answer is the fresher reading anyway.
   */
  const writing = useRef(0);
  const take = useCallback((open: TabsOpen) => {
    setSettled(true);
    setTabs(shape(open));
    setActive(open.active);
  }, []);
  /** A write, and the bookkeeping that keeps the poll off its toes. */
  const sending = useCallback(
    <T,>(go: Promise<T>): Promise<T> => {
      writing.current += 1;
      return go.finally(() => {
        writing.current -= 1;
      });
    },
    [],
  );

  useEffect(() => {
    let live = true;
    setSettled(false);
    fetchTabs(kind)
      .then((open) => live && take(open))
      .catch(() => {
        // **A store that will not answer is not an empty store**, so
        // `settled` stays false: the landing waits rather than saying
        // nothing is open.
        if (live) setTabs([]);
      });
    return () => {
      live = false;
    };
  }, [kind, take]);

  const add = useCallback(() => {
    sending(openTab(kind, "", "").then(() => fetchTabs(kind)))
      .then(take)
      .catch(() => {});
  }, [kind, sending, take]);

  const focus = useCallback(
    (id: string) => {
      sending(focusTab(kind, id)).then(take).catch(() => {});
    },
    [kind, sending, take],
  );

  const goTo = useCallback(
    (device: string, path: string, title = "") => {
      // The open tab, by the id this render is standing on. A tab closed
      // out from under this press is refused by name on the far side,
      // which is the truthful answer: the thing it was aimed at is not
      // there.
      const id = front.current;
      if (id) sending(goTab(kind, id, device, path, title)).then(take).catch(() => {});
    },
    [kind, sending, take],
  );

  const open = useCallback(
    (device: string, path: string) => {
      sending(openTab(kind, device, path).then(() => fetchTabs(kind)))
        .then(take)
        .catch(() => {});
    },
    [kind, sending, take],
  );

  const clear = useCallback(() => {
    const id = front.current;
    if (id) sending(goTab(kind, id, "", "", "")).then(take).catch(() => {});
  }, [kind, sending, take]);

  const close = useCallback(
    (id: string) => sending(closeTab(kind, id)).then(take),
    [kind, sending, take],
  );

  const pin = useCallback(
    (id: string, on: boolean) => sending(pinTab(kind, id, on)).then(take),
    [kind, sending, take],
  );

  // **The beat, and only while the column is on screen** (see
  // `WATCH_MS`). It asks for the same thing the mount read asked for
  // and hands it to the same setter — there is no second way into this
  // state, so a tick and a press cannot disagree about its shape.
  useEffect(() => {
    if (!watch) return;
    let live = true;
    const tick = () => {
      if (writing.current > 0) return;
      fetchTabs(kind)
        .then((open) => live && take(open))
        .catch(() => {
          /* a store that will not answer keeps the list that is drawn */
        });
    };
    const beat = setInterval(tick, WATCH_MS);
    // …and at once on arrival, so walking back onto the landing shows
    // what the terminal did while you were away rather than waiting a
    // beat for it.
    tick();
    return () => {
      live = false;
      clearInterval(beat);
    };
  }, [kind, watch, take]);

  const current = tabs.find((t) => t.id === active) ?? tabs[0] ?? PENDING;
  return {
    tabs,
    current,
    ready: current !== PENDING,
    empty: settled && tabs.length === 0,
    focus,
    add,
    goTo,
    open,
    clear,
    close,
    pin,
  };
}
