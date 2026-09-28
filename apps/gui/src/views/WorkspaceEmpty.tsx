// 中栏没有标签时 (工作区第五轮 第 37 条). What the middle column says
// when no file is open.
//
// # Not a blank, and not an illustration either
//
// A blank column is the app having nothing to say at the moment a
// person has just arrived. An illustration is worse: it is a picture
// where an answer belongs, and it is the same picture on every
// workspace, so the second time somebody sees it they have learned
// nothing (docs/UX.md 设置 — a default that answers beats a decoration).
//
// What goes here is what khor already knows and the person came to find
// out: what they had open last time. Every line of it is a fact, and
// every line is pressable. With nothing opened yet it says so, in one
// sentence (20-states 空), rather than going blank.
//
// **Not the name, the path or the branch** (2026-09-28). Those stood at
// the top of this column until R0 learned to say them — the name, then
// 「⎇ main · 5 个文件已修改 · peer:/home/peer/work」 under it — and a
// page says who it is once (10-layout「身份只出现一次」). Said twice,
// the second copy is the one people learn to skip.
//
// # The recent list is this face's own
//
// It is a habit about one screen — the same class of thing as a
// column's width and the open tabs themselves — so it lives in this
// browser's storage and nowhere near the replicated documents
// (`WorkspacePane`'s `tabsKey` says the whole of it). Storage refusing
// costs the list and nothing else.
import { gui } from "@/gen/catalog";
import { FileKindIcon } from "@/components/FileKindIcon";
import { EmptyState } from "@/components/RegionState";
import { cn } from "@/lib/utils";
import { basename } from "@/words";

/** How many names are kept. **Eight, not「all of them」**: this list is
    read at a glance on the way to one file, and a column of forty is a
    search rather than a memory. */
export const MOST_RECENT = 8;

const recentKey = (machine: string, root: string) => `khor.ws-recent.${machine}:${root}`;

/** The files this workspace was last opened on, newest first. Anything
    outside the root is dropped whatever storage holds — the ceiling is
    the whole ruling (工作区批⑦), and a name from somewhere else is one
    the tree cannot explain. */
export function storedOpened(machine: string, root: string): string[] {
  try {
    const raw = window.localStorage.getItem(recentKey(machine, root));
    const list: unknown = raw === null ? [] : JSON.parse(raw);
    if (!Array.isArray(list)) return [];
    return list
      .filter(
        (p): p is string =>
          typeof p === "string" && (root === "" ? p.startsWith("/") : p.startsWith(`${root}/`)),
      )
      .slice(0, MOST_RECENT);
  } catch {
    return [];
  }
}

/** Puts one file at the front, newest first, and answers the new list.
    **Opening a file already in the list moves it rather than repeating
    it**: what this remembers is when, not how often. */
export function rememberOpened(machine: string, root: string, path: string): string[] {
  const now = [path, ...storedOpened(machine, root).filter((p) => p !== path)].slice(
    0,
    MOST_RECENT,
  );
  try {
    window.localStorage.setItem(recentKey(machine, root), JSON.stringify(now));
  } catch {
    /* the list is lost and the files are not; nothing else is affected */
  }
  return now;
}

export function WorkspaceEmpty({
  root,
  recent,
  onOpen,
}: {
  /** The workspace's own directory, absolute — what the paths below are
      said relative to. Empty on the browsing face, where they are whole. */
  root: string;
  recent: string[];
  onOpen: (path: string) => void;
}) {
  if (recent.length === 0) {
    return (
      <div data-ws-empty={root} className="flex min-h-0 flex-1 flex-col">
        <EmptyState data-ws-empty-none>{gui.ws_empty_none}</EmptyState>
      </div>
    );
  }
  return (
    <div data-ws-empty={root} className="min-h-0 flex-1 overflow-auto p-[var(--gap-pane)]">
      <div className="flex flex-col gap-[var(--gap-in)]">
        <div className="text-caption text-muted-foreground">{gui.ws_empty_recent}</div>
        <div className="flex flex-col">
          {recent.map((p) => (
            <button
              type="button"
              key={p}
              data-ws-recent={p}
              onClick={() => onOpen(p)}
              className={cn(
                "flex min-h-touch w-full min-w-0 items-center gap-2 rounded-sm px-2 text-left text-callout",
                "hover:bg-muted/50",
              )}
            >
              <FileKindIcon name={basename(p)} className="size-5 flex-none" />
              <span className="min-w-0 flex-1 truncate">{basename(p)}</span>
              {/* The path it sits at, so two `mod.rs` are two rows
                  a person can tell apart. Relative to the root for
                  the tab strip's reason: the absolute one is the
                  same forty characters on every line. */}
              <span className="min-w-0 flex-[2] truncate text-right text-caption text-muted-foreground">
                {root !== "" && p.startsWith(`${root}/`) ? p.slice(root.length + 1) : p}
              </span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
