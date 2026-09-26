// 自检 (docs/MORE.md 第二轮 自检页): a verdict, then only what is wrong.
//
// **The page is the node's answer and 「重新检查」 and nothing else.**
// Every judgment — which checks exist, what each one came out as, what it
// says, which group it belongs to, whether khor can mend it — is
// `khor_node::doctor`'s, and this side paints it. That is not modesty: a
// page that decided any of those would be a second opinion about the
// machine, and the day the two disagree the one on screen is the one
// nobody can act on.
//
// **It was twelve rows of 「name / 好 / one sentence」 until 2026-09-21**,
// and the user's reading of that was 「人没法读, 也觉得丑陋」: a row that
// is fine and a row that is broken were the same size, the same shape and
// the same weight, so the page said nothing until you had read all of it.
// What replaced it is one sentence at the top, a card for each problem,
// and every healthy check folded into a single line — which is the whole
// of 「不是所有的设置都要去铺在界面上」 applied to readings instead of
// settings.
//
// **It is not polled.** A checklist is a thing you ask for — it spawns
// processes, dials relays and walks the store — and a page that re-asked
// on a beat would be doing all of that to a machine nobody is looking
// at. Asked once when the page opens, and again when somebody presses.
import { useCallback, useEffect, useState } from "react";

import { doctorFix, fetchDoctor, type DoctorRow } from "@/api";
import { IconReload } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { MoreFold, MoreFrame } from "@/views/more/MorePage";

/**
 * The ink one state wears.
 *
 * **ok and 未量 are told apart by their words, not by their colours**,
 * and both are quiet on purpose: the difference between them is a claim
 * (「looked」 against 「did not look」) that a colour cannot make. What
 * colour carries here is only「something wants you」, which is the two
 * states that do.
 *
 * ⚠️ **These are the session state inks and they are not being reused
 * for a session state.** 状态词只从状态机出 (docs/UX.md) is a rule about
 * *words*: the six words are the session machine's and nothing else may
 * mint one. A check's state is its own vocabulary of four, and the inks
 * say what they say everywhere in this app — amber is「look at this」,
 * red is「this is broken」.
 */
const INK: Record<string, string> = {
  ok: "text-muted-foreground",
  unmeasured: "text-muted-foreground",
  warn: "text-state-errored",
  fail: "text-state-failed",
};

/** The word, from the catalog by the wire key — never spelled here. */
// ⚠️ **A function, not a `const`** (批 D3): catalog reads taken at
// module level keep one language for the life of the tab.
const stateWord = (): Record<string, string> => ({
  ok: cli.doctor_ok,
  warn: cli.doctor_warn,
  fail: cli.doctor_fail,
  unmeasured: cli.doctor_unmeasured,
});

/**
 * Whether a row is one of the ones this page leads with.
 *
 * **未量 counts as needing a person**, and the verdict line is why: a
 * page that said 「一切正常」 while a check could not be taken would be
 * making the claim this module's own head says it must not
 * (`khor_node::doctor`: 未量 is not ok). So 「everything is fine」 is
 * said only when every check came back ok, and anything else is listed.
 */
const wants = (r: DoctorRow) => r.state !== "ok";

export function DoctorPage({ onClose }: { onClose: () => void }) {
  const [rows, setRows] = useState<DoctorRow[] | null>(null);
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** What a remedy said it did, by the key it was run for. Cleared the
      moment that check is asked again — a sentence about a repair that
      has since been re-measured is worse than no sentence. */
  const [fixed, setFixed] = useState<Record<string, string>>({});

  const ask = useCallback(() => {
    setAsking(true);
    setError(null);
    setFixed({});
    fetchDoctor().then(
      (r) => {
        setRows(r);
        setAsking(false);
      },
      (e: unknown) => {
        setError(String(e instanceof Error ? e.message : e));
        setAsking(false);
      },
    );
  }, []);

  useEffect(ask, [ask]);

  // Two-argument `then`: the rejection handler must see the remedy
  // failing and nothing else. The re-ask is not in the chain — a
  // checklist that threw afterwards would read as a repair that failed
  // while the machine is already mended.
  const mend = useCallback(
    (key: string) => {
      doctorFix(key).then(
        (said) => {
          setFixed((was) => ({ ...was, [key]: said }));
          ask();
        },
        (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
      );
    },
    [ask],
  );

  const bad = rows?.filter(wants) ?? [];
  const well = rows?.filter((r) => !wants(r)) ?? [];

  return (
    <MoreFrame page="doctor" onClose={onClose}>
      {/* The verdict, and the one button, on one line. **The sentence
          comes before the list** because it is the answer to the
          question a person opened this page with; the list is what they
          read next only if the sentence says to. */}
      <div className="flex flex-wrap items-center justify-between gap-row">
        {rows && (
          <span
            data-doctor-verdict={bad.length === 0 ? "well" : "wants"}
            className={cn(
              "min-w-0 text-callout font-label",
              bad.length === 0 ? "text-muted-foreground" : "text-state-errored",
            )}
          >
            {bad.length === 0 ? cli.doctor_all_well : cli.doctor_needs_you(bad.length)}
          </span>
        )}
        <Button
          data-doctor-again
          disabled={asking}
          onClick={ask}
          className="min-h-touch flex-none gap-in"
        >
          <IconReload />
          {cli.doctor_again}
        </Button>
      </div>

      {error && (
        <div data-doctor-error className="text-callout text-destructive">
          {error}
        </div>
      )}

      {/* One card per problem. **Not a row in a table**: a table's shape
          says 「these are the twelve things」, and what this page has to
          say is 「these two want you」. */}
      {bad.length > 0 && (
        <ul data-doctor-list className="flex min-w-0 flex-col gap-row">
          {bad.map((r) => (
            <li
              key={r.key}
              data-doctor-card={r.key}
              data-doctor-state={r.state}
              className="flex min-w-0 flex-wrap items-center gap-row rounded-lg border p-3 text-callout"
            >
              <span className="min-w-0 flex-none font-label">{r.name}</span>
              <span
                data-doctor-word
                className={cn("flex-none", INK[r.state] ?? "text-muted-foreground")}
              >
                {stateWord()[r.state] ?? r.state}
              </span>
              <span data-doctor-said className="min-w-0 flex-1 break-all text-muted-foreground">
                {fixed[r.key] ?? r.said}
              </span>
              {r.fix && (
                <Button
                  size="sm"
                  data-doctor-fix={r.key}
                  onClick={() => mend(r.key)}
                  // Both axes of the finger floor: the word is two
                  // characters, so `sm` leaves it under the floor on the
                  // axis nobody thinks to check.
                  className="min-h-touch min-w-touch flex-none"
                >
                  {cli.doctor_mend}
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}

      {/* …and everything that is fine, in one line that opens.
          **Grouped by the node's own grouping** (`doctor::group`), which
          rides on the row — a page that sorted these into 本机 / 网络 /
          工具 itself would be a second table of that fact. */}
      {well.length > 0 && (
        <MoreFold name="doctor-well" label={cli.doctor_well_count(well.length)}>
          {[...new Set(well.map((r) => r.group))].map((g) => (
            <div key={g} data-doctor-group={g} className="flex min-w-0 flex-col">
              <span className="text-callout font-label text-muted-foreground">
                {well.find((r) => r.group === g)?.group_label}
              </span>
              {well
                .filter((r) => r.group === g)
                .map((r) => (
                  <div
                    key={r.key}
                    data-doctor-row={r.key}
                    data-doctor-state={r.state}
                    className="flex min-w-0 flex-wrap items-center gap-row border-b py-2 text-callout last:border-b-0"
                  >
                    <span className="min-w-0 flex-none font-label">{r.name}</span>
                    <span
                      data-doctor-said
                      className="min-w-0 flex-1 break-all text-muted-foreground"
                    >
                      {r.said}
                    </span>
                  </div>
                ))}
            </div>
          ))}
        </MoreFold>
      )}
    </MoreFrame>
  );
}
