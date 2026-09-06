import { test } from "node:test";
import assert from "node:assert/strict";
import { Gate, type VerbTable } from "../src/gate.ts";
import { khorTools, type Approver } from "../src/tools.ts";
import type { KhorRun, RunKhor } from "../src/verbs.ts";

function table(): VerbTable {
  return new Map([
    ["devices", "read"],
    ["close", "act"],
    ["_ghost", "inner"],
  ] as const);
}

function recordingRunner(script: Record<string, KhorRun>): { run: RunKhor; calls: string[][] } {
  const calls: string[][] = [];
  const run: RunKhor = async (args) => {
    calls.push([...args]);
    const hit = script[args.join(" ")];
    if (!hit) throw new Error(`the fake runner has no script for: ${args.join(" ")}`);
    return hit;
  };
  return { run, calls };
}

// The control arm: a free command must never reach the approver, and
// a test that cannot tell needs this to be able to fail.
const dead: Approver = {
  askPlan: async () => {
    throw new Error("the approver was consulted");
  },
  askCommand: async () => {
    throw new Error("the approver was consulted");
  },
};

function answering(planAnswer: boolean, commandAnswer: boolean) {
  const asked = { plans: 0, commands: 0 };
  const approver: Approver = {
    askPlan: async () => {
      asked.plans += 1;
      return planAnswer;
    },
    askCommand: async () => {
      asked.commands += 1;
      return commandAnswer;
    },
  };
  return { approver, asked };
}

function toolNamed(tools: ReturnType<typeof khorTools>, name: string) {
  const tool = tools.find((t) => t.name === name);
  assert.ok(tool, `no tool named ${name}`);
  return tool!;
}

test("a read command runs without the approver being consulted", async () => {
  const { run, calls } = recordingRunner({ devices: { stdout: "two machines\n", stderr: "", code: 0 } });
  const tools = khorTools(new Gate(table(), "manual"), run, dead);
  const result = await toolNamed(tools, "khor").execute("t1", { args: ["devices"] });
  assert.equal(result.content[0]!.type, "text");
  assert.match((result.content[0] as { text: string }).text, /two machines/);
  assert.equal(calls.length, 1);
});

test("a declined out-of-plan action does not run", async () => {
  const { run, calls } = recordingRunner({});
  const { approver, asked } = answering(true, false);
  const tools = khorTools(new Gate(table(), "manual"), run, approver);
  await assert.rejects(
    () => toolNamed(tools, "khor").execute("t1", { args: ["close", "tui/x"] }),
    /declined/,
  );
  assert.equal(asked.commands, 1);
  assert.equal(calls.length, 0);
});

test("an approved plan step then runs without asking again", async () => {
  const { run, calls } = recordingRunner({ "close tui/x": { stdout: "closed\n", stderr: "", code: 0 } });
  const { approver, asked } = answering(true, true);
  const gate = new Gate(table(), "manual");
  const tools = khorTools(gate, run, approver);
  await toolNamed(tools, "plan").execute("t1", {
    steps: [{ args: ["close", "tui/x"], why: "it is settled and done" }],
  });
  assert.equal(asked.plans, 1);
  await toolNamed(tools, "khor").execute("t2", { args: ["close", "tui/x"] });
  assert.equal(asked.commands, 0);
  assert.equal(calls.length, 1);
});

test("a plan of only reads returns without consulting the user", async () => {
  const { run } = recordingRunner({});
  const gate = new Gate(table(), "manual");
  const tools = khorTools(gate, run, dead);
  const result = await toolNamed(tools, "plan").execute("t1", {
    steps: [{ args: ["devices"], why: "look around" }],
  });
  assert.match((result.content[0] as { text: string }).text, /freely/);
  assert.equal(gate.outstanding(), 0);
});

test("a plan naming an inner or unknown verb dies before the user sees it", async () => {
  const { run } = recordingRunner({});
  const tools = khorTools(new Gate(table(), "manual"), run, dead);
  await assert.rejects(
    () => toolNamed(tools, "plan").execute("t1", { steps: [{ args: ["_ghost"], why: "?" }] }),
    /machinery/,
  );
  await assert.rejects(
    () => toolNamed(tools, "plan").execute("t2", { steps: [{ args: ["frobnicate"], why: "?" }] }),
    /frobnicate/,
  );
});

test("a refused command never reaches the runner", async () => {
  const { run, calls } = recordingRunner({});
  const tools = khorTools(new Gate(table(), "bypass"), run, dead);
  await assert.rejects(() => toolNamed(tools, "khor").execute("t1", { args: ["_ghost"] }), /machinery/);
  assert.equal(calls.length, 0);
});

test("a failing command surfaces stderr as the error", async () => {
  const { run } = recordingRunner({ devices: { stdout: "", stderr: "no such store\n", code: 2 } });
  const tools = khorTools(new Gate(table(), "manual"), run, dead);
  await assert.rejects(() => toolNamed(tools, "khor").execute("t1", { args: ["devices"] }), /no such store/);
});

test("long output is cut with a notice", async () => {
  const long = "x".repeat(64 * 1024);
  const { run } = recordingRunner({ devices: { stdout: long, stderr: "", code: 0 } });
  const tools = khorTools(new Gate(table(), "manual"), run, dead);
  const result = await toolNamed(tools, "khor").execute("t1", { args: ["devices"] });
  const text = (result.content[0] as { text: string }).text;
  assert.ok(text.length < long.length);
  assert.match(text, /output truncated/);
});
