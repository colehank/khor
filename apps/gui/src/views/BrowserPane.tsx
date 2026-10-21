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
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import { hidePage, inApp, movePage, onPageWord, pageIcon, showPage } from "@/api";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { openNativePage } from "@/lib/native-page-open";
import { nativePageYielded, visiblePageOwner, watchNativePageYield } from "@/lib/native-page-yield";
import { holdBorrow, holdPage } from "@/views/held";

/** A page's scheme is optional to type; a bare host means https. */
export function normalize(raw: string): string | null {
  const t = raw.trim();
  if (!t) return null;
  return /^[a-z]+:\/\//i.test(t) ? t : `https://${t}`;
}

/** Native webpages cannot be covered by DOM. Task UI takes the whole
 * page out of presentation until it finishes; the same handle stays alive.
 * Tooltip wrappers are filtered separately: hints never take the page away. */
const OVERLAYS =
  '[data-deck-selector],[data-radix-popper-content-wrapper],[data-omni-menu],[data-scrim],[data-slot="dialog-overlay"],[data-slot="popover-content"],[aria-modal="true"],[role="dialog"],[role="alertdialog"],[role="menu"],[role="listbox"]';

/** Presentation belongs to the latest mounted owner of a tab. Async work
 * may finish after that owner has left, but must not present its page. */
const showing = new Map<string, { refresh: () => void }>();
const presentations = new Map<string, Promise<unknown>>();
function present(tab: string, valid: () => boolean, work: () => Promise<unknown>): void {
  const previous = presentations.get(tab) ?? Promise.resolve();
  const next = previous.catch(() => {}).then(() => valid() ? work() : undefined).catch(() => {});
  presentations.set(tab, next);
  void next.finally(() => { if (presentations.get(tab) === next) presentations.delete(tab); });
}

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
  suspended = false,
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
  /** The shell is editing/choosing in this page's body. Keep its native
      handle mounted, but yield all of its presentation to DOM content. */
  suspended?: boolean;
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
  const presented = useRef<boolean | null>(null);
  const owner = useRef<object | null>(null);
  const revision = useRef(0);
  const suspension = useRef(suspended);
  suspension.current = suspended;
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

  /** Recompute presentation from the current owner and UI, never from the
   * overlay which just closed. A stale editor release cannot restore a page.
   * Overlays never modify the page's rectangle: hide, then restore in place. */
  const place = useCallback(() => {
    const el = host.current, me = owner.current;
    if (!el || !me || showing.get(tab) !== me) return;
    const r = el.getBoundingClientRect();
    let covered = suspension.current || nativePageYielded(tab) || !visiblePageOwner(el);
    for (const o of document.querySelectorAll(OVERLAYS)) {
      if (covered) break;
      if (o.contains(el) || !visiblePageOwner(o)) continue;
      // Radix wraps a tooltip in the same popper as a menu. The tooltip
      // itself declines overlapping native pages (ui/tooltip), without
      // hiding the site just because a pointer paused over a control.
      if (o.matches('[data-radix-popper-content-wrapper]') && o.querySelector('[role="tooltip"],[data-slot="tooltip-content"]')) continue;
      // A modal's scrim covers the whole window even when its short
      // question is beside this page. Its own page-bearing dialog is
      // ground, though: Radix renders that dialog just after its scrim.
      const ownDialog = el.closest('[role="dialog"],[role="alertdialog"]');
      if (o.matches('[data-scrim],[data-slot="dialog-overlay"]') && o.nextElementSibling === ownDialog) continue;
      if (o.matches('[aria-modal="true"]')) { covered = true; continue; }
      const b = o.getBoundingClientRect();
      if (b.width <= 0 || b.height <= 0) continue;
      covered = b.bottom > r.top && b.top < r.bottom && b.right > r.left && b.left < r.right;
    }
    const native = el.closest('[data-session-screen]')?.querySelector('[data-session-bar][data-native-deck]');
    const at = {x:r.left, y:r.top, w:r.width, h:(native ? native.getBoundingClientRect().bottom : r.bottom)-r.top, view:window.innerHeight};
    covered ||= at.w <= 0 || at.h < 24;
    const visible = !covered;
    if (el.dataset.nativePageVisible !== String(visible)) el.dataset.nativePageVisible = String(visible);
    document.dispatchEvent(new CustomEvent('khor:page-place', {detail:{tab, covered, top:at.y, h:at.h}}));
    const same = `${at.x},${at.y},${at.w},${at.h}`;
    // A redundant native move cancels pointer presses. Only actual host
    // geometry changes move it; a menu's geometry never does.
    const moved = visible && same !== put.current;
    const changed = away.current === visible;
    if (!moved && !changed && presented.current === visible) return;
    away.current = !visible;
    const version = ++revision.current;
    const current = () => owner.current === me && showing.get(tab) === me && revision.current === version;
    present(tab, current, async () => {
      if (!visible) {
        await hidePage(tab);
        if (owner.current === me) presented.current = false;
        return;
      }
      if (same !== put.current) {
        await movePage(tab, at);
        if (owner.current === me) put.current = same;
      }
      if (current() && presented.current !== true) {
        await showPage(tab);
        if (owner.current === me) presented.current = true;
      }
    });
  }, [tab]);

  useLayoutEffect(() => { place(); }, [suspended, place]);
  useLayoutEffect(() => watchNativePageYield(place), [place]);

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
    const me = {refresh: () => { presented.current = null; place(); }};
    showing.set(tab, me);
    owner.current = me;
    away.current = false;
    presented.current = null;
    put.current = "";
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
    openNativePage(tab, here ? "" : machine, want, at).then(
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
          const current = showing.get(tab);
          if (current) current.refresh();
          else present(tab, () => !showing.has(tab), () => hidePage(tab));
          return;
        }
        // open_page can attach/show after an editor already asked it to
        // hide. Reassert the latest owner's presentation after attachment.
        presented.current = null;
        place();
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
    place();
    return () => {
      stopped = true;
      if (owner.current === me) owner.current = null;
      if (showing.get(tab) !== me) return;
      showing.delete(tab);
      ++revision.current;
      if (host.current) host.current.dataset.nativePageVisible = "false";
      // **Hidden, not closed.** Leaving this tab is not closing it, and
      // a page that was torn down here would lose its scroll, its
      // half-filled form and its live connection every time somebody
      // glanced at another tab. Closing is the ledger's job, when the
      // tab itself is gone (`views/held`).
      present(tab, () => !showing.has(tab), () => hidePage(tab));
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
    let frame = 0;
    const track = () => {
      place();
      if (document.querySelector(OVERLAYS) && document.getAnimations().some(a => a.playState === "running" && a.effect instanceof KeyframeEffect && a.effect.target instanceof Element && a.effect.target.closest(OVERLAYS))) frame = requestAnimationFrame(track);
      else frame = 0;
    };
    const changed = () => { place(); if (!frame) frame = requestAnimationFrame(track); };
    const watch = new MutationObserver(changed);
    // Retained panes may be hidden without unmounting, and a popper can
    // move into the body after insertion. Re-evaluate those visibility
    // and geometry changes; unchanged frames still issue no native move.
    watch.observe(document.body, {childList:true, subtree:true, attributes:true,
      attributeFilter:['hidden','inert','aria-hidden','aria-modal','data-state','style','class']});
    // **An overlay that rises into place is measured again when it has
    // risen** (UX-28, the owner's phone: the native page covered every
    // sheet). The phone's sheets slide up from under the screen's foot,
    // so at the mutation the tabs sheet stood at y 874..1082 on an 874
    // screen — nothing over the page — and nothing fired after it had
    // arrived. Its motion's end is the second look.
    const settled = (e: Event) => {
      if (e.target instanceof Element && e.target.closest(OVERLAYS)) place();
    };
    document.addEventListener("animationstart", changed, true);
    document.addEventListener("transitionrun", changed, true);
    document.addEventListener("animationend", settled, true);
    document.addEventListener("transitionend", settled, true);
    place();
    return () => {
      watch.disconnect();
      cancelAnimationFrame(frame);
      document.removeEventListener("animationstart", changed, true);
      document.removeEventListener("transitionrun", changed, true);
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
