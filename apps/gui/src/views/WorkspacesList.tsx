// The workspace landing's list: every declared workspace in one column
// (docs/UX.md 收件箱型 — this list is the thing being watched, because
// what happens inside a workspace happens while you are not looking).
//
// **One read, and no machine is asked for it.** A workspace is a
// decision and decisions replicate (`khor_sync::workspaces`), so the
// whole list is on this disk. The four states this file used to keep —
// 没问过 / 在问 / 它答不了 / 它答了 — are gone with the question: there
// is no machine that could refuse a list nobody asks it for.
//
// **What still needs a far machine is what a row shows**, and those are
// asked one question per row: its git marks and when it last changed.
// Either can fail, and when it does the row keeps its place and loses
// that corner — never the row. A workspace whose machine is asleep is
// still a workspace, and it is still openable.
//
// **Re-asked on the way back into the window, never on a timer** — the
// same rule `WorkspacePane` holds for the git marks, and the same
// reason: a beat is a dial plus a walk of somebody else's disk for an
// answer that is almost always the one already on screen, and coming
// back to the window is the moment staleness starts to mislead.
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";

import {
  fetchCheckout,
  fetchGitStatus,
  fetchTouched,
  fetchWorkspaces,
  wsPin,
  wsRm,
  type DeviceRow,
  type GitStatus,
  type WorkspaceRow,
} from "@/api";
import { IconBranch, IconFiles } from "@/components/icons";
import { type OmniAxis } from "@/components/Omnibox";
import { type FilterOption } from "@/components/PaneBar";
import { type RowAction } from "@/components/RowMenu";
import { SideList, type SideRow } from "@/components/SideList";
import { gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { type ListAxis, listAxes, listOptions, sieve } from "@/lib/list-filter";
import { ago, axisOf, groupLabel, valueOf } from "@/words";
import { IGNORED } from "@/views/fileRow";

/** One row's identity, the same pair the table is keyed by: two
    machines can hold the same path, and the machine is half the answer.
    Joined on `\u0000`, written as an escape rather than as itself
    (`scripts/check-bytes.mjs`) — it is the one byte a path cannot
    contain, so no pair of rows can collide here. */
function keyOf(r: { device: string; path: string }): string {
  return `${r.device}\u0000${r.path}`;
}

/**
 * **What this list has learned, for the landing's overview**
 * (`WorkspacesNow`, 2026-09-28). The branch and the last change are one
 * question each to a far machine, and the overview beside the list
 * shows the same rows — asking twice would be two walks of somebody
 * else's disk for one answer. So the list, which already asks, publishes
 * what it heard, and the overview reads it. Module-level on purpose:
 * one list is on screen at a time, and the overview must still read it
 * across the list's remounts.
 */
export type WorkspaceFacts = {
  list: WorkspaceRow[];
  marks: Record<string, GitStatus>;
  /** Whether each row is a repository — the list's corner mark. */
  repos: Record<string, boolean>;
  times: Record<string, number>;
  /** Whether the table has been read **in this run of the app**. */
  read: boolean;
  /** Whether there is a list to show at all: read in this run, or
      remembered from the last (`KNOWN_KEY`). */
  known: boolean;
};

/**
 * **What the list last knew outlives the list** (2026-09-29, the owner:
 * 「侧边栏内容的抽象没有缓存？每次新进入工作区，都要从原始的工作区顺序
 * 到置顶设置好的顺序。角标也从普通到git」).
 *
 * The list's order and its corner marks are answers from far machines —
 * when each row last changed, whether it is a repository — and they
 * arrive a row at a time. Kept only in this component, they were thrown
 * away every time the page was left, so every return drew the list in
 * path order with a folder on every row, then reshuffled it and swapped
 * the folders for branches as the answers came back: the same answers
 * the screen had shown a minute ago. That is 20-states' 「加载中，看过」,
 * whose rule is **上次的内容原样留着** — not 先清空再填.
 *
 * So this module holds them (`facts`, which the overview already read
 * across the list's remounts), the list starts from them, and every
 * answer is still re-asked on the way in and replaces its row's. A
 * **snapshot of the order-bearing part** — the rows, when each last
 * changed, which are repositories — is also kept in this browser's
 * storage, so the first look after a restart is not the reshuffle
 * either. The branch and its count are not kept there: they are a walk
 * of somebody's tree, stale the moment they are written, and filling
 * them in moves no row.
 */
const KNOWN_KEY = "khor.ws-known";
function storedKnown(): WorkspaceFacts {
  const none: WorkspaceFacts = { list: [], marks: {}, repos: {}, times: {}, read: false, known: false };
  try {
    const raw = window.localStorage.getItem(KNOWN_KEY);
    if (raw === null) return none;
    const o: unknown = JSON.parse(raw);
    if (typeof o !== "object" || o === null) return none;
    const { list, repos, times } = o as Record<string, unknown>;
    // Every field a row is drawn from, checked: a snapshot written by
    // another version of this file must be dropped, not half-drawn.
    const rows = Array.isArray(list)
      ? list.filter(
          (r): r is WorkspaceRow =>
            typeof r === "object" &&
            r !== null &&
            typeof r.device === "string" &&
            typeof r.machine === "string" &&
            typeof r.path === "string" &&
            typeof r.name === "string" &&
            typeof r.pinned === "boolean",
        )
      : null;
    if (rows === null) return none;
    const table = <T,>(v: unknown, is: (x: unknown) => x is T): Record<string, T> =>
      typeof v === "object" && v !== null
        ? Object.fromEntries(Object.entries(v).filter((e): e is [string, T] => is(e[1])))
        : {};
    return {
      list: rows,
      marks: {},
      repos: table(repos, (x): x is boolean => typeof x === "boolean"),
      times: table(times, (x): x is number => typeof x === "number"),
      read: false,
      known: true,
    };
  } catch {
    return none;
  }
}
function rememberKnown(f: Pick<WorkspaceFacts, "list" | "repos" | "times">) {
  try {
    window.localStorage.setItem(KNOWN_KEY, JSON.stringify({ list: f.list, repos: f.repos, times: f.times }));
  } catch {
    /* the next cold start reshuffles once, and nothing else is lost */
  }
}
let facts: WorkspaceFacts = storedKnown();
const listeners = new Set<() => void>();
function publish(next: WorkspaceFacts) {
  facts = next;
  for (const l of listeners) l();
}
function subscribe(l: () => void) {
  listeners.add(l);
  return () => {
    listeners.delete(l);
  };
}
export function useWorkspaceFacts(): WorkspaceFacts {
  return useSyncExternalStore(subscribe, () => facts);
}

/**
 * The corner a row wears, and so the 类型 it files under (2026-09-29):
 * `branch` where the row is a repository, `folder` otherwise.
 *
 * **A repository is what the cheap answer says; where it has not
 * answered, one whose status came back at all** — a detached HEAD names
 * no branch and is still a checkout. A row nobody has answered about
 * yet wears the folder, and the filter reads the same corner: it sieves
 * what is on the glass, not a fact the glass has not shown.
 */
function kindOf(
  k: string,
  repos: Record<string, boolean>,
  marks: Record<string, GitStatus>,
): "branch" | "folder" {
  return (repos[k] ?? marks[k] !== undefined) ? "branch" : "folder";
}

/**
 * **What the workspaces list can be narrowed by** (2026-09-29, the
 * owner: 「所有的session list应该都可以filter吧？」) — the sessions'
 * filter, the same menu and chips and rule (`lib/list-filter`):
 *
 * - `dev:` 机器 — the machine the row is on, **by its id** (`device`):
 *   the name on the row is its current display name, and a tick kept in
 *   this browser must survive somebody renaming the machine. A row whose
 *   machine has left the device table prints only its path, and files
 *   under `""` 「未知」 — that is what the row says about its machine.
 * - `kind:` 类型 — the row's corner, `branch` or `folder` (`kindOf`).
 *
 * The overview beside the list (`WorkspacesNow`) is not narrowed: it is
 * the landing's summary, not this list.
 */
export const WORKSPACE_KEYS = ["dev:", "kind:"] as const;
/** What a row answers on the two axes, and all the filter reads of it. */
export type WorkspaceFacet = { device: string; machine: string; kind: "branch" | "folder" };
const workspacePick = {
  "dev:": (r: WorkspaceFacet) => (r.machine === "" ? "" : r.device),
  "kind:": (r: WorkspaceFacet) => r.kind,
};
const WORKSPACE_SIEVE = WORKSPACE_KEYS.map((key) => ({ key, pick: workspacePick[key] }));
/** The two axes with their headings and words, for the menu and the box.
    `rows` names the machines: each row carries its machine's display
    name, which is the name it prints; the device table answers for a
    ticked machine no row is on any more. */
export const workspaceAxes = (rows: readonly WorkspaceFacet[], devices: DeviceRow[]): ListAxis<WorkspaceFacet>[] => [
  {
    key: "dev:",
    heading: gui.devices_tab,
    pick: workspacePick["dev:"],
    label: (v) => {
      if (v === "") return gui.filter_unknown;
      return (
        rows.find((r) => r.device === v && r.machine !== "")?.machine ??
        devices.find((d) => d.id === v)?.shown ??
        gui.filter_unknown
      );
    },
  },
  {
    key: "kind:",
    // 「类型」, the 额度 page's word for the same question (the catalog
    // allows one name per text in a section, and it is one word).
    heading: gui.quota_kind,
    pick: workspacePick["kind:"],
    label: (v) => (v === "branch" ? gui.ws_kind_repo : v === "folder" ? gui.ws_kind_folder : v),
  },
];

/**
 * What the list has learned, as the filter reads it — for the pane bar,
 * which the shell draws above this list and which offers the options.
 *
 * **It answers a new array only when an answer on an axis changed**:
 * the shell is the whole window, and a branch name or a time landing on
 * a row (several per row, on every return to the page) is nothing the
 * filter's menu shows. So the snapshot is the rows' facets, kept as long
 * as they read the same.
 */
let facetCache: { said: string; rows: WorkspaceFacet[] } = { said: "", rows: [] };
function facetsNow(): WorkspaceFacet[] {
  const rows = facts.list.map((r) => ({
    device: r.device,
    machine: r.machine,
    kind: kindOf(keyOf(r), facts.repos, facts.marks),
  }));
  const said = JSON.stringify(rows);
  if (said !== facetCache.said) facetCache = { said, rows };
  return facetCache.rows;
}
function useWorkspaceFacets(): WorkspaceFacet[] {
  return useSyncExternalStore(subscribe, facetsNow);
}

/** The pane bar's half of it: the menu's options and the box's axes,
    one list read two ways (`listAxes`). A machine's chip wears its face,
    the way the sessions' machine chips do. */
export function useWorkspaceFilter(
  devices: DeviceRow[],
  ticked: string[],
): { options: FilterOption[]; axes: OmniAxis[] } {
  const facets = useWorkspaceFacets();
  return useMemo(() => {
    const options = listOptions(facets, ticked, workspaceAxes(facets, devices), WORKSPACE_KEYS);
    const face = (key: string) =>
      axisOf(key, WORKSPACE_KEYS) === "dev:"
        ? (devices.find((d) => d.id === valueOf(key, WORKSPACE_KEYS))?.face ?? null)
        : undefined;
    return { options, axes: listAxes(options, WORKSPACE_KEYS, face) };
  }, [facets, devices, ticked]);
}
/** The key a row's facts are filed under — `keyOf`, for the overview. */
export const factKey = (r: { device: string; path: string }) => keyOf(r);

/**
 * **Which workspaces this face opened, newest first** (the overview's
 * 「最近打开」). A habit about one screen, like the middle column's own
 * recent files (`WorkspaceEmpty`), so it lives in this browser's
 * storage and never in a replicated document; storage refusing costs
 * the list and nothing else.
 */
const OPENED_KEY = "khor.ws-opened";
const OPENED_MAX = 8;
export function storedWsOpened(): { device: string; path: string }[] {
  try {
    const raw = window.localStorage.getItem(OPENED_KEY);
    const list: unknown = raw === null ? [] : JSON.parse(raw);
    if (!Array.isArray(list)) return [];
    return list
      .filter(
        (o): o is { device: string; path: string } =>
          typeof o === "object" && o !== null && typeof o.device === "string" && typeof o.path === "string",
      )
      .slice(0, OPENED_MAX);
  } catch {
    return [];
  }
}
function rememberWsOpened(device: string, path: string) {
  const now = [{ device, path }, ...storedWsOpened().filter((o) => o.device !== device || o.path !== path)].slice(
    0,
    OPENED_MAX,
  );
  try {
    window.localStorage.setItem(OPENED_KEY, JSON.stringify(now));
  } catch {
    /* the overview's list is lost and nothing else is */
  }
}

/** A line cut at its **start** when it does not fit: `…/working/khor`
    keeps the end, which is the part a person reads in a path. */
function HeadCut({ text, ...data }: { text: string; [mark: `data-${string}`]: unknown }) {
  return (
    <span
      {...(data as Record<string, unknown>)}
      dir="rtl"
      title={text}
      className="block min-w-0 truncate text-left"
    >
      <bdi dir="ltr">{text}</bdi>
    </span>
  );
}

export function WorkspacesList({
  devices,
  query = "",
  words = [],
  reloadAt = 0,
  selected,
  onOpen,
  treeOpen,
  onTree,
}: {
  /** Only for the faces: a row's avatar is its machine's, and the
      device table is where faces live. The list itself comes from the
      workspace table and not from here. */
  devices: DeviceRow[];
  /** What is typed in the pane bar's box (2026-09-17 ruling 50:「工作区
      list上的搜索框没有生效. 它应该针对不同机器、工作区路径可以搜索」—
      the user). Matched against the three things a row says out loud;
      see `matches`. */
  query?: string;
  /** The ticked filter keys (`WORKSPACE_KEYS`), from the pane bar. */
  words?: string[];
  /** Bumped by the shell when **it** has changed the table — the pane
      bar's 「+」 lives up there (`App`). Every other read this list
      makes is its own; this is the one change it has no way to
      notice. */
  reloadAt?: number;
  /** The open workspace, or null. Two fields rather than one joined
      key, for `keyOf`'s reason. */
  selected: { device: string; path: string } | null;
  onOpen: (row: WorkspaceRow) => void;
  /** Whether the open workspace's file tree is showing, for the chevron
      on its row (2026-09-16:「再次点击工作区 list 的某个,也是收起和展开
      该工作区的文件 list」). **A chevron beside the name, not a second
      meaning on the row**: pressing a row opens it, everywhere in this
      app, and a row whose second press quietly folded a column would be
      a mode nothing on the glass announces. The chevron says the tree
      can be folded from here, and says which way it is. */
  treeOpen: boolean;
  /** Fold or unfold it. Absent on the narrow face, where the list is a
      screen and the tree is the next one — nothing to fold. */
  onTree?: () => void;
}) {
  // Every piece starts from what the list last knew (`facts`), and is
  // re-asked below as it always was.
  const [list, setList] = useState<WorkspaceRow[]>(() => facts.list);
  /** What the two per-row questions came back with. Absent means「not
      answered」— never asked yet, or that machine could not — and the
      corner is blank either way, because the two lead to the same next
      move: look at the row, not at the column. */
  const [marks, setMarks] = useState<Record<string, GitStatus>>(() => facts.marks);
  /** Whether each row is a repository, asked apart from `marks`
      (docs/SHELL.md 第五轮 第 6 条): 「is this a repository」 is one
      stat, 「what changed in it」 is a walk, and the corner used to wait
      on the walk — so a repository whose status was slow, failed, or
      named no branch wore a folder. Absent means not answered. */
  const [repos, setRepos] = useState<Record<string, boolean>>(() => facts.repos);
  const [times, setTimes] = useState<Record<string, number>>(() => facts.times);
  /** A row action that did not take, in the node's own sentence. It
      outlives one opening of the menu, which is why it is here and not
      in `RowMenu` (that file's `note` says so). */
  const [refused, setRefused] = useState<Map<string, string>>(new Map());
  /** Whether the table has been read once. **Not a fifth state** — the
      four this file used to keep were about machines, and no machine is
      asked any more. This is only the difference between「还没读」and
      「读了，是空的」, which is one frame long and would otherwise flash
      「还没有工作区」at somebody who has plenty. */
  const [read, setRead] = useState(() => facts.read);
  /** A list to draw: read now, or remembered (`facts.known`). */
  const known = read || facts.known;
  const hidden = useHidden();
  const [round, setRound] = useState(0);

  /** Whether the table has **answered** this run — `read` is also true
      after a read that failed, and a failed read must not be written
      over the snapshot as if it were the table (an empty list would
      greet the next start with 「还没有工作区」). */
  const answered = useRef(false);
  const reload = useCallback(
    () =>
      fetchWorkspaces()
        .then(
          (rows) => {
            answered.current = true;
            setList(rows);
          },
          // The table is on this disk; a read of it failing is the
          // backend being unreachable, which the shell already says
          // above this pane (`App`'s 后端 line). Nothing to add here.
          () => {},
        )
        .finally(() => setRead(true)),
    [],
  );

  useEffect(() => {
    void reload();
  }, [reload, round, reloadAt]);

  // What the overview beside this list reads (`WorkspaceFacts`), and
  // what the next mount of this list starts from.
  useEffect(() => {
    publish({ list, marks, repos, times, read, known: read || facts.known });
  }, [list, marks, repos, times, read]);
  // …and the part of it that orders the rows, for the next start of the
  // app. Only once the table has answered this run: until then it is
  // the snapshot itself.
  useEffect(() => {
    if (answered.current) rememberKnown({ list, repos, times });
  }, [list, repos, times]);

  // An opened workspace — by this list, the overview or ⌘K: whatever
  // opened it, the selection is where it shows. A machine's whole disk
  // (`path` empty, 看它的文件) is not a workspace and is not kept.
  const openedDevice = selected?.device ?? null;
  const openedPath = selected?.path ?? null;
  useEffect(() => {
    if (openedDevice !== null && openedPath) rememberWsOpened(openedDevice, openedPath);
  }, [openedDevice, openedPath]);

  useEffect(() => {
    if (hidden) return;
    setRound((n) => n + 1);
  }, [hidden]);

  // Two git questions (is it one; what changed) and one mtime question
  // per row, all three at once. **The status and the mtime forget
  // their old answer when the new one fails**, rather than leaving the
  // last good one on screen: a corner that keeps saying `main` about a
  // machine that has stopped answering is the only reading here that
  // could be wrong rather than missing.
  //
  // **A row with no machine name is not asked at all.** Its machine has
  // left the device table, and an empty name means「this machine」to
  // every verb the node has — so asking would answer with *this* Mac's
  // branch and mtime for a path that lives on another one. That is the
  // failure this repo already has a name for: a branch that cannot
  // answer does not go blank, it becomes its neighbour. Here the
  // neighbour would be believable, which is worse.
  useEffect(() => {
    // **Asked for the table's rows, never for the remembered ones**: the
    // list a mount starts from (`facts`) is only there to be drawn, and
    // the table answers it a moment later from this disk. Asking for
    // both was every far machine walked twice on every return.
    if (!answered.current) return;
    let live = true;
    const drop =
      <T,>(k: string) =>
      (m: Record<string, T>): Record<string, T> => {
        const next = { ...m };
        delete next[k];
        return next;
      };
    for (const r of list) {
      if (r.machine === "") continue;
      const k = keyOf(r);
      fetchCheckout(r.machine, r.path).then(
        (yes) => live && setRepos((m) => ({ ...m, [k]: yes })),
        // **The one answer that is kept when a re-ask fails**, against
        // the rule above: whether a directory is a repository is not
        // something it stops being between two looks, and a flaky far
        // machine would otherwise flip the corner to a folder on every
        // failed round. The branch name beside it still goes blank.
        () => {},
      );
      fetchGitStatus(r.machine, r.path).then(
        (g) => live && setMarks((m) => ({ ...m, [k]: g })),
        () => live && setMarks(drop<GitStatus>(k)),
      );
      fetchTouched(r.machine, r.path).then(
        (t) => live && setTimes((m) => ({ ...m, [k]: t })),
        () => live && setTimes(drop<number>(k)),
      );
    }
    return () => {
      live = false;
    };
  }, [list]);

  // Pinned lead; the rest by when they were last changed, newest
  // first; a row with no time sinks to the foot (0 and 「not answered」
  // are one answer here, `marks`' note). Ties by path, so two machines
  // holding the same tree print in one order.
  // The box's rule is `SideList`'s now (`find` below reads the same
  // three things `matches` did); this only orders.
  const rows = [...list].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    const ta = times[keyOf(a)] ?? 0;
    const tb = times[keyOf(b)] ?? 0;
    return tb - ta || a.path.localeCompare(b.path);
  });

  const said = (k: string, e: unknown) =>
    setRefused((m) => new Map(m).set(k, String(e instanceof Error ? e.message : e)));


  const actions = (r: WorkspaceRow): RowAction[] => {
    const k = keyOf(r);
    const run = (go: () => Promise<unknown>) => async () => {
      try {
        await go();
        setRefused((m) => {
          const next = new Map(m);
          next.delete(k);
          return next;
        });
        await reload();
      } catch (e) {
        said(k, e);
        // Rejecting is how an action says it did not happen: the menu
        // stays open where the press was made (`RowMenu`).
        throw e;
      }
    };
    // **Both are keyed by the row's own device id**, never by its
    // machine name: the row this matters most for is the one whose
    // machine has left the table, which has no name and which somebody
    // opened this menu to take off (`api.ts` `wsRm`).
    return [
      {
        key: "pin",
        // The word is what pressing it *does*, so it is the opposite of
        // the row's current state — the same rule the session rows'
        // pin follows.
        label: r.pinned ? gui.unpin : gui.pin,
        note: refused.get(k),
        run: run(() => wsPin(r.device, r.path, !r.pinned)),
      },
      {
        key: "remove",
        // What deleting a workspace does is the window's sentence: the
        // row comes off the list and the directory is not touched (the
        // user, 2026-09-14) — the fact 移出 used to carry in the word.
        label: gui.delete,
        askLabel: gui.delete_ask,
        destructive: true,
        ask: gui.ws_remove_confirm,
        note: refused.get(k),
        run: run(() => wsRm(r.device, r.path)),
      },
    ];
  };

  if (!known) return <div className="flex min-h-0 flex-1 flex-col" />;

  // Each row with the corner it wears (`kindOf`), which is also what the
  // 类型 tick reads — then the rows the ticks leave standing.
  const kept = sieve(
    rows.map((r) => ({ ...r, kind: kindOf(keyOf(r), repos, marks) })),
    words,
    WORKSPACE_SIEVE,
    WORKSPACE_KEYS,
  );
  const side: SideRow[] = kept.map((r) => {
    const k = keyOf(r);
    const isOpen = selected?.device === r.device && selected.path === r.path;
    const git = marks[k];
    const at = times[k] ?? 0;
    // **Ignored paths are rows too since 2026-09-16, and they are not
    // changes** (`khor_node::git::IGNORED`).
    const changed = git ? git.rows.filter((w) => w.mark !== IGNORED).length : 0;
    // Only the branch *name* waits on the walk; whether it is a
    // repository at all is `kindOf`'s.
    const repo = r.kind === "branch";
    return {
      // Not `keyOf`: that joins on a NUL, which no attribute selector can
      // name. The machine's id and the path, with a colon between — the
      // shape the CLI prints.
      id: `${r.device}:${r.path}`,
      group: r.pinned ? "pin" : "",
      pinned: r.pinned,
      face: { machine: devices.find((d) => d.id === r.device)?.face ?? null },
      // 「工作区: 文件夹, 是 git 仓库时换分支」 (docs/SHELL.md 第四轮 第 2 条).
      // On the face or on its corner by the 外观 setting (`FaceMark`).
      mark: repo
        ? { key: "branch", glyph: (c: string) => <IconBranch className={c} /> }
        : { key: "folder", glyph: (c: string) => <IconFiles className={c} /> },
      title: (
        <span data-ws-name className="block min-w-0 truncate">
          {r.name}
        </span>
      ),
      name: r.name,
      // Where it is, in full — the machine and the absolute path are one
      // fact, and this pair is what the CLI prints. **Not shortened to
      // `~`**: whose home that is, is a fact about the far machine. **Cut
      // at its start** (F2.5; N10): the last segment is the part a person
      // reads, and the end is where it is.
      sub: <HeadCut data-ws-where text={r.machine === "" ? r.path : `${r.machine}:${r.path}`} />,
      // The row's meta at the right end, as every list's (F2.1): the
      // branch — or nothing at all: not a repository, a machine that could
      // not be asked, a detached HEAD all arrive here as nothing — over
      // when it was last touched, caption, tabular.
      trailing: (
        <span className="flex max-w-[45%] min-w-0 flex-none flex-col items-end gap-xxs text-caption tabular-nums text-muted-foreground">
          <span data-ws-git className="max-w-full truncate">
            {git && git.branch !== ""
              ? changed > 0
                ? `${git.branch} · ${gui.ws_marked(changed)}`
                : git.branch
              : ""}
          </span>
          <span data-ws-touched>{at > 0 ? ago(at) : ""}</span>
        </span>
      ),
      // **The three things the row says out loud**: its name, its
      // machine, its path (2026-09-17 ruling 50).
      find: `${r.name} ${r.machine} ${r.path}`,
      selected: isOpen,
      menu: actions(r),
      // **The row is the control, and pressing the open one folds its
      // tree** (2026-09-16): a row that is not open means 「open this
      // workspace」 and nothing else; only the one already in front has
      // a second meaning left, and the selection never changes under it.
      onOpen: () => (isOpen && onTree !== undefined ? onTree() : onOpen(r)),
      // ⚠️ 墓碑 (the audit fix plan F2.5; W17): a cell at every row's end
      // held the open row's tree chevron, an empty gutter on the others
      // that cut every row 28 short of the other lists. The row is the
      // control and the tree is beside it; the open row says so by its
      // block.
      // Whether this row's tree is out, said on the line rather than only
      // drawn: the chevron is `aria-hidden` and may not be there at all.
      rowData: {
        "data-ws-line": "",
        "data-tree": isOpen && onTree !== undefined ? treeOpen : undefined,
        "data-pinned": r.pinned,
      },
      openData: { "data-ws-row": "", "data-ws-pinned": r.pinned, "data-selected": isOpen },
    };
  });

  // One sentence for 「nothing matched」, whether the box or the ticks
  // did it — **and the same sentence every list says** (2026-09-29, the
  // owner: 「5统一」): until then this pane had its own, 「没有匹配的工作区」
  // since ruling 50. A bare span, like the other lists': `SideList` lays
  // it in its own `EmptyState`, so a second one here would pad it twice.
  const noneMatch = <span data-ws-none-match>{gui.no_matches}</span>;
  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
      <SideList
        // Never read and nothing remembered: the rows' skeleton (V18).
        loading={!known}
        rows={side}
        query={query}
        filtered={words.length > 0}
        label={groupLabel}
        empty={
          // **A list emptied by the ticks is not an empty list either**
          // — the box's rule below, for the same reason: 「还没有工作区」
          // over a filter would hide three projects behind one untick.
          list.length > 0 ? (
            noneMatch
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 py-10 text-center">
              <span className="text-body font-title text-foreground">{gui.workspaces_empty}</span>
              <span className="text-callout text-muted-foreground">{gui.workspaces_how}</span>
            </div>
          )
        }
        // **A list emptied by the box is not an empty list** (ruling 50):
        // 「还没有工作区」 under a typed word would be khor telling somebody
        // they have no projects while three are one backspace away.
        none={noneMatch}
      />
    </div>
  );
}
