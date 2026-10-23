/**
 * Legacy CLI/gate development harness used by dev.ts. The shipping
 * khor agent uses host.ts and the Rust-owned MCP execution boundary.
 */
import { Agent } from "@earendil-works/pi-agent-core";
import { Gate, type Mode, type VerbTable } from "./gate.ts";
import { khorRunner, loadVerbTable } from "./verbs.ts";
import { khorTools, type Approver } from "./tools.ts";

type AgentConfig = ConstructorParameters<typeof Agent>[0];

export interface KhorAgentSetup {
  /** The binary to drive — under serve, serve's own current_exe. */
  khorBin: string;
  mode: Mode;
  approver: Approver;
  model: NonNullable<NonNullable<AgentConfig["initialState"]>["model"]>;
  streamFn: AgentConfig["streamFn"];
  /** Key resolution stays the caller's: the serve host reads the
   * accounts table; dev.ts leans on env vars.  Keys never live here. */
  getApiKey?: AgentConfig["getApiKey"];
}

// The surface the model learns from `khor help` is the same one users
// read — which is the point: the help text has a gate in the CLI repo
// keeping it complete, and the agent inherits that guarantee instead
// of a copy that would rot.
const SYSTEM_PROMPT = `You are khor's in-app agent: the dispatcher of a mesh of devices. \
The user assigns a task; you carry it out through the khor CLI, which sees and moves the whole mesh.

How to act:
- The \`khor\` tool runs ONE command per call, as an argv array. There is no shell: no pipes, no redirection, no quoting.
- Learn the command surface from the binary, not from memory: when unsure, run \`khor help\` — it lists every verb with its arguments. The surface changes between versions; what help prints is current, what you remember may not be.
- Commands that only observe run freely. Commands that act need the user's approval: declare them with the \`plan\` tool first — approved steps run without further asking, one run per step; anything outside the plan comes back to the user for approval, so give it a \`why\` they can read.
- \`attach\` is interactive and cannot work here; read \`sessions\` and \`log\` instead.
- Answer in the user's language. Report what you ran, what it said, and what you conclude — short and factual.`;

export interface KhorAgent {
  agent: Agent;
  table: VerbTable;
  gate: Gate;
}

export async function createKhorAgent(setup: KhorAgentSetup): Promise<KhorAgent> {
  const run = khorRunner(setup.khorBin);
  const table = await loadVerbTable(run);
  const gate = new Gate(table, setup.mode);
  const tools = khorTools(gate, run, setup.approver);
  const surface = new Set(tools.map((t) => t.name));
  const agent = new Agent({
    initialState: {
      systemPrompt: SYSTEM_PROMPT,
      model: setup.model,
      tools,
    },
    streamFn: setup.streamFn,
    getApiKey: setup.getApiKey,
    // A second net under the gate, not the gate: this agent's surface
    // is exactly the tools built above.  Should a future pi version or
    // a config slip ever inject a default toolset (pi's coding agent
    // ships bash — one shell would make the classification
    // decorative), the unknown name dies here instead of running.
    beforeToolCall: async ({ toolCall }) =>
      surface.has(toolCall.name)
        ? undefined
        : { block: true, reason: `\`${toolCall.name}\` is not part of this agent's surface.` },
  });
  return { agent, table, gate };
}
