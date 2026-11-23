/** Birthplace is the last fallback for a session's workspace. */
export type SessionDirectory = {
  place?: string | null;
  mux?: boolean;
  doing?: { cwd?: string | null } | null;
};

export function sessionCwd(row: SessionDirectory | null | undefined, liveCwd?: string | null): string | null {
  const known = (path: string | null | undefined) => path?.trim() ? path : null;
  // The backend reads the current shell/pane, including tmux nested inside
  // a hosted shell. A terminal redraw need not replay OSC 7 after a switch.
  const runtime = known(row?.doing?.cwd);
  const terminal = known(liveCwd);
  return runtime ?? terminal ?? known(row?.place);
}

/** Read marks only from this session's visible terminal, never a kept tab. */
export function sessionCwdOnScreen(row: (SessionDirectory & { id: string }) | null | undefined): string | null {
  const active = typeof document === "undefined" ? null : document.querySelector<HTMLElement>("[data-kept-terminal][data-active]");
  const box = active?.dataset.keptTerminal === row?.id
    ? active?.querySelector<HTMLElement & { __marks?: { cwd: () => string | undefined } }>("[data-terminal]")
    : null;
  return sessionCwd(row, box?.__marks?.cwd());
}

/** A manually navigated workspace remains pinned across shell/pane changes. */
export function workspaceCwd(row: SessionDirectory | null | undefined, target: { dir: string | null; follow?: boolean } | null, liveCwd?: string | null): string | null {
  return target && !followsSessionCwd(row, target) ? target.dir : sessionCwd(row, liveCwd);
}

/** Older default slices stored only birthplace; explicit files stay pinned. */
export function followsSessionCwd(row: SessionDirectory | null | undefined, target: { dir: string | null; follow?: boolean; start?: unknown; spot?: { file: string | null } | null }): boolean {
  // **Same place, however it is spelled.** A saved slice and a session row
  // can name one directory two ways (`\\?\C:\w` and `C:\w`); comparing the
  // strings made an old default look like a deliberate pin, which then
  // stopped following the shell.
  const same = target.dir === null || row?.place == null ? target.dir === row?.place : samePath(target.dir, row.place);
  return target.follow ?? (!target.start && !target.spot?.file && same);
}

/**
 * The comparison key for a path: **one place, one string**.
 *
 * Windows is why this exists. The same directory arrives spelled several
 * ways — `\\?\C:\work`, `C:/work`, `c:\WORK\` — and this module compared raw
 * strings, so a declared workspace could silently list nothing: the failure
 * `WORKSPACE.md` names as this repository's oldest ("an unanswerable branch
 * does not stay blank, it becomes its neighbour").
 *
 * **Detected by shape, not by the viewer's platform.** A row's cwd belongs to
 * the machine that ran it, which need not be the machine drawing the list;
 * `navigator.platform` would fold the case of a Linux path and miss the case
 * of a Windows one, depending on who is looking.
 */
export function pathKey(path: string): string {
  let p = path.trim();
  // The extended-length prefix is a spelling, not part of the name.
  if (p.startsWith("\\\\?\\UNC\\")) p = `\\\\${p.slice(8)}`;
  else if (p.startsWith("\\\\?\\")) p = p.slice(4);
  const windows = /^[A-Za-z]:/.test(p) || p.startsWith("\\\\");
  if (!windows) {
    // Unix is case-sensitive, so only the separators fold.
    const unix = p.replace(/\/+/g, "/").replace(/\/+$/, "");
    return unix === "" && p.startsWith("/") ? "/" : unix === "" ? p : unix;
  }
  const unc = p.startsWith("\\\\");
  let rest = (unc ? p.slice(2) : p).replace(/\//g, "\\").replace(/\\+/g, "\\").replace(/\\+$/, "");
  // A bare drive is folded to its root. Strictly, `C:` is the drive's
  // *current* directory, but khor only ever compares absolute paths — a cwd
  // from the shell or from OSC 7 always carries one — and two keys for the
  // same drive root would be a trap for no gain.
  if (/^[A-Za-z]:$/.test(rest)) rest = `${rest}\\`;
  // NTFS is case-insensitive; a path that is not a path is left folded only
  // in its separators (an empty key stays empty).
  return rest === "" ? "" : `${unc ? "\\\\" : ""}${rest}`.toLowerCase();
}

/** Whether two spellings name the same place. */
export const samePath = (a: string, b: string): boolean => pathKey(a) === pathKey(b);

/**
 * Whether `cwd` is at, or under, `root` — the boundary is a separator.
 *
 * `C:\work` and `C:\workshop` are neighbours, not relatives; that is the one
 * error this function exists to prevent (`sessionInWorkspace`'s own note).
 */
export function underPath(cwd: string, root: string): boolean {
  const key = pathKey(cwd);
  const base = pathKey(root);
  if (!key || !base) return false;
  if (key === base) return true;
  const sep = base.endsWith("\\") || base.endsWith("/") ? "" : base.includes("\\") ? "\\" : "/";
  return key.startsWith(`${base}${sep}`);
}

/** Workspace membership follows cwd, with an exact machine and path boundary. */
export function sessionInWorkspace(row: SessionDirectory & { home: string }, machine: string | null, root: string): boolean {
  if (!root || machine === null || row.home !== machine) return false;
  const cwd = sessionCwd(row);
  if (!cwd) return false;
  return underPath(cwd, root);
}
