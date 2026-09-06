/**
 * Terminal dev loop for the khor agent — scaffolding, not the seat.
 * The real seat is a conversation session under `khor serve`
 * (docs/AGENT.md); until that host exists, this REPL is the way to sit
 * where the user will sit: same gate, same plan flow, approvals on
 * stdin instead of 待批.
 *
 * Env:
 *   KHOR_BIN             which binary to drive (default: `khor` on PATH)
 *   KHOR_AGENT_MODE      manual (default) | bypass
 *   KHOR_AGENT_PROVIDER  anthropic (default) | newapi
 *   KHOR_AGENT_MODEL     model id (default: claude-sonnet-4-6)
 *
 * anthropic:  ANTHROPIC_API_KEY carries the key.
 * newapi:     KHOR_AGENT_BASE_URL names the gateway (…/v1 is added),
 *             KHOR_AGENT_KEY_FILE names a file holding the key —
 *             point it at the account's file under ~/.khor/keys/.
 *             Reading the accounts table itself belongs to the serve
 *             host, not here.
 */
import { readFileSync } from "node:fs";
import * as readline from "node:readline/promises";
import { stdin, stdout } from "node:process";
import { createModels, createProvider, type Model } from "@earendil-works/pi-ai";
import { anthropicProvider } from "@earendil-works/pi-ai/providers/anthropic";
import { openAICompletionsApi } from "@earendil-works/pi-ai/api/openai-completions.lazy";
import { createKhorAgent } from "./session.ts";
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

function pickModel(): { model: Model<any>; models: ReturnType<typeof createModels> } {
  const models = createModels();
  const provider = process.env.KHOR_AGENT_PROVIDER ?? "anthropic";

  if (provider === "anthropic") {
    models.setProvider(anthropicProvider());
    const id = process.env.KHOR_AGENT_MODEL ?? "claude-sonnet-4-6";
    const model = models.getModel("anthropic", id);
    if (!model) {
      console.error(`pi-ai knows no model \`anthropic/${id}\``);
      process.exit(2);
    }
    return { model, models };
  }

  if (provider === "newapi") {
    const base = process.env.KHOR_AGENT_BASE_URL;
    const keyFile = process.env.KHOR_AGENT_KEY_FILE;
    const id = process.env.KHOR_AGENT_MODEL;
    if (!base || !keyFile || !id) {
      console.error("newapi needs KHOR_AGENT_BASE_URL, KHOR_AGENT_KEY_FILE and KHOR_AGENT_MODEL");
      process.exit(2);
    }
    const key = readFileSync(keyFile, "utf8").trim();
    const baseUrl = base.replace(/\/$/, "") + "/v1";
    const model: Model<"openai-completions"> = {
      id,
      name: id,
      api: "openai-completions",
      provider: "newapi",
      baseUrl,
      reasoning: false,
      input: ["text"],
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
      contextWindow: 128_000,
      maxTokens: 8_192,
    };
    models.setProvider(
      createProvider({
        id: "newapi",
        name: "newapi",
        baseUrl,
        auth: { apiKey: { name: "newapi key", resolve: async () => ({ auth: { apiKey: key } }) } },
        models: [model],
        api: openAICompletionsApi(),
      }),
    );
    return { model, models };
  }

  console.error(`KHOR_AGENT_PROVIDER must be anthropic or newapi, not \`${provider}\``);
  process.exit(2);
}

const bin = process.env.KHOR_BIN ?? "khor";
const mode = process.env.KHOR_AGENT_MODE ?? "manual";
if (mode !== "manual" && mode !== "bypass") {
  console.error(`KHOR_AGENT_MODE must be manual or bypass, not \`${mode}\``);
  process.exit(2);
}

const { model, models } = pickModel();
const rl = readline.createInterface({ input: stdin, output: stdout });
const { agent, table } = await createKhorAgent({
  khorBin: bin,
  mode: mode as Mode,
  approver: terminalApprover(rl),
  model,
  streamFn: models.streamSimple.bind(models),
});

agent.subscribe(async (event) => {
  if (event.type === "message_update" && event.assistantMessageEvent.type === "text_delta") {
    stdout.write(event.assistantMessageEvent.delta);
  }
  if (event.type === "tool_execution_start") {
    stdout.write(`\n[${event.toolName} ${JSON.stringify(event.args)}]\n`);
  }
});

console.log(`khor agent (dev) — ${table.size} verbs learned from \`${bin}\`, mode ${mode}, model ${model.provider}/${model.id}`);
if ((process.env.KHOR_AGENT_PROVIDER ?? "anthropic") === "anthropic" && !process.env.ANTHROPIC_API_KEY) {
  console.log("note: ANTHROPIC_API_KEY is not set; the first prompt will fail until it is");
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
