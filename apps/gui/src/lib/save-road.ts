// **The editor saves on its own** (save-a, the owner 2026-10-08: 「编辑
// 文件要实时保存，保存按钮只是作为一个额外的确定」).
//
// A file is written a second after the typing stops and at once when the
// person leaves it (another tab, the tab closed, the page left, the app
// hidden or quit). ✓ and ⌘S write at once and say 已保存 on the button
// for a moment. Every write is the compare-and-write it always was
// (`khor_node::files::write_whole`): written only if the file is still
// what khor read, answering the new digest so editing goes on without a
// read.
//
// # Why a road outside the view
//
// `FileView` is remounted on every file, and leaving the file is exactly
// when the last write has to go — so the write cannot belong to the
// mount that asked for it. This module holds, per file:
//
// - **one write in flight, and one waiting per line of editing.** Newer
//   text replaces waiting text from the same buffer. Separate buffers
//   keep their own place and their own compare-and-write check.
// - **what khor's own writes moved the digest from and to, per line of
//   editing** (`by`, one per buffer read from the disk). A write pinned to
//   a digest this same buffer's earlier write already moved on from is
//   pinned forward, not refused: the mount that left and the mount that
//   came back hold one buffer. A buffer read from the disk separately is
//   another line and is never moved forward — two views of one file must
//   refuse each other, not overwrite each other.
// - **the trouble**, which outlives the mount: a clash (the file changed
//   somewhere else; autosave stops until the person picks a way out) or a
//   failure (unreachable, refused by the disk; edits stay in the buffer
//   and the road tries again, 2 s doubling to a minute).
//
// What the panes hold — the drafts and their dots — stays theirs
// (`lib/drafts`); a write that lands with no view mounted for its file
// tells the pane through the `orphan` its view handed over on leaving.
import { useSyncExternalStore } from "react";

import { fetchFileText, said, saveFileText, type FileText } from "@/api";

/** A write that landed. `by` is the line of editing it came from. */
export type Landed = {
  machine: string;
  path: string;
  by: string;
  text: string;
  digest: string;
  at_ms: number;
};

/** Why a file is not saved. `clash`: it changed on the disk since khor
    read it, and nothing is written until the person chooses. `failed`:
    the write did not happen (the machine, the disk); the road retries. */
export type Trouble = { kind: "clash" | "failed"; said: string };

/** What a file's road says to a screen. A new object on every change, so
    `useSyncExternalStore` sees it move and nothing else. */
export type RoadState = {
  trouble: Trouble | null;
  /** A write is flying or waiting. */
  busy: boolean;
  /** An asked-for save (✓, ⌘S) landed a moment ago: the button says
      已保存 instead of being a button. */
  confirmed: boolean;
};

/** A write to make: the text, the digest it is pinned to, its line. */
export type Pinned = { text: string; was: string; by: string };

/** Read a fresh base for an explicit overwrite. A pin starts a new editing
    line and is revoked by discard, even after the read has already resolved. */
export async function readForOverwrite(
  machine: string,
  path: string,
): Promise<{ file: FileText; pin: (text: string) => Pinned | null }> {
  const r = road(machine, path);
  const epoch = r.epoch;
  const file = await fetchFileText(machine, path);
  return {
    file,
    pin(text) {
      if (file.digest === "" || epoch !== r.epoch || hushAll || hushed.has(keyOf(machine, path))) return null;
      return { text, was: file.digest, by: newLine() };
    },
  };
}

type Waiter = { ok: (l: Landed) => void; no: (t: Trouble) => void };
type Job = Pinned & { epoch: number; asked: boolean; orphan?: () => void; waiters: Waiter[] };
type Line = { head: string; bases: Set<string> };

type Road = {
  machine: string;
  path: string;
  flying: Job | null;
  next: Job[];
  /** Confirmed head and known bases, per buffer. Content digests can repeat. */
  lines: Map<string, Line>;
  /** Forgetting a buffer revokes old jobs' retry and UI callbacks. */
  epoch: number;
  trouble: Trouble | null;
  /** The write that failed, held for the retry. */
  failed: Job | null;
  tries: number;
  retry: ReturnType<typeof setTimeout> | null;
  until: number;
  state: RoadState;
  heard: Set<(l: Landed) => void>;
};

/** **How long the typing has to stop** before a write. One second, the
    owner's number; the same on every machine until a slower road is
    measured to need more (save-a's report has the measurement). */
export const PAUSE_MS = 1000;
/** How long ✓ says 已保存 after an asked-for save lands. */
export const CONFIRM_MS = 1500;
const RETRY_FIRST_MS = 2000;
const RETRY_MOST_MS = 60_000;
/** Retries of a write no mounted view is listening for: 2 + 4 + … + 60
    s, about four minutes. */
const RETRY_UNSEEN = 8;

const roads = new Map<string, Road>();
const watchers = new Set<() => void>();
const anyLanded = new Set<(l: Landed) => void>();
let version = 0;

const keyOf = (machine: string, path: string) => `${machine}\u0000${path}`;

function road(machine: string, path: string): Road {
  const k = keyOf(machine, path);
  let r = roads.get(k);
  if (r === undefined) {
    r = {
      machine,
      path,
      flying: null,
      next: [],
      lines: new Map(),
      epoch: 0,
      trouble: null,
      failed: null,
      tries: 0,
      retry: null,
      until: 0,
      state: { trouble: null, busy: false, confirmed: false },
      heard: new Set(),
    };
    roads.set(k, r);
  }
  return r;
}

function changed(r: Road) {
  const state: RoadState = {
    trouble: r.trouble,
    busy: r.flying !== null || r.next.length > 0,
    confirmed: r.until > Date.now(),
  };
  const s = r.state;
  if (s.trouble !== state.trouble || s.busy !== state.busy || s.confirmed !== state.confirmed) {
    r.state = state;
  }
  version += 1;
  for (const w of watchers) w();
}

/** Where khor's own writes on this line took `was`. */
function forward(r: Road, by: string, was: string): string {
  const line = r.lines.get(by);
  return line?.bases.has(was) ? line.head : was;
}

/** **Writes a file through the road.** Resolves when this text (or a
    newer one that took its place) landed; rejects with the trouble when
    it did not. `asked` marks a save a person pressed for: it is said on
    the button when it lands. `orphan` is called when the write lands and
    no view of the file is listening and nothing newer waits — the view
    that asked has gone, and this is how its pane hears. */
export function write(
  machine: string,
  path: string,
  pin: Pinned,
  how: { asked?: boolean; orphan?: () => void } = {},
): Promise<Landed> {
  const r = road(machine, path);
  const asked = how.asked === true;
  // Thrown away on purpose (不保存): the write a view makes as it leaves
  // must not bring it back. A save somebody presses for still goes.
  if (!asked && (hushAll || hushed.has(keyOf(machine, path)))) {
    return Promise.reject<Landed>({ kind: "failed", said: "" } satisfies Trouble);
  }
  return new Promise<Landed>((ok, no) => {
    const waiter = { ok, no };
    // The same text from the same line is already flying: wait for it.
    const f = r.flying;
    if (f !== null && f.epoch === r.epoch && r.next.length === 0 && f.by === pin.by && f.text === pin.text) {
      f.waiters.push(waiter);
      f.asked ||= asked;
      f.orphan = how.orphan ?? f.orphan;
      return;
    }
    if (r.retry !== null) {
      clearTimeout(r.retry);
      r.retry = null;
    }
    r.failed = null;
    const job: Job = { ...pin, epoch: r.epoch, asked, orphan: how.orphan, waiters: [waiter] };
    if (r.flying !== null) {
      // Only the same buffer's newer text can answer its older waiters.
      const at = r.next.findIndex((n) => n.by === job.by && n.epoch === job.epoch);
      if (at >= 0) {
        const n = r.next[at];
        job.waiters.unshift(...n.waiters);
        job.asked ||= n.asked;
        job.orphan ??= n.orphan;
        r.next[at] = job;
      } else r.next.push(job);
      changed(r);
      return;
    }
    start(r, job);
  });
}

function start(r: Road, job: Job) {
  r.flying = job;
  r.failed = null;
  changed(r);
  const was = forward(r, job.by, job.was);
  const done = (l: Landed) => {
    if (job.epoch === r.epoch) {
      const line = r.lines.get(job.by) ?? { head: l.digest, bases: new Set<string>() };
      line.bases.add(was);
      line.bases.add(l.digest);
      line.head = l.digest;
      r.lines.set(job.by, line);
      r.trouble = null;
      r.failed = null;
      r.tries = 0;
      if (job.asked) confirm(r);
    }
    // Cancellation cannot undo a write already accepted by the disk.
    for (const h of anyLanded) h(l);
    for (const h of r.heard) h(l);
    if (job.epoch === r.epoch && r.heard.size === 0 && r.next.length === 0) job.orphan?.();
    for (const w of job.waiters) w.ok(l);
  };
  const landed = (digest: string, at_ms: number): Landed => ({
    machine: r.machine,
    path: r.path,
    by: job.by,
    text: job.text,
    digest,
    at_ms,
  });
  saveFileText(r.machine, r.path, was, job.text)
    .then((w) => done(landed(w.digest, w.at_ms)))
    .catch(async (e: unknown) => {
      const why = said(e);
      // **What the disk holds now decides which trouble it is**, not the
      // words of the refusal (they are the far machine's, in its
      // language). Read once: our own text there means the write landed
      // and only its answer was lost; the digest we were pinned to means
      // nothing moved and the write failed; anything else changed under
      // the buffer. Unreadable is a failure, and the retry finds out.
      let now: { text: string; digest: string; at_ms: number } | null = null;
      try {
        now = await fetchFileText(r.machine, r.path);
      } catch {
        now = null;
      }
      if (now !== null && now.digest !== "" && now.text === job.text) {
        done(landed(now.digest, now.at_ms));
        return;
      }
      const clash = now !== null && now.digest !== was;
      const t: Trouble = { kind: clash ? "clash" : "failed", said: why };
      if (job.epoch === r.epoch) {
        r.trouble = t;
        if (!clash) {
          r.failed = { ...job, asked: false, waiters: [] };
          r.tries += 1;
        }
      }
      for (const w of job.waiters) w.no(t);
    })
    .finally(() => {
      r.flying = null;
      const n = r.next.shift();
      if (n !== undefined) start(r, n);
      else {
        // **Retried while somebody can see it**, and a few minutes more
        // when nobody can: a file deleted or renamed after its view left
        // is refused every time, and a road nobody reads must not knock
        // on a machine for ever. The pane still holds the draft and its
        // dot, so the next open writes again.
        if (r.failed !== null && r.failed.epoch === r.epoch && r.retry === null && (r.heard.size > 0 || r.tries <= RETRY_UNSEEN)) {
          const wait = Math.min(RETRY_MOST_MS, RETRY_FIRST_MS * 2 ** Math.max(0, r.tries - 1));
          r.retry = setTimeout(() => {
            r.retry = null;
            const j = r.failed;
            if (j === null || j.epoch !== r.epoch || r.flying !== null) return;
            r.failed = null;
            start(r, j);
          }, wait);
        }
        changed(r);
      }
    });
}

function confirm(r: Road) {
  r.until = Date.now() + CONFIRM_MS;
  setTimeout(() => changed(r), CONFIRM_MS + 20);
  changed(r);
}

/** An asked-for save with nothing to write: the file already is what is
    on screen, and the button says so. */
export function confirmSaved(machine: string, path: string) {
  confirm(road(machine, path));
}

/** **Forget the trouble and anything waiting** — the person threw the
    buffer away (重新读取, 不保存) or merged it onto what is there now.
    A write already flying finishes; nothing is retried after it. */
export function forget(machine: string, path: string) {
  const r = roads.get(keyOf(machine, path));
  if (r === undefined) return;
  r.epoch += 1;
  r.lines.clear();
  if (r.retry !== null) clearTimeout(r.retry);
  r.retry = null;
  r.failed = null;
  r.tries = 0;
  r.trouble = null;
  r.until = 0;
  for (const n of r.next) for (const w of n.waiters) w.no({ kind: "failed", said: "" });
  r.next = [];
  changed(r);
}

const hushed = new Set<string>();
let hushAll = false;

/** **The person said 不保存** for this file (a tab closed over its
    draft): forget it, and refuse the write its view makes on the way
    out. Lifted when the file is opened again (`reopened`). */
export function discard(machine: string, path: string) {
  forget(machine, path);
  hushed.add(keyOf(machine, path));
}

/** A view of the file mounted: writes from it are wanted again. */
export function reopened(machine: string, path: string) {
  hushed.delete(keyOf(machine, path));
}

/** **不保存，退出**: nothing more is written on the way out, by a
    leaving view or by the window being hidden as it closes. */
export function discardAll() {
  hushAll = true;
  for (const r of roads.values()) forget(r.machine, r.path);
}

/** What the road says about one file right now. */
export function roadOf(machine: string, path: string): RoadState {
  return roads.get(keyOf(machine, path))?.state ?? IDLE;
}
const IDLE: RoadState = { trouble: null, busy: false, confirmed: false };

function subscribe(w: () => void) {
  watchers.add(w);
  return () => {
    watchers.delete(w);
  };
}

/** One file's road, as a screen reads it. */
export function useRoad(machine: string, path: string): RoadState {
  return useSyncExternalStore(subscribe, () => roadOf(machine, path));
}

/** A version that moves whenever any road changes, for a holder of many
    files (a tab strip) that reads `roadOf` for each. */
export function useRoads(): number {
  return useSyncExternalStore(subscribe, () => version);
}

/** The view of one file hears its writes land, whichever mount asked. */
export function onLanded(machine: string, path: string, h: (l: Landed) => void): () => void {
  const r = road(machine, path);
  r.heard.add(h);
  return () => {
    r.heard.delete(h);
  };
}

/** Every write that lands, for the caches that outlive a view. */
export function onAnyLanded(h: (l: Landed) => void): () => void {
  anyLanded.add(h);
  return () => {
    anyLanded.delete(h);
  };
}

/** A new line of editing: a buffer just read from the disk. */
let lines = 0;
export function newLine(): string {
  lines += 1;
  return `l${lines}`;
}

// ── leaving everything at once ──────────────────────────────────────

/** Each mounted editor's 「write what you would autosave, now」, by
    file, for the moments nothing will unmount it first: the app hidden,
    the window closing, a tab closed by ×. Answers `null` when there is
    nothing it would write (clean, a clash, a merge not looked at). */
type Flush = () => Promise<unknown> | null;
const flushers = new Map<string, Set<Flush>>();

export function holdFlush(machine: string, path: string, f: Flush): () => void {
  const k = keyOf(machine, path);
  let set = flushers.get(k);
  if (set === undefined) flushers.set(k, (set = new Set()));
  set.add(f);
  return () => {
    set.delete(f);
    if (set.size === 0) flushers.delete(k);
  };
}

/** One file's mounted editors write now; `null` when none would. */
export function flushFile(machine: string, path: string): Promise<unknown> | null {
  const all: Promise<unknown>[] = [];
  for (const f of flushers.get(keyOf(machine, path)) ?? []) {
    const p = f();
    if (p !== null) all.push(p);
  }
  return all.length === 0 ? null : Promise.allSettled(all);
}

/** Writes every open editor's unsaved text now, and resolves when every
    write has landed or failed — never rejects. */
export async function flushAll(): Promise<void> {
  const all: Promise<unknown>[] = [];
  for (const set of flushers.values()) {
    for (const f of set) {
      const p = f();
      if (p !== null) all.push(p);
    }
  }
  // Writes that were already under way count too.
  for (const r of roads.values()) {
    if (r.flying !== null || r.next.length > 0) {
      all.push(
        new Promise<void>((done) => {
          const j = r.next.at(-1) ?? r.flying;
          j?.waiters.push({ ok: () => done(), no: () => done() });
        }),
      );
    }
  }
  await Promise.allSettled(all);
}

// **Hidden is leaving** (the phone going to the background, the window
// minimised, the tab switched in a browser): the last thing a page is
// sure to be told. `pagehide` for a browser tab closing.
if (typeof document !== "undefined") {
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) void flushAll();
  });
  window.addEventListener("pagehide", () => void flushAll());
}
