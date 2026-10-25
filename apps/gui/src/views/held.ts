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
import { dropWebBorrow } from "@/api";
import { closeNativePage } from "@/lib/native-page-open";

class RetentionOwners {
  private collections = new Map<string, readonly string[]>();
  private published = new Set<string>();

  acquire(value: string, owner: string): void {
    if (this.published.has(owner)) return;
    const provisional = this.collections.get(owner) ?? [];
    if (!provisional.includes(value)) this.collections.set(owner, [...provisional, value]);
  }

  update(keep: readonly string[], owner: string): void {
    this.published.add(owner);
    this.collections.set(owner, [...keep]);
  }

  wanted(): Set<string> {
    return new Set([...this.collections.values()].flat());
  }
}

const borrowOwners = new RetentionOwners();
const pageOwners = new RetentionOwners();

/** device id → the borrow session opened for it. */
const open = new Map<string, string>();
const dropping = new Set<string>();

/**
 * Remembers the borrow a page was opened through.
 *
 * Idempotent by the same fact that makes the ledger machine-keyed: the
 * second page on a machine is handed back the first page's session, so
 * writing it again writes the same value.
 */
export function holdBorrow(device: string, session: string, owner = "page"): void {
  open.set(device, session);
  borrowOwners.acquire(device, owner);
  sweepBorrows();
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
export function releaseBorrows(keep: readonly string[], owner = "page"): void {
  borrowOwners.update(keep, owner);
  sweepBorrows();
}

function sweepBorrows(): void {
  const wanted = borrowOwners.wanted();
  for (const [device, session] of [...open]) {
    if (wanted.has(device)) continue;
    open.delete(device);
    if (dropping.has(session)) continue;
    dropping.add(session);
    void dropWebBorrow(session).catch(() => {
      /* nothing holds it but the serve's own process; see above */
    }).finally(() => { dropping.delete(session); });
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
 * The module regression checks the retirement calls and LRU ordering.
 * Those controlled native calls do not measure real page memory.
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
export function holdPage(tab: string, owner = "page"): void {
  const at = pages.indexOf(tab);
  if (at >= 0) pages.splice(at, 1);
  pages.push(tab);
  pageOwners.acquire(tab, owner);
  sweepPages();
}

/**
 * Ends the page of every tab that is no longer open.
 *
 * `keep` is the whole set of open tab ids, for the reason `releaseBorrows`
 * takes the whole set of machines: a caller that computed the difference
 * would need to know what this ledger holds.
 */
export function releasePages(keep: readonly string[], owner = "page"): void {
  pageOwners.update(keep, owner);
  sweepPages();
}

function sweepPages(): void {
  const wanted = pageOwners.wanted();
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
    closeNativePage(tab).catch(() => {
      /* a webview whose window is gone is already the state we wanted */
    });
  }
}

