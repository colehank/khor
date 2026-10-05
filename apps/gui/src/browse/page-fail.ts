/**
 * **What a page that could not be opened says** (UX-22, the owner
 * 2026-10-05: a page that fails to load said nothing). The app hears
 * WebKit's failure (`page:fail`, `src-tauri/src/page_fail.rs`) as
 * `code|domain|description|url`; this sorts the code into the catalog's
 * few reasons — the ones a person can do something about — and keeps the
 * system's own sentence for anything else. Pure, so the face gate reads
 * it directly.
 */
import { gui } from "@/gen/catalog";

export type PageFail = { reason: string; url: string };

/** NSURLErrorDomain's codes, by what a person can do about them. */
const REASONS: [number[], () => string][] = [
  // NSURLErrorCannotFindHost, NSURLErrorDNSLookupFailed
  [[-1003, -1006], () => gui.page_fail_host],
  // NSURLErrorCannotConnectToHost, NSURLErrorNetworkConnectionLost
  [[-1004, -1005], () => gui.page_fail_refused],
  // NSURLErrorTimedOut
  [[-1001], () => gui.page_fail_timeout],
  // NSURLErrorNotConnectedToInternet, NSURLErrorInternationalRoamingOff,
  // NSURLErrorDataNotAllowed
  [[-1009, -1018, -1020], () => gui.page_fail_offline],
  // NSURLErrorSecureConnectionFailed … NSURLErrorClientCertificateRequired
  [[-1200, -1201, -1202, -1203, -1204, -1205, -1206], () => gui.page_fail_tls],
  // NSURLErrorAppTransportSecurityRequiresSecureConnection,
  // NSURLErrorUnsupportedURL
  [[-1022, -1002], () => gui.page_fail_blocked],
];

/** The word for one `page:fail`; `null` for one that cannot be read. */
export function pageFail(text: string): PageFail | null {
  const [code, domain, said, ...rest] = text.split("|");
  if (code === undefined || domain === undefined) return null;
  const n = Number(code);
  const url = rest.join("|");
  // khor's own proxy: the machine lent could not reach the address
  // (`khor_node::tunnel::PAGE_FAILED_TITLE`, a plain page), or WebKit's
  // word for the same refusal of a CONNECT (CFNetwork 310, an https page).
  if (domain === "khor" || (domain === "kCFErrorDomainCFNetwork" && n === 310)) return { reason: gui.page_fail_route, url };
  if (domain === "NSURLErrorDomain") {
    const hit = REASONS.find(([codes]) => codes.includes(n));
    if (hit) return { reason: hit[1](), url };
  }
  // WebKit's own refusals: the content cannot be shown, or the plug-in
  // or the policy stopped it.
  if (domain === "WebKitErrorDomain" && (n === 100 || n === 101 || n === 103 || n === 104)) {
    return { reason: gui.page_fail_blocked, url };
  }
  return { reason: (said ?? "").trim() || `${domain} ${code}`, url };
}
