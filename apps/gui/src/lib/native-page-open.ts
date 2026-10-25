/** A native page label can be constructed only once at a time. StrictMode
 * can mount two owners before the first IPC returns, so share identical
 * pending openings and serialize different destinations for that label.
 * Completed work is never cached: a later resume still calls open_page,
 * whose native reuse path preserves the page's own session history. */
import { closePage, openPage, type PageBox, type WebBorrow } from '@/api';

type Pending = { kind: "open"; target: string; promise: Promise<WebBorrow> }
  | { kind: "close"; promise: Promise<void> };
const pending = new Map<string, Pending>();

export function openNativePage(tab: string, machine: string, url: string, at: PageBox): Promise<WebBorrow> {
  const target = JSON.stringify([machine, url]), before = pending.get(tab);
  if (before?.kind === "open" && before.target === target) return before.promise;
  const start = () => openPage(tab, machine, url, at);
  const promise = before ? before.promise.catch(() => {}).then(start) : start();
  track(tab, { kind: "open", target, promise });
  return promise;
}

export function closeNativePage(tab: string): Promise<void> {
  const before = pending.get(tab);
  if (before?.kind === "close") return before.promise;
  const start = () => closePage(tab).then(() => {});
  const promise = before ? before.promise.catch(() => {}).then(start) : start();
  track(tab, { kind: "close", promise });
  return promise;
}

function track(tab: string, job: Pending): void {
  pending.set(tab, job);
  const retire = () => { if (pending.get(tab) === job) pending.delete(tab); };
  void job.promise.then(retire, retire);
}
