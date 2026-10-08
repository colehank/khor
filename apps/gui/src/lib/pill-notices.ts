// **What stops this session, for its pill** (NOTICE-BRIEF, the owner
// 2026-10-08: 「能上药丸且持久不消失的，只能是这个session那些阻碍继续的」).
//
// Two kinds reach the pill besides a transfer under way (which is the
// tab's act, `lib/tab-acts`): a transfer that did not get there, which
// stays until 重试 or 取消, and the session reconnecting, which goes by
// itself when the session is back. The view that holds the session — a
// terminal, a conversation — **offers** them here by tab id, as it
// offers its act, and the pill **reads** them: the phone's deck band, the
// wide strip's front tab, and the view the pill grows into.
//
// Nothing else goes here: no other session's events (the list's lights
// say them), no 待批 (the face's light), no notes that change no action,
// no momentary flashes.
import { useSyncExternalStore } from "react";

import type { Miss } from "@/api";
import { gui } from "@/gen/catalog";

/** A line in parts: words are never cut, the one name is cut in its
    middle when the pill is short of room. */
export type Say = (string | { name: string })[];

export type NoticeAct = {
  key: "retry" | "cancel" | "now";
  word: string;
  /** The quiet one of a pair (取消 beside 重试). */
  quiet?: boolean;
  run: () => void;
};

export type PillNotice = {
  /** Unique within its tab; a notice offered again under the same id
      replaces the one there (a second failure of the same drop). */
  id: string;
  kind: "failed" | "rejoin";
  /** On the pill. */
  short: Say;
  /** Grown: the whole sentence. */
  whole: string;
  /** Grown, under it in `secondary`: why, or what it waits for. */
  how?: string;
  /** 详情: the words the machine or the network came back with. */
  raw?: string;
  /** Reconnecting: since when (the seconds trailing the line). */
  since?: number;
  /** When it came: the newest first within a kind. */
  at: number;
  acts: NoticeAct[];
};

const notices = new Map<string, Map<string, PillNotice>>();
const listeners = new Set<() => void>();
let version = 0;

function changed() {
  version += 1;
  for (const l of listeners) l();
}

/** The view that holds tab `tab` offers a notice; returns its withdrawal. */
export function offerNotice(tab: string, n: PillNotice): () => void {
  let mine = notices.get(tab);
  if (!mine) notices.set(tab, (mine = new Map()));
  mine.set(n.id, n);
  changed();
  return () => withdrawNotice(tab, n.id, n);
}

/** Takes notice `id` off tab `tab` — only `which`, when given, so a
    late withdrawal does not take a newer notice under the same id. */
export function withdrawNotice(tab: string, id: string, which?: PillNotice) {
  const mine = notices.get(tab);
  const n = mine?.get(id);
  if (!mine || !n || (which && n !== which)) return;
  mine.delete(id);
  if (mine.size === 0) notices.delete(tab);
  changed();
}

/** Reconnecting before a failure — nothing else works until the session
    is back — and the newest first within each. */
const RANK = { rejoin: 0, failed: 1 } as const;

/** What stops tab `tab`, the most urgent first. */
export function noticesOf(tab: string): PillNotice[] {
  const mine = notices.get(tab);
  if (!mine) return [];
  return [...mine.values()].sort((a, b) => RANK[a.kind] - RANK[b.kind] || b.at - a.at);
}

/** The notices as the pills read them: a version that moves on every
    offer and withdrawal. */
export function useNotices(): number {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => version,
    () => version,
  );
}

/** A catalog line with one name in it, in parts (`Say`): the template is
    asked with a mark where the name goes and cut there. */
export function sayWith(line: (name: string) => string, name: string): Say {
  const MARK = "\u0000";
  const [head, tail = ""] = line(MARK).split(MARK);
  return [head, { name }, tail].filter((p) => p !== "");
}

export function plainOf(say: Say): string {
  return say.map((p) => (typeof p === "string" ? p : p.name)).join("");
}

const IMAGE = /\.(png|jpe?g|gif|heic|heif|webp|bmp|tiff?|avif)$/i;

/**
 * **A transfer that did not get there, said in the pill's words** (the
 * failure in parts, `khor_gui_core::miss`): 「图片未传到 Mac-mini」 on the
 * pill, 「图片没有传到 Mac-mini」 grown with why under it, the machine's
 * own words under 详情 only where they say more than ours (a dial's, a
 * far refusal's). 重试 where trying again can help — not for a khor too
 * old to take files or a file over the cap — and always 取消.
 */
export function failedNotice({
  id,
  miss,
  machine,
  what,
  at = Date.now(),
  capMb,
  retry,
  cancel,
}: {
  id: string;
  miss: Miss;
  /** The session's machine, for a failure that names none. */
  machine: string | null;
  /** The files' names, or `say` for a conversation's message. */
  what: string[] | "say";
  at?: number;
  /** The drop's cap, for `too-big`. */
  capMb?: number;
  retry: () => void;
  cancel: () => void;
}): PillNotice {
  const m = miss.machine ?? machine ?? gui.machine_here;
  const lines =
    what === "say"
      ? [gui.notice_say_failed_short, gui.notice_say_failed]
      : what.length > 0 && what.every((n) => IMAGE.test(n))
        ? [gui.notice_image_failed_short, gui.notice_image_failed]
        : [gui.notice_file_failed_short, gui.notice_file_failed];
  const how = {
    unreached: gui.notice_why_unreached(m),
    "host-lost": gui.notice_why_unreached(m),
    "serve-gone": gui.notice_why_serve_gone,
    predates: gui.notice_why_predates(m),
    "too-big": gui.notice_why_too_big(capMb ?? 0),
    cancelled: undefined,
    other: gui.notice_why_other(m),
  }[miss.kind];
  const again = miss.kind !== "predates" && miss.kind !== "too-big";
  const raw = (miss.kind === "unreached" || miss.kind === "other") && miss.raw ? miss.raw : undefined;
  return {
    id,
    kind: "failed",
    short: sayWith(lines[0], m),
    whole: lines[1](m),
    how,
    raw,
    at,
    acts: [
      ...(again ? [{ key: "retry" as const, word: gui.region_retry, run: retry }] : []),
      { key: "cancel" as const, word: gui.delete_keep, quiet: again, run: cancel },
    ],
  };
}

/**
 * **The session reconnecting** (NOTICE-BRIEF): 「重新连接 Mac-mini」 and
 * its seconds on the pill — the pill's ~120 for words holds that whole;
 * grown, 「正在重新连接 Mac-mini」 and what it waits for, the machine or
 * its host for this session, and 立即重试.
 */
export function rejoinNotice({ stall, machine, since, now }: { stall: Miss; machine: string | null; since: number; now: () => void }): PillNotice {
  const m = stall.machine ?? machine;
  return {
    id: "rejoin",
    kind: "rejoin",
    short: m ? sayWith(gui.notice_rejoin_short, m) : [gui.term_rejoining_here],
    whole: m ? gui.term_rejoining(m) : gui.term_rejoining_here,
    how: m ? (stall.kind === "host-lost" ? gui.notice_host_lost_how(m) : gui.term_rejoining_how(m)) : gui.term_rejoining_how_here,
    raw: stall.raw || undefined,
    since,
    at: since,
    acts: m ? [{ key: "now", word: gui.notice_retry_now, run: now }] : [],
  };
}
