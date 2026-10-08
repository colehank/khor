// 诊断 (docs/MORE.md 第二轮 自检页): a verdict, then only what is wrong.
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

import { doctorFix, fetchDoctor, type DeviceRow, type DoctorRow } from "@/api";
import { Waiting } from "@/components/RegionState";
import { ActionRow } from "@/components/settings/ActionRow";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { MoreFold, MoreFrame, MoreSection } from "@/views/more/MorePage";
import { NetStatusSections } from "@/views/more/NetStatus";

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

export function DoctorPage({
  devices,
  onClose,
}: {
  /** The shell's own polled table, for the network's readings under the
      checks (`NetStatusSections`, UX-16). */
  devices: DeviceRow[];
  onClose: () => void;
}) {
  const [rows, setRows] = useState<DoctorRow[] | null>(null);
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** What a remedy said it did, by the key it was run for. Cleared the
      moment that check is asked again — a sentence about a repair that
      has since been re-measured is worse than no sentence. */
  const [fixed, setFixed] = useState<Record<string, string>>({});
  /** When the checks on screen were answered — R0's second line, the
      page's 更新于 the way a machine's reading has one. */
  const [answeredAt, setAnsweredAt] = useState<number | null>(null);
  /** When the first asking began: until it answers the page is loading,
      and says so the way every page does (`Waiting`). */
  const [since] = useState(() => Date.now());

  const ask = useCallback(() => {
    setAsking(true);
    setError(null);
    setFixed({});
    fetchDoctor().then(
      (r) => {
        // **An answer that is not a table is no answer** (afix-a; the
        // audit's R1: a node that answered `null` threw on the rows below
        // and blanked the window). Not 「一切正常」 — nothing was checked —
        // and not loading for ever: the ask failed, and 重新检查 is there.
        if (!Array.isArray(r)) {
          setError(gui.doctor_no_answer);
          setAsking(false);
          return;
        }
        setRows(r);
        setAnsweredAt(Date.now());
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
    <MoreFrame
      page="doctor"
      onClose={onClose}
      // When these checks were taken, in the machine page's words
      // (`updated-at`): the verdict is the first row, the time is not on
      // the page anywhere else.
      subtitle={
        answeredAt !== null
          ? cli.updated_at(new Date(answeredAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }))
          : undefined
      }
    >
      {/* **Settings groups, like every other 「…」 page**
          (components/SettingsGroup). The first group has no name — R0
          already says 诊断 — and holds the verdict with 重新检查 as its
          one act, then one row per problem. **The sentence comes before
          the list** because it is the answer to the question a person
          opened this page with; the rows are what they read next only if
          the sentence says to.

          ⚠️ 墓碑 (2026-09-27): each problem was a bordered card of its
          own (`rounded-lg border p-3`) under a free-standing verdict
          line. A box with a border is the one depth this system does not
          have — depth is `card` on `canvas` — and a stack of them was a
          settings page drawn its own way. */}
      <MoreSection name="doctor" mark={{ "data-doctor-list": "" }}>
        <ActionRow
          name="doctor-again"
          label={
            rows ? (
              <span
                data-doctor-verdict={bad.length === 0 ? "well" : "wants"}
                className={cn("font-label", bad.length === 0 ? "text-muted-foreground" : "text-state-errored")}
              >
                {bad.length === 0 ? cli.doctor_all_well : cli.doctor_needs_you(bad.length)}
              </span>
            ) : (
              ""
            )
          }
          hint={error ? <span data-doctor-error data-selectable className="text-destructive">{error}</span> : undefined}
          actionLabel={cli.doctor_again}
          busy={asking}
          onPress={ask}
          mark={{ "data-doctor-again": "" }}
        />
        {/* **Loading until the first answer** (the owner's rule, 2026-10-04:
            before anything is known the state is loading — the skeleton,
            the spinner after a second). A check takes seconds (the relay's
            three), and the page sat as an empty verdict over nothing. */}
        {rows === null && error === null && <Waiting since={since} />}
        {/* One row per problem: its name and its word on the title line,
            the machine's own sentence under it, and 修复 where khor can
            mend it — on the spot, so no ellipsis. A check khor cannot
            mend has no button, and a button that refuses is worse than
            none, so that row is a fact, not an action row. */}
        {bad.map((r) => {
          const title = (
            <span className="flex min-w-0 flex-wrap items-baseline gap-row">
              <span className="min-w-0 font-label">{r.name}</span>
              <span data-doctor-word className={cn("flex-none text-callout", INK[r.state] ?? "text-muted-foreground")}>
                {stateWord()[r.state] ?? r.state}
              </span>
            </span>
          );
          const said = (
            <span data-doctor-said data-selectable className="whitespace-pre-line [overflow-wrap:anywhere]">
              {fixed[r.key] ?? r.said}
            </span>
          );
          return r.fix ? (
            <ActionRow
              key={r.key}
              name={`doctor-${r.key}`}
              rowMark={{ "data-doctor-card": r.key, "data-doctor-state": r.state }}
              label={title}
              hint={said}
              actionLabel={cli.doctor_mend}
              onPress={() => mend(r.key)}
              mark={{ "data-doctor-fix": r.key }}
            />
          ) : (
            <div key={r.key} data-doctor-card={r.key} data-doctor-state={r.state} className="flex min-w-0 flex-col">
              <span className="text-body">{title}</span>
              <span className="text-callout text-muted-foreground">{said}</span>
            </div>
          );
        })}
      </MoreSection>

      {/* …and everything that is fine, in one line that opens.
          **Grouped by the node's own grouping** (`doctor::group`), which
          rides on the row — a page that sorted these into 本机 / 网络 /
          工具 itself would be a second table of that fact. Each group is
          a settings group of facts. */}
      {well.length > 0 && (
        <MoreFold name="doctor-well" label={cli.doctor_well_count(well.length)}>
          {[...new Set(well.map((r) => r.group))].map((g) => (
            <MoreSection
              key={g}
              name={`doctor-${g}`}
              label={well.find((r) => r.group === g)?.group_label}
              mark={{ "data-doctor-group": g }}
            >
              {well
                .filter((r) => r.group === g)
                .map((r) => (
                  <div
                    key={r.key}
                    data-doctor-row={r.key}
                    data-doctor-state={r.state}
                    className="flex min-w-0 flex-wrap items-baseline gap-row text-body"
                  >
                    <span className="min-w-0 flex-none">{r.name}</span>
                    <span data-doctor-said data-selectable className="min-w-0 flex-1 [overflow-wrap:anywhere] text-callout text-muted-foreground">
                      {r.said}
                    </span>
                  </div>
                ))}
            </MoreSection>
          ))}
        </MoreFold>
      )}
      {/* The network's readings (UX-16): 网络's status rows and its
          per-machine 连接 group, readings rather than settings. */}
      <NetStatusSections devices={devices} />
    </MoreFrame>
  );
}
