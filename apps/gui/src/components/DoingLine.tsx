// **What a terminal session is doing, in a line** (roadmap 2.1): the row's
// second line from its shell's command ledger (`khor_core::Doing`) —
// "Running pytest tests/ · 3 min" while it runs, "make · exit 0 · 12 s"
// after — never the prompt's own glyphs. The command is set in the
// terminal's type, the rest in the row's.
import type { Doing } from "@/gen/bindings/Doing";
import { gui } from "@/gen/catalog";
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
