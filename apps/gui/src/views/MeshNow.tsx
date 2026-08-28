import type { DeviceRow, SessionRow, Usage } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Bar } from "@/components/Gauge";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
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
 *
 * **The machines here carry no readings, and that is the same ruling
 * the star map's seats lost theirs to** (2026-08-28,「看不懂,可以取消」
 * about three unlabelled bars; `MandalaMap`'s head carries the whole
 * reasoning). This section is the other overview of the same machines,
 * so it is the same question: which machine, not how busy it is. A
 * ruling that only reached one of two screens showing one thing would
 * have left the bars the user could not read alive one landing away.
 *
 * 今天 keeps its bar, and the difference is the denominator: it is one
 * quantity against **the busiest day khor knows**, said in words right
 * beside it — not a share of something nobody named.
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
            {/* `min-h-touch`: the one act this screen offers was 24
                tall. The `sm` look is what a control beside a heading
                wears; the floor is a different axis and rides on top of
                it (tokens.css `--touch` — it is not a fourth `--ctl-*`),
                and the ghost's hover block paints the grown box. */}
            <Button
              size="sm"
              variant="ghost"
              data-empty-new
              className="ms-auto min-h-touch"
              onClick={onNew}
            >
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
                  // The floor (tokens.css `--touch`): these rows were 30
                  // tall, and they are the rows that exist because
                  // something is waiting on a person — the last place in
                  // the app where a press should be able to miss.
                  className="flex min-h-touch w-full items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-accent"
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
          {/* **A column of names, and the grid that used to align their
              readings went with the readings.** It was a two-column
              subgrid so that every machine's bars started on one left
              edge — a real alignment for a real column, and with nothing
              in the second column it would be a layout holding a space
              open for something that is not coming back. */}
          <div data-now-machines-list className="flex flex-col">
            {devices.map((d) => (
              <button
                key={d.id}
                type="button"
                data-now-device={d.id}
                onClick={() => onDevice(d)}
                className={cn(
                  // Same floor as the waiting rows above it, and for the
                  // same reason they are worth stating once: a list of
                  // rows 30 tall is the shape where a press lands on the
                  // neighbour (tokens.css `--touch`).
                  "flex min-h-touch items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-accent",
                  // Never answered = greyed, the card's vitals_never fact
                  // painted instead of said; no word is invented for it.
                  !d.vitals && "opacity-50",
                )}
              >
                <MachineAvatar face={d.face} className="size-kind-mark flex-none" />
                <span data-now-name className="min-w-0 truncate text-sm">
                  {d.name}
                  {d.me ? ` ${cli.this_machine}` : ""}
                </span>
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

/** The list's group-heading shape: what the rows under it share. The
    margin is the caller's (mb-in where it stands alone), because the
    first section seats it inside a flex row where a margin of its own
    would knock it off the row's centre line. */
function Heading({ text, className }: { text: string; className?: string }) {
  return <div className={cn("text-xs font-label text-muted-foreground", className)}>{text}</div>;
}

/**
 * Today's spending: one number, and whether that is a lot.
 *
 * **Output, and a bar against the busiest day khor knows.** It printed
 * all four token counts in a row — 输入 734 读缓存 245.9M 写缓存 7.8M
 * 输出 270.2k — four quantities of wildly different magnitude, at one
 * size, with nothing to compare any of them to.
 *
 * Two judgments hold it up, and **this is the only place either of them
 * is still written down** — the day ledger that argued them out was
 * removed from the app on 2026-08-28 and its file went with it:
 *
 *   - **output, never a sum.** A total would be almost entirely cache
 *     reads (measured on the development machine: 16.2 billion cached
 *     input against 32 million output), so it moves with how often a
 *     conversation was resumed rather than with how much work was done
 *     (`khor_core::Tokens` carries that judgment);
 *   - **against the other days, never against an allowance.** Nobody
 *     has told khor what an allowance is, so a share of one would be a
 *     share of something invented. 「Busy day or quiet one」 has an
 *     obvious denominator and this bar uses that one.
 *
 * The remaining three numbers are one landing away — `khor usage`, and
 * that verb is now the only place they are drawn at all.
 *
 * "Today" is each machine's own civil day (`UsageDay::day`'s rule); this
 * screen filters by its clock's date, which is that rule read from the
 * one machine the person is sitting at.
 */
function Today({ usage }: { usage: Usage | null }) {
  // Nothing until the first answer: a 「nothing spent」 said while the
  // answer is still coming would be telling a machine's whole history
  // wrong for as long as it took to arrive.
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

/**
 * A count as a person reads it: 1000-based, because tokens are counted,
 * not stored.
 *
 * **It lives here because this is the only screen left that draws one.**
 * It was exported from the day ledger's file, which had four callers in
 * it; that file is gone and re-exporting from a shared bag would give a
 * one-caller helper a second home to drift from.
 *
 * The CLI has its own copy (`crates/cli` `count`), deliberately —
 * formatting is painting, not judgment, and nothing depends on the two
 * producing identical characters. **Not the same function as the byte
 * one either**: 1024 is a property of memory, and a thousand tokens is a
 * thousand tokens.
 */
function count(n: number): string {
  const units = ["", "k", "M", "G"];
  let v = n;
  let u = 0;
  while (v >= 1000 && u + 1 < units.length) {
    v /= 1000;
    u += 1;
  }
  return u === 0 ? `${n}` : `${v.toFixed(1)}${units[u]}`;
}
