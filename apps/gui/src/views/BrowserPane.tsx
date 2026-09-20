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

import { fetchWebPins, hidePage, movePage, onPageWord, openPage, pinWeb, showPage } from "@/api";
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
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
  '[data-radix-popper-content-wrapper],[data-omni-menu],[role="dialog"],[role="tooltip"],[role="menu"]';

/** What the open action is doing or has done, said in place. */
type Opening = { url: string; state: "opening" | "opened" | "failed"; word: string };

export function BrowserPane({
  tab,
  machine,
  device,
  url,
  onOpened,
  onWent,
  onPopped,
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
  // This machine's pinned pages — only so the control below can show
  // which of its two jobs it is about to do.
  const [pins, setPins] = useState<string[]>([]);

  const loadPins = () => {
    fetchWebPins()
      .then((rows) => setPins(rows.filter((r) => r.device === device).map((r) => r.url)))
      .catch(() => {});
  };
  useEffect(loadPins, [device]);

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
    let covered = false;
    for (const o of document.querySelectorAll(OVERLAYS)) {
      const b = o.getBoundingClientRect();
      if (b.width === 0 || b.height === 0) continue;
      const misses = b.bottom <= r.top || b.top >= r.bottom || b.right <= r.left || b.left >= r.right;
      if (misses) continue;
      // **A hand's width of tolerance, and it is not a fudge**: the
      // completion menu hangs from the address bar with a 4px gap under
      // it, so a test for "starts exactly at the page's top edge" put it
      // in the other branch and blanked the site — measured 2026-09-09,
      // and the whole reason this rule was written.
      if (b.top <= r.top + HANGS_FROM_ABOVE) top = Math.max(top, b.bottom);
      else covered = true;
    }
    const h = r.bottom - top;
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
    setOpening({
      url: want,
      state: "opening",
      word: gui.web_opening(machine),
    });
    openPage(tab, machine, want, at).then(
      (borrow) => {
        // **Written down before the unmount check, on purpose.** The
        // borrow is open on the far machine whether or not this pane is
        // still on screen; a record skipped because the tab was closed
        // mid-dial is a lease nobody can give back.
        holdBorrow(device, borrow.session);
        holdPage(tab);
        if (stopped) return;
        setOpening({
          url: want,
          state: "opened",
          word: gui.web_opened(machine),
        });
        onOpened(want);
      },
      (e) => {
        if (stopped) return;
        setOpening({
          url: want,
          state: "failed",
          word: String(e instanceof Error ? e.message : e),
        });
      },
    );
    return () => {
      stopped = true;
      // **Hidden, not closed.** Leaving this tab is not closing it, and
      // a page that was torn down here would lose its scroll, its
      // half-filled form and its live connection every time somebody
      // glanced at another tab. Closing is the ledger's job, when the
      // tab itself is gone (`views/held`).
      hidePage(tab).catch(() => {});
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, machine, url]);

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
    place();
    return () => watch.disconnect();
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
  useEffect(() => onPageWord("page:went", tab, (u) => went.current(u)), [tab]);
  useEffect(() => onPageWord("page:popup", tab, (u) => popped.current(u)), [tab]);
  // …and a link that asked for a new window becomes a new tab. Carried,
  // not decided: `target="_blank"` on one link and not another is the
  // page author's distinction, and khor's job is to keep it.


  const here = normalize(url);
  const pinned = here !== null && pins.includes(here);

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
            opening.state === "failed" ? "text-fact" : "text-sm text-muted-foreground",
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
      <div ref={host} data-page-host className="relative min-h-0 flex-1" />
      {here && (
        // The one place a page becomes a shortcut. Two words for one
        // control, the pin's rule: it shows what pressing it will do.
        <div className="flex-none border-t p-2">
          <Button
            variant="ghost"
            size="sm"
            data-pin-web
            data-on={pinned}
            aria-label={pinned ? gui.unpin_place : gui.pin_place}
            // **The finger's floor, missing until 2026-09-21.** It
            // measured 78×24 — `size="sm"` is what a control *looks*
            // like beside text and `--touch` is what a finger needs,
            // and the two are allowed to disagree only when the floor
            // is written down as well (tokens.css). Invisible to
            // `check-face` for the same reason the address's resting
            // face was: the browser stop's fixture had no tab pointed
            // anywhere, so nothing that only draws over a live page was
            // ever walked.
            className="min-h-touch text-muted-foreground"
            onClick={() => {
              pinWeb(machine, here, !pinned)
                .then(loadPins)
                .catch(() => {});
            }}
          >
            <IconPin pinned={pinned} />
            <span className="ml-2">{pinned ? gui.unpin : gui.pin}</span>
          </Button>
        </div>
      )}
    </section>
  );
}
