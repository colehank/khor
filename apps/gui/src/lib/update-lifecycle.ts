import { inApp } from "@/api";
import { listenNativeEvent } from "./native-events";
import { updateReadiness } from "./update-readiness";

export function followUpdatePreparation(): () => void {
  if (!inApp) return () => {};
  if ((window as Window & { __KHOR_UPDATE_BOOT__?: boolean }).__KHOR_UPDATE_BOOT__) document.documentElement.inert = true;
  let active: string | null = null;
  let wasInert = false;
  const release = () => {
    if (active === null) return;
    active = null;
    document.documentElement.inert = wasInert;
  };
  const prepare = listenNativeEvent<string>("app-update:prepare", async nonce => {
    if (active !== null) return;
    active = nonce;
    wasInert = document.documentElement.inert;
    document.documentElement.inert = true;
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      await invoke("app_update_prepared", { nonce, blocked: updateReadiness.prepare() });
    } catch { release(); }
  });
  const resume = listenNativeEvent<string>("app-update:resume", nonce => { if (active === nonce) release(); });
  return () => { prepare(); resume(); release(); };
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
