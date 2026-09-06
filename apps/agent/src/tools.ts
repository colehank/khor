/**
 * The agent's whole tool surface: `plan` and `khor`.  Nothing else.
 *
 * The gate is only as strong as this surface is narrow.  In particular
 * there is no bash and no file tool here — pi's coding agent ships
 * those by default, and one shell would make the whole read/act
 * classification decorative.
 */
import { Type } from "typebox";
import type { AgentTool } from "@earendil-works/pi-agent-core";
import type { Gate, PlanStep } from "./gate.ts";
import type { RunKhor } from "./verbs.ts";

export interface Approver {
  /** The whole intended plan, for the user to approve or decline. */
  askPlan(steps: readonly PlanStep[]): Promise<boolean>;
  /** One out-of-plan action coming back for approval. */
  askCommand(args: readonly string[], why: string | undefined): Promise<boolean>;
}

// What of a command's output the model gets to read.  `khor log` can
// print megabytes; the model needs the head and the honest fact that
// there was more, not the whole of it.
const OUTPUT_CAP = 48 * 1024;

function capped(text: string): string {
  if (text.length <= OUTPUT_CAP) return text;
  return (
    text.slice(0, OUTPUT_CAP) +
    `\n[output truncated: ${text.length - OUTPUT_CAP} of ${text.length} characters cut]`
  );
}

const PLAN_PARAMS = Type.Object({
  steps: Type.Array(
    Type.Object({
      args: Type.Array(Type.String(), {
        minItems: 1,
        description: 'One khor command as argv, e.g. ["close", "tui/abc"]',
      }),
      why: Type.String({
        description: "Why this step, in the user's language — they read it",
      }),
    }),
    { minItems: 1 },
  ),
});

const KHOR_PARAMS = Type.Object({
  args: Type.Array(Type.String(), {
    minItems: 1,
    description: "One khor command as argv — no shell, no pipes, no quoting",
  }),
  why: Type.Optional(
    Type.String({
      description:
        "For an action outside the plan: why, in the user's language — they read it when asked to approve",
    }),
  ),
});

export function khorTools(gate: Gate, run: RunKhor, approver: Approver): AgentTool<any>[] {
  const plan: AgentTool<typeof PLAN_PARAMS> = {
    name: "plan",
    label: "Plan",
    description:
      "Declare the actions you intend to take, for the user to approve as one plan. " +
      "Approved steps then run without further asking — one run per step, exact argv only. " +
      "Steps that only read need no plan: reads always run freely.",
    parameters: PLAN_PARAMS,
    // Approvals must not interleave: two parallel asks would race each
    // other for the one user.
    executionMode: "sequential",
    execute: async (_id, params) => {
      const steps: PlanStep[] = params.steps;
      // A step the gate would refuse anyway dies here, before the
      // user is asked to approve it.
      for (const step of steps) {
        const word = step.args[0]!;
        const reach = gate.table.get(word);
        if (reach === undefined) {
          throw new Error(`\`${word}\` is not a verb this session knows; run \`khor help\` first.`);
        }
        if (reach === "inner") {
          throw new Error(`\`${word}\` is khor-internal machinery, not for agents or plans.`);
        }
      }
      const actions = steps.filter((s) => gate.table.get(s.args[0]!) === "act");
      if (gate.mode === "bypass") {
        return {
          content: [{ type: "text", text: "Bypass mode: nothing needs approval; run the steps directly." }],
          details: { licensed: 0 },
        };
      }
      if (actions.length === 0) {
        return {
          content: [{ type: "text", text: "Every step only reads, and reads run freely — no plan needed." }],
          details: { licensed: 0 },
        };
      }
      const approved = await approver.askPlan(steps);
      if (!approved) {
        throw new Error("The user declined the plan. Ask what should change; do not retry it unchanged.");
      }
      gate.license(actions.map((s) => s.args));
      return {
        content: [
          {
            type: "text",
            text: `Approved: ${actions.length} action step(s) may each run once, exactly as declared.`,
          },
        ],
        details: { licensed: actions.length },
      };
    },
  };

  const khor: AgentTool<typeof KHOR_PARAMS> = {
    name: "khor",
    label: "khor",
    description:
      "Run one khor command. args is argv — no shell, no pipes, no quoting. " +
      "`khor help` prints every verb with its arguments; when unsure, read that first. " +
      "Reads run freely; actions need an approved plan step or the user's approval on the spot.",
    parameters: KHOR_PARAMS,
    executionMode: "sequential",
    execute: async (_id, params, signal) => {
      const decision = gate.decide(params.args);
      if (decision.kind === "refuse") {
        throw new Error(decision.why);
      }
      if (decision.kind === "ask") {
        const approved = await approver.askCommand(params.args, params.why);
        if (!approved) {
          throw new Error("The user declined this command. Ask what should change; do not retry it unchanged.");
        }
      }
      const r = await run(params.args, signal);
      if (r.code !== 0) {
        const said = r.stderr.trim() || r.stdout.trim();
        throw new Error(said ? `khor exited ${r.code}: ${capped(said)}` : `khor exited ${r.code}`);
      }
      return {
        content: [{ type: "text", text: capped(r.stdout) || "(no output)" }],
        details: { args: params.args, code: r.code },
      };
    },
  };

  return [plan, khor];
}
