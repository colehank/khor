import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { loadVerbTable, khorRunner, type RunKhor } from "../src/verbs.ts";

function printing(stdout: string): RunKhor {
  return async () => ({ stdout, stderr: "", code: 0 });
}

test("a well-formed table loads", async () => {
  const run = printing('[{"word":"devices","reach":"read"},{"word":"close","reach":"act"},{"word":"_verbs","reach":"inner"}]');
  const table = await loadVerbTable(run);
  assert.equal(table.size, 3);
  assert.equal(table.get("devices"), "read");
  assert.equal(table.get("close"), "act");
  assert.equal(table.get("_verbs"), "inner");
});

test("a reach word the gate does not know fails the whole load — no silent row-skip", async () => {
  const run = printing('[{"word":"devices","reach":"read"},{"word":"top","reach":"reed"}]');
  await assert.rejects(() => loadVerbTable(run), /reed/);
});

test("a duplicated word fails the load — a table that answers twice is as bad as one that does not answer", async () => {
  const run = printing('[{"word":"devices","reach":"read"},{"word":"devices","reach":"act"}]');
  await assert.rejects(() => loadVerbTable(run), /twice/);
});

test("non-JSON output fails the load", async () => {
  await assert.rejects(() => loadVerbTable(printing("Usage: khor <verb>")), /not JSON/);
});

test("an empty table fails the load", async () => {
  await assert.rejects(() => loadVerbTable(printing("[]")), /no verbs/);
});

test("a non-zero exit fails the load and carries stderr", async () => {
  const run: RunKhor = async () => ({ stdout: "", stderr: "store is locked", code: 3 });
  await assert.rejects(() => loadVerbTable(run), /store is locked/);
});

// Against the real binary when the workspace has one built.  Counts
// change as verbs are added, so assert shape and known words, not
// totals.
test("the real binary's table loads and classifies known words", async (t) => {
  const bin = fileURLToPath(new URL("../../../target/release/khor", import.meta.url));
  if (!existsSync(bin)) {
    t.skip(`no built binary at ${bin}`);
    return;
  }
  const table = await loadVerbTable(khorRunner(bin));
  assert.equal(table.get("devices"), "read");
  assert.equal(table.get("close"), "act");
  assert.equal(table.get("_verbs"), "inner");
  for (const [word, reach] of table) {
    assert.ok(["read", "act", "inner"].includes(reach), `\`${word}\` has reach \`${reach}\``);
  }
});
