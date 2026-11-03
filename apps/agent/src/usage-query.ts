import { createAgentSession, createEventBus, DefaultResourceLoader, readStoredCredential, SessionManager, SettingsManager, type EventBus, type ExtensionContext, type ModelRuntime } from "@earendil-works/pi-coding-agent";
import type { Model } from "@earendil-works/pi-ai";
import { fileURLToPath } from "node:url";
import type { UsageQueryResultV1, UsageScopeV1, UsageUnavailableReasonV1 } from "@narumitw/pi-usage/src/usage-api-types.ts";

export type UsageResult = UsageQueryResultV1 | {
  status: "unavailable";
  providerId: string;
  reason: "no-service" | "protocol-error";
};

const unavailableReasons: Record<UsageUnavailableReasonV1, true> = {
  "not-ready": true, "no-model": true, unsupported: true, "auth-unavailable": true,
  "selection-required": true, "cache-miss": true, "query-failed": true,
  cancelled: true, timeout: true, "stale-context": true,
};

function isUsageResult(value: unknown, provider: string): value is UsageQueryResultV1 {
  if (!value || typeof value !== "object") return false;
  const result = value as Partial<UsageQueryResultV1>;
  if (result.providerId !== undefined && result.providerId !== provider) return false;
  if (result.status === "unavailable") return Object.hasOwn(unavailableReasons, result.reason ?? "");
  return result.status === "ready" && Number.isFinite(result.expiresAt)
    && result.report?.providerId === provider && Number.isFinite(result.report.capturedAt)
    && Array.isArray(result.report.buckets) && Array.isArray(result.report.metrics);
}

/** Pi's event bus offers a promise synchronously; emit itself returns void. */
export function requestUsage(events: EventBus, scope: UsageScopeV1, providerId: string, signal?: AbortSignal, deadlineMs = 15_000): Promise<UsageResult> {
  if (signal?.aborted) return Promise.resolve({ status: "unavailable", providerId, reason: "cancelled" });
  return new Promise(resolve => {
    const controller = new AbortController();
    let accepting = true, offered = false, settled = false;
    const finish = (result: UsageResult) => {
      if (settled) return;
      settled = true;
      if (result.status === "unavailable") controller.abort();
      clearTimeout(timer);
      signal?.removeEventListener("abort", cancel);
      resolve(result);
    };
    const abort = (reason: "timeout" | "cancelled") => {
      controller.abort();
      finish({ status: "unavailable", providerId, reason });
    };
    const cancel = () => abort("cancelled");
    const timer = setTimeout(() => abort("timeout"), deadlineMs);
    signal?.addEventListener("abort", cancel, { once: true });
    try {
      events.emit("usage:query:v1", {
        ...scope, providerId, signal: controller.signal, refresh: true,
        respond(value: unknown) {
          const promise = Promise.resolve(value);
          if (!accepting || offered) {
            void promise.catch(() => {});
            return false;
          }
          offered = true;
          void promise.then(result => {
            try {
              finish(isUsageResult(result, providerId) ? result : { status: "unavailable", providerId, reason: "protocol-error" });
            } catch {
              finish({ status: "unavailable", providerId, reason: "protocol-error" });
            }
          }, () => finish({ status: "unavailable", providerId, reason: "protocol-error" }));
          return true;
        },
      });
    } catch {
      finish({ status: "unavailable", providerId, reason: "protocol-error" });
    } finally {
      accepting = false;
    }
    if (!offered) finish({ status: "unavailable", providerId, reason: "no-service" });
  });
}

export async function readPiUsage(runtime: ModelRuntime, model: Model<any>, agentDir: string, credentialPath: string, signal?: AbortSignal): Promise<UsageResult> {
  if (signal?.aborted) return { status: "unavailable", providerId: model.provider, reason: "cancelled" };
  // The published entrypoint is TypeScript: embed it in Bun and use the same
  // pinned Jiti loader as Pi for Node, with account-scoped factory dependencies.
  const usageExtension = "Bun" in globalThis
    ? (await import("@narumitw/pi-usage/dist/index.ts")).default
    : await (await import("jiti")).createJiti(import.meta.url, { moduleCache: false })
      .import<typeof import("@narumitw/pi-usage/dist/index.ts").default>(fileURLToPath(import.meta.resolve("@narumitw/pi-usage/dist/index.ts")), { default: true });
  const events = createEventBus();
  const settings = SettingsManager.inMemory({ retry: { enabled: false }, compaction: { enabled: false } });
  let context: ExtensionContext | undefined;
  const resources = new DefaultResourceLoader({
    cwd: agentDir, agentDir, settingsManager: settings, eventBus: events,
    noExtensions: true, disabledBuiltinExtensions: ["mcp"], noSkills: true,
    noPromptTemplates: true, noThemes: true, noContextFiles: true,
    extensionFactories: [
      api => usageExtension(api, { credentialReader: (provider: string) => readStoredCredential(provider, credentialPath) }),
      api => { api.on("session_start", (_event, ctx) => { context = ctx; }); },
    ],
  });
  await resources.reload();
  const { session, extensionsResult } = await createAgentSession({
    cwd: agentDir, agentDir, modelRuntime: runtime, model,
    resourceLoader: resources, settingsManager: settings,
    sessionManager: SessionManager.inMemory(agentDir), tools: [], noTools: "all",
  });
  try {
    if (extensionsResult.errors.length) throw new Error("Usage extension initialization failed");
    await session.bindExtensions({ mode: "rpc" });
    if (!context) return { status: "unavailable", providerId: model.provider, reason: "no-service" };
    return await requestUsage(events, { session: context.sessionManager, sessionId: context.sessionManager.getSessionId() }, model.provider, signal);
  } finally {
    try { await session.extensionRunner?.emit({ type: "session_shutdown", reason: "quit" }); }
    finally { session.dispose(); }
  }
}
