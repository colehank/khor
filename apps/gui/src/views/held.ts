// What the browsing landing is holding, and what gives each of it back.
//
// **Two things, swept by two different questions.** A borrow is held per
// **exit machine** (one machine is one borrow however many tabs point at
// it), so it goes when the last tab stops naming that machine. A page
// webview is held per **tab**, so it goes when that tab closes. Reading
// one rule off the other is how a page would outlive its tab, or a
// network be given back under a page still using it.
//
// **Why a ledger and not a `useEffect` cleanup.** The obvious place to
// close a borrow is where it was opened — `BrowserPane`, on unmount.
// That is wrong here, and quietly: the browsing shell mounts only the
// **open** tab's body (`BrowseLanding`), so a body unmounts every time
// somebody looks at another tab. Releasing there would tear down the far
// machine's lease on a tab switch, and the tab switched *away from*
// would come back to a dead page.
//
// **Why keyed by machine and not by tab.** One exit machine is one
// borrow however many pages point at it (`Node::start_borrow`), so the
// session two tabs on the same machine hold is the same session — the
// question worth asking is not "did this tab close" but "is anybody
// still pointed at this machine".
//
// **Module state, deliberately.** It has to outlive every component
// here: the landing itself unmounts when somebody walks to another
// screen, and the browsing windows it opened do not close with it.
import { closePage, dropWebBorrow } from "@/api";

/** device id → the borrow session opened for it. */
const open = new Map<string, string>();

/**
 * Remembers the borrow a page was opened through.
 *
 * Idempotent by the same fact that makes the ledger machine-keyed: the
 * second page on a machine is handed back the first page's session, so
 * writing it again writes the same value.
 */
export function holdBorrow(device: string, session: string): void {
  open.set(device, session);
}

/**
 * Gives back every borrow no tab is pointed at any more.
 *
 * `keep` is the machines the open tabs name — the whole set, not a
 * difference. A caller that computed the difference itself would have to
 * know what this ledger holds, and then two places would have to agree
 * about it.
 *
 * **Forgets before it asks, and does not put back on failure.** A drop
 * that did not land leaves a proxy port held by the serve's own process
 * and nothing else, so it goes when the serve does; a drop retried
 * forever would ask a machine that is not answering on every keystroke
 * that changes the tabs.
 *
 * That is the honest cost of this borrow having no row: nothing sweeps
 * it on a tick, because there is no row to notice missing. The trade was
 * taken deliberately (docs/NET.md 可达性) — a row that exists only so
 * something can reap it is a session in the list that is not a session.
 */
export function releaseBorrows(keep: readonly string[]): void {
  const wanted = new Set(keep);
  for (const [device, session] of [...open]) {
    if (wanted.has(device)) continue;
    open.delete(device);
    dropWebBorrow(session).catch(() => {
      /* nothing holds it but the serve's own process; see above */
    });
  }
}

/** Tab ids that have a live page webview behind them. */
const pages = new Set<string>();

/**
 * Remembers that a tab has a page webview.
 *
 * The set is the face's, not the app's: the app knows its webviews by
 * label and would answer "which labels exist", which is not the question
 * — the question is which of *this landing's tabs* are still open, and
 * only this side has that list.
 */
export function holdPage(tab: string): void {
  pages.add(tab);
}

/**
 * Ends the page of every tab that is no longer open.
 *
 * `keep` is the whole set of open tab ids, for the reason `releaseBorrows`
 * takes the whole set of machines: a caller that computed the difference
 * would need to know what this ledger holds.
 */
export function releasePages(keep: readonly string[]): void {
  const wanted = new Set(keep);
  for (const tab of [...pages]) {
    if (wanted.has(tab)) continue;
    pages.delete(tab);
    closePage(tab).catch(() => {
      /* a webview whose window is gone is already the state we wanted */
    });
  }
}
