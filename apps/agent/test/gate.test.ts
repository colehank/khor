import { test } from "node:test";
import assert from "node:assert/strict";
import { Gate, type VerbTable } from "../src/gate.ts";

function table(): VerbTable {
  return new Map([
    ["devices", "read"],
    ["top", "read"],
    ["close", "act"],
    ["tell", "act"],
    ["_ghost", "inner"],
  ] as const);
}

test("a read verb runs free in manual mode", () => {
  const gate = new Gate(table(), "manual");
  assert.deepEqual(gate.decide(["devices"]), { kind: "free" });
});

test("an act verb asks in manual mode when no plan covers it", () => {
  const gate = new Gate(table(), "manual");
  assert.deepEqual(gate.decide(["close", "tui/x"]), { kind: "ask" });
});

test("an act verb runs free in bypass mode", () => {
  const gate = new Gate(table(), "bypass");
  assert.deepEqual(gate.decide(["close", "tui/x"]), { kind: "free" });
});

test("an inner verb is refused even in bypass — bypass waives approvals, not machinery", () => {
  const gate = new Gate(table(), "bypass");
  const decision = gate.decide(["_ghost"]);
  assert.equal(decision.kind, "refuse");
});

test("an unknown word is refused even in bypass — absence must not become a neighbouring tier", () => {
  const gate = new Gate(table(), "bypass");
  const decision = gate.decide(["frobnicate"]);
  assert.equal(decision.kind, "refuse");
  assert.match((decision as { why: string }).why, /frobnicate/);
});

test("nothing typed is refused", () => {
  const gate = new Gate(table(), "manual");
  assert.equal(gate.decide([]).kind, "refuse");
});

test("an approved step frees exactly that argv once — the second identical run asks again", () => {
  const gate = new Gate(table(), "manual");
  gate.license([["close", "tui/x"]]);
  assert.deepEqual(gate.decide(["close", "tui/x"]), { kind: "free" });
  assert.deepEqual(gate.decide(["close", "tui/x"]), { kind: "ask" });
});

test("a step licenses only its exact argv — one extra flag and it asks", () => {
  const gate = new Gate(table(), "manual");
  gate.license([["close", "tui/x"]]);
  assert.deepEqual(gate.decide(["close", "tui/x", "--force"]), { kind: "ask" });
  assert.equal(gate.outstanding(), 1);
});

test("a licensed step does not free a different act verb", () => {
  const gate = new Gate(table(), "manual");
  gate.license([["close", "tui/x"]]);
  assert.deepEqual(gate.decide(["tell", "tui/x", "hi"]), { kind: "ask" });
});
