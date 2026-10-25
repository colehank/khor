import type { Event } from "@tauri-apps/api/event";

type CallbackRuntime = {
  unregisterCallback?: (id: number) => void;
};

export function listenNativeEvent<T>(
  event: string,
  receive: (payload: T) => void,
): () => void {
  let active = true;
  let handler: typeof receive | null = receive;
  let handlerId: number | null = null;
  let callbackReleased = false;
  let releaseHandler: (() => void) | null = null;
  let nativeId: number | null = null;
  let unlisten: ((id: number) => Promise<void>) | null = null;
  let removing = false;
  const report = (phase: string, error: unknown) => {
    console.error(`khor: native event ${event} ${phase} failed`,
      { handlerId, nativeId, callbackReleased }, error);
  };
  const releaseCallback = () => {
    const release = releaseHandler;
    releaseHandler = null;
    try {
      release?.();
    } catch (error) {
      report("callback cleanup", error);
    }
  };
  const removeNative = async () => {
    if (removing || nativeId === null || !unlisten) return;
    removing = true;
    try {
      await unlisten(nativeId);
    } catch (error) {
      report("unlisten", error);
    }
  };
  const start = async () => {
    let phase = "load";
    try {
      const { invoke, transformCallback } = await import("@tauri-apps/api/core");
      if (!active) return;
      phase = "compatibility";
      // Tauri 2.11.5 / SDK 2.11.1 expose no public callback disposer. Keep this
      // one internal dependency behind the genuine-runtime contract fixture.
      // SDK event cleanup looks up a registry entry whose eval can arrive late.
      const runtime = (window as Window & { __TAURI_INTERNALS__?: CallbackRuntime })
        .__TAURI_INTERNALS__;
      if (typeof runtime?.unregisterCallback !== "function") {
        throw new Error("Tauri callback cleanup is unavailable");
      }
      const unregister = runtime.unregisterCallback.bind(runtime);
      const callbackId = transformCallback<Event<T>>((message) => {
        if (active) handler?.(message.payload);
      });
      handlerId = callbackId;
      releaseHandler = () => {
        unregister(callbackId);
        callbackReleased = true;
      };
      unlisten = (id) => invoke<void>("plugin:event|unlisten", { event, eventId: id });
      phase = "listen";
      const id = await invoke<number>("plugin:event|listen", {
        event,
        target: { kind: "Any" },
        handler: callbackId,
      });
      if (!Number.isInteger(id) || id < 0 || id > 0xffffffff) {
        throw new Error("Tauri returned an invalid listener id");
      }
      nativeId = id;
      if (!active) await removeNative();
    } catch (error) {
      active = false;
      handler = null;
      releaseCallback();
      report(phase, error);
    }
  };
  void start();
  return () => {
    active = false;
    handler = null;
    releaseCallback();
    void removeNative();
  };
}
