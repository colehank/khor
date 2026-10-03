// **Where the face was, kept across a cold start** (P3, 2026-10-03, the
// owner's ruling): the landing, the 「…」 page, the narrow screen and the
// machine or workspace open on it. A relaunch comes back to that place,
// as an iPhone app restores its tab and its pushed page; what the place
// shows is asked for again, as on any arrival.
//
// One per-device key in this face's `localStorage`, beside the
// navigation's order and the type size (`lib/nav-order`, `lib/type-size`):
// a fact about this screen, never synced. A face that refuses storage
// starts where it always did. What no longer exists (a machine gone, a
// workspace removed) falls back to the landing's list by the shell's own
// rules: the detail it names simply is not found.

const KEY = "khor.place";

export type Place = {
  landing: string;
  more: string | null;
  screen: "list" | "detail";
  device: string | null;
  ws: { device: string; machine: string; path: string } | null;
};

export function storedPlace(): Partial<Place> {
  try {
    const raw = JSON.parse(window.localStorage.getItem(KEY) ?? "null") as unknown;
    if (raw === null || typeof raw !== "object") return {};
    const p = raw as Record<string, unknown>;
    const ws = p.ws as Record<string, unknown> | null | undefined;
    return {
      landing: typeof p.landing === "string" ? p.landing : undefined,
      more: typeof p.more === "string" ? p.more : null,
      screen: p.screen === "detail" ? "detail" : "list",
      device: typeof p.device === "string" ? p.device : null,
      ws:
        ws && typeof ws.device === "string" && typeof ws.machine === "string" && typeof ws.path === "string"
          ? { device: ws.device, machine: ws.machine, path: ws.path }
          : null,
    };
  } catch {
    return {};
  }
}

export function rememberPlace(place: Place): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(place));
  } catch {
    // Without storage the next start is the default one.
  }
}
