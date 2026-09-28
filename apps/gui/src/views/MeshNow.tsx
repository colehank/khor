import type { DeviceRow, SessionRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { connectionWord, word } from "@/words";

/**
 * The network's now — what the biggest canvas in the app shows when no
 * session is picked. Two sections and no more: what waits on you, and
 * the machines. Every number and word here is a second reading of data
 * the shell already polls (the session rows, the device table) —
 * nothing is asked again, and nothing is derived that the node did not
 * say (docs/UX.md 状态呈现).
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
 * **There was a third section, 今天, and it went the same way one
 * landing later.** 「session 页这个今天输出 xx k 也不要」 — the user,
 * 2026-08-29, and it is the second half of a ruling this file had
 * already half-received: 「不显示输出 token 什么的」 took the day ledger
 * off the home screen on 2026-08-28 while this canvas kept a bar and a
 * number saying the same thing one press away. **A screen removed on
 * one landing and kept on another is not removed** — the same mistake
 * the bars above were caught in, and the reason this paragraph exists
 * rather than a silent deletion.
 *
 * 花了多少 is not homeless: `khor usage` prints every day, every
 * machine and all four token counts, and it is now the only place any
 * of them is drawn. Nothing was hidden one press deeper — the app
 * simply stopped being where that question is asked.
 */
export function MeshNow({
  rows,
  devices,
  onPick,
  onDevice,
}: {
  rows: SessionRow[];
  devices: DeviceRow[];
  onPick: (row: SessionRow) => void;
  onDevice: (row: DeviceRow) => void;
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
          {/* ⚠️ 墓碑 (2026-09-28 UI 审计): this heading row also held
              「新会话…」 at its right end — the same act R0 carries over
              this overview (D9: 页面级按钮只在 R0 右端，不许两边都有).
              One act, one place: R0's. */}
          <div className="mb-in flex items-center">
            <Heading text={gui.waiting_on_you} />
          </div>
          {waiting.length === 0 ? (
            <div data-now-clear className="text-callout text-muted-foreground">
              {gui.none_waiting}
            </div>
          ) : (
            <>
              {/* The count, at the one display size (--text-title-lg since 第十五轮): when
                  anything waits, how much is the reason this screen
                  exists, so the number is the visual protagonist — body
                  weight (rank must not be said twice) in the state's own
                  colour, which is the same channel the rows under it
                  wear. When nothing waits there is no hero: a big 0
                  would be a loud way of saying nothing. */}
              <div
                data-now-hero
                className="mb-in text-title-lg tabular-nums"
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
                  // Under the pointer the neutral `fill`, never the
                  // brand wash — that is the row being located to
                  // (README 颜色, `check-face` 4K).
                  className="flex min-h-touch w-full items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-fill"
                >
                  <MachineAvatar face={r.face} className="size-kind-mark flex-none" />
                  <span className="min-w-0 flex-1 truncate text-callout">{r.title}</span>
                  <span className="flex-none text-caption" style={{ color: "var(--state-blocked)" }}>
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
                  "flex min-h-touch items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-fill",
                  // Never answered = greyed, the card's vitals_never fact
                  // painted instead of said; no word is invented for it.
                  !d.vitals && "opacity-50",
                )}
              >
                <MachineAvatar face={d.face} className="size-kind-mark flex-none" />
                <span data-now-name className="min-w-0 truncate text-callout">
                  {d.name}
                </span>
                {/* The road, the machines list's own word for it
                    (`connectionWord`): 本机 for this one, never a
                    bracket on the name. */}
                {connectionWord(d) && (
                  <span data-now-connection className="flex-none text-callout text-muted-foreground">
                    {connectionWord(d)}
                  </span>
                )}
              </button>
            ))}
          </div>
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
  return <div className={cn("text-caption font-label text-muted-foreground", className)}>{text}</div>;
}
