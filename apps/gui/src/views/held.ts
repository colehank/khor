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
const borrowOwners = new Map<string, readonly string[]>();
const pageOwners = new Map<string, readonly string[]>();
export function releaseBorrows(keep: readonly string[], owner = "page"): void {
  borrowOwners.set(owner, keep);
  const wanted = new Set([...borrowOwners.values()].flat());
  for (const [device, session] of [...open]) {
    if (wanted.has(device)) continue;
    open.delete(device);
    dropWebBorrow(session).catch(() => {
      /* nothing holds it but the serve's own process; see above */
    });
  }
}

/**
 * How many tabs keep a **live** page behind them (docs/SHELL.md 第 2 条:
 * 「藏着的标签页不设上限会吃光 16G 的机器。落一个上限……最近用过的 N 枚
 * 留着活的，其余只留地址和标题，点到再加载」).
 *
 * # 量过了 (验收方, 2026-09-21)
 *
 * On the real install, borrowing turing's network and opening one tab
 * at a time with `khor tabs open -m turing <url>`, twenty seconds
 * apart: **one `com.apple.WebKit.WebContent` process per tab**, and
 * resident memory up by 58 MB (rust-lang.org), 68 (wikipedia), 29
 * (news.ycombinator), 29 (github) — call it **46 MB a tab**. Closing
 * all four took the process count back to where it started (18 → 22 →
 * 18), so the release is real and not a leak wearing a cap.
 *
 * Eight is therefore two to three gigabytes at worst on a 16 GB
 * machine, which is what the ruling was protecting. **Eight stays.**
 *
 * ⚠️ **Those are light pages, freshly loaded.** A video site, or a web
 * app of the VS Code kind, runs to hundreds of megabytes each and was
 * **not** measured. If this number is ever revisited, that is the case
 * to measure — the light one is already known and it is not the one
 * that would hurt.
 *
 * ⚠️ **No gate watches the cap, and none can watch it cheaply.** It
 * only bites at the ninth live page, and 「the ninth tab let the first
 * one go」 is invisible on the glass: a tab whose page was released
 * looks exactly like a tab nobody has pressed yet, which is the point
 * of the design. The seam a gate would read is [`releasePages`]'s
 * `closePage` calls, and reaching nine of them means nine real borrows
 * against an engine this repo does not test with. So the ordering and
 * the arithmetic below are reviewed, not measured — the first person to
 * change either of them should know that.
 */
export const LIVE_TABS = 8;

/**
 * Tab ids with a live page webview, **most recently opened last**.
 *
 * An array rather than a set, because the cap needs an order and 「when
 * was this tab last used」 is not something the set could answer.
 * [`holdPage`] is the touch: it fires when a tab's page is opened, and
 * a tab's page is opened when somebody looks at it (`BrowserPane`
 * mounts only for the tab in front). So the tail of this list is the
 * most recently looked at, and the head is the one to let go of.
 */
const pages: string[] = [];

/**
 * Remembers that a tab has a page webview.
 *
 * The set is the face's, not the app's: the app knows its webviews by
 * label and would answer "which labels exist", which is not the question
 * — the question is which of *this landing's tabs* are still open, and
 * only this side has that list.
 */
export function holdPage(tab: string): void {
  const at = pages.indexOf(tab);
  if (at >= 0) pages.splice(at, 1);
  pages.push(tab);
}

/**
 * Ends the page of every tab that is no longer open.
 *
 * `keep` is the whole set of open tab ids, for the reason `releaseBorrows`
 * takes the whole set of machines: a caller that computed the difference
 * would need to know what this ledger holds.
 */
export function releasePages(keep: readonly string[], owner = "page"): void {
  pageOwners.set(owner, keep);
  const wanted = new Set([...pageOwners.values()].flat());
  // **Two reasons a page goes, and the order matters.** First the tabs
  // that are not open any more — those are gone for good and must not
  // count against the cap. What is left is every live page whose tab is
  // still open, most recently looked at last; anything past
  // [`LIVE_TABS`] from that end is let go.
  const closed = pages.filter((t) => !wanted.has(t));
  const alive = pages.filter((t) => wanted.has(t));
  const capped = alive.slice(0, Math.max(0, alive.length - LIVE_TABS));
  pages.length = 0;
  pages.push(...alive.slice(Math.max(0, alive.length - LIVE_TABS)));
  for (const tab of [...closed, ...capped]) {
    closePage(tab).catch(() => {
      /* a webview whose window is gone is already the state we wanted */
    });
  }
}

