import type { SessionConfigOption } from "./acp-session";
import type { KhorAgentAccount } from "@/gen/bindings/KhorAgentAccount";
import { khorAgent } from "@/api";
export type PiProvider = { id: string; name: string; chat: boolean; methods: { id: string; label: string }[] };
export type PiModel = { id: string; name: string; contextWindow: number; reasoning: boolean; image: boolean };
export type AuthPrompt = { type: "text" | "secret" | "select" | "manual_code"; message: string; placeholder?: string; options?: { id: string; label: string }[] };
export type AuthEvent = { type: string; message?: string; url?: string; userCode?: string; verificationUri?: string; instructions?: string; links?: { url: string; label?: string }[] };
export type PiJob = { job?: string; status: string; ask?: number; prompt?: AuthPrompt; events?: AuthEvent[]; account?: string; accountInfo?: KhorAgentAccount; error?: string; result?: { models?: PiModel[]; trouble?: string; credentialType?: string; configOptions?: SessionConfigOption[]; model?: string; options?: Record<string,string|boolean> } };
export const managePi = async (machine: string, payload: Record<string, unknown>) => (await khorAgent("manage", machine, payload)).data as PiJob;
export async function piJob(machine: string, request: Record<string, unknown>, signal: AbortSignal, progress?: (job: PiJob, id: string) => void) {
  const started = await managePi(machine, request), id = started.job;
  if (!id) throw new Error(started.error ?? "Missing setup operation");
  const cancel = () => { void managePi(machine, { operation: "cancel", job: id }).catch(() => {}); };
  signal.addEventListener("abort", cancel, { once: true });
  let finished = false;
  try {
    if (signal.aborted) { cancel(); signal.throwIfAborted(); }
    for (;;) {
      signal.throwIfAborted();
      const state = await managePi(machine, { operation: "poll", job: id });
      if (state.status === "done") { finished = true; return state; }
      if (state.status === "failed" || state.status === "cancelled") { finished = true; throw new Error(state.error ?? state.status); }
      progress?.(state, id);
      await new Promise<void>((resolve, reject) => {
        const abort = () => { clearTimeout(timer); reject(signal.reason); };
        const timer = setTimeout(() => { signal.removeEventListener("abort", abort); resolve(); }, 250);
        signal.addEventListener("abort", abort, { once: true });
      });
    }
  } finally { signal.removeEventListener("abort", cancel); if (!finished && !signal.aborted) cancel(); }
}
