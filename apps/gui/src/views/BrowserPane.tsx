// What one browser tab is showing: a page opened through some machine's
// network (docs/NET.md 借网), and whether that worked.
//
// **Thin on purpose** (浏览器接壳批). This used to be the whole landing —
// its own header, its own address box, its own list of pinned pages. All
// three moved: the machine's name is the address bar's chip, the address
// box is the address bar, and the pins are the fresh tab. What is left is
// the half that is genuinely about borrowing a network: doing it, saying
// whether it worked, and keeping the page if it is worth keeping.
//
// **Opening is a real act, and that is why it happens here rather than
// in the shell.** `BrowseLanding`'s `onPlace` only says *where to go*;
// for a directory that costs a listing, but for a page it opens a real
// borrow session on the far machine — something nobody can use is
// something nobody thinks to close (`api.ts` on `openPage`). So the body
// is the one that decides when to spend it, and the shell is told only
// once it worked (`onLanded` → the visit is recorded).
import { useCallback, useEffect, useRef, useState } from "react";

import { hidePage, inApp, movePage, onPageWord, openPage, pageIcon, showPage } from "@/api";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { holdBorrow, holdPage } from "@/views/held";

/** A page's scheme is optional to type; a bare host means https. */
export function normalize(raw: string): string | null {
  const t = raw.trim();
  if (!t) return null;
  return /^[a-z]+:\/\//i.test(t) ? t : `https://${t}`;
}

/** Everything in this app that draws itself over the page. Radix puts
    its poppers in one wrapper; the address bar's completions and the
    dialogs are the two that are not Radix's.

    ⚠️ **The completions are usually not in this document any more**
    (2026-09-09). They are drawn in a small window of the app's own, which
    is above a child webview by construction — that is what `float` on the
    address bar's box buys, and it is the only way anything genuinely
    floats over a page. They land back here when that window cannot be
    had, so the selector stays; what it no longer describes is the common
    case. */
/** How far below the page's top edge an overlay may begin and still
    count as hanging down from above it rather than floating over it.

    ⚠️ **Written for the completions, and they left** (see above). What is
    left in this document floats in the middle of the page — dialogs,
    poppers, tooltips — and none of it hangs from the top edge, so this
    branch is now the fallback's and only the fallback's. It is kept
    because the fallback needs it: pushed down is a page you can still
    read, and taken away is a black rectangle. */
const HANGS_FROM_ABOVE = 48;

const OVERLAYS =
  '[data-deck-selector],[data-radix-popper-content-wrapper],[data-omni-menu],[role="dialog"],[role="tooltip"],[role="menu"]';

/** **Which pane holds each tab's page now**: the open errand of a pane
    that has gone hides the page it arrived with only when no pane has
    taken the tab up since. */
const showing = new Map<string, object>();

/** What the open action is doing or has done, said in place. */
type Opening = { url: string; state: "opening" | "opened" | "failed"; word: string };

export function BrowserPane({
  tab,
  machine,
  device,
  url,
  here,
  onOpened,
  onWent,
  onPopped,
  onTitle,
  onIcon,
  onBusy,
}: {
  /** This tab's handle. The page webview is keyed by it on the app side,
      so every call about that page names it. */
  tab: string;
  /** The exit machine's name — whose network a page leaves through. */
  machine: string;
  /** Its device id — what a web pin is keyed by. */
  device: string;
  /** The page this tab is pointed at; `""` before one has been named. */
  url: string;
  /** The exit is this machine (docs/SHELL.md 第四轮 第 5 条:「本机支持直接
      上网, 同样在khor里打开」): nothing is borrowed, the page goes out
      of this machine's own network, in a page view of khor's own. */
  here: boolean;
  /** What the page calls itself, as it says so. */
  onTitle: (title: string) => void;
  /** The site's icon, fetched through its own exit, and the URL that
      requested it (which may no longer be the selected page). */
  onIcon?: (src: string, at: string) => void;
  /**
   * khor has errands out for this tab, or they have all come back
   * (docs/SHELL.md 第十二轮).
   *
   * **This is not「the page loaded」and must not be read as it.** What
   * this pane can see is its own work — opening the borrow and asking
   * the site for its icon through it. In the app the page itself says
   * when it started and finished (`page:load`), and that is the fact the
   * sidebar uses; this is what the web face has instead, where there is
   * no page to ask (`browse/use-loading.ts` decides which is which).
   */
  onBusy?: (busy: boolean) => void;
  /** It really opened. Only on success: a borrow that was refused is
      not a place this person has been. */
  onOpened: (url: string) => void;
  /** The page went somewhere on its own — a link, a redirect, a form.
      **The tab has to follow it**, or the address bar keeps saying where
      the tab was pointed rather than where it is. */
  onWent: (url: string) => void;
  /** The page asked for a new window. **Carried, not decided**: whether
      a link opens here or beside here is the page author's call. */
  onPopped: (url: string) => void;
  /* ⚠️ **Tombstone: `direct` opened a page without borrowing anybody's
     network** (工作区批5 → 2026-09-17 ruling 46). Its one caller was a
     workspace's VS Code, whose URL is a loopback address of khor's own
     tunnel — nothing to borrow, and a proxy would have sent the request
     out through the far machine to its own loopback. The editor is a
     window of the app's own now (`api.ts` `codeSeat`), not a pane in
     this shell, so every tab this file draws is a place somebody
     browses through a machine again. A branch with no caller is a
     second behaviour nobody can see. */
}) {
  const [opening, setOpening] = useState<Opening | null>(null);
  /** The proxy this page's webview wears — its borrow's local address,
      `""` for a page this machine opens itself. The icon is asked
      through the same one (`khor_node::favicon`). */
  const exit = useRef("");
  const icon = useRef(onIcon);
  icon.current = onIcon;
  // **It hands back the errand**, so the caller can wait for it: a site
  // that has not answered its icon has not answered anything, and on a
  // face with no page of its own that is the whole of what「still
  // loading」can mean (第十二轮).
  const iconOf = useCallback(
    (at: string): Promise<void> =>
      icon.current
        ? pageIcon(at, exit.current).then(
            (src) => icon.current?.(src, at),
            // No icon is a row wearing the generic page mark — nothing to say.
            () => {},
          )
        : Promise.resolve(),
    [],
  );
  const busy = useRef(onBusy);
  busy.current = onBusy;

  // Where the page goes: the rectangle this body occupies, in CSS pixels,
  // which are the app's logical pixels. **Measured here because only this
  // side can see the layout** — the app knows it has a window and nothing
  // about what is drawn in it.
  const host = useRef<HTMLDivElement | null>(null);
  const away = useRef(false);
  /** The rectangle the page was last put at, so it is not moved to where
      it already is (`place`). */
  const put = useRef("");
  const boxOf = useCallback(() => {
    const el = host.current;
    if (!el) return null;
    const r = el.getBoundingClientRect();
    // `view` is this viewport's own height, and it is not decoration:
    // the app measures the window differently (`PageBox.view`), and this
    // is the only side that knows the true one.
    return { x: r.left, y: r.top, w: r.width, h: r.height, view: window.innerHeight };
  }, []);

  /**
   * Puts the page where it belongs, **around whatever is floating over
   * it**.
   *
   * A native child webview always paints above this page's DOM — it is
   * not a z-index that can be raised, it is another layer of the window —
   * so anything that has to be seen over the page has to be made room
   * for. The first version of this simply took the page away while an
   * overlay was up, and the address bar's completions therefore blanked
   * the whole site: measured, and the reason this is not that.
   *
   * **An overlay that hangs down from above the page pushes its top
   * down**; the page stays on screen, shorter. That is the address bar's
   * completions, which is the one people see constantly. Anything
   * floating in the middle of the page still takes it away, because
   * there is no shape of rectangle that leaves a hole.
   */
  const place = useCallback(() => {
    const el = host.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    let top = r.top;
    let bottom = Infinity;
    let covered = false;
    for (const o of document.querySelectorAll(OVERLAYS)) {
      // **An overlay the page is inside is its ground, not over it**
      // (UX-37 bug A, the 附栏's browser seat on the phone): the sheet
      // is a dialog, and the page framed to the sheet's body was taken
      // for covered and put away — the seat could not open a page.
      if (o.contains(el)) continue;
      const b = o.getBoundingClientRect();
      if (b.width === 0 || b.height === 0) continue;
      const misses = b.bottom <= r.top || b.top >= r.bottom || b.right <= r.left || b.left >= r.right;
      if (misses) continue;
      // The pill's selector rises from the deck: keep the current native
      // page visible above it, with the same WebView and navigation state.
      if (o.matches('[data-deck-selector]')) { bottom = Math.min(bottom, b.top); continue; }
      // **A hand's width of tolerance, and it is not a fudge**: the
      // completion menu hangs from the address bar with a 4px gap under
      // it, so a test for "starts exactly at the page's top edge" put it
      // in the other branch and blanked the site — measured 2026-09-09,
      // and the whole reason this rule was written.
      if (b.top <= r.top + HANGS_FROM_ABOVE) top = Math.max(top, b.bottom);
      else covered = true;
    }
    // **Under a native deck the page runs to the screen's foot** (UX-13,
    // `lib/native-deck`): the deck is drawn over it and the page keeps a
    // bottom inset of the deck's height, so it scrolls under the glass.
    const native = host.current?.closest("[data-session-screen]")?.querySelector("[data-session-bar][data-native-deck]");
    const h = Math.min(bottom, native ? native.getBoundingClientRect().bottom : r.bottom) - top;
    // What was decided, for the gate (`check-face` bar r: a sheet over
    // the page takes it away).
    document.dispatchEvent(new CustomEvent("khor:page-place", { detail: { tab, covered, top, h } }));
    // A page squeezed to nothing is a page nobody can read; below this it
    // is honest to take it away rather than draw a sliver.
    if (covered || h < 24) {
      if (!away.current) {
        away.current = true;
        hidePage(tab).catch(() => {});
      }
      return;
    }
    // **Only when it actually moved, and this is not an optimisation.**
    // Moving a native webview cancels whatever the mouse is in the middle
    // of, so a `setFrame` that lands between a press and its release eats
    // the click. This is watched on the whole document, khor's own face
    // updates on its own beat, and before this every one of those beats
    // moved the page by nothing at all — measured 2026-09-09: the page
    // drew, scrolled, and would not answer a single click.
    const same = `${r.left},${top},${r.width},${h}`;
    if (same !== put.current) {
      put.current = same;
      movePage(tab, { x: r.left, y: top, w: r.width, h, view: window.innerHeight }).catch(() => {});
    }
    if (away.current) {
      away.current = false;
      showPage(tab).catch(() => {});
    }
  }, [tab]);

  // **Every address asks, and the machine decides whether that costs a
  // borrow.** The tab arriving pointed at a page is the same act as
  // somebody pressing Enter on it — both come through as a new `url`,
  // and both have to be asked, because a new address is a real
  // navigation even when the network it leaves through is the one
  // already held.
  //
  // This used to claim it spent one borrow per address, which read like
  // thrift and was the leak: `start_borrow` dialled a fresh lease, port
  // and session row on every call, so three pages on one machine left
  // three of each and the session list filled with rows nobody could
  // tell apart. **The count is settled where the borrows are** — one per
  // exit machine, so the second address here is handed the first one's
  // session and nothing new is spent.
  //
  // **It also runs when you come back to this tab**, because the body is
  // mounted afresh (the shell mounts only the open tab's). That is not a
  // reload: the app answers a page already pointed where it is asked to
  // go by showing it, untouched (`open_page`).
  useEffect(() => {
    const want = normalize(url);
    if (!want) return;
    const at = boxOf();
    if (!at) return;
    let stopped = false;
    const me = {};
    showing.set(tab, me);
    setOpening({
      url: want,
      state: "opening",
      word: here ? gui.web_opening_here : gui.web_opening(machine),
    });
    // The errand is out from this moment — the press, not the answer
    // (第十二轮). **Told even if this pane is about to be unmounted**:
    // the sidebar row outlives the pane, and the errand is really out.
    busy.current?.(true);
    // **An empty exit is the app's 「go direct」** (`open_page`): no
    // borrow, no proxy — this machine's own network, in khor's own page
    // view. It used to be refused as 「这就是本机的网,不用借」, which was
    // true of the *borrow* and said nothing about opening the page.
    openPage(tab, here ? "" : machine, want, at).then(
      (borrow) => {
        // **Written down before the unmount check, on purpose.** The
        // borrow is open on the far machine whether or not this pane is
        // still on screen; a record skipped because the tab was closed
        // mid-dial is a lease nobody can give back.
        if (!here) holdBorrow(device, borrow.session);
        holdPage(tab);
        exit.current = borrow.addr;
        // ⚠️ **Nothing is asked for a pane that is gone, and this is
        // where that rule was nearly lost.** The icon used to be fetched
        // after this check and was moved above it while wiring 第十二轮
        // up; that asked the node for an icon on every abandoned open —
        // a 5-second blocking call each time on the browser road — and
        // the window stopped seeing tabs another process had opened
        // within the time the gate allows (measured 2026-09-23:
        // 「the column never held two tabs」, green on the commit before).
        //
        // **And it says nothing about the errand either.** 「Loading」 is
        // a fact about the *tab*, and this pane is not the one holding
        // that tab any more — a live pane may already have started the
        // load this one is abandoning, and reporting 「done」 here would
        // put its ring out (measured the same day: React mounts an
        // effect, tears it down and mounts it again in development, and
        // the first one's answer arrived after the second had started).
        // A load nobody is left to finish is the ceiling's to end
        // (`browse/loading-rule.ts`).
        //
        // **Left during the dial: hidden again here, the second end of
        // the fix** (the owner, 2026-10-05, iPhone: 「出了浏览器这个页面
        // 还在」). The unmount's `hidePage` went out before the page
        // existed; the app now keeps that as state (`page_wanted`), and
        // this says it once more after the attach — unless another pane
        // has taken the tab up since (a tab left and come back to), whose
        // page this would hide.
        if (stopped) {
          if (!showing.has(tab)) hidePage(tab).catch(() => {});
          return;
        }
        setOpening({
          url: want,
          state: "opened",
          word: here ? gui.web_opened_here : gui.web_opened(machine),
        });
        onOpened(want);
        // **In the app the page itself says it is loading** (UX-38 §2):
        // the errand ends here, with the page pointed, and a load it began
        // keeps the light (`use-loading`). The icon is not loading.
        if (inApp) busy.current?.(false);
        // **The icon is the last errand, so it is the one that ends it**
        // — and it ends it however it goes: a site that answered「no
        // icon」answered. `finally` rather than a success branch, for the
        // same reason (第十二轮).
        void iconOf(want).finally(() => {
          // Same rule: a pane torn down while its icon was in flight is
          // not the one to say the tab has stopped loading.
          if (!stopped) busy.current?.(false);
        });
      },
      (e) => {
        if (stopped) return;
        // Nothing more is coming for this tab; whether it opened or not,
        // khor has stopped waiting.
        busy.current?.(false);
        setOpening({
          url: want,
          state: "failed",
          word: String(e instanceof Error ? e.message : e),
        });
      },
    );
    return () => {
      stopped = true;
      if (showing.get(tab) === me) showing.delete(tab);
      // **Hidden, not closed.** Leaving this tab is not closing it, and
      // a page that was torn down here would lose its scroll, its
      // half-filled form and its live connection every time somebody
      // glanced at another tab. Closing is the ledger's job, when the
      // tab itself is gone (`views/held`).
      hidePage(tab).catch(() => {});
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, machine, url, here]);

  // The page is a native view laid over this rectangle, not something in
  // the layout, so nothing moves it when the layout moves. This is the
  // only thing that does.
  //
  // **It catches size, not every move.** A rectangle that changes shape
  // fires the observer; one that slides without resizing does not, and
  // that is a real gap rather than a solved problem — nothing in this
  // landing does it today, and a page that ever drifts out of place is
  // this comment's fault, not a mystery.
  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const watch = new ResizeObserver(place);
    watch.observe(el);
    window.addEventListener("resize", place);
    return () => {
      watch.disconnect();
      window.removeEventListener("resize", place);
    };
  }, [tab, place]);

  // Every overlay in this app lands in the document as one of these, so
  // nothing has to remember to tell the page it is coming: a control
  // written next month is accounted for the day it exists.
  useEffect(() => {
    const watch = new MutationObserver(place);
    // Overlays arrive and leave as elements, so `childList` is the whole
    // question. `attributes` was in here too and is not: it fires on
    // every class this app rewrites, which is constantly.
    watch.observe(document.body, { childList: true, subtree: true });
    // **An overlay that rises into place is measured again when it has
    // risen** (UX-28, the owner's phone: the native page covered every
    // sheet). The phone's sheets slide up from under the screen's foot,
    // so at the mutation the tabs sheet stood at y 874..1082 on an 874
    // screen — nothing over the page — and nothing fired after it had
    // arrived. Its motion's end is the second look.
    const settled = (e: Event) => {
      if (e.target instanceof Element && e.target.closest(OVERLAYS)) place();
    };
    document.addEventListener("animationend", settled, true);
    document.addEventListener("transitionend", settled, true);
    place();
    return () => {
      watch.disconnect();
      document.removeEventListener("animationend", settled, true);
      document.removeEventListener("transitionend", settled, true);
    };
  }, [place]);

  // **The page is the one that knows where it is.** A link, a redirect, a
  // form post and the back arrow all move it without anybody here asking,
  // and before this the address bar went on showing the address somebody
  // typed — measured on 2026-09-09 with a link on baidu: the page moved
  // and the tab did not.
  //
  // **Subscribed once per tab, and the callbacks reach it through refs.**
  // Both of them arrive as inline arrows, so their identity changes every
  // render; an effect that depended on them would unsubscribe and
  // resubscribe each time, and since subscribing is asynchronous (the
  // event module is imported on demand) the teardown usually won.
  // Measured 2026-09-09: links stopped doing anything at all, because the
  // listener was almost never alive when the event arrived.
  const went = useRef(onWent);
  const popped = useRef(onPopped);
  went.current = onWent;
  popped.current = onPopped;
  useEffect(
    () =>
      onPageWord("page:went", tab, (u) => {
        went.current(u);
        // A page that moved may have moved to another site.
        iconOf(u);
      }),
    [tab, iconOf],
  );
  // What the page calls itself (docs/SHELL.md 第四轮:「标题是网页的标题而
  // 非网址」), as the webview reports it.
  const titled = useRef(onTitle);
  titled.current = onTitle;
  useEffect(() => onPageWord("page:title", tab, (t) => t.trim() && titled.current(t.trim())), [tab]);
  useEffect(() => onPageWord("page:popup", tab, (u) => popped.current(u)), [tab]);
  // …and a link that asked for a new window becomes a new tab. Carried,
  // not decided: `target="_blank"` on one link and not another is the
  // page author's distinction, and khor's job is to keep it.




  return (
    <section className="flex h-full min-w-0 flex-col">
      {opening && opening.state !== "opened" && (
        // **The one thing this must never be is blank.** A borrow that
        // was refused is the whole of what this pane has to say, in the
        // node's own sentence — the same rule the files body follows for
        // a directory it could not list.
        <div
          data-web-status={opening.state}
          data-cannot-reach={opening.state === "failed" ? "" : undefined}
          className={cn(
            "flex-none px-4 py-3",
            opening.state === "failed" ? "text-body" : "text-callout text-muted-foreground",
          )}
        >
          {opening.word}
        </div>
      )}
      {/* **The page goes here, and nothing in this page draws it.** The
          app lays a second webview over exactly this rectangle, so what
          is inside this div is whatever is left visible around the page
          — which is nothing, once one is up. It keeps the same shape
          whether a page is up or not, because it is the rectangle being
          reported and a box that changed size with the status would
          move the page under the person reading it. */}
      <div ref={host} data-native-page-tab={tab} data-page-host className="relative min-h-0 flex-1" />
      {/* ⚠️ 墓碑: a 置顶 button stood under the page here — 「the one
          place a page becomes a shortcut」. docs/SHELL.md 第四轮 第 3 条
          took it off (「浏览器底部不要有置顶按钮, 统一到侧边栏去操作」):
          pinning is the sidebar's, on the tab's own menu. */}
    </section>
  );
}
