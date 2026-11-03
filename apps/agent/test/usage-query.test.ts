import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createEventBus, SessionManager } from "@earendil-works/pi-coding-agent";
import { manage } from "../src/management.ts";
import { requestUsage } from "../src/usage-query.ts";

const scope = () => { const session = SessionManager.inMemory(tmpdir()); return { session, sessionId: session.getSessionId() }; };

test("usage has a concrete no-service result without waiting for a timeout", async () => {
  assert.deepEqual(await requestUsage(createEventBus(), scope(), "deepseek"), { status: "unavailable", providerId: "deepseek", reason: "no-service" });
});

test("usage offers bind the exact Pi session and accept only the first response", async () => {
  const bus = createEventBus(), owner = scope();
  bus.on("usage:query:v1", (raw: unknown) => {
    const request = raw as any;
    assert.equal(request.session, owner.session); assert.equal(request.sessionId, owner.sessionId);
    assert.equal(request.providerId, "deepseek"); assert.equal(request.refresh, true);
    assert.equal(request.respond(Promise.resolve({ status: "unavailable", providerId: "deepseek", reason: "auth-unavailable" })), true);
    assert.equal(request.respond(Promise.reject(new Error("ignored duplicate"))), false);
  });
  assert.equal((await requestUsage(bus, owner, "deepseek")).status, "unavailable");
});

test("a late response is declined and its rejection is observed", async () => {
  const bus = createEventBus(); let accepted: boolean | undefined;
  bus.on("usage:query:v1", (raw: unknown) => {
    queueMicrotask(() => { accepted = (raw as any).respond(Promise.reject(new Error("late"))); });
  });
  const result = await requestUsage(bus, scope(), "deepseek");
  assert.equal(result.status === "unavailable" && result.reason, "no-service");
  await new Promise(resolve => setImmediate(resolve)); assert.equal(accepted, false);
});

test("cancellation before dispatch never asks a provider", async () => {
  const bus = createEventBus(); let called = false;
  bus.on("usage:query:v1", () => { called = true; });
  const controller = new AbortController(); controller.abort();
  const result = await requestUsage(bus, scope(), "deepseek", controller.signal);
  assert.equal(result.status === "unavailable" && result.reason, "cancelled"); assert.equal(called, false);
});

test("the request deadline aborts provider work and ignores its late result", async () => {
  const bus = createEventBus(); let signal: AbortSignal | undefined, complete: ((v: unknown) => void) | undefined;
  bus.on("usage:query:v1", (raw: unknown) => { const request = raw as any; signal = request.signal; request.respond(new Promise(resolve => { complete = resolve; })); });
  const result = await requestUsage(bus, scope(), "deepseek", undefined, 10);
  assert.equal(result.status === "unavailable" && result.reason, "timeout"); assert.equal(signal?.aborted, true);
  complete?.({ status: "unavailable", reason: "auth-unavailable" });
});

test("another provider's response is a protocol error, not this account's balance", async () => {
  const bus = createEventBus();
  bus.on("usage:query:v1", (raw: unknown) => (raw as any).respond({ status: "unavailable", providerId: "other-account", reason: "auth-unavailable" }));
  const result = await requestUsage(bus, scope(), "deepseek");
  assert.equal(result.status === "unavailable" && result.reason, "protocol-error");
});

test("unknown reasons and throwing reply fields fail closed without leaving a deadline alive", async () => {
  for (const reply of [
    { status: "unavailable", reason: "invented-status" },
    { get providerId() { throw new Error("Malformed reply"); } },
  ]) {
    const bus = createEventBus();
    bus.on("usage:query:v1", (raw: unknown) => (raw as any).respond(reply));
    const result = await requestUsage(bus, scope(), "deepseek");
    assert.equal(result.status === "unavailable" && result.reason, "protocol-error");
  }
});

test("real Pi and upstream usage query two isolated accounts without model calls or duplicate credentials", { timeout: 15000 }, async () => {
  const root = await mkdtemp(join(tmpdir(), "khor-pi-usage-")), originalFetch = globalThis.fetch;
  const previousDir = process.env.PI_CODING_AGENT_DIR; process.env.PI_CODING_AGENT_DIR = root;
  const requests: { url: string; authorization: string | null }[] = [];
  globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
    const url = input instanceof Request ? input.url : String(input), authorization = new Headers(init?.headers).get("authorization");
    requests.push({ url, authorization }); assert.equal(url, "https://api.deepseek.com/user/balance");
    return new Response(JSON.stringify({ is_available: true, balance_infos: [
      { currency: "CNY", total_balance: authorization === "Bearer fixture-A" ? "42.00" : "17.00", granted_balance: "0", topped_up_balance: "42.00" },
      { currency: "USD", total_balance: "0.00", granted_balance: "0", topped_up_balance: "0" },
    ] }), { headers: { "content-type": "application/json" } });
  }) as typeof fetch;
  try {
    for (const [key, expected] of [["fixture-A", 42], ["fixture-B", 17]] as const) {
      const dir = join(root, key); await mkdir(join(dir, ".pi", "extensions"), { recursive: true });
      await writeFile(join(dir, ".pi", "extensions", "unexpected.ts"), "throw new Error('Project extension must not load')");
      await writeFile(join(dir, "auth.json"), JSON.stringify({ deepseek: { type: "api_key", key } }));
      const managed = await manage({ type: "manage", operation: "usage", account: { provider: "deepseek" }, credentialPath: join(dir, "auth.json"), deviceId: "fixture" },
        async () => { throw new Error("Usage must not prompt for login"); }, () => { throw new Error("Usage must not change authentication"); });
      const result = managed.usage; assert(result);
      assert.equal(result.status, "ready"); if (result.status !== "ready") throw new Error(JSON.stringify(result));
      assert(result.report.metrics.some(metric => metric.currency === "CNY" && Number(metric.value) === expected));
      assert(result.report.metrics.some(metric => metric.currency === "USD" && Number(metric.value) === 0));
      assert(!JSON.stringify(result).includes(key), "public usage reports never include credentials");
    }
    const firstB = requests.findIndex(request => request.authorization === "Bearer fixture-B");
    assert(firstB > 0);
    assert(requests.slice(0, firstB).every(request => request.authorization === "Bearer fixture-A"));
    assert(requests.slice(firstB).every(request => request.authorization === "Bearer fixture-B"));
  } finally {
    globalThis.fetch = originalFetch;
    if (previousDir === undefined) delete process.env.PI_CODING_AGENT_DIR; else process.env.PI_CODING_AGENT_DIR = previousDir;
    await rm(root, { recursive: true, force: true });
  }
});

test("cancelling a real Pi usage management request releases its upstream fetches", { timeout: 5000 }, async () => {
  const root = await mkdtemp(join(tmpdir(), "khor-pi-usage-cancel-")), originalFetch = globalThis.fetch;
  const previousDir = process.env.PI_CODING_AGENT_DIR; process.env.PI_CODING_AGENT_DIR = root;
  const signals: AbortSignal[] = [];
  let enter!: () => void; const entered = new Promise<void>(resolve => { enter = resolve; });
  globalThis.fetch = ((_input: string | URL | Request, init?: RequestInit) => {
    const signal = init?.signal; assert(signal); signals.push(signal); enter();
    return new Promise<Response>((_resolve, reject) => {
      const abort = () => reject(new DOMException("Fixture cancelled", "AbortError"));
      if (signal.aborted) abort(); else signal.addEventListener("abort", abort, { once: true });
    });
  }) as typeof fetch;
  try {
    const controller = new AbortController();
    const pending = manage({ type: "manage", operation: "usage", account: { provider: "deepseek", key: "fixture-cancel" }, credentialPath: join(root, "auth.json"), deviceId: "fixture" },
      async () => { throw new Error("Unexpected login"); }, () => {}, controller.signal);
    await entered; controller.abort(); const result = (await pending).usage;
    assert(result); assert.equal(result.status === "unavailable" && result.reason, "cancelled");
    assert(signals.length > 0 && signals.every(signal => signal.aborted), "no fetch remains owned after management finishes");
  } finally {
    globalThis.fetch = originalFetch;
    if (previousDir === undefined) delete process.env.PI_CODING_AGENT_DIR; else process.env.PI_CODING_AGENT_DIR = previousDir;
    await rm(root, { recursive: true, force: true });
  }
});
