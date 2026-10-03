// **What a terminal session is doing, in a line** (roadmap 2.1): the row's
// second line from its shell's command ledger (`khor_core::Doing`) —
// "Running pytest tests/ · 3 min" while it runs, "make · exit 0 · 12 s"
// after — never the prompt's own glyphs. The command is set in the
// terminal's type, the rest in the row's.
import type { Doing } from "@/gen/bindings/Doing";
import type { RowAction } from "@/components/RowMenu";
import { gui } from "@/gen/catalog";
import { askNewSession, askOpenDir } from "@/lib/composer-handoff";
import { lasted } from "@/words";

/** Where the command goes in the catalog's sentence. */
const SLOT = "\u0001";

export function DoingLine({ doing, now = Date.now() }: { doing: Doing; now?: number }) {
  if (!doing.command) return null;
  const known = doing.started_ms > 0;
  const took = known ? lasted((doing.ended_ms ?? now) - doing.started_ms) : null;
  const text = doing.running
    ? took
      ? gui.row_running(SLOT, took)
      : gui.row_running_bare(SLOT)
    : doing.exit !== null && took
      ? gui.row_ran(SLOT, doing.exit, took)
      : took
        ? gui.row_ran_plain(SLOT, took)
        : SLOT;
  const [before, after] = text.split(SLOT);
  return (
    <span data-doing={doing.running ? "running" : "ran"} data-exit={doing.exit ?? undefined} className="block min-w-0 truncate">
      {before}
      <span data-doing-command className="font-term">
        {doing.command}
      </span>
      {after}
    </span>
  );
}

/** The last segment of a path, for a name. */
export function lastSegment(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? "/";
}

/** The first word of a command line: the program, for a name. */
export function programOf(command: string): string {
  const first = command.trim().split(/\s+/)[0] ?? "";
  return lastSegment(first);
}

/** A name nobody gave: empty, or the shell's own. */
const UNNAMED = /^(-?(zsh|bash|fish|sh|pwsh|cmd)|shell)?$/;

/**
 * **What a terminal tab is called** (roadmap 2.5): a name the person gave
 * stays; an unnamed shell's tab follows it — the title a program set
 * (OSC 0/2), else the last command's program, else the directory's last
 * segment. The same order as iTerm2's and Terminal.app's default tab
 * titles (the job's name, then the directory).
 */
export function termName(r: { title: string; doing: Doing | null }): string {
  const d = r.doing;
  if (!d || !UNNAMED.test(r.title.trim())) return r.title;
  return d.title || (d.command ? programOf(d.command) : "") || (d.cwd ? lastSegment(d.cwd) : "") || r.title;
}

/** A directory for a narrow line: its last two segments. */
export function shortDir(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts.length <= 2 ? `/${parts.join("/")}` : `…/${parts.slice(-2).join("/")}`;
}

/** A terminal directory's two acts, for its tab's and its row's menus
    (roadmap 2.5): its files on that machine, a new session there. */
export function hereActs(machine: string, cwd: string): RowAction[] {
  return [
    { key: "open-dir", label: gui.open_dir, run: () => askOpenDir(machine, cwd) },
    { key: "new-here", label: gui.new_session_here, run: () => askNewSession(machine, cwd) },
  ];
}
