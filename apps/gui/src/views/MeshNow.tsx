import type { DeviceRow, SessionRow, Usage } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Bar, VitalBars } from "@/components/Gauge";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { doingTracks } from "@/lib/vitals";
import { count } from "@/views/UsagePanel";
import { word } from "@/words";

/**
 * The network's now — what the biggest canvas in the app shows when no
 * session is picked. Three sections and no more: what waits on you,
 * the machines, today's spending. Every number and word here is a
 * second reading of data the shell already polls (the session rows,
 * the device table, the usage answer) — nothing is asked again, and
 * nothing is derived that the node did not say (docs/UX.md 状态呈现).
 *
 * The good news is said out loud: an empty section under 等你的事
 * would read as a query that failed, not as a clear conscience.
 */
export function MeshNow({
  rows,
  devices,
  usage,
  onPick,
  onDevice,
  onNew,
}: {
  rows: SessionRow[];
  devices: DeviceRow[];
  usage: Usage | null;
  onPick: (row: SessionRow) => void;
  onDevice: (row: DeviceRow) => void;
  onNew: () => void;
}) {
  const waiting = rows.filter((r) => r.word === "blocked");
  return (
    // The column centres on both axes of the canvas — `my-auto` rather
    // than `items-center` on the scroller, because auto margins collapse
    // to zero when the content outgrows the window and the top stays
    // reachable (a centered flex child inside a scroller clips its head).
    // Content this small huddling in a corner of the app's biggest
    // canvas read as a failure to load, measured before this: ~230 of
    // content pinned to the top of an 860-tall canvas. (Numbers carry
    // no unit: check-px reads lines, comments included.)
    <div data-mesh-now className="flex h-full min-h-0 flex-col items-center overflow-y-auto p-6">
      <div className="my-auto flex w-full max-w-md flex-col gap-pane">
        <section data-now-waiting>
          {/* The section's heading row also holds the one act this
              screen offers. 新会话 lives here rather than as a fourth
              block: starting a session is the answer to both faces of
              this section — nothing waits (start something) and
              something waits (it sits quietly aside, a ghost). */}
          <div className="mb-in flex items-center">
            <Heading text={gui.waiting_on_you} />
            <Button size="sm" variant="ghost" data-empty-new className="ms-auto" onClick={onNew}>
              {gui.new_session}
            </Button>
          </div>
          {waiting.length === 0 ? (
            <div data-now-clear className="text-sm text-muted-foreground">
              {gui.none_waiting}
            </div>
          ) : (
            <>
              {/* The count, at the one display size (--text-hero): when
                  anything waits, how much is the reason this screen
                  exists, so the number is the visual protagonist — body
                  weight (rank must not be said twice) in the state's own
                  colour, which is the same channel the rows under it
                  wear. When nothing waits there is no hero: a big 0
                  would be a loud way of saying nothing. */}
              <div
                data-now-hero
                className="mb-in text-hero tabular-nums"
                style={{ color: "var(--state-blocked)" }}
              >
                {waiting.length}
              </div>
              {waiting.map((r) => (
                <button
                  key={r.id}
                  type="button"
                  data-now-blocked={r.id}
                  onClick={() => onPick(r)}
                  className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-accent"
                >
                  <MachineAvatar face={r.face} className="size-kind-mark flex-none" />
                  <span className="min-w-0 flex-1 truncate text-sm">{r.title}</span>
                  <span className="flex-none text-xs" style={{ color: "var(--state-blocked)" }}>
                    {word(r.word)}
                  </span>
                </button>
              ))}
            </>
          )}
        </section>
        <section data-now-machines>
          <Heading text={gui.devices_tab} className="mb-in" />
          {/* One grid for the whole section, rows spanning it through
              subgrid: the readings start on one shared left edge — a
              true column — instead of being pushed to the row's far end
              (measured before this: a 248-wide gulf between a name's
              end and its readings' start on a 448-wide row, eyes losing
              the line in between). The name column is as wide as the widest
              name; hostnames are short, and a pathological one costs
              this section its readings column, not the app anything. */}
          <div data-now-machine-grid className="grid grid-cols-[max-content_1fr] gap-x-row">
            {devices.map((d) => (
              <button
                key={d.id}
                type="button"
                data-now-device={d.id}
                onClick={() => onDevice(d)}
                className={cn(
                  "col-span-2 grid grid-cols-subgrid items-center rounded-md px-2 py-1.5 text-left hover:bg-accent",
                  // Never answered = greyed, the card's vitals_never fact
                  // painted instead of said; no word is invented for it.
                  !d.vitals && "opacity-50",
                )}
              >
                <span className="flex min-w-0 items-center gap-2">
                  <MachineAvatar face={d.face} className="size-kind-mark flex-none" />
                  <span data-now-name className="min-w-0 truncate text-sm">
                    {d.name}
                    {d.me ? ` ${cli.this_machine}` : ""}
                  </span>
                </span>
                {d.vitals && <Meters row={d} />}
              </button>
            ))}
          </div>
        </section>
        <section data-now-today>
          <Heading text={gui.today} className="mb-in" />
          <Today usage={usage} />
        </section>
      </div>
    </div>
  );
}

/**
 * The one call the mark's place makes: how many sessions are waiting on
 * an answer, and a way into the first of them.
 *
 * **Only when there are any.** `MeshNow`'s empty section says 没有等你的事
 * out loud, and it is right to — an empty block where a query's answer
 * belongs reads as a query that failed. This is not that block: the mark
 * is not a checklist's landing, so on a quiet mesh it simply is not
 * there, and the picture starts at the top.
 *
 * **It exists because the rail's badge cannot say this.** That number
 * counts rows that are waiting *or* unread and cannot tell you which,
 * and 待批 is the one state in the six with a person on the other end of
 * it (docs/SESSION.md). The mark's place had the whole mesh and what it
 * cost, and nothing at all about whether something was stuck.
 */
export function WaitingCall({
  rows,
  onPick,
}: {
  rows: SessionRow[];
  onPick: (row: SessionRow) => void;
}) {
  const waiting = rows.filter((r) => r.word === "blocked");
  if (waiting.length === 0) return null;
  return (
    <button
      type="button"
      data-mark-waiting={waiting.length}
      onClick={() => onPick(waiting[0])}
      className="flex items-center gap-row rounded-md px-3 py-2 text-left hover:bg-accent"
    >
      {/* The count at the display size, in the state's own colour — the
          same two channels `MeshNow` gives it, so one glance recognises
          the same fact in either place. Body weight: rank is already
          said by size. */}
      <span
        data-mark-waiting-count
        className="text-hero tabular-nums"
        style={{ color: "var(--state-blocked)" }}
      >
        {waiting.length}
      </span>
      <span className="text-fact">{gui.waiting_on_you}</span>
    </button>
  );
}

/** The list's group-heading shape: what the rows under it share. The
    margin is the caller's (mb-in where it stands alone), because the
    first section seats it inside a flex row where a margin of its own
    would knock it off the row's centre line. */
function Heading({ text, className }: { text: string; className?: string }) {
  return <div className={cn("text-xs font-label text-muted-foreground", className)}>{text}</div>;
}

/**
 * One machine's readings in miniature — **as bars, not as a sentence**.
 *
 * They were `CPU 23% · 内存 81% · GPU 67%`, one string per machine, at
 * the same size and colour as the machine's own name beside it. Three of
 * those rows and the section was nine numbers nobody was going to read.
 * The strip is `components/Gauge`'s, shared with the star map, drawn
 * from `vitalsTracks`' fractions — the same arithmetic the card's rings
 * magnify, so no two screens can disagree about one machine.
 *
 * The percentages are still available here without leaving: the strip
 * speaks them as its label, and the machine's card draws them in full.
 */
function Meters({ row }: { row: DeviceRow }) {
  const tracks = doingTracks(row);
  if (!tracks) return null;
  // No `ms-auto`: the readings sit in the machine grid's second column,
  // left edges aligned across rows — the column itself is the alignment,
  // not a push to the far end.
  return <VitalBars tracks={tracks} className="justify-start" />;
}

/**
 * Today's spending: one number, and whether that is a lot.
 *
 * **Output, and a bar against the busiest day khor knows.** It printed
 * all four token counts in a row — 输入 734 读缓存 245.9M 写缓存 7.8M
 * 输出 270.2k — which is the usage panel's wall in miniature: four
 * quantities of wildly different magnitude, at one size, with nothing to
 * compare any of them to. `UsagePanel`'s head carries the reasoning for
 * both halves of the fix (output rather than a total, other days rather
 * than an allowance); this is the same judgment at a glance's depth, and
 * the four numbers live one landing away.
 *
 * "Today" is each machine's own civil day (`UsageDay::day`'s rule); this
 * screen filters by its clock's date, which is that rule read from the
 * one machine the person is sitting at.
 */
function Today({ usage }: { usage: Usage | null }) {
  // Nothing until the first answer — a "nothing spent" said while the
  // answer is still coming would be wrong for as long as it took
  // (UsagePanel's opening rule).
  if (!usage) return null;
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  const rows = usage.days.filter((d) => d.day === today);
  if (rows.length === 0) {
    return <div data-now-unspent className="text-sm text-muted-foreground">{gui.today_none}</div>;
  }
  // Same-field sums across machines and vendors are the mark place's own
  // precedent.
  const output = rows.reduce((n, r) => n + r.tokens.output, 0);
  const peak = Math.max(1, ...usage.days.map((d) => d.tokens.output));
  return (
    <div data-now-tokens className="flex items-center gap-row">
      <Bar anchor="today" fraction={output / peak} len="100%" className="min-w-0 flex-1" />
      <span data-now-output className="flex-none text-fact tabular-nums">
        {cli.tokens_output(count(output))}
      </span>
    </div>
  );
}
