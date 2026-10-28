// A rendering context resolves source-machine paths and opens content in
// the containing pane. Activation chooses internal or client-system intent.
// Existence queries remain cached per source directory and machine.
import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";

import { fetchLs, type WorkspaceRow } from "@/api";

export type LinkHome = {
  /** The source machine used to resolve the conversation's paths. */
  machine: string;
  /** The project's own directory, absolute. The ceiling: a path outside
      it is not painted, the same ruling the tree keeps. */
  root: string;
  /** Whether that absolute path is a file on that machine. `null` while
      khor is still asking — drawn as text until it knows, never as a
      link that might turn out to point nowhere. */
  known: (abs: string) => boolean | null;
  /** Register interest in a path, so the directory it is in gets
      listed. Called from an effect, never during a render. */
  see: (abs: string) => void;
  /** Open it in the middle column, at that line where one was named. */
  open: (abs: string, line: number | null) => void;
  /** A session URL opens in its own content side panel. */
  openPage?: (url: string) => void;
  launch: (abs: string) => void;
  /** **The file opens beside the prose**, in the middle column of the
      pane that holds both (`WorkspacePane`). A conversation elsewhere
      shows a pressed file in its own preview first (round CHAT,
      `chat-files`' `FilePreview`), whose 「在文件视图中打开」 is this
      context's `open`; beside a tree, the tree's own view already is
      that preview. */
  beside?: boolean;
  /** **A second directory whose files are painted too** (round CHAT):
      the folder a conversation's files are put in on its machine
      (`chat-files`), which is outside the project and would otherwise
      be refused by the ceiling. Absolute paths only; its own answers. */
  more?: { root: string; known: (abs: string) => boolean | null; see: (abs: string) => void };
};

const Home = createContext<LinkHome | null>(null);

export const LinkHomeProvider = Home.Provider;

/** The workspace a piece of prose is being painted inside, or `null`
    where there is none. */
export function useLinkHome(): LinkHome | null {
  return useContext(Home);
}

/** **Where 「在文件视图中打开」 goes** (round CHAT): the workspace that
    holds the file, opened at it; a file no workspace holds opens in its
    machine's files, on the machine page (`MachineFiles`). Provided by
    the shell, which owns both landings; `null` where nothing does. */
export type ShowFile = (machine: string, abs: string) => void;

const Show = createContext<ShowFile | null>(null);

export const ShowFileProvider = Show.Provider;

export function useShowFile(): ShowFile | null {
  return useContext(Show);
}

/** Resolves what an agent wrote against a project root.
 *
 * Takes: an absolute path already under the root, `src/a.ts`, `./b`,
 * and any of those with a `:12` line suffix. Answers `null` for
 * everything else — including an absolute path outside the root, which
 * is a real path khor deliberately does not paint (the ceiling).
 *
 * **Exported for the gate**, which is the only way to check the refusals
 * without a conversation to put them in. */
export function underRoot(
  root: string,
  written: string,
): { abs: string; line: number | null } | null {
  if (root === "") return null;
  const cut = /^(.*?):(\d+)$/.exec(written);
  const bare = cut === null ? written : cut[1];
  const line = cut === null ? null : Number(cut[2]);
  if (bare === "") return null;
  // No `..` anywhere: a path that climbs is a path that can leave the
  // project, and the one thing this must not do is paint a link out of
  // it. Checked on the written form rather than on a resolved one —
  // resolving first and testing after is how a climb gets normalised
  // into something that looks like it was always inside.
  if (bare.split("/").some((seg) => seg === "..")) return null;
  const abs = bare.startsWith("/")
    ? bare
    : `${root}/${bare.startsWith("./") ? bare.slice(2) : bare}`;
  if (!abs.startsWith(`${root}/`)) return null;
  return { abs, line };
}

/** The workspace on that machine that **holds** this path, or `null`
 * (2026-09-18 ruling 54).
 *
 * **Keyed by the device id, never by the name on the glass.** A
 * workspace row carries both — `device` is the hex and `machine` is
 * whatever that machine is currently called — and a name is a thing
 * the user retypes (2026-09-17: 改名也进命令行). Two machines can be
 * called the same thing for a poll; only one of them has that id.
 *
 * **The deepest one wins.** Somebody with a workspace at a checkout and
 * another at a directory inside it named both on purpose, and the
 * nearer of the two is the project the file actually belongs to.
 *
 * The separator is what keeps `/home/peer/workshop` out of
 * `/home/peer/work` — the same prefix collision `WorkspacePane`'s
 * session column names, and this repo's oldest failure by shape.
 */
export function holding(
  rows: WorkspaceRow[],
  device: string,
  abs: string,
): WorkspaceRow | null {
  if (device === "") return null;
  let best: WorkspaceRow | null = null;
  for (const r of rows) {
    if (r.device !== device || r.path === "") continue;
    const under = r.path.endsWith("/") ? r.path : `${r.path}/`;
    if (!abs.startsWith(under)) continue;
    if (best === null || r.path.length > best.path.length) best = r;
  }
  return best;
}

/** Holds the answers for one workspace. The pane builds the rest of the
    context around this — which machine, which root, what opening one
    does — because those are its own facts. */
export function useKnownFiles(machine: string, root: string): {
  known: (abs: string) => boolean | null;
  see: (abs: string) => void;
} {
  /** Absolute path → is it a file. Only directories khor has listed
      contribute, so a miss is「not in this directory」 and an absent
      directory is「not asked yet」 — two different answers, kept apart
      the way the tree keeps them (`FileTree`'s `failed`). */
  const [files, setFiles] = useState<Map<string, boolean>>(new Map());
  const [dirs, setDirs] = useState<Set<string>>(new Set());
  /** What is already on the wire. A ref for `FileTree`'s reason: two
      renders between the dial and the answer must not become two
      dials. */
  const asked = useRef(new Set<string>());
  /** Which machine and which project these answers are about.
   *
   * **The workspace pane is remounted per workspace and the session
   * landing is not** (2026-09-18 ruling 54): one shell walks from a
   * session on one machine to a session on another without this hook
   * ever unmounting, and an answer cached under an absolute path is an
   * answer about whichever disk it came from. Kept rather than made a
   * `key`, because the caller that has no key is exactly the one that
   * needs this. */
  const on = useRef(`${machine} ${root}`);
  useEffect(() => {
    const now = `${machine} ${root}`;
    if (on.current === now) return;
    on.current = now;
    asked.current = new Set();
    setFiles(new Map());
    setDirs(new Set());
  }, [machine, root]);

  const see = useCallback(
    (abs: string) => {
      const dir = abs.slice(0, abs.lastIndexOf("/")) || "/";
      if (asked.current.has(dir)) return;
      asked.current.add(dir);
      fetchLs(machine, dir)
        .then((l) => {
          setFiles((was) => {
            const now = new Map(was);
            for (const e of l.entries) {
              now.set(`${l.path === "/" ? "" : l.path}/${e.name}`, !e.dir);
            }
            return now;
          });
          setDirs((was) => new Set(was).add(dir));
        })
        .catch(() => {
          // A directory khor could not list answers「no」for everything
          // in it, which is the same thing it answers for a directory
          // that is not there. Nothing is painted either way, and a
          // refusal in the middle of somebody's prose would be a
          // sentence about khor in the middle of a sentence about code.
          setDirs((was) => new Set(was).add(dir));
        });
    },
    [machine],
  );

  const known = useCallback(
    (abs: string): boolean | null => {
      if (!abs.startsWith(`${root}/`)) return false;
      const hit = files.get(abs);
      if (hit !== undefined) return hit;
      const dir = abs.slice(0, abs.lastIndexOf("/")) || "/";
      return dirs.has(dir) ? false : null;
    },
    [files, dirs, root],
  );

  return { known, see };
}
