import { test } from "node:test";
import assert from "node:assert/strict";
import { GATEWAY, gatewayBase, listModels, openModels, pickModel, providerIdFor } from "../src/models.ts";

interface Seen {
  url: string;
  authorization: string | undefined;
}

function answering(status: number, body: unknown): { fetchImpl: typeof fetch; seen: Seen[] } {
  const seen: Seen[] = [];
  const fetchImpl = (async (input: string | URL | Request, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    seen.push({ url: String(input), authorization: headers.get("authorization") ?? undefined });
    return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
  }) as typeof fetch;
  return { fetchImpl, seen };
}

test("khor's account words map onto pi-ai provider ids", () => {
  assert.equal(providerIdFor("claude"), "anthropic");
  assert.equal(providerIdFor("Anthropic"), "anthropic");
  assert.equal(providerIdFor("new-api"), GATEWAY);
  assert.equal(providerIdFor("deepseek"), "deepseek");
});

test("an unknown provider word is refused, naming what is known", () => {
  assert.throws(() => providerIdFor("frobnicate"), /frobnicate.*anthropic/s);
});

test("a gateway base URL means the same gateway however it is spelled", () => {
  assert.equal(gatewayBase("https://gw.example"), "https://gw.example/v1");
  assert.equal(gatewayBase("https://gw.example/"), "https://gw.example/v1");
  assert.equal(gatewayBase("https://gw.example/v1/"), "https://gw.example/v1");
});

test("a gateway lists what /v1/models answers, asked with the account's key", async () => {
  const { fetchImpl, seen } = answering(200, { data: [{ id: "alpha" }, { id: "beta" }] });
  const seated = openModels({ provider: "newapi", baseUrl: "https://gw.example", key: "k-1" }, fetchImpl);
  const listing = await listModels(seated);
  assert.deepEqual(
    listing.models.map((m) => m.id),
    ["alpha", "beta"],
  );
  assert.equal(listing.trouble, undefined);
  assert.equal(seen.length, 1);
  assert.equal(seen[0]!.url, "https://gw.example/v1/models");
  assert.equal(seen[0]!.authorization, "Bearer k-1");
});

test("a gateway's refusal comes back with its status and its reason", async () => {
  const { fetchImpl } = answering(403, { error: { message: "group not permitted for this token" } });
  const seated = openModels({ provider: "newapi", baseUrl: "https://gw.example", key: "k-1" }, fetchImpl);
  const listing = await listModels(seated);
  assert.equal(listing.models.length, 0);
  assert.match(listing.trouble?.message ?? "", /403/);
  assert.match(listing.trouble?.message ?? "", /group not permitted/);
});

test("a gateway without a key lists nothing and says why", async () => {
  const { fetchImpl, seen } = answering(200, { data: [{ id: "alpha" }] });
  const seated = openModels({ provider: "newapi", baseUrl: "https://gw.example" }, fetchImpl);
  const listing = await listModels(seated);
  assert.equal(listing.models.length, 0);
  assert.match(listing.trouble?.message ?? "", /no key/);
  assert.equal(seen.length, 0);
});

test("an id the gateway did not list is still picked, marked unlisted, on the gateway's endpoint", async () => {
  const { fetchImpl } = answering(200, { data: [{ id: "alpha" }] });
  const seated = openModels({ provider: "newapi", baseUrl: "https://gw.example", key: "k-1" }, fetchImpl);
  await listModels(seated);
  const pick = pickModel(seated, "gamma");
  assert.equal(pick.listed, false);
  assert.equal(pick.model.id, "gamma");
  assert.equal(pick.model.api, "openai-completions");
  assert.equal(pick.model.baseUrl, "https://gw.example/v1");
  assert.equal(pickModel(seated, "alpha").listed, true);
});

test("a vendor account picks from pi-ai's catalog", () => {
  const seated = openModels({ provider: "claude", key: "k-1" });
  const pick = pickModel(seated, "claude-haiku-4-5");
  assert.equal(pick.listed, true);
  assert.equal(pick.model.api, "anthropic-messages");
  assert.equal(pick.model.provider, "anthropic");
});

test("an id newer than the catalog is built from a sibling — same API and endpoint, only the id differs", () => {
  const seated = openModels({ provider: "claude", key: "k-1" });
  const pick = pickModel(seated, "claude-fable-9");
  assert.equal(pick.listed, false);
  assert.equal(pick.model.id, "claude-fable-9");
  assert.equal(pick.model.api, "anthropic-messages");
  assert.equal(pick.model.baseUrl, "https://api.anthropic.com");
});

test("an account's base URL moves a vendor model to that endpoint, spelled as given", () => {
  const seated = openModels({ provider: "openai", baseUrl: "https://proxy.example/openai/v1/", key: "k-1" });
  const pick = pickModel(seated, "gpt-4");
  assert.equal(pick.model.baseUrl, "https://proxy.example/openai/v1");
});

test("the account's key reaches pi-ai with no environment variable set", async () => {
  const saved = process.env.ANTHROPIC_API_KEY;
  delete process.env.ANTHROPIC_API_KEY;
  try {
    const bare = openModels({ provider: "claude" });
    assert.equal(await bare.models.getAuth("anthropic"), undefined);
    const seated = openModels({ provider: "claude", key: "k-1" });
    const auth = await seated.models.getAuth("anthropic");
    assert.equal(auth?.auth.apiKey, "k-1");
    assert.equal(auth?.source, "stored credential");
  } finally {
    if (saved !== undefined) process.env.ANTHROPIC_API_KEY = saved;
  }
});
