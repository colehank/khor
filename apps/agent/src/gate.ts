/**
 * The approval gate of khor's in-app agent.
 *
 * Pi ships no permission layer, on purpose — its README says "No
 * permission popups. Run in a container, or build your own confirmation
 * flow."  Embedding Pi makes khor the vendor, and mcp.rs already wrote
 * down what that means from the other side: an agent that can approve
 * itself is not bound by anything a tool list says, so the gate has to
 * be the vendor's own.  This file is that gate.
 *
 * It sits in the tools' execute path and nowhere else.  It must NOT be
 * an event subscriber: pi's tool_execution_start/update/end are
 * post-hoc lifecycle notifications, and a subscriber that "denies" one
 * has denied nothing that is not already running.  (pi's
 * beforeToolCall hook can genuinely block; session.ts uses it as a
 * second net under this gate, not as the gate.)
 *
 * What feeds it is the binary itself: `khor _verbs` prints every verb
 * with how far it reaches (read / act / inner), so this file keeps no
 * verb list of its own.  A copied list would be a second place to
 * forget a verb — the shape the CLI has already paid for twice (its
 * usage text needed a gate of its own; mcp.rs hand-picks six tools out
 * of ~50 verbs).
 */

export type Reach = "read" | "act" | "inner";

/** Every verb the driven binary answers to, by its first word. */
export type VerbTable = ReadonlyMap<string, Reach>;

/**
 * manual: actions run only inside an approved plan, or after a one-off
 * approval (docs/AGENT.md).  bypass: actions run freely — the user
 * chose that explicitly.  Reads are free in both; inner verbs run in
 * neither.
 */
export type Mode = "manual" | "bypass";

/** One intended action, exactly as the user will read it. */
export interface PlanStep {
  args: string[];
  why: string;
}

export type Decision =
  | { kind: "free" }
  | { kind: "ask" }
  | { kind: "refuse"; why: string };

function sameArgv(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((word, i) => word === b[i]);
}

export class Gate {
  readonly mode: Mode;
  readonly table: VerbTable;

  /**
   * Approved-but-not-yet-run steps.  A step is a licence for ONE run
   * of EXACTLY that argv: consumed on use, so repeating a command
   * means asking again.  Looser matching (by verb, by prefix) can only
   * err in the direction that costs — an approved `close <this
   * session>` must never free `close` of any other.
   */
  private plan: string[][] = [];

  constructor(table: VerbTable, mode: Mode) {
    this.table = table;
    this.mode = mode;
  }

  decide(args: readonly string[]): Decision {
    if (args.length === 0) {
      return { kind: "refuse", why: "Nothing typed: pass at least the verb." };
    }
    const word = args[0]!;
    const reach = this.table.get(word);

    // A word the table cannot answer for is refused — not asked about,
    // not run.  An unanswerable tier must not land in a neighbouring
    // tier: "ask" would teach the agent that inventing words is a way
    // to reach the user, and "free" would run something nobody
    // classified.  If the word is real but newer than this session's
    // table (the keeper swaps binaries between sessions), a fresh
    // session relearns the surface — say so here, because that is the
    // one fact the binary's own "no such verb" error cannot carry.
    if (reach === undefined) {
      return {
        kind: "refuse",
        why:
          `\`${word}\` is not a verb this session knows. ` +
          "Run `khor help` for the current surface; if the verb is newer than this session, a new session learns it.",
      };
    }

    // Inner verbs are the binary talking to itself (_host, _ghost,
    // _verbs …).  bypass waives approvals, not machinery, so this
    // refusal holds in both modes.
    if (reach === "inner") {
      return { kind: "refuse", why: `\`${word}\` is khor-internal machinery, not for agents.` };
    }

    if (reach === "read") return { kind: "free" };

    if (this.mode === "bypass") return { kind: "free" };

    const licensed = this.plan.findIndex((step) => sameArgv(step, args));
    if (licensed >= 0) {
      this.plan.splice(licensed, 1);
      return { kind: "free" };
    }
    return { kind: "ask" };
  }

  /** Called only after the user approved these steps. */
  license(steps: readonly (readonly string[])[]): void {
    for (const step of steps) this.plan.push([...step]);
  }

  /** Approved steps that have not run yet. */
  outstanding(): number {
    return this.plan.length;
  }
}
