import type { Event, EventTarget } from "@tauri-apps/api/event";
import type { DragDropEvent } from "@tauri-apps/api/webview";

type NativeEventCleanup = (() => void) & { ready: Promise<boolean> };
type NativeEventOptions = { target?: EventTarget; onReady?: () => void };

type CallbackRuntime = {
  unregisterCallback?: (id: number) => void;
};

export function listenNativeEvent<T>(
  event: string,
  receive: (payload: T, event: Event<T>, isActive: () => boolean) => void | Promise<void>,
  options: NativeEventOptions = {},
): NativeEventCleanup {
  let active = true;
  let onReady = options.onReady;
  const target = options.target ?? { kind: "Any" };
  let handler: typeof receive | null = receive;
  let handlerId: number | null = null;
  let callbackReleased = false;
  let releaseHandler: (() => void) | null = null;
  let nativeId: number | null = null;
  let unlisten: ((id: number) => Promise<void>) | null = null;
  let removing = false;
  const report = (phase: string, error: unknown) => {
    console.error(`khor: native event ${event} ${phase} failed`,
      { target, handlerId, nativeId, callbackReleased }, error);
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
      if (!active) return false;
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
        if (!active || !handler) return;
        try {
          const pending = handler(message.payload, message, () => active);
          if (pending) void Promise.resolve(pending).catch(error => report("handler", error));
        } catch (error) {
          report("handler", error);
        }
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
        target,
        handler: callbackId,
      });
      if (!Number.isInteger(id) || id < 0 || id > 0xffffffff) {
        throw new Error("Tauri returned an invalid listener id");
      }
      nativeId = id;
      if (!active) await removeNative();
      else {
        phase = "ready";
        const ready = onReady;
        onReady = undefined;
        ready?.();
      }
      return active;
    } catch (error) {
      active = false;
      handler = null;
      onReady = undefined;
      releaseCallback();
      await removeNative();
      report(phase, error);
      return false;
    }
  };
  const ready = start();
  const close = () => {
    active = false;
    handler = null;
    onReady = undefined;
    releaseCallback();
    void removeNative();
  };
  return Object.assign(close, { ready });
}

/** SDK drag payloads, with each of the four listeners owned independently.
 * A later refusal rolls back the whole group, including a pending admission. */
export function listenNativeDragDrop(receive: (event: Event<DragDropEvent>) => void): () => void {
  let active = true;
  let handler: typeof receive | null = receive;
  const listeners: NativeEventCleanup[] = [];
  const close = () => {
    active = false;
    handler = null;
    for (const listener of listeners) listener();
  };
  void (async () => {
    try {
      const [{ getCurrentWebview }, { PhysicalPosition }, { TauriEvent }] = await Promise.all([
        import("@tauri-apps/api/webview"), import("@tauri-apps/api/dpi"), import("@tauri-apps/api/event"),
      ]);
      if (!active) return;
      const target: EventTarget = { kind: "Webview", label: getCurrentWebview().label };
      for (const [event, type] of [
        [TauriEvent.DRAG_ENTER, "enter"], [TauriEvent.DRAG_OVER, "over"],
        [TauriEvent.DRAG_DROP, "drop"], [TauriEvent.DRAG_LEAVE, "leave"],
      ] as const) {
        if (!active) return;
        const listener = listenNativeEvent<{ paths: string[]; position: { x: number; y: number } }>(event, (payload, envelope) => {
          if (!active) return;
          const shaped: DragDropEvent = type === "leave" ? { type } : type === "over"
            ? { type, position: new PhysicalPosition(payload.position) }
            : { type, paths: payload.paths, position: new PhysicalPosition(payload.position) };
          handler?.({ ...envelope, payload: shaped });
        }, { target });
        listeners.push(listener);
        if (!await listener.ready) { close(); return; }
      }
    } catch (error) {
      close();
      console.error("khor: native drag/drop setup failed", error);
    }
  })();
  return close;
}
