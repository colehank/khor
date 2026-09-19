// 自检 (docs/MORE.md 自检): `khor doctor`'s checklist, as a page.
//
// **The page is the table and 「再查一遍」 and nothing else.** Every
// judgment — which checks exist, what each one came out as, what it says,
// whether khor can mend it — is the node's (`khor_node::doctor`), and
// this side paints it. That is not modesty: a page that decided any of
// those would be a second opinion about the machine, and the day the two
// disagree the one on screen is the one nobody can act on.
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
import { MoreFrame } from "@/views/more/MorePage";

/**
 * The ink one state wears.
 *
 * **ok and 未量 are told apart by their words, not by their colours**,
 * and both are quiet on purpose: a row that is fine and a row nobody
 * could take must neither of them shout, and the difference between them
 * is a claim (「looked」 against 「did not look」) that a colour cannot
 * make. What colour carries here is only「something wants you」, which is
 * the two states that do.
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
const WORD: Record<string, string> = {
  ok: cli.doctor_ok,
  warn: cli.doctor_warn,
  fail: cli.doctor_fail,
  unmeasured: cli.doctor_unmeasured,
};

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

  return (
    <MoreFrame page="doctor" onClose={onClose}>
      <div className="flex items-center justify-between gap-row">
        <Button
          variant="outline"
          data-doctor-again
          disabled={asking}
          onClick={ask}
          className="min-h-touch gap-in"
        >
          <IconReload />
          {cli.doctor_again}
        </Button>
      </div>
      {error && (
        <div data-doctor-error className="text-sm text-destructive">
          {error}
        </div>
      )}
      {rows && (
        <ul data-doctor-list className="flex flex-col">
          {rows.map((r) => (
            <li
              key={r.key}
              data-doctor-row={r.key}
              data-doctor-state={r.state}
              className="flex min-w-0 flex-wrap items-center gap-row border-b py-2 text-sm last:border-b-0"
            >
              {/* The name, then the state word, then the sentence — the
                  same three fields in the same order `khor doctor`
                  prints, so somebody who has read one can read the
                  other. */}
              <span className="min-w-0 flex-none font-label">{r.name}</span>
              <span
                data-doctor-word
                className={cn("flex-none tabular-nums", INK[r.state] ?? "text-muted-foreground")}
              >
                {WORD[r.state] ?? r.state}
              </span>
              <span data-doctor-said className="min-w-0 flex-1 break-all text-muted-foreground">
                {fixed[r.key] ?? r.said}
              </span>
              {r.fix && (
                <Button
                  variant="ghost"
                  size="sm"
                  data-doctor-fix={r.key}
                  onClick={() => mend(r.key)}
                  // Both axes of the finger floor: the word is one
                  // character, so `sm` leaves it 35 wide — under the
                  // floor on the axis nobody thinks to check.
                  className="min-h-touch min-w-touch flex-none"
                >
                  {cli.doctor_mend}
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </MoreFrame>
  );
}
