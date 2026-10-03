// What this machine can do, as its node said on its own row
// (`DeviceRow.hosts`, docs/IOS.md 第二步).
//
// **One fact, read where it is needed, never guessed from the browser.**
// A phone cannot host a session — no tmux, no process it may start, no
// VS Code, no claude, no second window, nothing to hand a path to — and
// the node knows it from the build it runs (`khor_core::hosts_here`).
// The shell reads it off this machine's row and hands it down through
// this context; every act that needs a computer *under the face* (pull a
// file here, open a window, ask an agent, show the web face's key) asks
// it, and every act that needs a computer *at the far end* (a session
// there, its VS Code, its process table) reads that row's own `hosts`.
//
// **`true` until the table has answered**, so a computer's face draws
// exactly what it drew before this existed and never flickers; a phone's
// shows these for the moment before its first device read, which lands
// long before anyone reaches them.
import { createContext, useContext } from "react";

import type { DeviceRow } from "@/api";

const HereHosts = createContext(true);

export const HereHostsProvider = HereHosts.Provider;

/** Whether this machine can host — `false` on a phone. */
export function useHereHosts(): boolean {
  return useContext(HereHosts);
}

/** The answer off a device table: this machine's row says, and a table
    that has not answered yet is read as a computer (the head says why). */
export function hereHosts(devices: DeviceRow[]): boolean {
  return devices.find((d) => d.me)?.hosts ?? true;
}

/** **Whether a page can open in this face** (round UX-3), which is not
    whether this machine hosts: a phone hosts nothing and still browses,
    since the iOS page is a WKWebView of the app's own (`phone_page.rs`).
    It needs `proxyConfigurations` to browse through a machine, which is
    iOS 17 — older iOS refuses every borrow, so nothing offers one there.
    The Mac app and the web face answer yes; the web face says why a page
    needs the app when one is opened (`openPage`). */
export const pagesHere: boolean = (() => {
  const ios = /\b(?:iPhone|iPad|iPod)\b.*? OS (\d+)_/.exec(navigator.userAgent);
  return ios === null || Number(ios[1]) >= 17;
})();
