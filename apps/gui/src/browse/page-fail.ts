/** Short, actionable wording projected from the original bounded NSError.
 * The domain/code/description/URL remain available in the native snapshot;
 * a configured proxy alone is never evidence of a route failure. */
import { gui } from "@/gen/catalog";
import type { PageNavigationError } from "@/gen/bindings/PageNavigationError";

export type PageFail = { reason: string; url: string; pass?: boolean; detail: string };
/** Keep aligned with native page_trust's certificate override policy. */
const PASSABLE = [-1201, -1202, -1203, -1204];
const REASONS: [number[], () => string][] = [
  [[-1003, -1006], () => gui.page_fail_host],
  [[-1004], () => gui.page_fail_connect],
  [[-1005], () => gui.page_fail_lost],
  [[-1001], () => gui.page_fail_timeout],
  [[-1009, -1018, -1020], () => gui.page_fail_offline],
  [[-1200, -1201, -1202, -1203, -1204, -1205, -1206], () => gui.page_fail_tls],
  [[-1022, -1002], () => gui.page_fail_blocked],
];

export function pageFail(error: PageNavigationError | null): PageFail | null {
  if (!error) return null;
  const { domain, code, description, url } = error;
  const n = Number(code);
  const detail = [error, ...error.underlying].map(cause => `${cause.domain} ${cause.code}: ${cause.description}${cause.url ? `\n${cause.url}` : ""}`).join("\n");
  const result = (reason: string, pass?: boolean): PageFail => ({ reason, url, detail, ...(pass ? { pass } : {}) });
  // An explicit proxy failure document or CFNetwork proxy-connection
  // failure is evidence; NSURLErrorNetworkConnectionLost alone is not.
  if ((domain === "khor" && code === "route") || (domain === "kCFErrorDomainCFNetwork" && n === 310)) return result(gui.page_fail_route);
  if (domain === "NSURLErrorDomain") {
    if (PASSABLE.includes(n)) return result(gui.page_fail_cert, true);
    const reason = REASONS.find(([codes]) => codes.includes(n));
    if (reason) return result(reason[1]());
  }
  if (domain === "WebKitErrorDomain" && [100, 101, 103, 104].includes(n)) return result(gui.page_fail_blocked);
  return result(description.trim() || `${domain} ${code}`);
}
