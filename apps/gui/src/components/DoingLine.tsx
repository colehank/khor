// **What a terminal session is doing, in a line** (roadmap 2.1, UX-48):
// the row's second line from its shell's command ledger
// (`khor_core::Doing`) — the command itself, the running one or else the
// last, in the terminal's type, never the prompt's own glyphs.
// ⚠️ 墓碑 (UX-48, the owner 2026-10-07): 「正在运行 pytest tests/ · 3 分钟」
// and 「make · 退出 0 · 12 秒」. Whether it runs is the row's state word,
// how long is the time under it; an exit code is the terminal's to show.
import type { Doing } from "@/gen/bindings/Doing";
import type { RowEnding } from "@/gen/bindings/RowEnding";
import type { RowAction } from "@/components/RowMenu";
import { gui } from "@/gen/catalog";
import { askNewSession, askOpenDir } from "@/lib/composer-handoff";

// **Once it has finished, what it said** (the terminal row, the
// owner 2026-10-08: 「a finished command still showing its command line
// reads as still running」): the last non-empty line of its own output,
// in the terminal's type; with nothing printed, the command and how it
// ended in words (`row-ran-ok` / `row-ran-failed`, no exit code). A
// command stopped by the person (130, ^C) or by a closed pipe (141) did
// not fail and did not succeed: the command alone, as before. Running,
// full-screen and a ledger with no end: the command, as before. No
// failure ink here — the row's state word says it (State::Failed).
export function DoingLine({ doing }: { doing: Doing }) {
  if (!doing.command) return null;
  const ended = !doing.running && doing.ended_ms !== null && !doing.full_screen;
  const said = ended ? doing.last_line : null;
  const how = ended && !said && doing.exit !== null && !QUIET_EXITS.includes(doing.exit) ? (doing.exit === 0 ? "ok" : "failed") : null;
  return (
    <span data-doing={doing.running ? "running" : "ran"} data-doing-shows={said ? "output" : how ? `ran-${how}` : "command"} className="block min-w-0">
      {said ? (
        <span data-doing-last className="block min-w-0 truncate font-term">
          {said}
        </span>
      ) : how ? (
        <RanSentence command={doing.command} failed={how === "failed"} />
      ) : (
        <span data-doing-command className="block min-w-0 truncate font-term">
          {doing.command}
        </span>
      )}
      {doing.running && doing.progress !== null && <ProgressLine doing={doing} />}
    </span>
  );
}

/** **How a dead row ended, in words** (P5-SHELL item 3): the exit its
    host recorded, or the host killed before its shell ended. The node
    reads which (`SessionRow.ended`); this only words it. */
export function endedWord(e: RowEnding): string {
  return e.exit === null ? gui.row_ended_host : gui.row_ended_exit(String(e.exit));
}

/** Exit codes that are not a failure: ^C (130) and a closed pipe (141). */
export const QUIET_EXITS = [130, 141];

/** "<command> succeeded / failed": the catalog's sentence around the
    command, the command in the terminal's type and cut first. */
function RanSentence({ command, failed }: { command: string; failed: boolean }) {
  const slot = "\u0001";
  const [before, after = ""] = (failed ? gui.row_ran_failed : gui.row_ran_ok)(slot).split(slot);
  return (
    <span data-doing-ran={failed ? "failed" : "ok"} className="flex min-w-0 whitespace-pre">
      {before}
      <span data-doing-command className="min-w-0 truncate font-term">
        {command}
      </span>
      <span className="flex-none">{after}</span>
    </span>
  );
}

/**
 * **A program's own progress, as a thin line** (roadmap 4.1): what it said
 * with `OSC 9 ; 4` (ConEmu's, read by Windows Terminal, Ghostty and
 * WezTerm for their tab and taskbar), the figure as the line's length, a
 * busy one with no figure as a full faint line, a failed one in the
 * failed ink.
 */
export function ProgressLine({ doing }: { doing: Doing }) {
  const p = doing.progress ?? 0;
  const busy = p === 255;
  return (
    <span data-progress={busy ? "busy" : p} data-progress-failed={doing.progress_failed || undefined} className="mt-xxs block h-[var(--ring-w)] overflow-hidden rounded-full bg-fill">
      <span
        className={doing.progress_failed ? "block h-full bg-[var(--state-failed)]" : busy ? "block h-full bg-[var(--state-busy)] opacity-40" : "block h-full bg-[var(--state-busy)]"}
        style={{ width: `${busy ? 100 : p}%` }}
      />
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
