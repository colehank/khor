/**
 * Which model answers, and through whom — pi-ai's provider layer bent
 * to khor's accounts table.
 *
 * Two facts shape this file.  First, an account's key lives on its
 * home machine and is handed to this process explicitly (a
 * CredentialStore holding exactly one credential), never through an
 * environment variable — `khor serve` runs under launchd with an
 * environment that has none.  Second, the catalog is advisory and the
 * person's choice is sovereign: pi-ai's built-in catalog lags every
 * vendor release, and a gateway's /models answer depends on the token's
 * group.  A model id nobody listed still has to be reachable, so an
 * unlisted id is built by cloning a listed sibling — same provider,
 * same API, same endpoint, only the id differs — and reported as
 * unlisted rather than refused.
 *
 * Every pi-ai provider is accepted by its own id; khor's account words
 * map onto them (`claude` → anthropic).  A `newapi` account is an
 * OpenAI-compatible gateway with a live model list.
 */
import {
  createModels,
  createProvider,
  type Credential,
  type CredentialStore,
  type Model,
  type MutableModels,
} from "@earendil-works/pi-ai";
import { builtinProviders, getBuiltinProviders } from "@earendil-works/pi-ai/providers/all";
import { openAICompletionsApi } from "@earendil-works/pi-ai/api/openai-completions.lazy";

/** The one provider id this file coins: any OpenAI-compatible gateway. */
export const GATEWAY = "newapi";

const BUILTIN: ReadonlySet<string> = new Set(getBuiltinProviders());

/** What the agent needs to know about an account.  No home, no budget
 * — those are the quota page's business. */
export interface AccountSeat {
  /** A khor account word (`newapi`, `claude`, `openai`) or any pi-ai provider id. */
  provider: string;
  /** Required for a gateway; optional override of a vendor's endpoint. */
  baseUrl?: string;
  /** Absent = let pi-ai's own ambient resolution (env vars) answer. */
  key?: string;
  /** Provider-scoped extras some vendors need (Azure resource, Cloudflare account id). */
  env?: Record<string, string>;
}

export interface Seated {
  models: MutableModels;
  providerId: string;
  seat: AccountSeat;
}

export function knownProviders(): string[] {
  return [GATEWAY, "claude", "moonshot", "moonshot-cn", ...BUILTIN];
}

/** khor's account words (`khor_sync::accounts::Provider::WORDS`) onto
 * pi-ai's ids; a word that is already a pi-ai id passes through. */
export function providerIdFor(word: string): string {
  const w = word.trim().toLowerCase();
  if (w === "claude") return "anthropic";
  if (w === "newapi" || w === "new-api") return GATEWAY;
  if (w === "moonshot") return "moonshotai";
  if (w === "moonshot-cn") return "moonshotai-cn";
  if (BUILTIN.has(w)) return w;
  throw new Error(`\`${word}\` is not a provider this agent knows. Known: ${knownProviders().join(", ")}`);
}

function seatCredentials(providerId: string, seat: AccountSeat): CredentialStore {
  const held = new Map<string, Credential>();
  if (seat.key) held.set(providerId, { type: "api_key", key: seat.key, env: seat.env });
  return {
    async read(id) {
      return held.get(id);
    },
    async list() {
      return [...held].map(([id, c]) => ({ providerId: id, type: c.type }));
    },
    async modify(id, fn) {
      const next = await fn(held.get(id));
      if (next) held.set(id, next);
      return held.get(id);
    },
    async delete(id) {
      held.delete(id);
    },
  };
}

/** `https://gw.example`, `https://gw.example/`, `https://gw.example/v1/` all mean the same gateway. */
export function gatewayBase(url: string): string {
  const bare = url.trim().replace(/\/+$/, "");
  return bare.endsWith("/v1") ? bare : `${bare}/v1`;
}

function gatewayModel(baseUrl: string, id: string): Model<"openai-completions"> {
  // A gateway lists ids, not sizes.  These are the figures a modern
  // model clears; a person who knows better picks a listed sibling.
  return {
    id,
    name: id,
    api: "openai-completions",
    provider: GATEWAY,
    baseUrl,
    reasoning: false,
    input: ["text"],
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    contextWindow: 128_000,
    maxTokens: 8_192,
  };
}

async function bodyHint(res: Response): Promise<string> {
  try {
    const text = (await res.text()).trim();
    return text ? `: ${text.slice(0, 200)}` : "";
  } catch {
    return "";
  }
}

function gatewayProvider(baseUrl: string, fetchImpl: typeof fetch) {
  return createProvider<"openai-completions">({
    id: GATEWAY,
    name: "OpenAI-compatible gateway",
    baseUrl,
    auth: {
      apiKey: {
        name: "gateway key",
        resolve: async ({ credential }) =>
          credential?.key ? { auth: { apiKey: credential.key }, source: "khor account" } : undefined,
      },
    },
    models: [],
    // The live list.  A refusal is thrown whole — status and body —
    // because a gateway's "no" carries the reason (which group the
    // token lacks), and an empty list would read as "no models".
    fetchModels: async ({ credential, signal }) => {
      const headers: Record<string, string> = {};
      if (credential?.type === "api_key" && credential.key) headers.authorization = `Bearer ${credential.key}`;
      const res = await fetchImpl(`${baseUrl}/models`, { headers, signal });
      if (!res.ok) {
        throw new Error(`${baseUrl}/models answered ${res.status}${await bodyHint(res)}`);
      }
      const body = (await res.json()) as { data?: { id?: unknown }[] };
      const ids = (body.data ?? []).map((m) => m.id).filter((id): id is string => typeof id === "string");
      return ids.map((id) => gatewayModel(baseUrl, id));
    },
    api: openAICompletionsApi(),
  });
}

export function openModels(seat: AccountSeat, fetchImpl: typeof fetch = fetch): Seated {
  const providerId = providerIdFor(seat.provider);
  const models = createModels({ credentials: seatCredentials(providerId, seat) });
  if (providerId === GATEWAY) {
    if (!seat.baseUrl) throw new Error("a gateway account needs a base URL");
    models.setProvider(gatewayProvider(gatewayBase(seat.baseUrl), fetchImpl));
  } else {
    const provider = builtinProviders().find((p) => p.id === providerId);
    if (!provider) throw new Error(`pi-ai has no provider \`${providerId}\``);
    models.setProvider(provider);
  }
  return { models, providerId, seat };
}

export interface Listing {
  models: readonly Model<any>[];
  /** Why the list may be short: a gateway that refused, or no key to ask with. */
  trouble?: Error;
}

/** Refreshes the live list where there is one (a vendor's catalog is
 * static and returns as it is) and reports what stopped it. */
export async function listModels(seated: Seated, signal?: AbortSignal): Promise<Listing> {
  const { models, providerId, seat } = seated;
  const result = await models.refresh({ providers: [providerId], force: true, signal });
  const listed = models.getModels(providerId);
  let trouble = result.errors.get(providerId);
  // refresh() skips a provider it cannot authenticate, silently; for a
  // gateway that is the whole list gone, so say why.
  if (!trouble && providerId === GATEWAY && listed.length === 0 && !seat.key) {
    trouble = new Error("no key: the gateway is not asked for its list without one");
  }
  return { models: listed, trouble };
}

export interface Pick {
  model: Model<any>;
  /** false = built from a sibling because nobody listed this id. */
  listed: boolean;
}

export function pickModel(seated: Seated, id: string): Pick {
  const { models, providerId, seat } = seated;
  const known = models.getModel(providerId, id);
  let model: Model<any>;
  let listed = true;
  if (known) {
    model = known;
  } else {
    listed = false;
    const sibling = models.getModels(providerId)[0];
    model = sibling ? { ...sibling, id, name: id } : gatewayModel(gatewayBase(seat.baseUrl ?? ""), id);
  }
  if (seat.baseUrl && providerId !== GATEWAY) {
    // A vendor account pointed elsewhere (a proxy, a regional mirror):
    // the person's URL is used as given, because vendors differ on
    // whether `/v1` belongs in it.
    model = { ...model, baseUrl: seat.baseUrl.trim().replace(/\/+$/, "") };
  }
  return { model, listed };
}
