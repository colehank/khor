/**
 * Legacy standalone REPL for CLI/gate development. The product runtime
 * is host.ts: Pi SDK sessions with MCP and authorization owned by Rust.
 * This harness keeps its explicit terminal approvals for local experiments.
 *
 * Env:
 *   KHOR_BIN             which binary to drive (default: `khor` on PATH)
 *   KHOR_AGENT_MODE      manual (default) | bypass
 *   KHOR_AGENT_PROVIDER  a khor account word (newapi / claude / openai)
 *                        or any pi-ai provider id (default: anthropic)
 *   KHOR_AGENT_BASE_URL  the gateway's address; for a vendor, an
 *                        optional endpoint override
 *   KHOR_AGENT_KEY_FILE  a file holding the key — point it at the
 *                        account's file under ~/.khor/keys/.  Without
 *                        it, the provider's own env var answers.
 *                        Reading the accounts table itself belongs to
 *                        the serve host, not here.
 *   KHOR_AGENT_MODEL     model id.  Omit it to see the list and stop.
 */
import { readFileSync } from "node:fs";
import * as readline from "node:readline/promises";
import { stdin, stdout } from "node:process";
import { createKhorAgent } from "./session.ts";
import { listModels, openModels, pickModel } from "./models.ts";
import type { Approver } from "./tools.ts";
import type { Mode } from "./gate.ts";

function terminalApprover(rl: readline.Interface): Approver {
  return {
    async askPlan(steps) {
      stdout.write("\n-- plan --\n");
      steps.forEach((s, i) => {
        stdout.write(`${i + 1}. khor ${s.args.join(" ")}\n   ${s.why}\n`);
      });
      const answer = await rl.question("approve plan? [y/N] ");
      return answer.trim().toLowerCase() === "y";
    },
    async askCommand(args, why) {
      stdout.write(`\n-- out-of-plan action --\nkhor ${args.join(" ")}\n`);
      if (why) stdout.write(`   ${why}\n`);
      const answer = await rl.question("approve? [y/N] ");
      return answer.trim().toLowerCase() === "y";
    },
  };
}

const bin = process.env.KHOR_BIN ?? "khor";
const mode = process.env.KHOR_AGENT_MODE ?? "manual";
if (mode !== "manual" && mode !== "bypass") {
  console.error(`KHOR_AGENT_MODE must be manual or bypass, not \`${mode}\``);
  process.exit(2);
}

const keyFile = process.env.KHOR_AGENT_KEY_FILE;
const seated = openModels({
  provider: process.env.KHOR_AGENT_PROVIDER ?? "anthropic",
  baseUrl: process.env.KHOR_AGENT_BASE_URL,
  key: keyFile ? readFileSync(keyFile, "utf8").trim() : undefined,
});

const listing = await listModels(seated);
const wanted = process.env.KHOR_AGENT_MODEL;
if (!wanted) {
  console.log(`${seated.providerId}: ${listing.models.length} model(s)`);
  for (const m of listing.models) {
    console.log(`  ${m.id}\t${m.contextWindow} ctx\t${m.reasoning ? "reasoning" : ""}`);
  }
  if (listing.trouble) console.log(`list is short: ${listing.trouble.message}`);
  console.log("set KHOR_AGENT_MODEL to one of these (or to an id you know) to start");
  process.exit(2);
}
const pick = pickModel(seated, wanted);
if (!pick.listed) {
  console.log(
    `note: \`${wanted}\` is not in ${seated.providerId}'s list` +
      (listing.trouble ? ` (${listing.trouble.message})` : "") +
      "; using it anyway on the same endpoint",
  );
}

const rl = readline.createInterface({ input: stdin, output: stdout });
const { agent, table } = await createKhorAgent({
  khorBin: bin,
  mode: mode as Mode,
  approver: terminalApprover(rl),
  model: pick.model,
  streamFn: seated.models.streamSimple.bind(seated.models),
});

agent.subscribe(async (event) => {
  if (event.type === "message_update" && event.assistantMessageEvent.type === "text_delta") {
    stdout.write(event.assistantMessageEvent.delta);
  }
  if (event.type === "tool_execution_start") {
    stdout.write(`\n[${event.toolName} ${JSON.stringify(event.args)}]\n`);
  }
});

console.log(
  `khor agent (dev) — ${table.size} verbs learned from \`${bin}\`, mode ${mode}, model ${pick.model.provider}/${pick.model.id}`,
);
if (!(await seated.models.getAuth(seated.providerId))) {
  console.log(`note: no key for ${seated.providerId}; the first prompt will fail until KHOR_AGENT_KEY_FILE names one`);
}

for (;;) {
  let line: string;
  try {
    line = (await rl.question("\nyou> ")).trim();
  } catch {
    break; // piped stdin ran out — same meaning as /quit
  }
  if (line === "") continue;
  if (line === "/quit") break;
  try {
    await agent.prompt(line);
    stdout.write("\n");
  } catch (failure) {
    console.error(`\n[error] ${failure instanceof Error ? failure.message : String(failure)}`);
  }
}
rl.close();
