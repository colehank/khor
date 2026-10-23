/** Provider-owned setup; credential files are account-scoped on the execution home. */
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import type { AuthPrompt, AuthEvent } from "@earendil-works/pi-ai";
import { openModels, seatCredentials, type AccountSeat } from "./models.ts";
export interface Management {
  type: "manage"; operation: "models" | "login";
  account: AccountSeat; credentialPath: string; authType?: "api_key" | "oauth";
  deviceId: string;
}
export async function manage(command: Management, prompt: (p: AuthPrompt) => Promise<string>, notify: (e: AuthEvent) => void, signal?: AbortSignal) {
  const seated = openModels(command.account);
  const runtime = await ModelRuntime.create({ ...(command.account.key ? { credentials: seatCredentials(seated.providerId, command.account) } : { authPath: command.credentialPath }), modelsPath: null, modelsStorePath: command.credentialPath + ".models", refreshOnCreate: false });
  runtime.registerNativeProvider(seated.models.getProvider(seated.providerId)!);
  if (command.operation === "login") {
    if (!command.authType) throw new Error("Missing authentication method");
    const credential = await runtime.login(seated.providerId, command.authType, { prompt, notify, signal }, { agentName: "khor", getDeviceId: () => command.deviceId });
    if (!(await runtime.listCredentials()).some((c) => c.providerId === seated.providerId)) throw new Error("Credential was not saved");
    return { credentialType: credential.type };
  }
  const refreshed = await runtime.refresh({ providers: [seated.providerId], force: true, signal: AbortSignal.timeout(12000) });
  const models = runtime.getModels(seated.providerId), trouble = refreshed.errors.get(seated.providerId);
  return { models: models.map((m) => ({ id: m.id, name: m.name, contextWindow: m.contextWindow, reasoning: m.reasoning, image: m.input.includes("image") })), trouble: trouble?.message ?? null };
}
