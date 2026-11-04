import { inApp } from "@/api";
import { listenNativeEvent } from "./native-events";
import { updateReadiness } from "./update-readiness";
import type { UpdatePreparation } from "@/gen/bindings/UpdatePreparation";

export function followUpdatePreparation(): () => void {
  if (!inApp) return () => {};
  if ((window as Window & { __KHOR_UPDATE_BOOT__?: boolean }).__KHOR_UPDATE_BOOT__) document.documentElement.inert = true;
  let active: string | null = null;
  let wasInert = false;
  let interacted = false;
  const used = (event: Event) => { if (event.isTrusted) interacted = true; };
  document.addEventListener("pointerdown", used, true);
  document.addEventListener("keydown", used, true);
  window.addEventListener("blur", used);
  const composing = new Set<Node>();
  const started = (event: CompositionEvent) => { if (event.target instanceof Node) composing.add(event.target); };
  const ended = (event: CompositionEvent) => { if (event.target instanceof Node) composing.delete(event.target); };
  const blurred = (event: FocusEvent) => { const target = event.target; queueMicrotask(() => { if (target instanceof Node && target !== document.activeElement) composing.delete(target); }); };
  document.addEventListener("compositionstart", started, true);
  document.addEventListener("compositionend", ended, true);
  document.addEventListener("blur", blurred, true);
  const composition = updateReadiness.register(() => {
    for (const target of composing) if (!target.isConnected) composing.delete(target);
    return composing.size ? "input-in-progress" : null;
  });
  const release = () => {
    if (active === null) return;
    active = null;
    document.documentElement.inert = wasInert;
  };
  const prepare = listenNativeEvent<UpdatePreparation>("app-update:prepare", async ({ nonce, automatic }) => {
    if (active !== null) return;
    active = nonce;
    wasInert = document.documentElement.inert;
    let blocked = updateReadiness.prepare();
    if (automatic && interacted) blocked.push("user-active");
    if (blocked.length === 0) {
      document.documentElement.inert = true;
      blocked = updateReadiness.prepare();
    }
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      await invoke("app_update_prepared", { nonce, blocked });
    } catch { release(); }
  });
  const resume = listenNativeEvent<string>("app-update:resume", nonce => { if (active === nonce) release(); });
  return () => {
    prepare(); resume(); release(); composition(); composing.clear();
    document.removeEventListener("compositionstart", started, true);
    document.removeEventListener("compositionend", ended, true);
    document.removeEventListener("blur", blurred, true);
    document.removeEventListener("pointerdown", used, true);
    document.removeEventListener("keydown", used, true);
    window.removeEventListener("blur", used);
  };
}

export async function confirmUpdateUiReady(): Promise<void> {
  const boot = window as Window & { __KHOR_UPDATE_BOOT__?: boolean };
  if (!inApp) return;
  await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("app_update_ui_ready");
  if (boot.__KHOR_UPDATE_BOOT__) {
    boot.__KHOR_UPDATE_BOOT__ = false;
    document.documentElement.inert = false;
  }
}
