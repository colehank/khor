import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { createServer } from "node:http";
import { manage } from "../src/management.ts";

test("Pi provider setup persists account-scoped credentials and returns models without exposing keys", async () => {
  const root = await mkdtemp(join(tmpdir(), "khor-provider-"));
  try {
    for (const [id, key] of [["first", "fixture-first-key"], ["second", "fixture-second-key"]]) {
      const credentialPath = join(root, id, "auth.json");
      const questions: string[] = [];
      const result = await manage({ type: "manage", operation: "login", account: { provider: "anthropic" }, credentialPath, authType: "api_key", deviceId: "fixture-host" }, async (p) => { questions.push(p.type); return key!; }, () => {});
      assert.deepEqual(questions, ["secret"]);
      assert.equal(result.credentialType, "api_key");
      assert(!JSON.stringify(result).includes(key!));
      assert.equal(JSON.parse(await readFile(credentialPath, "utf8")).anthropic.key, key);
      const listed = await manage({ type: "manage", operation: "models", account: { provider: "anthropic" }, credentialPath, deviceId: "fixture-host" }, async () => { throw new Error("must not request credentials while listing"); }, () => {});
      assert(listed.models!.length > 0); assert(!JSON.stringify(listed).includes(key!));
    }
    assert.equal(JSON.parse(await readFile(join(root,"first","auth.json"), "utf8")).anthropic.key, "fixture-first-key");
  } finally { await rm(root, { recursive: true, force: true }); }
});


test("a private gateway can explicitly use no authentication and refresh its model catalog", async () => {
  const root = await mkdtemp(join(tmpdir(), "khor-keyless-"));
  let authorization: string | undefined;
  const server = createServer((request, response) => {
    authorization = request.headers.authorization;
    assert.equal(request.url, "/v1/models");
    response.setHeader("content-type", "application/json"); response.end(JSON.stringify({ data: [{ id: "private-model" }] }));
  });
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  const address = server.address(); assert(address && typeof address === "object");
  const account = { provider: "newapi", baseUrl: `http://127.0.0.1:${address.port}` }, credentialPath = join(root, "auth.json");
  try {
    await manage({ type: "manage", operation: "login", account, credentialPath, authType: "api_key", deviceId: "fixture" }, async (prompt) => { assert.equal(prompt.type, "select"); return "none"; }, () => {});
    const result = await manage({ type: "manage", operation: "models", account, credentialPath, deviceId: "fixture" }, async () => { throw new Error("unexpected prompt"); }, () => {});
    assert.equal(result.trouble, null); assert.equal(result.models?.[0]?.id, "private-model"); assert.equal(authorization, undefined);
  } finally { server.closeAllConnections(); await new Promise<void>((done) => server.close(() => done())); await rm(root, { recursive: true, force: true }); }
});

test("standalone OAuth loads outside the repository and cancels without a token exchange", { timeout: 12000 }, async () => {
  const root = await mkdtemp(join(tmpdir(), "khor-oauth-bundle-"));
  const binary = process.env.KHOR_TEST_AGENT_BIN;
  const child = spawn(binary ? resolve(binary) : process.execPath, binary ? [] : [fileURLToPath(new URL("../src/host.ts", import.meta.url))], { cwd: root, env: { PATH: process.env.PATH }, stdio: ["pipe", "pipe", "pipe"] });
  const events: any[] = []; let error = "";
  child.stderr.on("data", (data) => { error += String(data); });
  const lines = createInterface({ input: child.stdout });
  try {
    const exit = new Promise<number | null>((done, reject) => { child.on("error", reject); child.on("exit", done); });
    const prompted = new Promise<void>((done, reject) => {
      lines.on("line", (line) => {
        const frame = JSON.parse(line); events.push(frame);
        if (frame.type === "failed") reject(new Error(frame.error));
        if (frame.type === "auth_prompt" && frame.prompt.type === "select") child.stdin.write(JSON.stringify({ type: "auth_reply", ask: frame.ask, value: "copy_code" }) + "\n");
        if (frame.type === "auth_event" && frame.event.type === "auth_url") { assert.match(frame.event.url, /^https:\/\/claude\.ai\/oauth\/authorize/); done(); }
      });
      child.once("exit", () => reject(new Error(`OAuth exited before authorization: ${error}`)));
    });
    child.stdin.write(JSON.stringify({ type: "manage", operation: "login", account: { provider: "anthropic" }, authType: "oauth", credentialPath: join(root, "auth.json"), deviceId: "fixture" }) + "\n");
    let timer: ReturnType<typeof setTimeout> | undefined;
    try { await Promise.race([prompted, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("OAuth prompt timeout")), 8000); })]); } finally { clearTimeout(timer); }
    child.stdin.end();
    await exit;
    assert(!events.some((e) => e.type === "result"), "cancelled OAuth must not create credentials");
    assert(!JSON.stringify(events).includes("Cannot find module"));
  } finally { child.kill(); lines.close(); await rm(root, { recursive: true, force: true }); }
});
