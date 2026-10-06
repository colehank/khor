/**
 * **What a page's download says** (UX-42 §16: a download is never
 * silent). The app tells `page:download` as `state|what`: `started|name`
 * while it runs, `offered|name` once the phone's share sheet holds the
 * file (nothing more to say), `saved|path` once the Mac has put it in
 * Downloads, `failed|name`. Pure, so the face gate reads it directly.
 */
import { gui } from "@/gen/catalog";

export type PageSaid = { text: string; reveal?: string };

/** The caption's sentence for one `page:download`; `null` says nothing. */
export function pageDownload(word: string): PageSaid | null {
  const cut = word.indexOf("|");
  if (cut < 0) return null;
  const state = word.slice(0, cut);
  const what = word.slice(cut + 1);
  if (state === "started") return { text: gui.page_downloading(what) };
  if (state === "saved") return { text: gui.took_file(what), reveal: what };
  if (state === "failed") return { text: gui.page_download_failed(what) };
  return null;
}
