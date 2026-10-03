// **A file waiting for the next conversation's composer** (roadmap 2.3,
// a command block's "Send to Agent"): the terminal leaves the block —
// command, status, output — here and opens the new-session wizard on that
// machine in that directory; the conversation that opens next finds it
// attached to its first line, the way a file pasted there would be
// (`ChatView`'s `docs`). Taken once, and only within a few minutes: a
// conversation opened much later is not the one the person asked for.

/** How long a handed-off file waits. */
const WAITS_MS = 5 * 60_000;

let waiting: { name: string; text: string; at: number } | null = null;

export function handOff(name: string, text: string) {
  waiting = { name, text, at: Date.now() };
}

export function takeHandOff(): [string, string] | null {
  const w = waiting;
  waiting = null;
  return w && Date.now() - w.at < WAITS_MS ? [w.name, w.text] : null;
}

/** Asks the app to open the new-session wizard (`App`'s listener). */
export const NEW_SESSION_EVENT = "khor:new-session";
export function askNewSession(machine: string, dir: string) {
  window.dispatchEvent(new CustomEvent(NEW_SESSION_EVENT, { detail: { machine, dir } }));
}

/** Asks the app to show a directory's files on a machine (roadmap 2.5,
    a terminal's 「open enclosing folder」; `App`'s listener). */
export const OPEN_DIR_EVENT = "khor:open-dir";
export function askOpenDir(machine: string, path: string) {
  window.dispatchEvent(new CustomEvent(OPEN_DIR_EVENT, { detail: { machine, path } }));
}
