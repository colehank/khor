import type { DeviceRow, SessionRow, Usage } from "@/api";
import type { Vitals } from "@/gen/bindings/Vitals";
import { MachineAvatar } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
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
    <div data-mesh-now className="flex h-full min-h-0 justify-center overflow-y-auto p-6">
      <div className="flex w-full max-w-md flex-col gap-group">
        <section data-now-waiting>
          <Heading text={gui.waiting_on_you} />
          {waiting.length === 0 ? (
            <div data-now-clear className="text-sm text-muted-foreground">
              {gui.none_waiting}
            </div>
          ) : (
            waiting.map((r) => (
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
            ))
          )}
        </section>
        <section data-now-machines>
          <Heading text={gui.devices_tab} />
          {devices.map((d) => (
            <button
              key={d.id}
              type="button"
              data-now-device={d.id}
              onClick={() => onDevice(d)}
              className={cn(
                "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-accent",
                // Never answered = greyed, the card's vitals_never fact
                // painted instead of said; no word is invented for it.
                !d.vitals && "opacity-50",
              )}
            >
              <MachineAvatar face={d.face} className="size-kind-mark flex-none" />
              <span className="min-w-0 truncate text-sm">
                {d.name}
                {d.me ? ` ${cli.this_machine}` : ""}
              </span>
              {d.vitals && <Meters v={d.vitals.vitals} />}
            </button>
          ))}
        </section>
        <section data-now-today>
          <Heading text={gui.today} />
          <Today usage={usage} />
        </section>
        <Button size="sm" variant="secondary" data-empty-new className="self-start" onClick={onNew}>
          {gui.new_session}
        </Button>
      </div>
    </div>
  );
}

/** The list's group-heading shape: what the rows under it share. */
function Heading({ text }: { text: string }) {
  return <div className="mb-1 text-xs text-muted-foreground">{text}</div>;
}

/**
 * One machine's readings in miniature: the percentages, under the same
 * names the card's rings wear, computed with the card's own fractions
 * (`MachineCard.Readings`) — one arithmetic, two magnifications.
 */
function Meters({ v }: { v: Vitals }) {
  const parts: string[] = [`${cli.vitals_name_cpu} ${Math.round(v.cpu_pct)}%`];
  if (v.mem.total > 0) {
    parts.push(`${cli.vitals_name_mem} ${Math.round((v.mem.used / v.mem.total) * 100)}%`);
  }
  if (v.gpu) parts.push(`${cli.vitals_name_gpu} ${Math.round(v.gpu.util_pct)}%`);
  return (
    <span data-now-meters className="ms-auto flex-none text-xs text-muted-foreground">
      {parts.join(" · ")}
    </span>
  );
}

/**
 * Today's spending, as the four token numbers — never one total:
 * `UsagePanel`'s measured judgment stands (a total is almost entirely
 * cache reads and moves with resumption, not with work). Same-field
 * sums across machines and vendors are the mark place's own precedent.
 * "Today" is each machine's own civil day (`UsageDay::day`'s rule);
 * this screen filters by its clock's date, which is that rule read
 * from the one machine the person is sitting at.
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
  const sum = { input: 0, cached_input: 0, cache_write: 0, output: 0 };
  for (const r of rows) {
    sum.input += r.tokens.input;
    sum.cached_input += r.tokens.cached_input;
    sum.cache_write += r.tokens.cache_write;
    sum.output += r.tokens.output;
  }
  return (
    <div data-now-tokens className="flex flex-wrap gap-x-3 text-sm text-muted-foreground">
      <span>{cli.tokens_input(count(sum.input))}</span>
      <span>{cli.tokens_cached(count(sum.cached_input))}</span>
      <span>{cli.tokens_cache_write(count(sum.cache_write))}</span>
      <span>{cli.tokens_output(count(sum.output))}</span>
    </div>
  );
}
