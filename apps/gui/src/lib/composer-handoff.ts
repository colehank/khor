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

/** The name of the file waiting, if one is, without taking it: what the
    wizard shows it will attach. */
export function peekHandOff(): string | null {
  return waiting && Date.now() - waiting.at < WAITS_MS ? waiting.name : null;
}

/** The wizard closed without starting a conversation: the file is not
    for whichever conversation opens next. */
export function dropHandOff() {
  waiting = null;
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

/** Asks the app to open a path a terminal printed (roadmap 3.2): a
    directory as that machine's files, a file in the file view at its
    line. `App`'s listener. */
export const OPEN_PATH_EVENT = "khor:open-path";
export function askOpenPath(machine: string, path: string, dir: boolean, line: number | null) {
  window.dispatchEvent(new CustomEvent(OPEN_PATH_EVENT, { detail: { machine, path, dir, line } }));
}

/** Asks the app to open a URL a terminal printed in a browsing tab that
    goes out through that terminal's machine (roadmap 3.2): `localhost`
    there is the dev server running there. `App`'s listener. */
export const OPEN_URL_EVENT = "khor:open-url";
export function askOpenUrl(machine: string, url: string) {
  window.dispatchEvent(new CustomEvent(OPEN_URL_EVENT, { detail: { machine, url } }));
}

/** Asks the app to show the next (`1`) or previous (`-1`) session tab
    (roadmap 4.4, a sideways swipe on a terminal). `App`'s listener. */
export const STEP_TAB_EVENT = "khor:step-tab";
export function askStepTab(by: 1 | -1) {
  window.dispatchEvent(new CustomEvent(STEP_TAB_EVENT, { detail: { by } }));
}
