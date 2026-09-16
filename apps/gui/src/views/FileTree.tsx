// The workspace's own tree (工作区批⑦). One project, one root, and no
// way above it.
//
// 「目录树不要有上一级。工作区不允许回退到工作区之外的地方。」「取消面包屑。」
// 「目录的打开同 vscode,是点击后展开。目录有类似 `>` 的符号表示可展开。」
// — the user, 2026-09-15.
//
// **This is not `FilesPane` with two controls taken off.** That pane
// answers 「where am I on this machine」 — it walks, it has 上一级, its
// address bar is the point — and it is still what 看它的文件 opens,
// because browsing a whole disk is that question. This one answers
// 「what is in this project」, which has no *where*: the root is the
// ceiling, so there is nothing for a breadcrumb to say and nowhere for
// an up row to go. The two share what a row *is* (`views/fileRow`) and
// nothing else, which is the split the ruling actually drew.
//
// **One directory, one `ls`, and only once it is open.** A project is a
// tree somebody opens two or three branches of; listing the whole thing
// would be a walk of somebody else's disk for rows nobody asked to see.
// Which branches are open is remembered per workspace in this face's own
// storage, so reopening a project finds it where it was left — the same
// class of memory as a column width, not a decision about the mesh.
import { useEffect, useMemo, useRef, useState } from "react";

import { fetchLs, type DirListing } from "@/api";
import { RemoveDialog } from "@/components/RemoveDialog";
import { RowMenu } from "@/components/RowMenu";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconChevronRight } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { Mark, markInk, useRemoveFlow, useTake } from "@/views/fileRow";

/** One row, already placed: the tree is flattened before it is drawn so
    that a row is a row whatever level it is on — the same reason the
    sessions list flattens its groups. */
type Row = { name: string; full: string; dir: boolean; depth: number };

/** The indent guides (2026-09-16:「目录展开后,有一条竖线,表示这些目录下
    文件属于该目录。如 vscode 一致」): one hairline per level above this
    row, each on the centre line of that level's chevron, so the rows
    under an open directory hang off a single line that runs down from
    it. **Drawn per row, not per directory**: the rows are a flat list
    (`walk`), so the only thing that knows a row's ancestry is its
    depth, and a line that every row at that depth draws for itself is
    continuous because the rows touch. The offset is the chevron's half
    width (12/2) past the level's own padding. */
function Guides({ depth }: { depth: number }) {
  return (
    <>
      {Array.from({ length: depth }, (_, k) => (
        <span
          key={k}
          data-guide={k}
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 w-px bg-border"
          style={{ left: `calc(var(--gap-row) + var(--tree-step) * ${k} + var(--tree-slot) / 2)` }}
        />
      ))}
    </>
  );
}

/** Where the open branches are kept, per workspace. **Keyed by the pair
    that is the workspace's identity** (docs/WORKSPACE.md 身份与名字):
    the same checkout on two machines is two projects, and remembering
    one tree's shape under the other's name would open branches that are
    not there. */
const openKey = (machine: string, root: string) => `khor.ws-tree.${machine}:${root}`;

function remembered(machine: string, root: string): Set<string> {
  try {
    const raw = window.localStorage.getItem(openKey(machine, root));
    const list: unknown = raw === null ? [] : JSON.parse(raw);
    // Only paths inside this root, whatever storage holds. The ceiling
    // is the whole ruling, and a path that wandered in from somewhere
    // else would be one row the tree cannot explain.
    return new Set(
      Array.isArray(list)
        ? list.filter((p): p is string => typeof p === "string" && p.startsWith(`${root}/`))
        : [],
    );
  } catch {
    // Storage refused (a private window, a locked profile). Then
    // nothing is remembered, which is the shape the tree opens in
    // anyway — a shut tree is never wrong, only less convenient.
    return new Set();
  }
}

export function FileTree({
  machine,
  root,
  here,
  marks,
  open,
  onOpen,
  onGone,
}: {
  /** The machine's name — what the node resolves. */
  machine: string;
  /** The workspace's own directory, absolute. The ceiling. */
  root: string;
  /** Whether these files are on the machine khor is running on
      (`FilesPane`'s `here` says the rest). */
  here: boolean;
  /** Absolute path → git's letter, folded so a directory carries the
      heaviest letter under it (`WorkspacePane`).
      **`null` is「nobody asked」or「nobody could answer」, which is not
      the same as an empty map** — an empty map says every file here
      matches HEAD, and a tree khor could not read must not claim that.
      Since 工作区批⑦ that is also the whole of what it says: no marks
      and no sentence (「拿不到 git 就一个字不说」). */
  marks: Map<string, string> | null;
  /** Which file is in front, so the row can say so. */
  open: string | null;
  /** A file was pressed. `pinned` is true when the press was one that
      keeps the tab — a double click — which is the strip's business and
      not this tree's (`WorkspacePane`, 工作区批⑧). */
  onOpen: (path: string, pinned: boolean) => void;
  /** That path is no longer on the disk. */
  onGone: (path: string) => void;
}) {
  /** Every directory this tree has an answer for. The root is asked at
      once; the rest are asked when they are opened. */
  const [listings, setListings] = useState<Map<string, DirListing>>(new Map());
  /** …and the ones that refused, in the node's own sentence. Kept apart
      from the listings for `WorkspacePane`'s reason: 「could not read」
      and 「empty」 are different facts and only one of them is a fact
      about the directory. */
  const [failed, setFailed] = useState<Map<string, string>>(new Map());
  const [expanded, setExpanded] = useState<Set<string>>(() => remembered(machine, root));
  /** Which directories this tree wants an answer for. Separate from the
      listings so that asking again after a removal is a change to *this*
      — the path did not move, and nothing else in the render would
      notice (`FilesPane`'s `again`, one level up). */
  const [want, setWant] = useState<Set<string>>(
    () => new Set([root, ...remembered(machine, root)]),
  );
  /** What is already on the wire. **A ref and not state**: two renders
      between the dial and the answer must not become two dials, and a
      piece of state for that would itself cause the render that starts
      the second one. */
  const asked = useRef(new Set<string>());
  const { took, take } = useTake(machine);

  // **The tree is remounted per workspace** (`WorkspacePane` keys it),
  // so `machine` and `root` are constants here and nothing resets: a
  // reset effect would be a second, later opinion about state that was
  // already initialised from the right props.
  // ⚠️ **No `live` flag on these answers, and that is the opposite of
  // every other fetch in this pane.** The usual shape — drop the answer
  // if the effect was torn down — is wrong the moment the dial is
  // deduplicated by a ref that outlives the teardown: React mounts every
  // effect twice in development, the first run starts the `ls` and marks
  // it asked, the cleanup would refuse its answer, and the second run
  // sees it already asked and dials nothing. The tree then never listed
  // anything at all, on the dev build only (found by `check-face`, which
  // is the build it runs against).
  //
  // Dropping the flag is safe here for a reason this component has and
  // most do not: it is remounted per workspace (`WorkspacePane` keys
  // it), so `machine` and `root` cannot change under an answer in
  // flight — every answer that arrives is about this tree.
  useEffect(() => {
    for (const path of want) {
      if (asked.current.has(path)) continue;
      asked.current.add(path);
      fetchLs(machine, path)
        .then((l) => setListings((was) => new Map(was).set(path, l)))
        .catch((e: unknown) =>
          setFailed((was) => new Map(was).set(path, String(e instanceof Error ? e.message : e))),
        );
    }
  }, [machine, want]);

  /** Ask that directory again. Everything below it goes too: a listing
      for a path under one that just changed is an answer about a tree
      that may no longer be there. */
  const reread = (dir: string) => {
    const under = (p: string) => p === dir || p.startsWith(`${dir}/`);
    for (const p of [...asked.current]) if (under(p)) asked.current.delete(p);
    const drop = <T,>(m: Map<string, T>) =>
      new Map([...m].filter(([p]) => !under(p)));
    setListings(drop);
    setFailed(drop);
    // A new set with the same members: the effect above keys on
    // identity, and what changed is not the membership but the answers.
    setWant((was) => new Set(was));
  };

  const { actions, dialog } = useRemoveFlow({
    machine,
    here,
    onGone: (full) => {
      onGone(full);
      reread(full.slice(0, full.lastIndexOf("/")) || "/");
    },
  });

  const toggle = (full: string) => {
    setExpanded((was) => {
      const now = new Set(was);
      if (now.has(full)) now.delete(full);
      else now.add(full);
      try {
        window.localStorage.setItem(openKey(machine, root), JSON.stringify([...now]));
      } catch {
        // The branch still opens; only the memory of it is lost, which
        // is the same bargain the column widths take.
      }
      return now;
    });
    setWant((was) => (was.has(full) ? was : new Set(was).add(full)));
  };

  /** The open branches, flattened in the node's order (it sorts, this
      never re-sorts — docs/UX.md). */
  const rows = useMemo(() => {
    const out: Row[] = [];
    const walk = (dir: string, depth: number) => {
      const l = listings.get(dir);
      if (l === undefined) return;
      for (const e of l.entries) {
        const full = `${dir === "/" ? "" : dir}/${e.name}`;
        out.push({ name: e.name, full, dir: e.dir, depth });
        if (e.dir && expanded.has(full)) walk(full, depth + 1);
      }
    };
    walk(root, 0);
    return out;
  }, [listings, expanded, root]);

  /** How far in one level sits (tokens.css `--tree-step`). */
  const indent = (depth: number) => ({
    paddingLeft: `calc(var(--gap-row) + var(--tree-step) * ${depth})`,
  });

  const rootError = failed.get(root);
  return (
    <section data-file-tree className="flex h-full min-w-0 flex-col">
      <div data-files-list className="min-h-0 flex-1 overflow-y-auto">
        {rootError !== undefined ? (
          // **The one thing this must never be is blank** — the node's
          // own sentence, at the size of something meant to be read
          // (`FilesPane` says the rest). A project khor cannot list is
          // not a project with no files in it.
          <div data-cannot-reach className="p-4 text-fact">
            {rootError}
          </div>
        ) : (
          <>
            {listings.has(root) && rows.length === 0 && (
              <div data-empty className="p-4 text-sm text-muted-foreground">
                {gui.empty_dir}
              </div>
            )}
            {rows.map((r) => {
              const mark = marks?.get(r.full);
              const faded = r.name.startsWith(".");
              const on = expanded.has(r.full);
              return r.dir ? (
                <div
                  key={r.full}
                  data-depth={r.depth}
                  style={indent(r.depth)}
                  // The wash is the row's and the target fills it, the
                  // arrangement both panes arrived at from opposite
                  // sides (`FilesPane`'s two rows).
                  className={cn(
                    "group relative flex min-h-touch items-center gap-2 pr-3 text-sm hover:bg-muted/50",
                    faded && "text-muted-foreground",
                  )}
                >
                  {/* **The whole row toggles, and the chevron is a
                      drawing on it.** VS Code's arrangement, and it is
                      also the only one with a target a finger can land
                      on: a 16-wide twisty of its own would be a control
                      under the floor sitting inside a row that is not
                      (`check-face` section 3).

                      The menu hangs on the button, never on the row
                      around it — `RowMenu`'s trigger carries an
                      `onPointerDown`, and a press handler on a bare
                      `<div>` is a target only a pointer can find. */}
                  <Guides depth={r.depth} />
                  <RowMenu items={actions(r.full, true)}>
                    <button
                      type="button"
                      data-dir={r.name}
                      data-on={on}
                      onClick={() => toggle(r.full)}
                      className="flex min-w-0 flex-1 items-center gap-2 self-stretch text-left"
                    >
                      {/* **12 across, not 16** (2026-09-16:「树里的 > 按钮
                          有点大了,占地方」), and the name with no
                          trailing slash — 「目录后面不用带 /,> 已经表明
                          了是目录」. */}
                      {/* The slot is 16 — the icon's size, text height —
                          and the 12px chevron sits centred in it, so a
                          directory's name and a file's start on one
                          line (2026-09-16:「文件类型 icon 太小了,应该
                          和字等高」, after 12 proved unreadable). */}
                      <span data-tree-slot className="flex size-[var(--tree-slot)] flex-none items-center justify-center" aria-hidden="true">
                        <IconChevronRight
                          className={cn("size-3 flex-none transition-transform", on && "rotate-90")}
                        />
                      </span>
                      {/* **The name wears the dot's ink** (2026-09-16:「目录
                          如果有状态,应该也上色,现在只有个状态灯,但目录仍
                          然是原色」) — VS Code paints the folder's name
                          in the heaviest change below it, and so does
                          this. */}
                      <span data-dir-name className={cn("min-w-0 flex-1 truncate", markInk(mark))}>{r.name}</span>
                      <Mark mark={mark} dir />
                    </button>
                  </RowMenu>
                </div>
              ) : (
                <div
                  key={r.full}
                  data-file={r.name}
                  data-depth={r.depth}
                  style={indent(r.depth)}
                  className={cn(
                    "group relative flex min-h-touch items-center gap-2 pr-3 text-sm hover:bg-muted/50",
                    faded && "text-muted-foreground",
                  )}
                >
                  <Guides depth={r.depth} />
                  <RowMenu items={actions(r.full, false)}>
                    <button
                      type="button"
                      onClick={() => onOpen(r.full, false)}
                      // 「双击、或在里面打了字,就钉住」— the second
                      // press is what says 「I am staying」, and the
                      // browser hands it over as its own event rather
                      // than making this count clicks against a clock.
                      onDoubleClick={() => onOpen(r.full, true)}
                      // **The name wears the letter's ink** — one
                      // decoration in two places (`views/fileRow`), and
                      // last in the list so it wins over the open row's
                      // own weight.
                      className={cn(
                        "flex min-w-0 flex-1 items-center gap-2 self-stretch text-left",
                        open === r.full && "font-title text-foreground",
                        markInk(mark),
                      )}
                    >
                      {/* The chevron's seat on a row that has no
                          chevron: without it a file's name would start
                          16 to the left of its sibling directory's and
                          the level would stop reading as a level. */}
                      {/* **The chevron's slot is the row's type slot**
                          (2026-09-16). A directory puts its chevron
                          there; a file puts the icon VS Code would
                          draw for it, the same 12 across, so every
                          name at one level starts on one line — the
                          arrangement the user first asked for as
                          「文件名对齐到 > 的左沿」 was the answer to an
                          *empty* slot reading as membership; a slot
                          with the file's own mark in it is VS Code's
                          picture, and the ruling that followed. */}
                      <FileKindIcon name={r.name} className="size-[var(--tree-slot)]" />
                      <span data-file-name className="min-w-0 flex-1 truncate">
                        {r.name}
                      </span>
                    </button>
                  </RowMenu>
                  <Mark mark={mark} />
                  {took?.path === r.full ? (
                    <span
                      data-took={took.state}
                      className={cn(
                        "max-w-[50%] truncate text-xs",
                        took.state === "failed" ? "text-state-failed" : "text-muted-foreground",
                      )}
                    >
                      {took.word}
                    </span>
                  ) : (
                    !here && (
                      <Button
                        size="sm"
                        variant="ghost"
                        data-take={r.name}
                        className="min-h-touch min-w-touch flex-none"
                        onClick={() => take(r.full)}
                      >
                        {gui.take_file}
                      </Button>
                    )
                  )}
                </div>
              );
            })}
            {/* A branch that refused, said where the branch is rather
                than over the whole tree: the rest of the project is
                still listed and still walkable. */}
            {[...failed].map(([dir, why]) =>
              dir === root || !expanded.has(dir) ? null : (
                <div
                  key={dir}
                  data-branch-refused={dir}
                  className="px-4 py-2 text-xs text-muted-foreground"
                >
                  {why}
                </div>
              ),
            )}
            {/* The cap said out loud, for `Response::Dir::truncated`'s
                reason: a listing that was cut and one that is simply
                short look exactly the same. */}
            {[...listings].map(([dir, l]) =>
              l.truncated ? (
                <div key={dir} className="p-4 text-xs text-muted-foreground">
                  {cli.dir_truncated(l.entries.length)}
                </div>
              ) : null,
            )}
          </>
        )}
      </div>
      {/* One dialog for the whole tree, not one per row: what is being
          asked about is state, and a sheet per row would be one mounted
          component per file in the project. */}
      <RemoveDialog {...dialog} />
    </section>
  );
}
