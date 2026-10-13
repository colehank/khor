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
  return target.follow ?? (!target.start && !target.spot?.file && target.dir === row?.place);
}
