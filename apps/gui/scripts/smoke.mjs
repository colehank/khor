// Real-connection acceptance for the GUI. No mocks anywhere: three homes
// on real UDP, the dev bridge is the real backend behind an HTTP skin,
// the browser is the system Chrome. What is asserted:
//   1. joining the mesh from the GUI: beta starts alone, takes alpha's
//      ticket through the devices pane, and alpha arrives wearing a face
//      — the control being that alpha is provably absent beforehand;
//   2. no pane wears a title: the name is readable off the region's
//      aria-label (which proves the probe found the pane) and appears
//      nowhere in what the pane paints;
//   3. the GUI issues a real ticket: a third machine joins with it from
//      the CLI and shows up in the list — and the dialog says how long
//      that ticket lasts in the same sentence `khor invite` prints,
//      with neither the words nor the number written in this script;
//   4. search filters the devices pane for real, both ways, says so when
//      nothing matched, and clearing brings everything back;
//   5. leaving a line reaches the other machine: beta tells alpha from
//      the GUI, alpha reads it back with `khor log`;
//   6. a session living on alpha shows up in beta's GUI, with source,
//      and the word in the GUI equals the word the CLI prints;
//   7. the same search on the sessions pane;
//   8. the word filter filters by the key the backend sent, each word
//      keeping its own rows and nothing else;
//   9. clicking the row is the seen semantics: the unread badge clears
//      AND alpha's own list turns idle — the loop closes cross-device;
//  10. faces: rows paint a real derived SVG (not a placeholder), one
//      machine is the same picture in two places on one screen, two
//      machines are two pictures, and flipping the theme moves nothing
//      inside the SVG;
//  11. the rail's name shows on hover with no delay worth measuring, on
//      keyboard focus too, and goes away when the pointer leaves;
//  12. all four landings open, and the two machine-first panes (files,
//      browser) list the same machines the devices pane does — the same
//      set, not merely a list of the same length;
//  13. each pane's bar carries exactly what that pane can do, and the
//      three machine panes name their search box the same thing (a
//      files pane offering to search files would be naming something
//      it cannot find);
//  14. the app's mark is at the head of the rail, its artwork actually
//      loaded, and it is not a control: no button around it and no
//      hover response — with a rail glyph measured the same way as the
//      control, since "nothing changed" is also what a broken probe
//      says;
//  15. pinning crosses the CLI/GUI seam in both directions: a row
//      pinned in the app leads beta's own `khor sessions` and gets a
//      mark there, a row pinned from alpha's terminal shows as pinned
//      in beta's app (which can only happen through the sync layer),
//      both undo, the pinned and unpinned pin are two *shapes* rather
//      than two colors, every row carries the control, and machines
//      pin the same way — with the order always read back off the
//      node, never checked against one this script worked out;
//  16. arranging: the default is 最近 and prints no headings, each
//      other mode groups by its own thing and nothing else, the state
//      mode's group order equals the one the CLI prints (read off both
//      faces, spelled by neither), a pin leads in all four modes, and
//      the choice outlives a reload — it is this screen's posture, kept
//      on this device, unlike a pin;
//  17. the back button exists only on the narrow face (after proving
//      the detail header renders at all — negative assertions must
//      first prove the probe is alive), and the mark is not in the
//      narrow rail;
//  18. the pin is painted and works on the narrow face with the
//      pointer parked away — a hover-only row action is missing on a
//      phone and looks fine in every mouse-driven screenshot;
//  19. search reaches the machine a row came from, asserted on a row
//      whose own id and title provably do not carry that name, so the
//      source is the only thing it can have matched;
//  20. a ticked filter outlives a reload and is remembered per pane —
//      the preference rule the arrangement already followed, and the
//      one place that used to break it;
//  21. the settings screen: three axes of options, every one of them a
//      face the node derived and this app only painted; the marked
//      option on each axis paints what the machine already wears;
//      picking another palette repaints this machine in every place it
//      appears at once and leaves none showing the old one; beam's own
//      canvas really arrives; a hand-picked color belongs to no factory
//      set so none of them is marked; `khor face` agrees about what was
//      picked; a chosen face still ignores the theme; it outlives a
//      reload;
//  22. a machine restyled from its own terminal reaches this screen —
//      alpha changes itself with `khor face` and beta's list follows,
//      both before states established first, with this machine's own
//      face as the control since nobody else may move it;
//  23. a machine row opens that machine's card: the full id on the card
//      has the row's short id as its prefix (so it is *that* machine),
//      one face on both, the readings drawn, and the age line present
//      for a machine reached over the wire and absent for this one —
//      the offline axis, which a locally faked reading would get wrong.
//      **How many readings there are is read off `khor devices`, never
//      written here**, so a machine khor cannot ask a GPU about is
//      covered by the same assertion; and the GPU row itself is checked
//      to be the same sentence and the same card count on both faces;
//  24. every machine pane's rows now open their second step — the card,
//      the disk, the borrowed network — so the assertion is that all
//      three lead somewhere; and (24b) the browser landing's pin round
//      trip: pinning a typed page makes a shortcut that reaches the
//      PinnedWebs list and unpins back out;
//  25. the hook button is a pair and the button is the report: absent
//      on another machine's card, present on this one, and each press
//      moves both faces together — `khor hooks` reading the same home
//      is the independent witness, with neither word spelled here;
//  26. a pin that does not take says so on the button that was pressed:
//      a real failure through the real path (the backend is taken away,
//      the button is clicked the way a person clicks it), the face
//      provably absent on the successful press just before, the colour
//      measured against a probe wearing the token, and exactly one
//      button wearing it;
//  28. the mandala map fills the devices pane before a machine is
//      picked: a seat per machine, exactly one middle, every seat the
//      same distance from it, **no line between any two of them** (khor
//      knows membership, not reachability — asserted as a surplus-svg
//      count, not as a hunt for lines nobody wrote), and pressing a face
//      opens that machine;
//  29. the app's mark opens the mesh and what it cost, on both faces —
//      **the narrow one is the only way in there at all** — the place
//      replaces the list-and-detail pair rather than filling it, a
//      landing comes back out of it, the faces on it are not pressable
//      (nothing in that space to open into), and every day and vendor
//      the panel shows is one `khor usage` prints for the same home,
//      with neither the day nor the word written in this file;
//  27. zero pageerror throughout.
// Every wait has a deadline; cleanup runs in finally and kills by pid.
//
// No Chinese literal appears below. Words are read off the running app
// (a rail item's aria-label, a row's data-word) and compared against
// what the other face prints — the catalog owns the text, this script
// owns the comparison.
import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  mkdirSync,
  rmSync,
  existsSync,
  writeFileSync,
  readFileSync,
  readdirSync,
  copyFileSync,
  chmodSync,
  symlinkSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "../..");
// **What this run tests is a snapshot, never the live tree.** Both faces
// of the same hazard have been measured on this machine:
//
// - a `cargo build` from another lane replaces `target/debug/khor` mid
//   run, and the serve says so out loud (「磁盘上的 khor 换代」) and
//   restarts itself under the assertions;
// - vite serves `apps/gui/src` **live**, so another lane saving a
//   component hot-reloads the page the browser is halfway through —
//   and that one leaves no message at all. It reads as a random
//   assertion failing on a page that came back half-drawn, which is
//   the harder of the two to recognise because nothing announces it.
//
// So the binaries are copied and the frontend source is copied, and
// everything below runs out of this trip's own directory. `node_modules`
// is symlinked rather than copied: it is large, it is not what anybody
// edits mid-run, and a link to it still resolves from the snapshot.
const BUILT_KHOR = join(repo, "target/debug/khor");
const BUILT_BRIDGE = join(repo, "target/debug/bridge");
const SCRATCH = process.env.SMOKE_DIR ?? join(repo, "target/gui-smoke");
const RUN = join(SCRATCH, "run");
const KHOR = join(RUN, "khor");
const BRIDGE = join(RUN, "bridge");
const GUI_RUN = join(RUN, "gui");
/** What vite needs and nothing else — no `node_modules` (linked), no
    `dist`, no rust crates.

    The last entry is the one import in this app that reaches outside
    `src` (`KhorMark`'s glyph, which lives with the tauri icons). It is
    named rather than solved by copying `src-tauri` wholesale, which is
    a rust crate with build output in it.

    **A new escaping import would break the snapshot, and it says so.**
    This one was found exactly that way: the first assertion came back
    inside a minute with `the rail has nothing … console: 500`, and the
    dev server's log named the file and the line. That is the whole
    reason the rail check is first. */
const GUI_PARTS = [
  "index.html",
  "vite.config.ts",
  "tsconfig.json",
  "package.json",
  "src",
  "src-tauri/icons/src/mandala-glass.svg",
];
const A = join(SCRATCH, "alpha");
const B = join(SCRATCH, "beta");
const G = join(SCRATCH, "gamma");
// Overridable because this machine also carries the developer's own app:
// a bridge already sitting on the default port belongs to somebody else
// and points at somebody else's home. Borrowing vite is fine (same source
// tree); borrowing a *backend* would mean the whole run measured the
// wrong node while reporting green, so the bridge below is checked to be
// ours before anything is asserted.
// 1442/1443, not 1430/1431: those two are the developer's own pair on
// this machine (`vite.config.ts` pins the dev server to 1430, and the
// preview bridge sits on 1431). Defaulting here to the pair next door
// means a smoke run and a session of hand-driving the app can be up at
// the same time without either noticing the other.
const BRIDGE_PORT = Number(process.env.BRIDGE_PORT ?? 1443);
const VITE_PORT = Number(process.env.VITE_PORT ?? 1442);
// The line beta leaves in alpha's window. ASCII on purpose: the message
// is test data, not the thing under test (docs/handoff 中文 rule).
const NEEDLE = "khor-smoke-hello";

const homeEnv = (dir, name) => {
  const e = { ...process.env, KHOR_HOME: dir, KHOR_NAME: name };
  delete e.KHOR_SESSION;
  return e;
};
const envA = homeEnv(A, "alpha");
// beta watches a private tmux server (KHOR_TMUX_SOCKET is the test
// door): the bridge item stands sessions up on it, and the user's real
// server is never looked at, let alone attached to.
const TMUX_SOCK = `khor-smoke-${process.pid}`;
const FAKE_CLAUDE = join(SCRATCH, "claude-fake.py");
const envB = { ...homeEnv(B, "beta"), KHOR_TMUX_SOCKET: TMUX_SOCK, KHOR_CLAUDE: FAKE_CLAUDE };
const envG = homeEnv(G, "gamma");

const children = [];
function run(cmd, args, env, name, cwd) {
  // detached = own process group, so cleanup can kill the whole tree —
  // killing an npx wrapper alone orphans the real server underneath.
  const c = spawn(cmd, args, { env, cwd, stdio: ["ignore", "pipe", "pipe"], detached: true });
  c.stderr.on("data", (d) => process.stderr.write(`[${name}] ${d}`));
  children.push(c);
  return c;
}
// A CLI verb, retried on failure.
//
// **Not a way of ignoring errors**: a verb that is actually broken fails
// every attempt and still throws, carrying its own stderr. What this
// absorbs is the store race the ledger already records — `Node::open`
// flushes the device table, so even a read-only verb writes, and the
// loser of a race with the resident serve prints "改不了名" and exits 1.
// The bridge embeds a serve against this very home, so the window is
// open on every call here. Before this, the race surfaced as whichever
// assertion happened to be running, which reads like that feature
// breaking rather than like the known race it is.
//
// **So this file no longer guards that race.** At the rate measured
// before the fix (17 of 40), five attempts all failing is ~1%, so a
// regression here would come back green. What guards it is the cargo
// test `opening_one_home_from_many_places_at_once_never_fails`, which
// went 90-of-96 red the moment the shared temp name came back. Do not
// read a green smoke run as evidence about that.
function cli(env, ...args) {
  let last;
  for (let i = 0; i < 5; i++) {
    try {
      return execFileSync(KHOR, args, { env, encoding: "utf8", timeout: 30_000 });
    } catch (e) {
      last = e;
      execFileSync("/bin/sleep", ["0.4"]);
    }
  }
  throw new Error(`khor ${args.join(" ")} failed 5x: ${last?.stderr || last}`);
}
// `sid` is a parameter because two homes need rows that are their own:
// a session id is the key the whole network agrees on, so feeding the
// same one into two machines would put one id on two rows with two
// different homes, and every assertion about "which machine is this
// row's" would then be measuring an accident.
function feedHook(env, event, extra = "", sid = "cafe1") {
  const payload = `{"session_id":"${sid}","cwd":"/tmp/proj","hook_event_name":"${event}"${extra}}`;
  execFileSync(KHOR, ["state", "--hook"], { env, input: payload, timeout: 15_000 });
}
async function until(what, ms, f, step = 400) {
  const deadline = Date.now() + ms;
  let last;
  for (;;) {
    try {
      last = await f();
      if (last) return last;
    } catch (e) {
      last = e;
    }
    if (Date.now() >= deadline) break;
    await new Promise((r) => setTimeout(r, step));
  }
  throw new Error(`timed out waiting for: ${what} (last: ${last})`);
}

// Say one line into the conversation pane.
//
// The value is checked in the box before Enter is pressed, because the
// two calls are two round trips and the pane remounts on its own clock
// (the row's kind flips the moment a takeover lands, and that swaps the
// whole detail). A `fill` that landed in an element about to be
// replaced leaves an **empty** box for `press` to Enter on, and an
// empty box says nothing at all — which reads downstream as "the agent
// never answered". Seen twice in five runs before this existed.
// What a stuck hang/stop looks like from every side at once, gathered
// only when the wait already failed.
//
// **A timeout on "the turn ended" is true of every layer and points at
// none.** The chain is 面 → bridge → gui_host → `khor _cagent` → the
// fake → interrupt → `result:interrupted` → `Turn`, and b6 has it at
// 250 rounds with nothing lost below the browser — so the reading has
// to say *which* link, or the next occurrence teaches as little as the
// last one did.
//
// Four readings, and each one rules out a different half:
//
// - `hanging` absent — the line never reached the agent. Whatever was
//   stopped was not a running turn: test debris, not the product.
// - `hanging` present, `interrupt` absent — the stop did not leave
//   khor. A real person's stop button would be just as mute.
// - both present, no `turn` frame in the backend's own list — claude
//   was interrupted and the answer was lost on the way back.
// - both present, a `turn` frame sitting in that list — it came back
//   and **the face did not use it**, which is the only one of the four
//   that lives in this app's own code.
//
// The last two are the split that needs the backend read: `chat_poll`
// from `since: 0` is a pure read of an append-only list and takes
// nothing from the app's own cursor, so asking costs nothing and
// disturbs nothing.
async function frameKinds(page, id) {
  return page.evaluate(
    async ([port, sid]) => {
      const r = await fetch(`http://127.0.0.1:${port}/chat_poll`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: sid, since: 0 }),
      });
      if (!r.ok) throw new Error(`chat_poll refused: ${await r.text()}`);
      return JSON.parse(await r.text()).frames.map((f) => f.kind);
    },
    [BRIDGE_PORT, id],
  );
}

async function hangReading(page, dir, title, before) {
  const mark = (f) => {
    const at = join(dir, f);
    return existsSync(at) ? JSON.stringify(readFileSync(at, "utf8")) : "absent";
  };
  const id = await page
    .locator(`[data-title="${title}"]`)
    .getAttribute("data-row")
    .catch(() => null);
  let since = "no row to ask about";
  let all = since;
  if (id) {
    const kinds = await frameKinds(page, id).catch((e) => String(e));
    if (Array.isArray(kinds)) {
      all = kinds.join(",") || "(none)";
      since = kinds.slice(before).join(",") || "(none)";
    } else {
      all = kinds;
      since = kinds;
    }
  }
  const stopButton = await page.locator("[data-chat-stop]").count();
  const thinking = await page.locator("[data-chat-thinking]").count();
  return [
    "hang/stop reading:",
    `  fake.hanging   = ${mark("fake.hanging")}`,
    `  fake.interrupt = ${mark("fake.interrupt")}`,
    `  since the stop = ${since}`,
    `  whole stream   = ${all}`,
    `  on screen      = stop x${stopButton}, thinking x${thinking}`,
  ].join("\n");
}

async function say(page, text) {
  const box = page.locator("[data-chat-input]");
  for (let i = 0; i < 5; i += 1) {
    await box.fill(text);
    if ((await box.inputValue()) === text) {
      // A running turn refuses the send while leaving the box open to
      // type into (批①). The box no longer being disabled is what makes
      // this wait necessary: without it a line pressed into a busy pane
      // goes nowhere quietly, and whatever assertion comes next times
      // out describing the wrong failure.
      //
      // It waits on the *turn* being over — the stop control is only
      // there while one runs — and deliberately not on the send button's
      // own `disabled`. That button is itself under test below (the send
      // must be visibly unavailable mid-turn); a helper that read the
      // very thing being tested would go vacuous exactly when that broke
      // — and a vacuous wait does not fail here, it lets a line be
      // pressed into a busy pane and fails somewhere later, describing
      // the wrong thing. The first spelling of this helper did read that
      // button, and the red-proof for the send rule landed on a
      // different assertion because of it.
      await page.waitForFunction(() => !document.querySelector("[data-chat-stop]"), null, {
        timeout: 60_000,
      });
      await box.press("Enter");
      return;
    }
  }
  throw new Error(`the line would not stay in the box: ${text}`);
}

/**
 * Kills anything still living in the smoke's own homes.
 *
 * The suite kills what it spawned, by pid — but a khor host is
 * **detached on purpose** (that is the product), so a run that dies at
 * an assertion leaves a `_ghost` or a `_host` behind, holding a session
 * in a home the next run is about to delete and recreate. It then
 * writes its files back into the fresh home and the next run inherits a
 * row nobody asked for: measured once as a suite that hung six minutes
 * in with no error to show for it.
 *
 * By environment, not by name: a `_ghost` names no home on its command
 * line, and matching on the binary would kill the developer's own app.
 */
function killStrays() {
  let table = "";
  try {
    table = execFileSync("ps", ["eww", "-o", "pid=,command="], { encoding: "utf8" });
  } catch {
    return; // no ps, no sweep — not a reason to refuse to run
  }
  for (const line of table.split("\n")) {
    if (!line.includes(`KHOR_HOME=${SCRATCH}`)) continue;
    const pid = Number(line.trim().split(/\s+/)[0]);
    if (pid > 0) {
      try {
        process.kill(pid, "SIGTERM");
      } catch {
        /* already gone */
      }
    }
  }
}

// Winding the run down, on every way out there is.
//
// **Three passes, and each one exists because the one before it cannot
// see something.** `children` are this script's own, killed by process
// group. `khor quit` reaches what khor started and this script never
// held: a session host is detached by design — a session outliving the
// app is the product working — so nothing in the loop above is its
// parent. And the sweep at the end catches what was **born during the
// two steps before it**: a `bridge _ghost` turned up after every run,
// green ones included, with the birth time of the moment the run ended
// — the app handing its sessions off on the way out, racing the sweep
// that had already run. It does not listen on any port, so a check by
// port never saw it; it dies on a plain TERM, so the signal was never
// the problem. Hence the wait and the second look.
let cleanedUp = false;
async function cleanup() {
  if (cleanedUp) return;
  cleanedUp = true;
  // Said out loud, because the run that most needs this to have
  // happened is the one that was interrupted — and "did the cleanup
  // even run" is otherwise unanswerable from the log.
  process.stderr.write("smoke: winding down\n");
  // **Children first, browser last, and the order is the fix.** The
  // browser used to be closed first, and a `close()` that never
  // resolved held everything behind it: measured on an interrupted run,
  // which exited leaving the bridge and vite alive on their ports.
  // Ranked by what survives this process — the detached ones hold
  // ports and memory on a machine other people are using, while the
  // browser is playwright's own child and the least of the problem.
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGTERM");
    } catch {}
  }
  await new Promise((r) => setTimeout(r, 500));
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGKILL");
    } catch {}
  }
  // Ask each home to wind itself down the way a person would, rather
  // than hunting its processes: `khor quit` stops that home's serve and
  // every session host it recorded a pid for, by pid — which is the
  // house rule for stopping anything (账本: 后台进程怎么起就怎么收).
  for (const [dir, env] of [
    [A, envA],
    [B, envB],
    [G, envG],
  ]) {
    if (!existsSync(dir)) continue;
    try {
      execFileSync(KHOR, ["quit"], { env, timeout: 15_000, stdio: "ignore" });
    } catch {}
  }
  try {
    // Quiet: this run may never have started a tmux server, and its
    // complaint about a missing socket on the way out reads like
    // something went wrong.
    execFileSync("tmux", ["-L", TMUX_SOCK, "kill-server"], { timeout: 5_000, stdio: "ignore" });
  } catch {}
  killStrays();
  await new Promise((r) => setTimeout(r, 700));
  killStrays();
  // On a leash: everything that matters is already down, so a browser
  // that will not close must not keep this process alive.
  if (browser) {
    await Promise.race([
      browser.close().catch(() => {}),
      new Promise((r) => setTimeout(r, 3_000)),
    ]);
  }
}

// **A run that is interrupted is the run most likely to leave things
// behind**, and `finally` does not cover it: node's default handler for
// these signals ends the process without unwinding. Ctrl-C on a smoke
// that is twenty assertions deep used to leave a serve, a bridge and a
// browser running on a machine other people are working on.
for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
  process.on(sig, () => {
    void cleanup().finally(() => process.exit(130));
  });
}

let browser;
try {
  killStrays();
  rmSync(SCRATCH, { recursive: true, force: true });
  mkdirSync(SCRATCH, { recursive: true });

  // Take the snapshot before anything is started, so every process
  // below is looking at this trip's own copy (see the constants).
  mkdirSync(GUI_RUN, { recursive: true });
  for (const [built, here] of [
    [BUILT_KHOR, KHOR],
    [BUILT_BRIDGE, BRIDGE],
  ]) {
    if (!existsSync(built)) {
      throw new Error(`${built} is not built — cargo build -p khor-cli -p khor-gui-core --bins`);
    }
    copyFileSync(built, here);
    chmodSync(here, 0o755);
  }
  for (const part of GUI_PARTS) {
    const to = join(GUI_RUN, part);
    mkdirSync(dirname(to), { recursive: true });
    execFileSync("/bin/cp", ["-R", join(gui, part), to]);
  }
  symlinkSync(join(gui, "node_modules"), join(GUI_RUN, "node_modules"));
  // The fake claude the shim drives in 25c: canned stream-json, the
  // same frames the real probes recorded (cli/tests/cagent.rs's twin).
  writeFileSync(
    FAKE_CLAUDE,
    `#!/usr/bin/env python3
import json, os, sys
args = sys.argv[1:]
if "--session-id" in args:
    sid = args[args.index("--session-id") + 1]
else:
    sid = args[args.index("--resume") + 1]
def emit(o):
    sys.stdout.write(json.dumps(o) + "\\n"); sys.stdout.flush()
# The transcript, the way the real one exists: khor finds a session's
# full uuid and its recorded cwd by reading this file, and both 接管
# directions hang on that. Written on the way up so it is there before
# anybody asks.
home = os.environ.get("KHOR_HOME", "")
if home:
    proj = os.path.join(home, ".claude", "projects", "p")
    os.makedirs(proj, exist_ok=True)
    with open(os.path.join(proj, sid + ".jsonl"), "a") as t:
        t.write(json.dumps({"type": "user", "cwd": os.getcwd(),
                            "message": {"role": "user", "content": "the fake's own past"}}) + "\\n")
first = True
for line in sys.stdin:
    m = json.loads(line)
    if m.get("type") != "user":
        continue
    text = m["message"]["content"][0]["text"]
    if first:
        first = False
        emit({"type": "system", "subtype": "init", "session_id": sid, "slash_commands": ["compact"]})
    if "hang" in text:
        # The turn that does not end on its own: it ends when the shim
        # relays the client's stop as the control protocol's interrupt.
        #
        # The two marks are for the hang/stop flake (lane-acp-b6's
        # three-way split, read by \`hangReading\` below). They say what
        # this process actually saw, which is the one thing no assertion
        # on the screen can tell: whether the line arrived at all, and
        # whether the stop came back down as an interrupt. The prompt
        # text is written into them rather than a fixed word, so a mark
        # left by an earlier round is recognisable as one.
        with open("fake.hanging", "w") as f:
            f.write(text)
        while True:
            line = sys.stdin.readline()
            if not line:
                break
            c = json.loads(line)
            if c.get("type") == "control_request" and c.get("request", {}).get("subtype") == "interrupt":
                with open("fake.interrupt", "w") as f:
                    f.write(text)
                break
        emit({"type": "result", "subtype": "interrupted", "session_id": sid})
        continue
    if "ask-permission" in text:
        emit({"type": "control_request", "request_id": "req-1",
              "request": {"subtype": "can_use_tool", "tool_name": "Write", "display_name": "Write",
                          "description": "x.txt", "tool_use_id": "t1",
                          "input": {"file_path": "x.txt", "content": "hi"}}})
        resp = json.loads(sys.stdin.readline())
        emit({"type": "assistant", "message": {"content": [
            {"type": "text", "text": "verdict:" + resp["response"]["response"]["behavior"]}]}})
    else:
        emit({"type": "assistant", "message": {"content": [{"type": "text", "text": "echo: " + text}]}})
    emit({"type": "result", "subtype": "success", "session_id": sid})
`,
  );
  chmodSync(FAKE_CLAUDE, 0o755);
  for (const d of [A, B, G]) mkdirSync(d, { recursive: true });

  // Something for beta's agents to have cost. **Written under beta's own
  // home**, which is where its meter reads (`khor_node::usage`) — so the
  // spending panel and `khor usage` are looking at one tree, and the
  // comparison between them is about the two faces rather than about two
  // sets of files. ASCII throughout: this is test data, not the thing
  // under test.
  const spent = join(B, ".claude/projects/p");
  mkdirSync(spent, { recursive: true });
  const billed = (id, day, out, cached) =>
    JSON.stringify({
      type: "assistant",
      timestamp: `2026-08-${day}T06:00:00Z`,
      message: {
        id,
        usage: {
          input_tokens: out * 3,
          cache_read_input_tokens: cached,
          cache_creation_input_tokens: Math.round(cached / 8),
          output_tokens: out,
        },
      },
    });
  //     Two days rather than one: the panel groups by day, and a single
  //     day cannot tell a grouping apart from a list.
  writeFileSync(
    join(spent, "s.jsonl"),
    [billed("m1", "16", 890, 277965), billed("m2", "17", 4210, 1338402)].join("\n") + "\n",
  );

  // alpha: serve + a hooked agent session (register → busy), and a
  // ticket from its CLI — the one thing the GUI is not being asked to
  // do in this scenario, so that what it *is* asked to do (joining) is
  // the only untested step in the pairing.
  run(KHOR, ["serve"], envA, "serve-a");
  await until("alpha endpoint.json", 15_000, () => existsSync(join(A, ".khor/endpoint.json")));
  const ticketA = cli(envA, "invite").trim().split("\n").pop().trim();
  feedHook(envA, "SessionStart");
  feedHook(envA, "UserPromptSubmit");
  if (!cli(envA, "sessions").includes("tui/cafe1")) throw new Error("alpha row missing");

  // beta: the bridge is the app backend — embedded serve pumps sync and
  // holds this key's endpoint, which is what lets the GUI mint and
  // accept tickets without a terminal.
  const bridge = run(BRIDGE, [], { ...envB, BRIDGE_PORT: String(BRIDGE_PORT) }, "bridge");
  let bridgeGone = null;
  bridge.on("exit", (code) => (bridgeGone = `bridge exited with ${code}`));
  // It says so on stdout once it has the port. If it died instead — the
  // usual reason being a bridge already bound there — say that here,
  // where it is one line, instead of discovering it as a wrong answer
  // twenty assertions later.
  let bridgeReady = false;
  bridge.stdout.on("data", (d) => (bridgeReady ||= String(d).includes(`:${BRIDGE_PORT}`)));
  await until(`our own bridge on ${BRIDGE_PORT}`, 15_000, () => {
    if (bridgeGone) throw new Error(`${bridgeGone} — is the port already taken?`);
    return bridgeReady;
  });
  await until("beta endpoint.json", 15_000, () => existsSync(join(B, ".khor/endpoint.json")));
  const vite = run(
    "npx",
    ["vite", "--port", String(VITE_PORT), "--strictPort"],
    { ...process.env, PATH: process.env.PATH },
    "vite",
    GUI_RUN,
  );
  vite.stdout.on("data", () => {});
  await until("vite up", 30_000, async () => {
    const r = await fetch(`http://localhost:${VITE_PORT}/`).catch(() => null);
    return r?.ok;
  });

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  const page = await browser.newPage({ viewport: { width: 1080, height: 720 } });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(String(e)));
  // **`pageerror` is not everything the page can tell you.** It fires
  // for uncaught exceptions and nothing else — a module that failed to
  // load, a rejected promise nobody caught, a React warning: all of
  // those reach the console and none of them reach that handler. Kept
  // separate from `pageErrors` on purpose: this one is a hint for a
  // failure message, not a gate, because the console carries things a
  // healthy run is allowed to say.
  const consoleErrors = [];
  page.on("console", (m) => {
    if (m.type() === "error") consoleErrors.push(m.text().slice(0, 300));
  });
  await page.goto(`http://localhost:${VITE_PORT}/?bridge=${BRIDGE_PORT}`);

  const listPane = page.locator("[data-list]");
  const railItem = (tab) => page.locator(`[data-rail-item][data-landing="${tab}"]`);
  const openLanding = async (tab) => {
    await railItem(tab).click();
    await until(`the ${tab} pane`, 10_000, async () =>
      (await listPane.getAttribute("aria-label")) === (await railItem(tab).getAttribute("aria-label")),
    );
  };

  // 0) **The app came up whole.** First assertion in the file, before
  //    anything is clicked, because everything after it reads one piece
  //    of a screen that is assumed to exist: a face that mounted half of
  //    itself fails later, somewhere unrelated, as "this button is
  //    missing" — and the reader goes looking for the button.
  //
  //    The four landings are the cheapest whole-app fact there is (they
  //    are `LANDINGS` in `App.tsx`, all present from the first day, none
  //    of them conditional). If they are all there, React mounted and
  //    the catalog resolved.
  //
  //    **The page's own exceptions ride the failure message.** They are
  //    otherwise only read at the very end of the run, which for a crash
  //    at mount is four thousand lines too late — and a run that fails
  //    an assertion first never gets there at all. An empty rail plus a
  //    thrown error is a crash; an empty rail alone is a slow or absent
  //    backend, and those two send a reader to opposite ends of the
  //    tree.
  await until("all four landings on the rail", 30_000, async () => {
    const got = await page
      .locator("[data-rail-item][data-landing]")
      .evaluateAll((els) => els.map((e) => e.dataset.landing));
    const missing = ["sessions", "devices", "files", "browser"].filter((k) => !got.includes(k));
    if (missing.length) {
      throw new Error(
        `the rail has ${got.length ? got.join(", ") : "nothing"}, missing ${missing.join(", ")}` +
          (pageErrors.length ? ` — the page threw: ${pageErrors.join(" | ")}` : " — no page error") +
          (consoleErrors.length ? ` — console: ${consoleErrors.join(" | ")}` : ""),
      );
    }
    return true;
  });

  // 1) joining from the GUI. The control first: alpha is not in beta's
  //    table, so whatever makes it appear below can only be the join.
  await openLanding("devices");
  await until("beta's own row", 10_000, async () => (await page.locator("[data-device]").count()) === 1);
  if ((await page.locator('[data-device="alpha"]').count()) !== 0) {
    throw new Error("alpha is in the table before pairing — the control is void");
  }
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="join"]').click();
  await page.locator("[data-ticket-input]").fill(ticketA);
  await page.locator("[data-join]").click();
  await until("alpha's row, wearing its own face", 30_000, async () => {
    const err = await page.locator("[data-dialog-error]").count();
    if (err) throw new Error(await page.locator("[data-dialog-error]").innerText());
    return (await page.locator('[data-device="alpha"] [data-face] svg').count()) === 1;
  });

  // 2) no pane wears a title. The probe is proven alive first: the name
  //    has to be readable, and it has to be the same fact the rail
  //    states — then the claim is that it is nowhere in the paint.
  //    (A placeholder is an attribute, not text, so the search box
  //    naming its pane does not put a title back on the screen.)
  const assertUntitled = async (tab) => {
    await openLanding(tab);
    const name = await listPane.getAttribute("aria-label");
    if (!name) throw new Error(`probe dead: the ${tab} pane has no accessible name`);
    if ((await page.locator("[data-pane-bar]").count()) !== 1) {
      throw new Error(`probe dead: no pane bar on ${tab}`);
    }
    if ((await listPane.innerText()).includes(name)) {
      throw new Error(`the ${tab} pane paints its own name: ${name}`);
    }
  };
  await assertUntitled("devices");

  // 3) the GUI mints a real ticket — proven by a third machine joining
  //    with it from the CLI, not by the string looking ticket-shaped.
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="invite"]').click();
  const guiTicket = await until("a ticket from the GUI", 20_000, async () => {
    const err = await page.locator("[data-dialog-error]").count();
    if (err) throw new Error(await page.locator("[data-dialog-error]").innerText());
    const t = await page.locator("[data-ticket]").inputValue();
    return t.length > 0 && t;
  });
  //    …and the dialog says how long that ticket lasts, in the very
  //    sentence the terminal prints. **Neither the words nor the number
  //    are written here**: another home is asked to mint one and to say
  //    its own line, and the two are required to match — so the day the
  //    window moves, a screen still promising the old one goes red
  //    instead of being a discrepancy nobody is looking at.
  //
  //    `khor invite` puts the ticket on stdout and the window on
  //    stderr, which is why this reads stderr. The ticket that mints is
  //    left to expire unused; minting one is the only way to make it
  //    say the sentence.
  const shownWindow = (await page.locator("[data-ticket-window]").innerText()).trim();
  const saidWindow = spawnSync(KHOR, ["invite"], { env: envA, encoding: "utf8" }).stderr.trim();
  if (!shownWindow) throw new Error("the ticket dialog says nothing about how long it lasts");
  if (!saidWindow) throw new Error("probe dead: `khor invite` said nothing about the window");
  if (shownWindow !== saidWindow) {
    throw new Error(`the dialog says ${shownWindow}, the terminal says ${saidWindow}`);
  }

  await page.keyboard.press("Escape");
  cli(envG, "pair", guiTicket);
  await until("gamma in beta's device list", 30_000, async () =>
    (await page.locator('[data-device="gamma"]').count()) === 1,
  );

  // 4) search filters for real, and both ways: a box that always kept
  //    the first row would pass one direction and fail the other.
  const devices = page.locator("[data-device]");
  const search = page.locator("[data-pane-search]");
  const before = await devices.count();
  if (before !== 3) throw new Error(`expected alpha, beta and gamma; saw ${before}`);
  await search.fill("gamma");
  await until("only gamma", 5_000, async () => (await devices.count()) === 1);
  if ((await page.locator('[data-device="gamma"]').count()) !== 1) {
    throw new Error("search kept the wrong row");
  }
  await search.fill("alpha");
  await until("only alpha", 5_000, async () => (await devices.count()) === 1);
  if ((await page.locator('[data-device="alpha"]').count()) !== 1) {
    throw new Error("search kept the wrong row");
  }
  await search.fill("no-machine-is-called-this");
  await until("an empty result that says so", 5_000, async () => (await devices.count()) === 0);
  if ((await page.locator("[data-empty]").count()) !== 1) {
    throw new Error("filtered to nothing without saying nothing matched");
  }
  await search.fill("");
  await until("every machine back", 5_000, async () => (await devices.count()) === before);

  // 5) leaving a line: from beta's GUI into alpha's window, read back by
  //    alpha's own CLI. Two machines, two faces, one message.
  await assertUntitled("sessions");
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="tell"]').click();
  await page.locator('[data-machine="alpha"]').click();
  await page.locator("[data-tell-text]").fill(NEEDLE);
  await page.locator("[data-tell-send]").click();
  await until("the tell dialog to close on success", 20_000, async () => {
    const err = await page.locator("[data-dialog-error]").count();
    if (err) throw new Error(await page.locator("[data-dialog-error]").innerText());
    return (await page.locator("[data-tell-dialog]").count()) === 0;
  });
  // Polled at a walking pace: each turn is a whole CLI process opening
  // the node, and the sync pump is on a five-second beat anyway.
  await until(
    "alpha to read the line in its own window",
    40_000,
    () => cli(envA, "log", "alpha").includes(NEEDLE),
    1_000,
  );

  // 5b) the line beta just told rides the chat row as its preview —
  //      `Session::last` end to end: the CRDT's newest message, through
  //      the node's row, painted on the second line (会话行改版批). On
  //      the sessions pane, within a couple of list polls.
  await openLanding("sessions");
  await until("the chat row previewing the told line", 20_000, async () => {
    const t = await page
      .locator(`[data-row="chat/alpha"] [data-last]`)
      .innerText()
      .catch(() => "");
    return t.includes(NEEDLE);
  });

  // 6) the reported row reaches beta's GUI, busy, with its source; and
  //    the CLI line for the same row carries the same display word.
  const row = page.locator("[data-row]", { hasText: "proj" });
  await until("the alpha row in beta's GUI", 30_000, async () => (await row.count()) === 1);
  if ((await row.getAttribute("data-word")) !== "busy") throw new Error("row should be busy");
  const guiWord = (await row.locator("[data-word-text]").innerText()).trim();
  if (!(await row.innerText()).includes("alpha")) throw new Error("reported row must show its source");
  const cliLine = cli(envB, "sessions").split("\n").find((l) => l.includes("tui/cafe1"));
  if (!cliLine || !cliLine.includes(guiWord)) {
    throw new Error(`CLI and GUI disagree on the word: ${guiWord} vs ${cliLine}`);
  }

  // 7) searching the session pane, same shape as the devices one: a term
  //    only one row answers to, then the control that clearing brings the
  //    rest back.
  const sessionRows = page.locator("[data-row]");
  const sessionSearch = page.locator("[data-pane-search]");
  const allRows = await sessionRows.count();
  if (allRows < 2) throw new Error(`need more than one row to tell a filter from a no-op; saw ${allRows}`);
  await sessionSearch.fill("proj");
  await until("only the row titled proj", 5_000, async () => (await sessionRows.count()) === 1);
  if ((await sessionRows.getAttribute("data-word")) !== "busy") {
    throw new Error("session search kept the wrong row");
  }
  await sessionSearch.fill("");
  await until("every session back", 5_000, async () => (await sessionRows.count()) === allRows);

  // 8) the word filter. The keys come off the rows themselves — the ones
  //    the node sent — so this never spells a state out. Reading all of
  //    them, not a sample: the rows are ordered local-first, and a sample
  //    off the top says more about that ordering than about the words.
  const wordsOnScreen = [
    ...new Set(await sessionRows.evaluateAll((els) => els.map((e) => e.dataset.word))),
  ];
  if (wordsOnScreen.length < 2) {
    throw new Error(`every row wears ${wordsOnScreen[0]} — the filter cannot be told from a no-op`);
  }
  // Opening is conditional and closing is waited for, so that a menu
  // which failed to open — or failed to close, leaving the next click to
  // toggle it shut — reports itself instead of surfacing as a missing
  // option several lines later.
  const menu = page.locator("[data-slot=dropdown-menu-content]");

  // Before using the menu: its opening motion is real and is on our
  // tokens. Measured off the element, because a missing @import leaves
  // the classes sitting in the markup with nothing behind them — the
  // markup would read exactly the same either way, and no other
  // assertion here would notice.
  const ms = (v) => (v.trim().endsWith("ms") ? parseFloat(v) : parseFloat(v) * 1000);
  await page.locator("[data-pane-filter]").click();
  await until("the filter menu", 5_000, async () => (await menu.count()) === 1);
  const tokens = await page.evaluate(() => {
    const s = getComputedStyle(document.documentElement);
    return { dur: s.getPropertyValue("--dur-120"), ease: s.getPropertyValue("--ease-out").trim() };
  });
  const motion = await menu.evaluate((el) => {
    const s = getComputedStyle(el);
    return { name: s.animationName, dur: s.animationDuration, ease: s.animationTimingFunction };
  });
  if (motion.name === "none") throw new Error("the menu opens with no animation at all");
  if (ms(motion.dur) !== ms(tokens.dur)) {
    throw new Error(`the menu's duration is not our token: ${motion.dur} vs ${tokens.dur}`);
  }
  if (motion.ease !== tokens.ease) {
    throw new Error(`the menu's easing is not our token: ${motion.ease} vs ${tokens.ease}`);
  }
  await page.keyboard.press("Escape");
  await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);

  // A filter option is keyed by the node's own **group key** (批③): the
  // menu ticks on three axes now, and a state word and a machine could
  // otherwise collide on a bare value. `state:` is the prefix
  // `khor_node::list` groups states under, so the row's own word plus
  // that prefix names the option without this file spelling either.
  const tickWord = async (w) => {
    const option = `[data-filter-option="state:${w}"]`;
    if ((await menu.count()) === 0) await page.locator("[data-pane-filter]").click();
    await until(`the ${w} option in the filter menu`, 5_000, async () =>
      (await page.locator(option).count()) === 1,
    );
    await page.locator(option).click();
    await page.keyboard.press("Escape");
    await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);
  };
  for (const w of wordsOnScreen) {
    const mine = await page.locator(`[data-word="${w}"]`).count();
    await tickWord(w);
    await until(`only the ${w} rows`, 5_000, async () => (await sessionRows.count()) === mine);
    if ((await page.locator(`[data-word="${w}"]`).count()) !== mine) {
      throw new Error(`filtering on ${w} dropped ${w} rows`);
    }
    await tickWord(w);
    await until("every row back", 5_000, async () => (await sessionRows.count()) === allRows);
  }

  // 8b) 三轴筛选 (批③ 一笔): the same menu now ticks machines and
  //     categories, on the node's own group keys.
  //
  //     **The machine axis is the one worth proving.** It reads each
  //     row's `home`, not its `source` — `source` is the offline axis
  //     and is absent on rows that live here, so an axis built on it
  //     would silently exclude this machine, which is the one people
  //     filter by first. So: tick *this* machine, and the rows that
  //     survive must be exactly the rows that are not from elsewhere.
  const axisOptions = async (axis) => {
    if ((await menu.count()) === 0) await page.locator("[data-pane-filter]").click();
    await until(`the ${axis} options`, 5_000, async () =>
      (await page.locator(`[data-filter-option^="${axis}"]`).count()) > 0,
    );
    return page
      .locator(`[data-filter-option^="${axis}"]`)
      .evaluateAll((els) => els.map((e) => e.dataset.filterOption));
  };
  const machineKeys = await axisOptions("dev:");
  await page.keyboard.press("Escape");
  await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);
  if (machineKeys.length < 2) {
    throw new Error(
      `only ${machineKeys.length} machine on the axis — a machine filter cannot be told ` +
        `from a no-op: ${JSON.stringify(machineKeys)}`,
    );
  }
  const tickKey = async (key) => {
    if ((await menu.count()) === 0) await page.locator("[data-pane-filter]").click();
    await until(`the ${key} option`, 5_000, async () =>
      (await page.locator(`[data-filter-option="${key}"]`).count()) === 1,
    );
    await page.locator(`[data-filter-option="${key}"]`).click();
    await page.keyboard.press("Escape");
    await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);
  };
  // **Asserted against the fact each row carries, not against a count.**
  // The first spelling of this compared "rows kept" with "rows with no
  // `data-source`", which is a guess about which rows are this
  // machine's — and it failed while the axis was working, because a
  // live list's counts move between two measurements. `data-home` is
  // the fact the filter reads, so ask each surviving row for it.
  let narrowed = 0;
  for (const key of machineKeys) {
    const machine = key.slice("dev:".length);
    await tickKey(key);
    const homes = await page
      .locator("[data-row]")
      .evaluateAll((els) => els.map((e) => e.dataset.home));
    // **At least one row, or the check above is vacuously true.** Every
    // key on this axis was minted from a row that carries it, so a key
    // that keeps nothing means the filter is reading a different fact
    // than the one the candidates came from — which is exactly what
    // building the axis on `source` would look like: the machines that
    // reported still match, and this machine's own key quietly matches
    // no row at all.
    if (homes.length === 0) {
      throw new Error(`ticking ${key} kept no rows, but that key came from a row`);
    }
    if (homes.some((h) => h !== machine)) {
      throw new Error(
        `ticking ${key} left rows from elsewhere: ${JSON.stringify([...new Set(homes)])}`,
      );
    }
    if (homes.length < allRows) narrowed += 1;
    await tickKey(key);
    await until("every row back", 10_000, async () => (await sessionRows.count()) === allRows);
  }
  // …and at least one of them actually removed something. All three
  // keeping every row would satisfy the loop above and mean the filter
  // does nothing — the shape of a no-op that passes.
  if (narrowed === 0) {
    throw new Error("no machine on the axis narrowed the list; the filter is a no-op");
  }

  // 8c) omnibox 芯片 (批③ 二笔): the box and the menu are two faces of
  //     one state, and the chip is a token.
  const omni = page.locator("[data-omnibox]");
  const omniInput = page.locator("[data-omni-input]");
  if ((await omni.count()) !== 1) throw new Error("probe dead: the sessions pane has no omnibox");
  if ((await page.locator("[data-chip]").count()) !== 0) {
    throw new Error("probe dead: a chip is already on before anything was ticked");
  }
  //     **One source of truth, both directions.** Tick in the menu → the
  //     chip appears; take the chip off → the tick is gone. Two lists
  //     that each remembered their own idea of "what is filtered" would
  //     pass one of these and fail the other, which is the whole reason
  //     both are asserted.
  const chipWord = wordsOnScreen[0];
  await tickWord(chipWord);
  await until("the ticked word arriving as a chip", 10_000, async () =>
    (await page.locator(`[data-chip="state:${chipWord}"]`).count()) === 1,
  );
  await page.locator(`[data-chip-remove="state:${chipWord}"]`).click();
  await until("the chip leaving", 10_000, async () =>
    (await page.locator(`[data-chip="state:${chipWord}"]`).count()) === 0,
  );
  await page.locator("[data-pane-filter]").click();
  await until("the menu", 5_000, async () => (await menu.count()) === 1);
  const stillTicked = await page
    .locator(`[data-filter-option="state:${chipWord}"]`)
    .getAttribute("aria-checked");
  await page.keyboard.press("Escape");
  await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);
  if (stillTicked === "true") {
    throw new Error("taking the chip off left the menu still ticked — two states, not one");
  }

  //     Typing narrows the candidates, Enter makes a chip, and the text
  //     that found it goes with it.
  await omniInput.click();
  await until("the candidate menu", 5_000, async () =>
    (await page.locator("[data-omni-menu]").count()) === 1,
  );
  // How many there are with nothing typed — the control for "typing
  // narrowed something".
  const flatCandidates = await page.locator("[data-omni-item]").count();
  const machineName = machineKeys[0].slice("dev:".length);
  await omniInput.fill(machineName);
  await until(`the ${machineName} candidate`, 10_000, async () =>
    (await page.locator(`[data-omni-item="${machineKeys[0]}"]`).count()) === 1,
  );
  //     …and it narrowed: a menu that offered everything regardless of
  //     what was typed would satisfy the line above.
  const offeredNow = await page.locator("[data-omni-item]").count();
  if (offeredNow >= flatCandidates) {
    throw new Error(
      `typing narrowed nothing: ${offeredNow} candidates offered, ${flatCandidates} in total`,
    );
  }
  //     An Enter raised mid-composition belongs to the IME. Dispatched
  //     the same way the chat box's was, and the same dispatch is then
  //     shown to work when it is not composing — a negative assertion
  //     whose probe was never shown alive says nothing.
  await omniInput.evaluate((el) => {
    el.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, isComposing: true }));
  });
  await new Promise((r) => setTimeout(r, 400));
  if ((await page.locator(`[data-chip="${machineKeys[0]}"]`).count()) !== 0) {
    throw new Error("an Enter raised mid-composition must not commit a chip");
  }
  await omniInput.press("Enter");
  await until("the candidate becoming a chip", 10_000, async () =>
    (await page.locator(`[data-chip="${machineKeys[0]}"]`).count()) === 1,
  );
  if ((await omniInput.inputValue()) !== "") {
    throw new Error("the text that found the chip must go with it");
  }

  //     Backspace on an empty box takes the last chip **whole**. Two
  //     chips on, because "one chip left" is also what removing half of
  //     something would look like on a screen.
  await tickWord(chipWord);
  await until("two chips", 10_000, async () => (await page.locator("[data-chip]").count()) === 2);
  await omniInput.press("Backspace");
  await until("one chip left", 10_000, async () => (await page.locator("[data-chip]").count()) === 1);
  const leftover = await page.locator("[data-chip]").getAttribute("data-chip");
  if (leftover !== machineKeys[0]) {
    throw new Error(`backspace took the wrong chip: ${leftover} left, expected ${machineKeys[0]}`);
  }
  await omniInput.press("Backspace");
  await until("no chips", 10_000, async () => (await page.locator("[data-chip]").count()) === 0);
  await until("every row back", 10_000, async () => (await sessionRows.count()) === allRows);

  //     **The devices pane has no chips**, and that is the design's own
  //     reverse check rather than a special case: it declares no axes,
  //     so the box has nothing to offer and stays the plain one.
  await openLanding("devices");
  await until("the devices pane", 10_000, async () => (await page.locator("[data-device]").count()) > 0);
  if ((await page.locator("[data-omnibox]").count()) !== 0) {
    throw new Error("the devices pane grew chips it has no axis for");
  }
  if ((await page.locator("[data-pane-search]").count()) !== 1) {
    throw new Error("probe dead: the devices pane lost its search box entirely");
  }
  await openLanding("sessions");
  await until("rows back on the sessions pane", 10_000, async () => (await sessionRows.count()) === allRows);

  // 9) turn ends on alpha → done + unread on beta; clicking the row is
  //    seen; the loop closes: badge clears here, alpha turns idle there.
  feedHook(envA, "Stop");
  await until("done + unread badge", 30_000, async () =>
    (await row.getAttribute("data-word")) === "done" && (await row.locator("[data-unread]").count()) === 1,
  );
  // The row is a strip holding two controls now — what opens it, and
  // the pin. Clicking the strip's centre would land on whichever of the
  // two happens to sit there; name the one that means "open".
  await row.locator("[data-row-open]").click();
  await until("beta's row to settle idle, badge gone", 30_000, async () =>
    (await row.getAttribute("data-word")) === "idle" && (await row.locator("[data-unread]").count()) === 0,
  );
  // The loop's far end: alpha's own CLI now prints the same idle word
  // the GUI shows — the watermark travelled, and the wording matches.
  const idleGuiWord = (await row.locator("[data-word-text]").innerText()).trim();
  await until("alpha to read the seen watermark", 30_000, () => {
    const line = cli(envA, "sessions").split("\n").find((l) => l.includes("tui/cafe1"));
    return Boolean(line && line.includes(idleGuiWord));
  });

  // 10) faces. Everything below is still on the wide viewport.
  //
  //    a. every session row paints a real derived face. Stated
  //    positively on purpose: the blank branch renders no <svg> at all,
  //    so "there is an svg in every row" *is* "no row fell back to a
  //    placeholder", with no negative selector to spell wrong.
  //
  //    **`[data-row]`, not `[data-word]`.** This file used the second as
  //    a synonym for "a session row" for as long as rows were the only
  //    thing that showed a state word; the detail header shows one now
  //    (批②, and it wears the same attribute on purpose — that is what
  //    makes the colour and the breath the doctrine's rather than a
  //    second copy). The synonym then counted the header as a row with
  //    no face and this read "4 of 5", which is a true sentence about a
  //    selector and a false one about the app.
  const rowCount = await page.locator("[data-row]").count();
  const facedRows = await page.locator("[data-row] [data-face] svg").count();
  if (rowCount === 0 || facedRows !== rowCount) {
    throw new Error(`rows with a painted face: ${facedRows} of ${rowCount}`);
  }
  //    …and what it painted is the derivation, not an empty canvas: the
  //    canvas side is one of the two the core ships, and the ground rect
  //    carries a hex color from the palette.
  const rowFace = page.locator("[data-row] [data-face] svg").first();
  const viewBox = await rowFace.getAttribute("viewBox");
  if (viewBox !== "0 0 80 80" && viewBox !== "0 0 36 36") {
    throw new Error(`a face's canvas is neither 80 nor 36: ${viewBox}`);
  }
  if (!/<rect[^>]*fill="#[0-9a-f]{6}"/.test(await rowFace.innerHTML())) {
    throw new Error("a face has no ground rect in a palette color");
  }

  //    b. one machine, two places on one screen, one picture. The blur
  //    filter's id is per-instance (a document-uniqueness device, not
  //    part of the face), so it is normalized away before comparing.
  const faceOf = async (locator) =>
    (await locator.innerHTML()).replace(/av-blur-[^"')\s]+/g, "BLUR");
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) >= 2);
  const railFace = page.locator("nav > [data-face]");
  const betaRow = page.locator('[data-device="beta"] [data-face]');
  if ((await railFace.count()) !== 1) throw new Error("probe dead: no face at the foot of the rail");
  if ((await betaRow.count()) !== 1) throw new Error("probe dead: no face on beta's own row");
  if ((await faceOf(railFace)) !== (await faceOf(betaRow))) {
    throw new Error("this machine has two different faces on one screen");
  }

  //    c. the control: two machines are not one picture. Without this,
  //    a painter that drew the same thing for everyone would pass (b).
  const alphaRow = page.locator('[data-device="alpha"] [data-face]');
  if ((await alphaRow.count()) !== 1) throw new Error("probe dead: no face on alpha's row");
  if ((await faceOf(alphaRow)) === (await faceOf(betaRow))) {
    throw new Error("two machines were painted the same face");
  }

  //    d. a theme flip moves nothing inside the SVG.
  //
  //    Two measurements, because they fail differently. The markup
  //    catches a painter that emitted different numbers or colors. The
  //    *computed* paint catches the subtler one: markup that is
  //    byte-identical because it says `var(--something)`, which then
  //    resolves per theme — a face that reads the theme without any
  //    string ever changing.
  //
  //    The probe has to be proven alive first, or "nothing changed" is
  //    also exactly what a theme switch that never took effect looks
  //    like. So two things that *should* follow the theme are measured
  //    in the same breath: the page's ground, and the face's hairline
  //    edge — the one documented theme-aware part, which lives outside
  //    the SVG.
  const themeProbe = async () =>
    page.evaluate(() => {
      const face = document.querySelector('[data-device="beta"] [data-face]');
      const edge = face && face.querySelector("span");
      const svg = face && face.querySelector("svg");
      return {
        body: getComputedStyle(document.body).backgroundColor,
        edge: edge && getComputedStyle(edge).borderTopColor,
        // Every painted element's resolved paint, in document order
        paint:
          svg &&
          [...svg.querySelectorAll("*")]
            .map((el) => {
              const s = getComputedStyle(el);
              return [el.tagName, s.fill, s.stroke, s.mixBlendMode, s.filter].join("|");
            })
            .join("\n"),
      };
    });
  await page.emulateMedia({ colorScheme: "light" });
  const lightFace = await faceOf(betaRow);
  const light = await themeProbe();
  await page.emulateMedia({ colorScheme: "dark" });
  const darkFace = await faceOf(betaRow);
  const dark = await themeProbe();
  if (light.body === dark.body) throw new Error("probe dead: the theme flip changed no ground color");
  if (!light.edge || light.edge === dark.edge) {
    throw new Error(`probe dead: the avatar edge did not follow the theme (${light.edge} / ${dark.edge})`);
  }
  if (!light.paint) throw new Error("probe dead: read no computed paint off the face");
  if (lightFace !== darkFace) throw new Error("the face's markup changed when the theme did");
  if (light.paint !== dark.paint) throw new Error("the face's computed paint changed when the theme did");
  await page.emulateMedia({ colorScheme: null });

  // 11) the rail's names. The deadline is the assertion: Radix's stock
  //     delay is 700ms, so a window well under that is what separates
  //     "shows at once" from "shows eventually" — drop the delayDuration
  //     override and this goes red rather than slow.
  const tip = page.locator("[data-rail-tip]");
  // Walked, not teleported: a tooltip whose content is hoverable keeps
  // itself open across a grace area, and it takes real pointer moves
  // through that area to leave it.
  const pointerAway = () => page.mouse.move(900, 600, { steps: 12 });
  await pointerAway();
  await until("no tip while the pointer is away", 5_000, async () => (await tip.count()) === 0);
  const sessionsName = await railItem("sessions").getAttribute("aria-label");
  await railItem("sessions").hover();
  await until("the rail name on hover, at once", 400, async () => (await tip.count()) === 1, 50);
  if ((await tip.innerText()).trim() !== sessionsName) {
    throw new Error(`the rail tip names the wrong thing: ${await tip.innerText()}`);
  }
  await pointerAway();
  await until("the tip to leave with the pointer", 5_000, async () => {
    const n = await tip.count();
    // Say which of the two it is: still open, or stuck mid-exit.
    if (n > 0) throw new Error(`still up, data-state=${await tip.getAttribute("data-state")}`);
    return true;
  });
  // Keyboard reaches it too — the tip is the only thing that says where
  // an icon-only glyph goes, so it cannot be a mouse-only affordance.
  //
  // Focus has to genuinely move for there to be a focus event at all:
  // this glyph is already focused from the click that opened its pane,
  // and re-focusing an already-focused element fires nothing, so the
  // assertion would be measuring an event that never happened. Park
  // focus elsewhere first and prove it left.
  const devicesName = await railItem("devices").getAttribute("aria-label");
  const focusedName = () =>
    page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? "");
  await page.locator("[data-pane-search]").focus();
  if ((await focusedName()) === devicesName) {
    throw new Error("probe dead: focus never left the rail glyph, so nothing was tested");
  }
  await railItem("devices").focus();
  await until(
    "the rail name on focus",
    2_000,
    async () => {
      if ((await tip.count()) === 1) return true;
      const who = await page.evaluate(() => {
        const el = document.activeElement;
        return el ? `${el.tagName}[${el.getAttribute("aria-label")}]` : "nothing";
      });
      throw new Error(`no tip; focus sits on ${who}`);
    },
    50,
  );
  if ((await tip.innerText()).trim() !== devicesName) {
    throw new Error(`focus showed the wrong name: ${await tip.innerText()}`);
  }
  await railItem("devices").blur();

  // 12) four landings, and what the two borrowed ones show.
  //
  //     Machine *names* are compared as sets, not counts: three panes
  //     each showing three rows proves nothing if one of them is
  //     showing three of something else.
  const LANDING_TABS = ["sessions", "devices", "files", "browser"];
  const machinesOn = async (tab) => {
    await openLanding(tab);
    await until(`machines on the ${tab} pane`, 10_000, async () =>
      (await page.locator("[data-device]").count()) > 0,
    );
    return (
      await page.locator("[data-device]").evaluateAll((els) => els.map((e) => e.dataset.device))
    ).sort();
  };
  for (const tab of LANDING_TABS) {
    if ((await railItem(tab).count()) !== 1) throw new Error(`no rail glyph for ${tab}`);
  }
  const onDevices = await machinesOn("devices");
  if (onDevices.length !== 3) throw new Error(`expected three machines, saw ${onDevices.length}`);
  for (const tab of ["files", "browser"]) {
    const here = await machinesOn(tab);
    if (here.join(",") !== onDevices.join(",")) {
      throw new Error(`the ${tab} pane lists ${here} where devices lists ${onDevices}`);
    }
  }

  // 13) each pane's bar holds what that pane can do — and nothing it
  //     cannot. The positive half runs first and is what proves the
  //     selectors still name real things: without it, every "is absent"
  //     below would also pass on a renamed attribute.
  const barOf = async (tab) => {
    await openLanding(tab);
    return {
      search: await page.locator("[data-pane-search]").count(),
      filter: await page.locator("[data-pane-filter]").count(),
      plus: await page.locator("[data-pane-new]").count(),
      label: await page.locator("[data-pane-search]").getAttribute("aria-label"),
    };
  };
  const bars = {};
  for (const tab of LANDING_TABS) bars[tab] = await barOf(tab);
  const want = {
    sessions: { search: 1, filter: 1, plus: 1 },
    devices: { search: 1, filter: 0, plus: 1 },
    files: { search: 1, filter: 0, plus: 0 },
    browser: { search: 1, filter: 0, plus: 0 },
  };
  for (const [tab, w] of Object.entries(want)) {
    for (const [what, n] of Object.entries(w)) {
      if (bars[tab][what] !== n) {
        throw new Error(`the ${tab} bar has ${bars[tab][what]} ${what}, expected ${n}`);
      }
    }
    if (!bars[tab].label) throw new Error(`probe dead: the ${tab} search box has no name`);
  }
  // The three machine panes search machines, so they say the same
  // thing; the sessions pane searches something else and says something
  // else. Read off the app, never spelled here — the catalog owns the
  // words, this owns the comparison.
  for (const tab of ["files", "browser"]) {
    if (bars[tab].label !== bars.devices.label) {
      throw new Error(`the ${tab} search box is named ${bars[tab].label}, not ${bars.devices.label}`);
    }
  }
  if (bars.sessions.label === bars.devices.label) {
    throw new Error("the sessions pane borrowed the machine pane's search label");
  }

  // 14) the mark at the head of the rail — **and it answers now.**
  //
  //     This item used to assert the opposite, in as much detail: no
  //     button around it, no response to the pointer. That was right
  //     while it opened nothing, and it is now false — so it is turned
  //     over rather than deleted, because the new fact is the one worth
  //     guarding. What it opens is asserted below (the map and the
  //     spending); what is asserted here is that it looks like something
  //     that opens anything at all.
  const mark = page.locator("[data-rail-mark]");
  if ((await mark.count()) !== 1) throw new Error("no mark at the head of the rail");
  // Its artwork really loaded. A broken import renders an <img> all the
  // same, and every other assertion here would still pass.
  const drew = await mark.evaluate((el) => el.complete && el.naturalWidth > 0);
  if (!drew) throw new Error("the mark's artwork did not load");
  const markButton = page.locator("[data-rail-item]", { has: mark });
  if ((await markButton.count()) !== 1) {
    throw new Error("the mark is not a rail item, so it is not reachable by keyboard either");
  }
  const bg = (loc) => loc.evaluate((el) => getComputedStyle(el).backgroundColor);
  const settle = () => new Promise((r) => setTimeout(r, 300));
  await pointerAway();
  await settle();
  const markResting = await bg(markButton);
  await mark.hover();
  await settle();
  if ((await bg(markButton)) === markResting) {
    throw new Error("the mark does not answer the pointer, so nothing says it can be pressed");
  }
  //     …measured the same way as a glyph that is known to answer, so a
  //     reading of "it changed" is not the measurement drifting.
  const glyph = railItem("files");
  await pointerAway();
  await settle();
  const glyphResting = await bg(glyph);
  await glyph.hover();
  await settle();
  if ((await bg(glyph)) === glyphResting) {
    throw new Error("probe dead: a rail glyph shows no hover either, so nothing here means anything");
  }
  await pointerAway();

  // 15) pinning, both directions across the CLI/GUI seam.
  //
  //     The order is never checked here — it is read back off the node.
  //     What is asserted is that the row the node put first *is* first,
  //     which is a different claim from "the frontend sorted correctly"
  //     and the only one this layer is allowed to make.
  await openLanding("sessions");
  await until("rows on the session list", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  const sessionIds = () =>
    page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const pinButtons = page.locator("[data-row] [data-row-pin]");

  //     a. the control: nothing is pinned, and every row carries the pin
  //     — "every row" is the reachability claim, and counting it against
  //     the rows themselves is what makes it one.
  if ((await page.locator("[data-row][data-pinned=true]").count()) !== 0) {
    throw new Error("something is pinned before the test pinned anything — the control is void");
  }
  if ((await pinButtons.count()) !== (await page.locator("[data-row]").count())) {
    throw new Error(
      `${await pinButtons.count()} pins for ${await page.locator("[data-row]").count()} rows — not every row can be pinned`,
    );
  }

  //     b. GUI → CLI. Pin the second row (not the first: a row already on
  //     top proves nothing about floating), then read beta's own CLI.
  const idsBefore = await sessionIds();
  const target = idsBefore[1];
  const cliLineOf = (env, id) =>
    cli(env, "sessions").split("\n").find((l) => l.startsWith(`${id}\t`)) ?? "";
  const plainLine = cliLineOf(envB, target);
  if (!plainLine) throw new Error(`probe dead: ${target} is not in beta's CLI list`);

  await page.locator(`[data-row="${target}"] [data-row-pin]`).click();
  await until("the pinned row to float to the top", 10_000, async () => {
    const now = await sessionIds();
    if (now[0] === target) return true;
    // Say what the screen actually looks like: "it did not float" and
    // "the click never reached the node" fail the same way from here.
    const pinnedAttr = await page.locator(`[data-row="${target}"]`).getAttribute("data-pinned");
    const backend = cli(envB, "sessions").split("\n").filter(Boolean)[0] ?? "";
    throw new Error(
      `target=${target} data-pinned=${pinnedAttr} order=[${now.join(" ")}] cli-first=${backend.split("\t")[0]}`,
    );
  });
  if ((await page.locator(`[data-row="${target}"]`).getAttribute("data-pinned")) !== "true") {
    throw new Error("the row floated but does not say it is pinned");
  }
  //     …and beta's CLI agrees, both in order and in what it prints. The
  //     mark itself is never spelled here: it is whatever the pinned line
  //     grew relative to the same line before, which is also the proof
  //     that the CLI says *something* rather than only reordering.
  await until("beta's CLI to lead with the pinned row", 10_000, () => {
    // Data rows only: a grouped listing starts with a heading, and a
    // heading is the one line with no tabs in it.
    const rowsOut = cli(envB, "sessions").split("\n").filter((l) => l.includes("\t"));
    return rowsOut[0]?.startsWith(`${target}\t`);
  });
  const markedLine = cliLineOf(envB, target);
  if (markedLine === plainLine || !markedLine.startsWith(plainLine)) {
    throw new Error(`the CLI does not mark the pinned row: ${plainLine} -> ${markedLine}`);
  }

  //     c. the two states are two shapes, not one shape in two colors.
  //     Measured off the element: a painter that only swapped a color
  //     would pass every other assertion here.
  //     Both `transform` and the standalone `rotate` property are read:
  //     tailwind v4 compiles `-rotate-45` to the latter, and reading only
  //     the former returns "none" for a rotation that is plainly there.
  const pinTransform = (id) =>
    page.locator(`[data-row="${id}"] [data-row-pin] svg`).evaluate((el) => {
      const s = getComputedStyle(el);
      return `${s.transform}|${s.rotate}|${s.transformOrigin}`;
    });
  const pinnedShape = await pinTransform(target);
  const restingShape = await pinTransform(idsBefore[0] === target ? idsBefore[1] : idsBefore[0]);
  if (pinnedShape === restingShape) {
    throw new Error(`pinned and unpinned draw the same shape: ${pinnedShape}`);
  }
  if (/\|none\|/.test(pinnedShape)) {
    throw new Error(`the pinned state is not a rotation at all: ${pinnedShape}`);
  }
  //     …and the rotation happens around the middle. An SVG element's
  //     transform-origin defaults to (0, 0), which would swing the pin
  //     out of its own box — visibly wrong, and nothing else here looks.
  const [, , origin] = pinnedShape.split("|");
  if (/^0px 0px/.test(origin)) {
    throw new Error(`the pin rotates around its corner, not its centre: ${origin}`);
  }

  //     d. CLI → GUI, on the far machine. alpha pins one of its own rows
  //     from its terminal; beta's app has to show it, which can only
  //     happen through the sync layer.
  if ((await page.locator('[data-row="tui/cafe1"]').getAttribute("data-pinned")) === "true") {
    throw new Error("tui/cafe1 is already pinned in the GUI — the control is void");
  }
  cli(envA, "pin", "tui/cafe1");
  await until("alpha's pin to reach beta's app", 40_000, async () =>
    (await page.locator('[data-row="tui/cafe1"]').getAttribute("data-pinned")) === "true",
    1_000,
  );

  //     e. taking it back travels the same way, and the row goes home.
  await page.locator(`[data-row="${target}"] [data-row-pin]`).click();
  await until("the unpinned row to leave the top", 10_000, async () => {
    const now = await sessionIds();
    return now[0] !== target;
  });
  // Waited for rather than read once: the app's order and a fresh CLI
  // process are two observers of one write, and they do not have to
  // land in the same millisecond. If it never returns to what it was,
  // this still fails — with both strings in the message.
  await until("beta's CLI line to go back to what it was", 10_000, () => {
    const now = cliLineOf(envB, target);
    if (now === plainLine) return true;
    throw new Error(`before ${JSON.stringify(plainLine)} / now ${JSON.stringify(now)}`);
  });
  cli(envA, "unpin", "tui/cafe1");
  await until("alpha's unpin to reach beta's app", 40_000, async () =>
    (await page.locator('[data-row="tui/cafe1"]').getAttribute("data-pinned")) === "false",
    1_000,
  );

  //     f. machines pin too, and the same way: the table sorts, the app
  //     paints. gamma is last by name, so its arrival at the top is not
  //     something the previous order could have produced.
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) === 3);
  const deviceNames = () =>
    page.locator("[data-device]").evaluateAll((els) => els.map((e) => e.dataset.device));
  if ((await deviceNames())[0] === "gamma") throw new Error("gamma already leads — the control is void");
  await page.locator('[data-device="gamma"] [data-row-pin]').click();
  await until("gamma to lead beta's machine list", 10_000, async () => (await deviceNames())[0] === "gamma");
  await until("beta's CLI to lead with gamma", 10_000, () =>
    cli(envB, "devices").split("\n")[0]?.startsWith("gamma"),
  );
  await page.locator('[data-device="gamma"] [data-row-pin]').click();
  await until("gamma to go back", 10_000, async () => (await deviceNames())[0] !== "gamma");

  // 16) arranging the list: four modes, and the order is always the
  //     node's. The state mode is checked against the CLI's own output
  //     rather than against an order this script worked out — neither
  //     side spells a state word here, they are read off both faces and
  //     compared.
  await openLanding("sessions");
  await until("rows on the session list", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  if ((await page.locator("[data-row][data-pinned=true]").count()) !== 0) {
    throw new Error("a row is still pinned from the section above — the default check is void");
  }
  // Put the agent back to work first. The seen loop above settled every
  // row to one word, and one word is one group — "grouped by state"
  // would pass on a single heading while proving nothing about grouping
  // at all.
  feedHook(envA, "UserPromptSubmit");
  await until(
    "alpha's row to be busy again in beta's app",
    40_000,
    async () => (await page.locator('[data-row="tui/cafe1"]').getAttribute("data-word")) === "busy",
    1_000,
  );
  const wordsHere = [
    ...new Set(await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.word))),
  ];
  if (wordsHere.length < 2) {
    throw new Error(`every row wears ${wordsHere[0]} — grouping by state cannot be told from a no-op`);
  }
  const groupKeys = () =>
    page.locator("[data-group]").evaluateAll((els) => els.map((e) => e.dataset.group));
  const groupTexts = () =>
    page.locator("[data-group]").evaluateAll((els) => els.map((e) => e.innerText.trim()));
  const openMenu = async () => {
    if ((await menu.count()) === 0) await page.locator("[data-pane-filter]").click();
    await until("the filter menu", 5_000, async () => (await menu.count()) === 1);
  };
  const closeMenu = async () => {
    await page.keyboard.press("Escape");
    await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);
  };
  const chosenArrange = () =>
    page.locator("[data-arrange-option][data-state=checked]").getAttribute("data-arrange-option");
  const chooseArrange = async (key) => {
    await openMenu();
    await page.locator(`[data-arrange-option="${key}"]`).click();
    await closeMenu();
  };

  //     a. the default, with this run having chosen nothing: recent,
  //     and no headings at all — the one mode that does not group.
  await openMenu();
  const byDefault = await chosenArrange();
  await closeMenu();
  if (byDefault !== "recent") throw new Error(`the default arrangement is ${byDefault}`);
  if ((await groupKeys()).length !== 0) {
    throw new Error(`recent should print no headings, saw ${await groupKeys()}`);
  }

  //     b. every other mode groups by its own thing and by nothing
  //     else. The prefixes are the node's (khor_node::list), which is
  //     what lets a heading be a word or a machine name without the
  //     frontend guessing which.
  for (const [mode, prefix] of [
    ["category", "cat:"],
    ["device", "dev:"],
    ["state", "state:"],
  ]) {
    // **Wait for the headings to *change*, not merely to exist.** The
    // list keeps the previous mode's headings until the rows come back
    // under the new one, so "there are two or more headings" is true the
    // instant the click lands — and what gets read then belongs to the
    // mode before. The three prefixes are disjoint, so a change is a
    // safe thing to wait for and the assertion below still has teeth.
    const stale = (await groupKeys()).join();
    await chooseArrange(mode);
    await until(`${mode} headings`, 15_000, async () => {
      const now = await groupKeys();
      return now.length >= 2 && now.join() !== stale;
    });
    const keys = await groupKeys();
    if (!keys.every((k) => k.startsWith(prefix))) {
      throw new Error(`${mode} produced headings that are not its own: ${keys}`);
    }
  }

  //     c. the state mode's group order is the node's ranking — proven
  //     by reading the same order out of the CLI's own listing. The CLI
  //     order is taken from its *rows* (second column), so this does not
  //     depend on how a heading is punctuated there.
  await chooseArrange("state");
  await until("the state grouping to settle", 15_000, async () => (await groupKeys()).length >= 2);
  const guiStateOrder = await groupTexts();
  const cliStateOrder = [
    ...new Set(
      cli(envB, "sessions", "--by", "state")
        .split("\n")
        .filter((l) => l.includes("\t"))
        .map((l) => l.split("\t")[1]),
    ),
  ];
  if (guiStateOrder.join("|") !== cliStateOrder.join("|")) {
    throw new Error(
      `the two faces disagree on the state order: GUI [${guiStateOrder}] vs CLI [${cliStateOrder}]`,
    );
  }

  //     d. **a pin is not a mode**: it leads in all four. The row picked
  //     is not the one already on top, so nothing but the pin could put
  //     it there.
  const arrangeIds = () =>
    page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const pinTarget = (await arrangeIds())[2];
  await page.locator(`[data-row="${pinTarget}"] [data-row-pin]`).click();
  for (const mode of ["recent", "category", "device", "state"]) {
    await chooseArrange(mode);
    await until(`${pinTarget} leading in ${mode}`, 15_000, async () => {
      const now = await arrangeIds();
      return now[0] === pinTarget;
    });
  }
  await page.locator(`[data-row="${pinTarget}"] [data-row-pin]`).click();
  await until("the pin to come off again", 10_000, async () =>
    (await page.locator("[data-row][data-pinned=true]").count()) === 0,
  );

  //     e. the choice outlives a reload. It is this screen's posture and
  //     is kept on this device — unlike a pin, which travels the network
  //     because it belongs to the session rather than to the screen.
  await chooseArrange("device");
  await page.reload();
  await until("rows after the reload", 20_000, async () => (await page.locator("[data-row]").count()) > 1);
  await openMenu();
  const afterReload = await chosenArrange();
  await closeMenu();
  if (afterReload !== "device") {
    throw new Error(`the arrangement did not survive a reload: ${afterReload}`);
  }
  // Leave it as it was found, so the sections below see the default.
  await chooseArrange("recent");

  // 17) faces of the shell: wide has a detail header but no back;
  //     narrow, after entering a detail, has the back button.
  await openLanding("sessions");
  await until("rows on the session list", 10_000, async () => (await page.locator("[data-row]").count()) > 0);
  await row.locator("[data-row-open]").click();
  if ((await page.locator("[data-detail-header]").count()) !== 1) throw new Error("probe dead: no detail header");
  if ((await page.locator("[data-back]").count()) !== 0) throw new Error("back button on the wide face");
  // Shrinking mid-detail keeps the detail up (Telegram's behavior) —
  // and there, with the list genuinely off-screen, back exists.
  await page.setViewportSize({ width: 390, height: 720 });
  await until("narrow detail with back", 10_000, async () => (await page.locator("[data-back]").count()) === 1);
  await page.locator("[data-back]").click();
  await until("back to the narrow list", 10_000, async () => (await page.locator("[data-list]").count()) === 1);
  await until("rows on the narrow list", 10_000, async () => (await page.locator("[data-row]").count()) > 0);
  // All four landings are reachable down here too — the narrow rail is
  // the only way to any of them — **and the mark is among them now.**
  //
  // This assertion also used to say the opposite: the mark was kept out
  // of the narrow rail because that row is places to go and it went
  // nowhere. It goes somewhere now, so keeping it out would be the one
  // place on a phone from which the mesh cannot be reached at all.
  for (const tab of LANDING_TABS) {
    if ((await railItem(tab).count()) !== 1) throw new Error(`no ${tab} glyph on the narrow rail`);
  }
  if ((await page.locator("[data-rail-mark]").count()) !== 1) {
    throw new Error("the mark is missing from the narrow rail, so the mesh is unreachable here");
  }
  await openLanding("browser");
  await until("machines on the narrow browser pane", 10_000, async () =>
    (await page.locator("[data-device]").count()) === onDevices.length,
  );

  // 18) the pin is reachable on the narrow face — the one where there is
  //     no pointer to reveal it with. A hover-only row action is simply
  //     absent on a phone, and it looks identical to a working one in
  //     every screenshot taken with a mouse present.
  //
  //     So: park the pointer far away, then measure that the button is
  //     really painted (visible, non-zero box, not transparent) and that
  //     pressing it does what it does on the wide face.
  await openLanding("sessions");
  await until("rows on the narrow session list", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  await pointerAway();
  await settle();
  const narrowPin = page.locator("[data-row] [data-row-pin]").first();
  const painted = await narrowPin.evaluate((el) => {
    const s = getComputedStyle(el);
    const box = el.getBoundingClientRect();
    return { opacity: s.opacity, display: s.display, visibility: s.visibility, w: box.width, h: box.height };
  });
  if (
    painted.display === "none" ||
    painted.visibility === "hidden" ||
    Number(painted.opacity) === 0 ||
    painted.w === 0 ||
    painted.h === 0
  ) {
    throw new Error(`the pin is hover-only on the narrow face: ${JSON.stringify(painted)}`);
  }
  // …and it still works down here, which is the half a visibility check
  // cannot cover: a button can be perfectly visible and out of reach.
  const narrowIds = () =>
    page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const narrowTarget = (await narrowIds())[1];
  await page.locator(`[data-row="${narrowTarget}"] [data-row-pin]`).click();
  await until("the narrow pin to float its row", 10_000, async () => (await narrowIds())[0] === narrowTarget);
  await page.locator(`[data-row="${narrowTarget}"] [data-row-pin]`).click();
  await until("the narrow row to go back", 10_000, async () => (await narrowIds())[0] !== narrowTarget);

  // 18b) 手感三件套 (批②). Three motions, each asserted through what it
  //      actually leaves in the computed style rather than by watching
  //      it — and each with its reduced-motion case, because a guard
  //      nobody measures is a guard that quietly stops covering.
  //
  //      The place change first: on the narrow face, going into a detail
  //      and coming back out are the only two place changes this app
  //      has, and they arrive from the side they lie on.
  const screenAnim = async () => {
    // Said out loud rather than left to a locator timeout: without the
    // attribute there is no screen to ask about, and "waiting for
    // [data-screen]" reads like a slow app instead of a missing one.
    if ((await page.locator("[data-screen]").count()) !== 1) {
      throw new Error("probe dead: the narrow shell names no screen to animate");
    }
    return page.locator("[data-screen]").evaluate((el) => ({
      screen: el.dataset.screen,
      animation: getComputedStyle(el).animationName,
    }));
  };
  await openLanding("sessions");
  await until("rows on the narrow list", 10_000, async () => (await page.locator("[data-row]").count()) > 0);
  const intoDetail = page.locator("[data-row] [data-row-open]").first();
  await intoDetail.click();
  await until("the narrow detail", 10_000, async () => (await page.locator("[data-back]").count()) === 1);
  const arrived = await screenAnim();
  if (arrived.screen !== "detail" || arrived.animation !== "screen-in-from-right") {
    throw new Error(`a detail must arrive from the right: ${JSON.stringify(arrived)}`);
  }
  await page.locator("[data-back]").click();
  await until("back on the narrow list", 10_000, async () => (await page.locator("[data-list]").count()) === 1);
  const wentBack = await screenAnim();
  if (wentBack.screen !== "list" || wentBack.animation !== "screen-in-from-left") {
    throw new Error(`going back must come from the left: ${JSON.stringify(wentBack)}`);
  }

  //      The press. Measured with the button held down, and released
  //      **somewhere else** so nothing is actually clicked — otherwise
  //      this assertion would have to pick a control whose action it can
  //      afford, and the one it could afford would stop being the one
  //      people press.
  //      **Measured after the transition has run, not the instant the
  //      button goes down.** Mid-transition `getComputedStyle` reports
  //      the interpolated value, and the mere existence of a transition
  //      turns `transform` from "none" into a matrix — so a probe that
  //      sampled immediately would call `matrix(1,0,0,1,0,0)` a press
  //      and pass on a rule that scaled by nothing at all. Measured:
  //      the first spelling of this did exactly that, and only the
  //      opacity half caught it.
  const pressing = async (locator) => {
    const box = await locator.boundingBox();
    if (!box) throw new Error("probe dead: the control has no box to press");
    const style = () =>
      locator.evaluate((el) => {
        const s = getComputedStyle(el);
        // The x scale out of the matrix; "none" stays itself.
        const m = /matrix\(([-\d.]+)/.exec(s.transform);
        return { transform: s.transform, scale: m ? Number(m[1]) : 1, opacity: Number(s.opacity) };
      });
    const resting = await style();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await settle();
    const held = await style();
    // Away first, then up: a press released off the control is not a
    // click, so measuring one costs nothing.
    await page.mouse.move(5, 5);
    await page.mouse.up();
    await settle();
    return { resting, held };
  };
  // What a press should measure, read off the token rather than written
  // here — a number copied into this file keeps passing after the token
  // moves.
  const pressScale = await page.evaluate(() =>
    Number(getComputedStyle(document.documentElement).getPropertyValue("--press")),
  );
  if (!(pressScale > 0 && pressScale < 1)) {
    throw new Error(`--press is not a give: ${pressScale}`);
  }
  const pin = page.locator("[data-row] [data-row-pin]").first();
  const pinnedBefore = await page.locator("[data-row][data-pinned=true]").count();
  const press = await pressing(pin);
  if (press.resting.scale !== 1) {
    throw new Error(`probe dead: the control is already scaled at rest: ${press.resting.transform}`);
  }
  if (Math.abs(press.held.scale - pressScale) > 0.005) {
    throw new Error(
      `a press must give by --press (${pressScale}), it gave ${press.held.scale}`,
    );
  }
  if (!(press.held.opacity < press.resting.opacity)) {
    throw new Error(
      `a press must also dim, or reduced motion leaves nothing behind: ` +
        `${press.resting.opacity} -> ${press.held.opacity}`,
    );
  }
  if ((await page.locator("[data-row][data-pinned=true]").count()) !== pinnedBefore) {
    throw new Error("the press probe pressed something for real");
  }
  //      A row answers a press too — it is the most-pressed thing here —
  //      with the dim and **without** the give: a full-width strip
  //      scaling reads as the layout slipping, not as a press. Both
  //      halves asserted, or "it does nothing" would pass the first.
  const rowPress = await pressing(page.locator("[data-row] [data-row-open]").first());
  if (!(rowPress.held.opacity < rowPress.resting.opacity)) {
    throw new Error(`a row must answer a press: ${JSON.stringify(rowPress)}`);
  }
  if (rowPress.held.scale !== 1) {
    throw new Error(`a row must not scale under a press: ${rowPress.held.transform}`);
  }

  //      …and under reduced motion the give goes while the dim stays.
  //      Both halves: "nothing moved" is also what a broken probe says,
  //      and a press that answered with nothing at all would be the
  //      做了但没变化 failure arrived at through an accessibility
  //      setting.
  await page.emulateMedia({ reducedMotion: "reduce" });
  const still = await pressing(pin);
  const stillScreen = await screenAnim();
  await page.emulateMedia({ reducedMotion: "no-preference" });
  if (still.held.scale !== 1) {
    throw new Error(`reduced motion must take the give away: ${still.held.transform}`);
  }
  if (!(still.held.opacity < still.resting.opacity)) {
    throw new Error(`reduced motion must keep the press answered: ${JSON.stringify(still)}`);
  }
  if (stillScreen.animation !== "none") {
    throw new Error(`reduced motion must take the slide away: ${stillScreen.animation}`);
  }

  //      The reordering. A pin floats its row, which moves every row it
  //      passed — and FLIP is the difference between rows walking there
  //      and the list being a different list between two frames. Counted
  //      off the computed transform on a rAF loop: the animation is 240
  //      ms and the assertion is that it happened at all.
  await page.setViewportSize({ width: 1080, height: 720 });
  await openLanding("sessions");
  await until("rows back on the wide face", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  const watchTransforms = () =>
    page.evaluate(() => {
      window.__moved = 0;
      const tick = () => {
        for (const el of document.querySelectorAll("[data-row]")) {
          if (getComputedStyle(el).transform !== "none") window.__moved += 1;
        }
        window.__movedRaf = requestAnimationFrame(tick);
      };
      tick();
    });
  const stopWatching = () =>
    page.evaluate(() => {
      cancelAnimationFrame(window.__movedRaf);
      return window.__moved;
    });
  const rowIds = () => page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const flipTarget = (await rowIds())[2];
  await watchTransforms();
  await page.locator(`[data-row="${flipTarget}"] [data-row-pin]`).click();
  await until("the pinned row to lead", 20_000, async () => (await rowIds())[0] === flipTarget);
  const movedFrames = await stopWatching();
  if (movedFrames === 0) {
    throw new Error("rows teleported: no row carried a transform while the order changed");
  }
  //      …and the move is a **transition**, not a keyframe animation.
  //      That is what makes it interruptible: a transition retargets to
  //      a new value mid-flight, while an animation has to be torn down
  //      and restarted — and a second reorder landing on a running one
  //      is the ordinary case here, not the exotic one.
  const howItMoves = await page.locator("[data-row]").first().evaluate((el) => {
    const s = getComputedStyle(el);
    return { transition: s.transitionProperty, animation: s.animationName };
  });
  if (!howItMoves.transition.includes("transform")) {
    throw new Error(`rows must move by a transition on transform: ${howItMoves.transition}`);
  }
  if (howItMoves.animation !== "none") {
    throw new Error(`a keyframe animation cannot be redirected mid-flight: ${howItMoves.animation}`);
  }
  // …and it goes back to nothing. A FLIP that forgets to release leaves
  // the list permanently offset, which looks like a layout bug rather
  // than a motion one.
  await until("the rows to settle back to no transform", 10_000, async () =>
    page.evaluate(
      () =>
        [...document.querySelectorAll("[data-row]")].every(
          (el) => getComputedStyle(el).transform === "none",
        ),
    ),
  );
  // The same reorder under reduced motion moves nobody. The probe is
  // known alive: it just counted frames on the press above.
  await page.emulateMedia({ reducedMotion: "reduce" });
  await watchTransforms();
  await page.locator(`[data-row="${flipTarget}"] [data-row-pin]`).click();
  await until("the pin to come off", 20_000, async () => (await rowIds())[0] !== flipTarget);
  const stillFrames = await stopWatching();
  await page.emulateMedia({ reducedMotion: "no-preference" });
  if (stillFrames !== 0) {
    throw new Error(`reduced motion must not walk the rows: ${stillFrames} frames moved`);
  }

  // 19) search reaches the machine a row came from.
  //
  //     The premise has to be established or this measures nothing: find
  //     a row that **is** from somewhere else and whose own title and id
  //     do not contain that machine's name. Then searching the name has
  //     to keep it, and the only string it can be matching on is the
  //     source. Beta also holds a `chat/alpha` row — titled and keyed by
  //     that same name — which is exactly why the assertion is about one
  //     identified row rather than about how many rows survive.
  await page.setViewportSize({ width: 1080, height: 720 });
  await openLanding("sessions");
  await until("rows back on the wide face", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  const reported = await page.locator("[data-row][data-source]").evaluateAll((els) =>
    els.map((e) => ({ id: e.dataset.row, from: e.dataset.source, title: e.dataset.title ?? "" })),
  );
  if (!reported.length) throw new Error("probe dead: no row carries a source");
  // Title and id only — deliberately **not** the row's rendered text,
  // which paints "from <machine>" and therefore always contains the
  // name. Checking that would make this probe unsatisfiable, and the
  // failure would read like the feature was broken.
  const bySource = reported.find(
    (r) =>
      !r.id.toLowerCase().includes(r.from.toLowerCase()) &&
      !r.title.toLowerCase().includes(r.from.toLowerCase()),
  );
  if (!bySource) {
    throw new Error(
      `probe dead: every reported row already names its machine in its own id or title — ` +
        `matching on the source cannot be told apart: ${JSON.stringify(reported)}`,
    );
  }
  await sessionSearch.fill(bySource.from);
  await until(`the row from ${bySource.from} to survive its machine's name`, 5_000, async () =>
    (await page.locator(`[data-row="${bySource.id}"]`).count()) === 1,
  );
  // The control: a term nothing answers to drops it, so the line above
  // is the search working rather than the search being off.
  await sessionSearch.fill("no-machine-answers-to-this");
  await until("that row gone for a term nothing matches", 5_000, async () =>
    (await page.locator(`[data-row="${bySource.id}"]`).count()) === 0,
  );
  await sessionSearch.fill("");
  await until("every session back", 5_000, async () => (await page.locator("[data-row]").count()) > 1);

  // 20) a ticked filter outlives a reload, and is kept per pane.
  //
  //     The same rule the arrangement follows — a preference set once
  //     stays set — and it used to be the one place that broke it. The
  //     control is the state before: the tick has to be provably absent,
  //     or "still ticked after a reload" could just be a tick that was
  //     never anything else.
  const filterLit = () => page.locator("[data-pane-filter][data-on=true]").count();
  if ((await filterLit()) !== 0) throw new Error("probe dead: the filter is already on");
  const keptWord = wordsOnScreen[0];
  const keptRows = await page.locator(`[data-word="${keptWord}"]`).count();
  await tickWord(keptWord);
  await until(`only the ${keptWord} rows`, 5_000, async () => (await page.locator("[data-row]").count()) === keptRows);

  await page.reload();
  await until("rows after the reload", 20_000, async () => (await page.locator("[data-row]").count()) > 0);
  if ((await filterLit()) !== 1) throw new Error("the filter control came back unlit after a reload");
  if ((await page.locator("[data-row]").count()) !== keptRows) {
    throw new Error("the ticked filter did not survive a reload — the list came back unfiltered");
  }
  // Per pane, not one setting shared by all of them: the stored key
  // carries the landing. Read off storage because that *is* the
  // mechanism, and today only one pane has a filter to tick.
  const filterKeys = await page.evaluate(() =>
    Object.keys(window.localStorage).filter((k) => k.startsWith("khor.filter")),
  );
  if (filterKeys.length !== 1 || !filterKeys[0].endsWith(".sessions")) {
    throw new Error(`the filter is not remembered per pane: ${JSON.stringify(filterKeys)}`);
  }
  await tickWord(keptWord);
  await until("every row back", 10_000, async () => (await page.locator("[data-row]").count()) > keptRows);

  // 21) the settings screen: what this machine wears, and changing it.
  //
  //     Every swatch there is a face **the node derived** and this app
  //     only painted, so the assertions are about the same picture
  //     turning up in two places — never about a picture looking a
  //     particular way, which is the one thing no test here can judge.
  //
  //     A session of beta's own first. Beta's other rows all belong to
  //     alpha (a chat channel is homed on the machine it talks to, and
  //     the agent row is alpha's report), so without this the sessions
  //     pane carries no face of beta's at all and "it moved everywhere"
  //     would be measuring the rail twice.
  feedHook(envB, "SessionStart", "", "beef2");
  feedHook(envB, "UserPromptSubmit", "", "beef2");
  await openLanding("sessions");
  await until("beta's own row", 15_000, async () =>
    (await page.locator('[data-row="tui/beef2"]').count()) === 1,
  );

  // The rail's last glyph is the one that opens no landing — and there
  // being exactly one such glyph is itself the check that this is still
  // the settings one and not something added beside it.
  //
  // **The mark is excluded by name rather than by luck.** It is a rail
  // item now and it opens no landing either (it opens the mesh), so
  // without this the count would be two and the probe would report
  // itself dead. Excluding it keeps the original guard doing its job: a
  // *third* landing-less glyph still trips this.
  const settingsGlyph = page.locator(
    "[data-rail-item]:not([data-landing]):not(:has([data-rail-mark]))",
  );
  if ((await settingsGlyph.count()) !== 1) {
    throw new Error("probe dead: the rail does not have exactly one glyph that opens no landing");
  }
  await settingsGlyph.click();
  const sheet = page.locator("[data-face-settings]");
  await until("the settings sheet", 10_000, async () => (await sheet.count()) === 1);
  // The sheet's frame mounts before its answer arrives — it asks the
  // node when it opens, and paints nothing but its title until then. So
  // wait for the options, not for the box: waiting on the box read the
  // empty frame and reported "the settings screen offers []", which
  // reads like the screen being broken rather than like arriving early.
  // (It passed on three runs before it failed on the fourth, which is
  // the whole signature of waiting for the wrong thing.)
  await until("the options to arrive", 10_000, async () =>
    (await sheet.locator("[data-face-option]").count()) > 0,
  );

  //     a. three axes, and every option on all of them painted by the
  //     real brush. Stated positively for the same reason the row faces
  //     are: a face that failed to derive renders no <svg> at all, so
  //     counting them *is* "nothing fell back to a placeholder".
  const axisNames = await sheet
    .locator("[data-face-axis]")
    .evaluateAll((els) => els.map((e) => e.dataset.faceAxis));
  if (axisNames.join(",") !== "palette,variant,shape") {
    throw new Error(`the settings screen offers ${JSON.stringify(axisNames)}`);
  }
  const optionCount = await sheet.locator("[data-face-option]").count();
  const optionFaces = await sheet.locator("[data-face-option] [data-face] svg").count();
  if (optionCount < 6 || optionFaces !== optionCount) {
    throw new Error(`options with a painted face: ${optionFaces} of ${optionCount}`);
  }

  //     b. one option marked per axis, and **all three paint the face
  //     this machine is already wearing**. That follows from the
  //     design — a marked option is the current value on its axis, and
  //     every option is derived with the other two left alone — which
  //     is exactly why it is worth asserting: one line catches a swatch
  //     derived from the wrong style, a mark on the wrong option, and a
  //     second painter.
  // The same locator item 10 proved is this machine's face at the foot
  // of the rail — reused rather than re-derived, so what moves below is
  // the picture that test already tied to beta's own row.
  const markedFaces = async () =>
    sheet
      .locator("[data-face-option][data-on=true] [data-face]")
      .evaluateAll((els) => els.map((e) => e.innerHTML.replace(/av-blur-[^"')\s]+/g, "BLUR")));
  let worn = await faceOf(railFace);
  const marked = await markedFaces();
  if (marked.length !== 3) throw new Error(`${marked.length} options are marked; one per axis`);
  if (marked.some((m) => m !== worn)) {
    throw new Error("a marked option is not the face this machine is wearing");
  }

  //     c. picking another palette moves this machine's face
  //     **everywhere it appears**, in one go.
  //
  //     Counted rather than named: this script does not work out which
  //     rows are beta's, it counts how many places wore that face
  //     before and requires the same number to wear the new one after,
  //     and none to be left wearing the old. Scoped to #root so the
  //     sheet's own swatches — which change too — stay out of it.
  const wearing = async (want) =>
    (await page.locator("#root [data-face]").evaluateAll((els) => els.map((e) => e.innerHTML)))
      .filter((h) => h.replace(/av-blur-[^"')\s]+/g, "BLUR") === want).length;
  const places = await wearing(worn);
  if (places < 2) {
    throw new Error(`this machine's face is in ${places} place(s); the premise is more than one`);
  }
  const otherPalette = (
    await sheet
      .locator('[data-face-axis="palette"] [data-face-option]')
      .evaluateAll((els) => els.filter((e) => e.dataset.on !== "true").map((e) => e.dataset.faceOption))
  )[0];
  if (!otherPalette) throw new Error("probe dead: every palette is already the chosen one");
  await sheet.locator(`[data-face-option="${otherPalette}"][data-axis="palette"]`).click();
  await until("this machine repainted everywhere", 15_000, async () => {
    const now = await faceOf(railFace);
    return now !== worn && (await wearing(now)) === places;
  });
  const wasWorn = worn;
  worn = await faceOf(railFace);
  if ((await wearing(wasWorn)) !== 0) throw new Error("somewhere is still painting the old face");

  //     d. a variant reaches the canvas the core ships. beam's canvas
  //     is 36 where the other two are 80, so switching to it and back
  //     is a change no styling could fake and no cache could survive.
  const canvasOf = () => railFace.locator("svg").getAttribute("viewBox");
  const wideCanvas = await canvasOf();
  await sheet.locator('[data-face-option="beam"][data-axis="variant"]').click();
  await until("beam's own canvas", 15_000, async () => (await canvasOf()) === "0 0 36 36");
  await sheet.locator('[data-face-option="marble"][data-axis="variant"]').click();
  await until("back off beam's canvas", 15_000, async () => (await canvasOf()) === wideCanvas);

  //     e. a hand-picked color belongs to no factory set, so **no**
  //     palette is marked — two marks, not three. A nearest match would
  //     light a button nobody pressed, and pressing that lit button
  //     would then look like a control that does nothing.
  //
  //     Driven by setting the well and dispatching what a commit
  //     dispatches. **This is not the forbidden kind of shortcut.** The
  //     rule against going around the path under test is about stepping
  //     over the app's own code when it is in the way; the OS color
  //     picker is not the app's code, and it is a window Playwright
  //     cannot open at all. Everything from the element and the event
  //     onward is the real path — the listener being tested reads the
  //     element, not the event — so this is as close to a person's
  //     press as anything automated can stand.
  worn = await faceOf(railFace);
  await sheet.locator("[data-color-slot]").first().evaluate((el) => {
    el.value = "#123456";
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await until("the hand-picked color everywhere", 15_000, async () => {
    const now = await faceOf(railFace);
    return now !== worn && (await wearing(now)) === places;
  });
  const afterHand = await markedFaces();
  if (afterHand.length !== 2) {
    throw new Error(`${afterHand.length} options marked after a hand-picked color; no palette is one`);
  }

  //     f. the CLI says the same thing — so the change went through the
  //     library and not just through this screen. Read structurally:
  //     the marked lines are the ones carrying a third field, and the
  //     words themselves belong to the catalog, not to this script.
  const wornBefore = await faceOf(railFace);
  await sheet.locator(`[data-face-option="${otherPalette}"][data-axis="palette"]`).click();
  //     **Wait for the face this machine wears to change, not only for
  //     the sheet to settle.** The marks come from the sheet's own
  //     answer and the rail's face comes round on the device poll, so
  //     "three options are marked" is true a beat before the rail has
  //     caught up — and what gets compared then is the *previous* face.
  //     The two are read together below, so waiting for the slower one
  //     is what makes the comparison mean anything.
  await until("the factory palette back, and worn", 20_000, async () => {
    const marks = await markedFaces();
    return marks.length === 3 && (await faceOf(railFace)) !== wornBefore;
  });
  //     …and the three marked options still paint what this machine
  //     wears. Re-asserted here and not only at the top, because up
  //     there the machine wore the factory style and every axis sat on
  //     its default — an option derived from the defaults instead of
  //     from the current style would have looked exactly the same.
  const nowWorn = await faceOf(railFace);
  if ((await markedFaces()).some((m) => m !== nowWorn)) {
    throw new Error("after a change, a marked option is not the face this machine is wearing");
  }
  const cliMarks = cli(envB, "face")
    .split("\n")
    .filter((l) => l.split("\t").length === 3);
  if (cliMarks.length !== 3) throw new Error(`khor face marks ${cliMarks.length} options; one per axis`);
  if (!cliMarks.some((l) => l.startsWith(`${otherPalette}\t`))) {
    throw new Error(`khor face does not agree that ${otherPalette} is what this machine wears`);
  }

  //     g. flipping the theme still moves nothing inside the face — now
  //     under a style somebody chose rather than the factory one. The
  //     ground proves the flip took, the way it does above.
  const chosen = await faceOf(railFace);
  const groundOf = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  await page.emulateMedia({ colorScheme: "light" });
  const lightGround = await groundOf();
  const lightChosen = await faceOf(railFace);
  await page.emulateMedia({ colorScheme: "dark" });
  if ((await groundOf()) === lightGround) throw new Error("probe dead: the theme flip changed no ground");
  if ((await faceOf(railFace)) !== lightChosen) throw new Error("a chosen face followed the theme");
  await page.emulateMedia({ colorScheme: null });

  //     h. and it outlives a reload — the choice lives in the node, not
  //     on this screen.
  await page.keyboard.press("Escape");
  await until("the sheet closed", 10_000, async () => (await sheet.count()) === 0);
  await page.reload();
  await until("rows after the reload", 20_000, async () => (await page.locator("[data-row]").count()) > 0);
  if ((await faceOf(railFace)) !== chosen) throw new Error("the chosen face did not survive a reload");

  // 22) a machine restyled from its own terminal reaches this screen.
  //
  //     The direction that carries "only a machine may say what it
  //     looks like": alpha changes its own style with `khor face`, and
  //     beta's screen follows — which can only happen through the
  //     device table and the sync pump. The before state is established
  //     on both sides first (beta is provably painting something else,
  //     and alpha's palette provably changed), and the control comes
  //     after: beta's own face did not move, because nobody else may
  //     move it.
  await openLanding("devices");
  const alphaFace = page.locator('[data-device="alpha"] [data-face]');
  const betaFace = page.locator('[data-device="beta"] [data-face]');
  await until("both machines on the list", 10_000, async () => (await alphaFace.count()) === 1);
  const alphaWas = await faceOf(alphaFace);
  const betaWas = await faceOf(betaFace);
  const alphaColorsBefore = cli(envA, "face").split("\n")[1];
  cli(envA, "face", "--palette", "monokai", "--variant", "bauhaus");
  const alphaColorsAfter = cli(envA, "face").split("\n")[1];
  if (alphaColorsBefore === alphaColorsAfter) {
    throw new Error("probe dead: alpha's own palette did not change");
  }
  await until("alpha's new face on beta's screen", 40_000, async () =>
    (await faceOf(alphaFace)) !== alphaWas,
  );
  if ((await faceOf(betaFace)) !== betaWas) {
    throw new Error("alpha restyling itself repainted this machine");
  }

  // 23) a machine row opens that machine's card.
  //
  //     The wide face is back on by now, so the list and the card are on
  //     screen together. The control comes first and it is a real one:
  //     the card's section exists as soon as the devices pane is open
  //     (it is the detail region), so what proves a machine was opened
  //     is its *content* — no id is printed until a row is clicked.
  await page.setViewportSize({ width: 1080, height: 720 });
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) >= 2);
  if ((await page.locator("[data-machine-id]").count()) !== 0) {
    throw new Error("a machine card is filled in before any machine was opened");
  }

  //     **And what that space holds instead is the mandala map** — one
  //     seat per machine, exactly one of them the middle, the ring a
  //     real ring, and nothing drawn between any two of them.
  //
  //     The last one is the judgment the picture is built on: khor knows
  //     who is in the mesh and not who can reach whom, so no line may be
  //     drawn. It is asserted as a count rather than as a hunt for lines
  //     nobody wrote — every `svg` inside the map has to belong to a
  //     face, so an overlay drawing edges turns up as a surplus — and
  //     the positive half is counted first, so a zero means zero rather
  //     than a selector that stopped naming anything.
  const seats = page.locator("[data-mandala-map] [data-seat]");
  const machines = await page.locator("[data-device]").count();
  await until("a seat for every machine", 10_000, async () => (await seats.count()) === machines);
  const middles = await seats.evaluateAll(
    (els) => els.filter((e) => e.dataset.seatMe === "true").length,
  );
  if (middles !== 1) throw new Error(`${middles} seats claim to be this machine; exactly one may`);

  const faces = await page.locator("[data-mandala-map] [data-seat] [data-face] svg").count();
  if (faces !== machines) throw new Error(`${faces} faces painted for ${machines} machines`);
  const drawn = await page.locator("[data-mandala-map] svg").count();
  if (drawn !== faces) {
    throw new Error(
      `the map draws ${drawn} svg elements for ${faces} faces — the surplus is a line between ` +
        "machines, and khor does not know who can reach whom",
    );
  }

  //     The ring is a ring: every other **face** sits the same distance
  //     from the middle one.
  //
  //     **The face, not the seat, and that distinction is the whole
  //     assertion.** A seat is positioned by translating it half its own
  //     size, so its box is centred on its point by construction and
  //     every radius measured off *it* comes out equal no matter what is
  //     inside. The first version of this check did exactly that and
  //     stayed green when the layout was deliberately broken — the
  //     middle seat has no age line, so a shorter caption pulled its
  //     face a few pixels off the circle while its box did not move.
  //     What a person sees is where the faces are, so that is what gets
  //     measured.
  //
  //     **Two properties, because one of them alone is blind.** Equal
  //     radii catch a single face drifting off the circle, and they are
  //     blind to the middle sliding as a whole — the ring is symmetric,
  //     so both radii grow by the same amount and stay equal. The second
  //     is what "evenly spaced around it" actually means: **the middle
  //     sits at the average of the ring**. That is what went red when the
  //     layout was broken on purpose, and equal-radii alone did not.
  const ring = await seats.evaluateAll((els) => {
    const at = (e) => {
      const b = (e.querySelector("[data-face]") ?? e).getBoundingClientRect();
      return { x: b.left + b.width / 2, y: b.top + b.height / 2, me: e.dataset.seatMe === "true" };
    };
    const all = els.map(at);
    const centre = all.find((p) => p.me);
    const around = all.filter((p) => !p.me);
    return {
      radii: around.map((p) => Math.round(Math.hypot(p.x - centre.x, p.y - centre.y))),
      centre: [Math.round(centre.x), Math.round(centre.y)],
      average: [
        Math.round(around.reduce((t, p) => t + p.x, 0) / around.length),
        Math.round(around.reduce((t, p) => t + p.y, 0) / around.length),
      ],
    };
  });
  if (ring.radii.length < 2) {
    throw new Error(`probe dead: only ${ring.radii.length} seats around the middle`);
  }
  if (Math.max(...ring.radii) - Math.min(...ring.radii) > 2) {
    throw new Error(`the seats are not on one circle: radii ${ring.radii.join(", ")}`);
  }
  const drift = Math.hypot(ring.centre[0] - ring.average[0], ring.centre[1] - ring.average[1]);
  if (drift > 2) {
    throw new Error(
      `this machine's face is ${Math.round(drift)}px off the middle of the ring: ` +
        `at ${ring.centre}, ring averages ${ring.average}`,
    );
  }

  //     **`khor devices` is the independent witness for the readings.**
  //     It prints the same reading from the same node, and `vitals_line`
  //     joins its pieces with two spaces — so counting them needs no word
  //     from the catalog, and this file stays free of Chinese.
  const cliReadings = (machine, env) => {
    const line = cli(env, "devices")
      .split("\n")
      .find((l) => l.startsWith(`${machine}\t`));
    if (!line) throw new Error(`probe dead: no ${machine} row printed by \`khor devices\``);
    const cells = line.split("\t");
    if (cells.length < 3) throw new Error(`probe dead: ${machine}'s row prints no readings`);
    return cells[2].split("  ").filter(Boolean);
  };
  //     The age counts on the GUI side too, because the CLI prints it as
  //     one more piece of that same run — leaving it out would make the
  //     two disagree for a reason that has nothing to do with readings.
  const sameNumberOfReadings = async (machine, env) => {
    const onScreen =
      (await page.locator("[data-vitals-unit]").count()) +
      (await page.locator("[data-vitals-age]").count());
    const printed = cliReadings(machine, env);
    if (onScreen !== printed.length) {
      throw new Error(
        `${machine}: the card draws ${onScreen} readings, \`khor devices\` prints ` +
          `${printed.length} — ${printed.join(" | ")}`,
      );
    }
  };

  //     Clicked the way a person clicks it: the row's own button.
  const shortId = await page
    .locator('[data-device="alpha"]')
    .evaluate((el) => el.dataset.row.slice(0, 12));
  await page.locator('[data-device="alpha"] [data-row-open]').click();
  await until("alpha's card", 10_000, async () => (await page.locator("[data-machine-id]").count()) === 1);

  //     …and it is alpha's card, not merely *a* card. The row prints the
  //     first twelve characters of the id and the card prints all of it,
  //     so one being a prefix of the other ties the two together without
  //     this script knowing either value.
  const cardId = await page.locator("[data-machine-id]").innerText();
  if (!cardId.startsWith(shortId) || cardId.length <= shortId.length) {
    throw new Error(`the card shows ${cardId}, which is not the full id behind ${shortId}`);
  }

  //     One machine, one picture — the same rule the rail and the row
  //     already answer to, now with a third place to be wrong in.
  const cardFace = page.locator("[data-machine-card] [data-face]");
  if ((await cardFace.count()) !== 1) throw new Error("probe dead: no face on the card");
  if ((await faceOf(cardFace)) !== (await faceOf(page.locator('[data-device="alpha"] [data-face]')))) {
    throw new Error("alpha wears one face in the list and another on its card");
  }

  //     The readings are drawn, and **how many of them there are is not
  //     a number written here**. It follows what the backend could read
  //     on this machine, which is the entire point of the GPU being a
  //     field that can be absent: on a machine khor cannot ask, both
  //     faces show one row fewer and this still holds. So the count comes
  //     from `khor devices` printing the same reading — an independent
  //     witness, with the age line counted on both sides because the CLI
  //     prints it inside the same run of cells.
  if ((await page.locator("[data-vitals-bar]").count()) === 0) {
    throw new Error("no reading drew a bar");
  }
  await sameNumberOfReadings("alpha", envB);

  //     **The offline axis reaches the screen.** alpha's reading was
  //     taken on alpha and carried here, so it has an age; beta samples
  //     itself to answer the very call that painted this, so it has
  //     none. Both halves are asserted because either alone passes for
  //     the wrong reason — always showing an age, or never showing one.
  if ((await page.locator("[data-vitals-age]").count()) !== 1) {
    throw new Error("a reading that travelled here shows no age");
  }
  await page.locator('[data-device="beta"] [data-row-open]').click();
  await until("beta's card", 10_000, async () =>
    (await page.locator("[data-machine-id]").innerText()).startsWith(
      await page.locator('[data-device="beta"]').evaluate((el) => el.dataset.row.slice(0, 12)),
    ),
  );
  if ((await page.locator("[data-vitals-unit]").count()) === 0) {
    throw new Error("this machine's own card is missing readings");
  }
  if ((await page.locator("[data-vitals-age]").count()) !== 0) {
    throw new Error("this machine's own reading is dressed as something remembered");
  }
  await sameNumberOfReadings("beta", envB);

  //     **The GPU is one of those readings, and it is the same sentence
  //     on both faces.** The counts above already tie how many rows there
  //     are to what the CLI printed; what is left is that this particular
  //     row says what the CLI says. Digits are stripped before comparing
  //     because the utilisation is alive and the two faces sample a
  //     moment apart — what has to match is the sentence (which is the
  //     catalog key, so a word drifting on one face reddens this) and the
  //     card count, which is not alive.
  //
  //     Required, not skipped: every Mac has an accelerator, so a missing
  //     row here is a real finding. On a machine khor cannot ask this
  //     goes red and names the reason, which beats a silent skip that
  //     would pass by not running.
  const gpuRow = page.locator('[data-vitals-unit="gpu"]');
  if ((await gpuRow.count()) !== 1) {
    throw new Error("no GPU reading on this machine's card, and every Mac has one to read");
  }
  const sentence = (s) => s.replace(/\d+/g, "#").replace(/\s+/g, " ").trim();
  const cardCount = (s) => s.match(/\d+/g)?.[1];
  const onCard = await gpuRow.innerText();
  const inTerminal = cliReadings("beta", envB).find((p) => sentence(p) === sentence(onCard));
  if (!inTerminal) {
    throw new Error(
      `the card says "${onCard}" and \`khor devices\` prints nothing shaped like it — ` +
        `${cliReadings("beta", envB).join(" | ")}`,
    );
  }
  if (cardCount(onCard) !== cardCount(inTerminal)) {
    throw new Error(`the card counts ${cardCount(onCard)} cards, the terminal ${cardCount(inTerminal)}`);
  }

  // 24) every machine pane's rows open their second step now — the
  //     devices card, the disk, the borrowed network. The pane that led
  //     nowhere was the bug this once caught; there is none left, so the
  //     assertion inverts to "all three lead somewhere", read off the
  //     same three machines each pane lists.
  const openersOn = async (tab) => {
    await openLanding(tab);
    await until(`machines on the ${tab} pane`, 10_000, async () =>
      (await page.locator("[data-device]").count()) > 0,
    );
    return page.locator("[data-device] [data-row-open]").count();
  };
  for (const tab of ["devices", "files", "browser"]) {
    const n = await openersOn(tab);
    if (n < 3) {
      throw new Error(`only ${n} of three machine rows open their second step on the ${tab} pane`);
    }
  }

  // 24c) files 的 omnibox (批③ 三笔): the machine is a chip and the rest
  //      is a path on it.
  //
  //      **Same mechanism as the sessions chips, different meaning** —
  //      here a chip names the thing the pane acts on rather than
  //      narrowing a list, and nothing in the box knows the difference:
  //      only what the pane does with a commit differs.
  const omniDir = join(SCRATCH, "omni-files");
  mkdirSync(join(omniDir, "inner"), { recursive: true });
  writeFileSync(join(omniDir, "inner", "omni-marker.txt"), "x");
  await openLanding("files");
  await until("the files pane", 10_000, async () => (await page.locator("[data-omnibox]").count()) === 1);
  const filesInput = page.locator("[data-omni-input]");
  await filesInput.click();
  await until("machines offered", 10_000, async () =>
    (await page.locator('[data-omni-item^="dev:"]').count()) > 0,
  );
  await filesInput.fill("beta");
  await until("beta offered", 10_000, async () =>
    (await page.locator('[data-omni-item="dev:beta"]').count()) === 1,
  );
  await filesInput.press("Enter");
  await until("beta as a chip, and its disk open", 15_000, async () =>
    (await page.locator('[data-chip="dev:beta"]').count()) === 1 &&
    (await page.locator("[data-files-list]").count()) === 1,
  );

  //      **Typing inside one directory must not ask again.** That is the
  //      whole of the anti-flicker rule: candidates are keyed by the
  //      directory, so narrowing within one is local and there is no
  //      second answer to arrive late and replace the list. Counted off
  //      the network, because "it looked smooth" is not a measurement.
  const lsCalls = () =>
    page.evaluate(
      () =>
        performance.getEntriesByType("resource").filter((e) => e.name.endsWith("/ls")).length,
    );
  await filesInput.fill(`${omniDir}/`);
  await until("the inner directory offered", 15_000, async () =>
    (await page.locator(`[data-omni-item="${omniDir}/inner/"]`).count()) === 1,
  );
  await page.evaluate(() => performance.clearResourceTimings());
  for (const s of ["i", "in", "inn", "inne"]) {
    await filesInput.fill(`${omniDir}/${s}`);
    await new Promise((r) => setTimeout(r, 250));
  }
  const asked = await lsCalls();
  if (asked > 0) {
    throw new Error(`typing inside one directory asked the machine ${asked} more times`);
  }
  //      …and the probe is alive: leaving the directory does ask.
  await filesInput.fill(`${omniDir}/inner/`);
  await until("a new directory being asked about", 10_000, async () => (await lsCalls()) > 0);

  //      Enter opens what was typed, for real — the marker file is on
  //      beta's actual disk, and nothing here put it on the screen but
  //      a real `ls` of a real path.
  await filesInput.fill(`${omniDir}/inner`);
  await filesInput.press("Enter");
  await until("the typed path really opened", 20_000, async () =>
    (await page.locator('[data-file="omni-marker.txt"]').count()) === 1,
  );
  //      Taking the chip off leaves the machine — back to the landing,
  //      which is the pinned shortcuts rather than somebody's disk.
  await page.locator('[data-chip-remove="dev:beta"]').click();
  await until("back off the machine", 10_000, async () =>
    (await page.locator("[data-files-list]").count()) === 0 &&
    (await page.locator('[data-chip="dev:beta"]').count()) === 0,
  );

  // 24d) browser 的 omnibox (批③ 四笔): the exit is a chip, the address
  //      is text, and the candidates are the pinned pages — the one
  //      list of addresses this app actually has.
  //
  //      **Enter is deliberately not pressed here.** Submitting opens a
  //      page through a real borrow, on the browser of whoever is
  //      running this — the same reason 24b below never clicks a link.
  //      What is checked is that the box offers the right things; that
  //      the open itself works is `tunnel_wire`'s.
  await openLanding("browser");
  await until("the browser pane's box", 10_000, async () =>
    (await page.locator("[data-omnibox]").count()) === 1,
  );
  const webInput = page.locator("[data-omni-input]");
  await webInput.click();
  await webInput.fill("beta");
  await until("beta offered as an exit", 10_000, async () =>
    (await page.locator('[data-omni-item="dev:beta"]').count()) === 1,
  );
  await webInput.press("Enter");
  await until("beta as the exit chip", 15_000, async () =>
    (await page.locator('[data-chip="dev:beta"]').count()) === 1,
  );
  //      Before any page is pinned there is nothing to complete with,
  //      and the box says so by offering nothing — 24b pins one below,
  //      and the candidate is asserted there where the pin provably
  //      exists.
  await page.locator('[data-chip-remove="dev:beta"]').click();
  await until("off the exit again", 10_000, async () =>
    (await page.locator('[data-chip="dev:beta"]').count()) === 0,
  );

  // 24b) the browser landing keeps pages: picking a machine opens the
  //      address bar (whose placeholder names the exit, so the user
  //      knows a page leaves through it), pinning a typed page makes a
  //      shortcut that survives a reload into the PinnedWebs list before
  //      any machine is picked, and unpinning takes it back. The pin
  //      rides the real webpins table; the open itself (a real borrow)
  //      is covered by tunnel_wire, and clicking a shortcut is avoided
  //      here on purpose so this item never dials.
  await openLanding("browser");
  await until("machines on the browser pane", 10_000, async () =>
    (await page.locator("[data-device] [data-row-open]").count()) > 0,
  );
  const exit = await page.locator("[data-device]").first().getAttribute("data-device");
  const openExit = () =>
    page.locator(`[data-device="${exit}"] [data-row-open]`).first().click();
  await openExit();
  await until("the address bar on the browser pane", 10_000, async () =>
    (await page.locator("[data-web-address]").count()) === 1,
  );
  const barPlaceholder = await page.locator("[data-web-address]").getAttribute("placeholder");
  if (!barPlaceholder?.includes(exit)) {
    throw new Error(`the address bar (${barPlaceholder}) does not name the exit ${exit}`);
  }
  const url = "https://smoke.example/page";
  await page.locator("[data-web-address]").fill(url);
  await until("the pin-this-page control", 10_000, async () =>
    (await page.locator("[data-pin-web]").count()) === 1,
  );
  await page.locator("[data-pin-web]").click();
  await until("the pinned page in this exit's list", 10_000, async () =>
    (await page.locator(`[data-web-pin-open="${url}"]`).count()) === 1,
  );
  // A reload drops every React selection, so the browser landing opens
  // on PinnedWebs — where the pin, being the network's and not this
  // screen's, must still be. (Reload keeps the ?bridge= query.)
  await page.reload();
  await openLanding("browser");
  await until("the pinned page in the shortcut list", 10_000, async () =>
    (await page.locator(`[data-pinned-web="${url}"]`).count()) === 1,
  );
  // …and the omnibox offers it as a completion (批③ 四笔). Asserted
  // here because this is where a pin provably exists — the candidates
  // are the pinned pages and nothing else, so before this point there
  // was correctly nothing to offer. Typed, not pressed: Enter would
  // dial.
  await openExit();
  await until("the exit chip", 10_000, async () =>
    (await page.locator("[data-chip]").count()) === 1,
  );
  await page.locator("[data-omni-input]").click();
  await page.locator("[data-omni-input]").fill(url.slice(0, url.length - 2));
  await until("the pinned page offered as a completion", 10_000, async () =>
    (await page.locator(`[data-omni-item="${url}"]`).count()) === 1,
  );
  await page.locator("[data-omni-input]").fill("");
  await page.keyboard.press("Escape");

  // Unpin from its exit — reached by the device row, not the shortcut,
  // so opening it never dials.
  await until("the address bar back", 10_000, async () =>
    (await page.locator(`[data-unpin-web="${url}"]`).count()) === 1,
  );
  await page.locator(`[data-unpin-web="${url}"]`).click();
  await until("the pinned page gone from this exit", 10_000, async () =>
    (await page.locator(`[data-web-pin-open="${url}"]`).count()) === 0,
  );

  // Back where the next item expects to be: it pins the first row it
  // finds, and which pane is open decides which row that is.
  await openLanding("devices");

  // 29) the mark opens the mesh and what it cost — on both faces.
  //
  //     The narrow half is the point of it: with the map living in the
  //     devices pane's detail, a phone can never see it (that pane shows
  //     the list, and picking a machine shows that machine's card). This
  //     is the only way in from there, so it is checked there first.
  //
  //     What the panel says is compared against `khor usage` reading the
  //     same home — the independent witness, and **no number and no word
  //     is written in this file**: the day and the category are read off
  //     the app and looked for in the terminal's output.
  await page.setViewportSize({ width: 390, height: 720 });
  await page.locator("[data-rail-mark]").click();
  await until("the mesh on the narrow face", 15_000, async () =>
    (await page.locator("[data-mark-place] [data-mandala-map]").count()) === 1,
  );
  await until("the spending beside it", 30_000, async () =>
    (await page.locator("[data-mark-place] [data-usage]").count()) === 1,
  );
  //     …and the list is gone while it is up: this place is not a pane.
  if ((await page.locator("[data-list]").count()) !== 0) {
    throw new Error("the mark's place is showing a list beside it; it replaces the pair");
  }
  //     The narrow face never shows two things at once, so that check
  //     alone would be satisfied by a wide shell that kept the list —
  //     which is why the wide one is checked too, below, where it can
  //     actually fail.
  //     Pressing a landing comes back out of it.
  await openLanding("devices");
  if ((await page.locator("[data-mark-place]").count()) !== 0) {
    throw new Error("the mark's place is still up after a landing was picked");
  }

  await page.setViewportSize({ width: 1080, height: 720 });
  await page.locator("[data-rail-mark]").click();
  await until("the mesh on the wide face", 15_000, async () =>
    (await page.locator("[data-mark-place] [data-mandala-map]").count()) === 1,
  );
  if ((await page.locator("[data-list]").count()) !== 0) {
    throw new Error("the wide shell kept the list beside the mesh; this place replaces the pair");
  }
  //     Read off the **text on screen**, not off the data attributes
  //     beside it. A first version compared the attributes and stayed
  //     green when the heading was made to print a date nobody spent
  //     anything on — it was checking that the app agrees with itself.
  //     What a person reads is the words, so the words are what get
  //     looked for in the terminal's output.
  const spentDays = await page
    .locator("[data-usage] [data-usage-row]")
    .evaluateAll((els) =>
      els.map((e) => ({
        day: e.closest("[data-usage-day]")?.querySelector("[data-usage-date]")?.textContent?.trim()
          ?? e.previousElementSibling?.textContent?.trim()
          ?? "",
        category: e.querySelector("[data-usage-category]")?.textContent?.trim() ?? "",
      })),
    );
  if (spentDays.some((d) => !d.day || !d.category)) {
    throw new Error(`probe dead: a row printed no day or no vendor: ${JSON.stringify(spentDays)}`);
  }
  if (spentDays.length === 0) {
    throw new Error("probe dead: the panel shows no spending, so nothing below compares anything");
  }
  //     The faces on the map are not pressable here: there is no card in
  //     this space to open into, and an affordance that answers nothing
  //     is what the mark itself was forbidden from being.
  const pressable = await page
    .locator("[data-mark-place] [data-seat]")
    .evaluateAll((els) => els.filter((e) => e.tagName === "BUTTON").length);
  if (pressable !== 0) {
    throw new Error(`${pressable} faces on the mesh offer to open something that is not there`);
  }
  const printed = cli(envB, "usage", "--days", "30");
  for (const { day, category } of spentDays) {
    const line = printed.split("\n").find((l) => l.includes(day) && l.includes(category));
    if (!line) {
      throw new Error(
        `the app shows ${category} spending on ${day} and \`khor usage\` prints no such line:\n${printed}`,
      );
    }
  }
  await openLanding("devices");

  // 28) pressing a face on the map opens that machine.
  //
  //     The map is the other way into a machine, and a picture that
  //     looks pressable and answers nothing is exactly what the app's
  //     mark was forbidden from being. A reload comes first because
  //     opening a machine is what replaces the map with the card — the
  //     two share the space, so getting the map back means having
  //     nothing open.
  await page.reload();
  await openLanding("devices");
  await until("the map again", 15_000, async () => (await seats.count()) > 1);
  const guest = await seats.evaluateAll(
    (els) => els.find((e) => e.dataset.seatMe !== "true")?.dataset.seat ?? "",
  );
  if (!guest) throw new Error("probe dead: no seat that is not this machine");
  await page.locator(`[data-seat="${guest}"]`).click();
  await until("the card the seat opened", 10_000, async () =>
    (await page.locator("[data-machine-id]").textContent().catch(() => "")) === guest,
  );
  if ((await page.locator("[data-mandala-map]").count()) !== 0) {
    throw new Error("the map is still on screen beside the card it opened");
  }
  await page.reload();
  await openLanding("devices");

  // 25) the hook button is a pair, and the button *is* the report.
  //
  //     The state it edits lives in a file, so the CLI is an independent
  //     witness: `khor hooks` reading beta's own home says the same
  //     thing the button does, before and after each press. Both faces,
  //     one fact — and neither word is spelled here, they are read off
  //     the two and required to move together.
  //
  //     **It writes into beta's smoke home, never the developer's.**
  //     `adaptor::vendor_home` roots at KHOR_HOME, which this run set,
  //     so `~/.claude` is not reachable from here at all.
  const hookWords = () =>
    cli(envB, "hooks")
      .split("\n")
      .filter((l) => l.includes("\t"))
      .map((l) => l.split("\t")[1]);
  const hookToggle = page.locator("[data-hooks-toggle]");

  //     The control: this control belongs to this machine, so alpha's
  //     card must not carry it. Read first, because "absent on alpha"
  //     would also pass if the selector named nothing anywhere.
  await openLanding("devices");
  await page.locator('[data-device="beta"] [data-row-open]').click();
  await until("the hook button on this machine's card", 10_000, async () =>
    (await hookToggle.count()) === 1,
  );
  await page.locator('[data-device="alpha"] [data-row-open]').click();
  await until("alpha's card", 10_000, async () =>
    (await page.locator("[data-machine-id]").count()) === 1,
  );
  if ((await hookToggle.count()) !== 0) {
    throw new Error("another machine's card offers to edit this machine's hooks");
  }
  await page.locator('[data-device="beta"] [data-row-open]').click();
  await until("back on this machine's card", 10_000, async () => (await hookToggle.count()) === 1);

  //     Not installed to begin with, said by both faces.
  const offWord = await hookToggle.innerText();
  const offCli = hookWords();
  if (offCli.length === 0) throw new Error("probe dead: `khor hooks` listed no events");
  if (await page.locator('[data-hooks][data-installed="true"]').count()) {
    throw new Error("hooks report themselves installed in a home that has never had any");
  }

  //     Pressed the way a person presses it.
  await hookToggle.click();
  await until("the hooks to go on", 10_000, async () =>
    (await page.locator('[data-hooks][data-installed="true"]').count()) === 1,
  );
  const onWord = await hookToggle.innerText();
  if (!onWord || onWord === offWord) {
    throw new Error(`the button kept its old name after installing: ${onWord}`);
  }
  const onCli = hookWords();
  if (onCli.some((w) => offCli.includes(w))) {
    throw new Error(`the CLI says the same thing before and after installing: ${onCli}`);
  }
  if (new Set(onCli).size !== 1) {
    throw new Error(`installing left the events disagreeing: ${onCli}`);
  }

  //     …and back off again, all the way to the words it started with.
  //     A one-way button would pass everything above.
  await hookToggle.click();
  await until("the hooks to come back off", 10_000, async () =>
    (await page.locator('[data-hooks][data-installed="false"]').count()) === 1,
  );
  if ((await hookToggle.innerText()) !== offWord) {
    throw new Error("the button did not go back to offering the install");
  }
  if (hookWords().join(",") !== offCli.join(",")) {
    throw new Error(`the CLI does not agree the hooks came out: ${hookWords()}`);
  }

  // 24t) a session khor hosts here paints a live terminal.
  //
  //      Open a `cat` on beta (detached), attach in the app, type, and
  //      see the bytes come back on the screen — the whole
  //      PTY→vt100→grid path over the real bridge. `cat` because a tty
  //      echoes it, so the marker is exact and not a prompt's guesswork.
  //      Closed at the end so its detached host does not outlive the run.
  //      Before #26 because that one ends the bridge.
  const catId = cli(envB, "open", "-d", "--title", "termcat", "--", "cat").trim();
  await openLanding("sessions");
  await until("the hosted cat row", 10_000, async () =>
    (await page.locator(`[data-row="${catId}"]`).count()) === 1,
  );
  await page.locator(`[data-row="${catId}"] [data-row-open]`).click();
  await until("the terminal attached", 10_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  // Type inside the wait, re-focusing each round: React's dev double-mount
  // briefly tears the attachment down and back up (chat's hold-count, and
  // the same window), and a keystroke that lands in that gap is dropped.
  // A real user types once the pane has settled; the smoke retries until a
  // keystroke sticks. `cat` echoes each attempt, so one landing paints it.
  const marker = "marker9";
  await until("the typed line painted by the terminal", 15_000, async () => {
    await page.locator("[data-terminal]").focus();
    await page.keyboard.type(marker);
    await page.keyboard.press("Enter");
    await new Promise((r) => setTimeout(r, 300));
    return (await page.locator("[data-terminal]").innerText()).includes(marker);
  });
  // The typed line also becomes the row's preview — the host derives its
  // last non-empty terminal line (last.txt, throttled) and the list
  // carries it as `last`. Cat echoes the marker, so the preview holds it.
  await until("the cat row previewing the typed line", 20_000, async () => {
    const t = await page
      .locator(`[data-row="${catId}"] [data-last]`)
      .innerText()
      .catch(() => "");
    return t.includes(marker);
  });
  cli(envB, "close", catId);
  // Wait for the closed row to leave the list before the next item, which
  // pins the first row — a half-closed cat at the top would be pinned into
  // a session that is going away.
  await until("the closed cat row gone", 10_000, async () =>
    (await page.locator(`[data-row="${catId}"]`).count()) === 0,
  );
  await openLanding("sessions");

  // 24v) an agent session shows two faces: the conversation by default
  //      (会话身份批 ruling), the terminal on the switch — and the
  //      choice is remembered per row.
  //
  //      A hosted tui (cat again), then claude's own hook naming it —
  //      which is what records the vendor leaf — and a transcript file
  //      under that vendor id. The row turns claude; **its preview turns
  //      with it** (the transcript outranks the hosted screen line —
  //      the line a real agent shows there is its chrome, "⏵⏵ bypass
  //      permissions on…"); the detail opens on the chat face reading
  //      the recorded words through the vendor-leaf bridge; the terminal
  //      face is one click away and is the face the row reopens on.
  const agentId = cli(envB, "open", "-d", "--tui", "--title", "fakeagent", "--", "cat").trim();
  const agentUuid = "b7e2fa10-1234-4cd9-9c33-aabbccddeeff";
  execFileSync(KHOR, ["state", "--hook"], {
    env: { ...envB, KHOR_SESSION: agentId },
    input: JSON.stringify({ session_id: agentUuid, cwd: "/tmp/proj", hook_event_name: "SessionStart" }),
    encoding: "utf8",
  });
  const recorded = "the recorded words of the fake agent";
  const tdir = join(B, ".claude", "projects", "p");
  mkdirSync(tdir, { recursive: true });
  writeFileSync(
    join(tdir, `${agentUuid}.jsonl`),
    JSON.stringify({ type: "user", message: { role: "user", content: recorded } }) + "\n",
  );
  await until("the hosted agent row wearing claude", 20_000, async () =>
    (await page.locator(`[data-row="${agentId}"] [data-kind-mark="claude"]`).count()) === 1,
  );
  await until("the agent row previewing the transcript, not the screen", 20_000, async () => {
    const t = await page
      .locator(`[data-row="${agentId}"] [data-last]`)
      .innerText()
      .catch(() => "");
    return t.includes(recorded);
  });
  await page.locator(`[data-row="${agentId}"] [data-row-open]`).click();
  await until("the view switch on an agent detail", 15_000, async () =>
    (await page.locator("[data-view-chat]").count()) === 1,
  );
  if ((await page.locator("[data-terminal]").count()) !== 0) {
    throw new Error("the default face of an agent is its conversation (会话身份批)");
  }
  await until("the recorded conversation on the default face", 15_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes(recorded);
  });
  await page.locator("[data-view-term]").click();
  await until("the terminal face on the switch", 15_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  // The choice is remembered per row: visit *another* row's detail (so
  // this one truly unmounts — a same-row re-click would keep the mounted
  // state and pass without any memory), then come back and land on the
  // terminal without another click.
  await page.locator(`[data-row="chat/alpha"] [data-row-open]`).click();
  await until("another detail in between", 10_000, async () =>
    (await page.locator("[data-terminal]").count()) === 0,
  );
  await page.locator(`[data-row="${agentId}"] [data-row-open]`).click();
  await until("the remembered face is the terminal", 15_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  cli(envB, "close", agentId);
  await until("the agent row gone", 10_000, async () =>
    (await page.locator(`[data-row="${agentId}"]`).count()) === 0,
  );

  // 24m) a discovered tmux session opens as a live terminal, and closing
  //      khor's view leaves the user's session standing.
  //
  //      A session on beta's private server prints a marker; the sweep
  //      lists it (nobody opened it through khor), clicking it makes the
  //      bridge stand a grouped client up under the same id, and the
  //      marker is on the painted screen. The control is the whole
  //      point: `khor close` must kill only khor's client — the session
  //      itself survives on the server, and its discovered row comes
  //      back.
  execFileSync("tmux", ["-L", TMUX_SOCK, "new-session", "-d", "-s", "bridgeme", "-x", "80", "-y", "24"], { timeout: 10_000 });
  execFileSync("tmux", ["-L", TMUX_SOCK, "send-keys", "-t", "bridgeme", "printf 'tmux-bridge-marker\\n'", "Enter"], { timeout: 10_000 });
  await openLanding("sessions");
  await until("the discovered tmux row", 20_000, async () =>
    (await page.locator('[data-title="bridgeme"]').count()) === 1,
  );
  const tmuxRowId = await page.locator('[data-title="bridgeme"]').getAttribute("data-row");
  await page.locator(`[data-row="${tmuxRowId}"] [data-row-open]`).click();
  await until("the tmux screen painted in the app", 20_000, async () => {
    const t = await page.locator("[data-terminal]").innerText().catch(() => "");
    return t.includes("tmux-bridge-marker");
  });
  cli(envB, "close", tmuxRowId);
  // The user's session survives khor's close — only the grouped client
  // died. `has-session` exits non-zero if it were gone.
  execFileSync("tmux", ["-L", TMUX_SOCK, "has-session", "-t", "bridgeme"], { timeout: 5_000 });
  // …and the discovered row returns once the registry entry is gone.
  await until("the discovered row back after close", 20_000, async () =>
    (await page.locator('[data-title="bridgeme"]').count()) === 1,
  );
  execFileSync("tmux", ["-L", TMUX_SOCK, "kill-session", "-t", "bridgeme"], { timeout: 5_000 });
  // …and wait for the killed session's row to leave the list before the
  // next item picks "the first row" — same trap the cat close hit.
  await until("the killed tmux row gone", 20_000, async () =>
    (await page.locator('[data-title="bridgeme"]').count()) === 0,
  );


  // 24g) an agent inside a tmux session is one row — the agent's — and
  //      its terminal face bridges into the pane it sits in (会话身份批:
  //      真实 session 一行到底; tmux 是路线, 不是身份).
  //
  //      A binary *named* claude (cat copied under the name — the sweep
  //      matches process names exactly) idles in a session on the
  //      private server; beta's vendor home gets claude's own status
  //      file naming that pid, plus a transcript. One row appears and it
  //      is the agent's — the tmux session's name is on no row. Its
  //      preview is the transcript, its default face the conversation,
  //      and the terminal face reaches the very pane: typing echoes,
  //      because the pane runs cat.
  const fakeClaudeDir = join(tmpdir(), `khor-smoke-claude-${process.pid}`);
  mkdirSync(fakeClaudeDir, { recursive: true });
  const fakeClaude = join(fakeClaudeDir, "claude");
  copyFileSync("/bin/cat", fakeClaude);
  chmodSync(fakeClaude, 0o755);
  execFileSync("tmux", ["-L", TMUX_SOCK, "new-session", "-d", "-s", "agenthome", "-x", "80", "-y", "24", fakeClaude], { timeout: 10_000 });
  const agentPid = execFileSync("tmux", ["-L", TMUX_SOCK, "list-panes", "-t", "agenthome", "-F", "#{pane_pid}"], { encoding: "utf8", timeout: 10_000 }).trim();
  const tmuxAgentUuid = "c9d4ab21-4321-4abc-9def-001122334455";
  const tmuxAgentRow = "tui/c9d4ab21-4321-4abc-9def";
  const sdir = join(B, ".claude", "sessions");
  mkdirSync(sdir, { recursive: true });
  writeFileSync(
    join(sdir, `${agentPid}.json`),
    JSON.stringify({
      pid: Number(agentPid), sessionId: tmuxAgentUuid, cwd: "/tmp/proj",
      startedAt: Date.now() - 1000, name: "tmuxagent", status: "busy", statusUpdatedAt: Date.now(),
    }),
  );
  const inTmux = "the words of the agent living in tmux";
  const agentCwd = join(SCRATCH, "agenthome-cwd");
  mkdirSync(agentCwd, { recursive: true });
  // `cwd` on the line the way real transcripts carry it — the takeover
  // resumes the agent in its recorded world, not in the resumer's.
  writeFileSync(
    join(tdir, `${tmuxAgentUuid}.jsonl`),
    JSON.stringify({ type: "user", cwd: agentCwd, message: { role: "user", content: inTmux } }) + "\n",
  );
  await openLanding("sessions");
  await until("the tmux-held agent row wearing claude", 20_000, async () =>
    (await page.locator(`[data-row="${tmuxAgentRow}"] [data-kind-mark="claude"]`).count()) === 1,
  );
  if ((await page.locator('[data-title="agenthome"]').count()) !== 0) {
    throw new Error("the tmux session holding the agent must not be a second row");
  }
  await until("the held agent previewing its transcript, not its screen", 20_000, async () => {
    const t = await page
      .locator(`[data-row="${tmuxAgentRow}"] [data-last]`)
      .innerText()
      .catch(() => "");
    return t.includes(inTmux);
  });
  await page.locator(`[data-row="${tmuxAgentRow}"] [data-row-open]`).click();
  await until("the conversation as the default face", 15_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes(inTmux) && (await page.locator("[data-terminal]").count()) === 0;
  });
  await page.locator("[data-view-term]").click();
  await until("the pane's terminal painted", 20_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  const paneMarker = "pane-marker4";
  await until("a keystroke reaching the real pane", 20_000, async () => {
    await page.locator("[data-terminal]").focus();
    await page.keyboard.type(paneMarker);
    await page.keyboard.press("Enter");
    await new Promise((r) => setTimeout(r, 300));
    return (await page.locator("[data-terminal]").innerText()).includes(paneMarker);
  });
  // 24m) bracketed paste (批④ 前置): a paste is sent as a paste, and the
  //      wrapping is the running program's choice rather than this app's.
  //
  //      **Both halves, on two real terminals.** One runs `cat -v` bare;
  //      the other turns DECSET 2004 on first. `cat -v` is what makes
  //      the answer readable — it prints the escape rather than acting
  //      on it, so the marks are visible as `^[[200~` instead of being
  //      swallowed by khor's own parser on the way back.
  //
  //      Without the negative half this would pass on an app that
  //      wrapped everything, which is the failure that matters: a
  //      program that never asked would be handed an escape it does not
  //      know and would paste it as text.
  //      `cat` is the reader on purpose: it does nothing to its input,
  //      and the tty's own echo prints ESC as `^[`, so the marks show up
  //      as text. A shell would be the wrong witness — readline
  //      *understands* bracketed paste and strips the marks, so the
  //      screen would look identical either way.
  //
  //      **Which program is running is checked, not assumed.** The whole
  //      command goes to tmux as one string (it runs it through a
  //      shell); passing it as separate arguments left a plain shell
  //      running instead, and then the negative half could not fail
  //      because the shell was swallowing the very marks it was there
  //      to notice. Measured — a planted "always wrap" break went green.
  const pasteText = "paste-marker-24m";
  for (const [name, cmd] of [
    ["pasteplain", "cat"],
    ["pastebrack", "printf '\\033[?2004h'; cat"],
  ]) {
    execFileSync(
      "tmux",
      ["-L", TMUX_SOCK, "new-session", "-d", "-s", name, "-x", "80", "-y", "24", cmd],
      { timeout: 10_000 },
    );
    await until(`${name} running cat`, 10_000, () => {
      const now = execFileSync(
        "tmux",
        ["-L", TMUX_SOCK, "list-panes", "-t", name, "-F", "#{pane_current_command}"],
        { encoding: "utf8", timeout: 5_000 },
      ).trim();
      if (now !== "cat") throw new Error(`probe dead: ${name} is running ${now}, not cat`);
      return true;
    });
  }
  await openLanding("sessions");
  const pasteInto = async (title) => {
    await until(`the ${title} row`, 25_000, async () =>
      (await page.locator(`[data-title="${title}"] [data-row-open]`).count()) === 1,
    );
    await page.locator(`[data-title="${title}"] [data-row-open]`).click();
    await until(`${title}'s terminal`, 20_000, async () =>
      (await page.locator("[data-terminal]").count()) === 1,
    );
    // Dispatched as a real paste event: the pane's own handler is what
    // decides this is a paste rather than typing, and that is the thing
    // under test.
    await page.locator("[data-terminal]").evaluate((el, text) => {
      const data = new DataTransfer();
      data.setData("text", text);
      el.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true }));
    }, pasteText);
    await until(`${title} echoing the paste`, 20_000, async () =>
      (await page.locator("[data-terminal]").innerText()).includes(pasteText),
    );
    return page.locator("[data-terminal]").innerText();
  };
  //      **This half is NOT red-proven, and that is worth knowing before
  //      trusting it.** A planted "always wrap" break left it green: the
  //      marks do not become visible on this pane in this rig even when
  //      they are sent, so the check cannot see the difference it is
  //      written to see. It is kept because the property is real and the
  //      line costs nothing — but it is not coverage, and whoever makes
  //      it observable should red-prove it then.
  const plainEcho = await pasteInto("pasteplain");
  if (plainEcho.includes("[200~")) {
    throw new Error(`a program that never asked for bracketed paste was sent the marks: ${plainEcho.slice(-120)}`);
  }
  const brackEcho = await pasteInto("pastebrack");
  if (!brackEcho.includes("[200~") || !brackEcho.includes("[201~")) {
    throw new Error(
      `a program that asked for bracketed paste did not get the marks: ${brackEcho.slice(-120)}`,
    );
  }
  // 24n) 拖文件进终端 (批④ 二笔): the path arrives quoted.
  //
  //      **The OS drag itself is not driven here and cannot be.** tauri
  //      takes the drop before the webview sees it, so the paths come on
  //      a tauri event that does not exist in this browser at all — the
  //      gesture is a user-acceptance item. Everything on both sides of
  //      it is automated: the quoting has its own Rust tests against a
  //      real `sh`, and the op the gesture calls is driven here for
  //      real, into a real terminal.
  const dropped = ["/tmp/a b.txt", "/tmp/it's.txt"];
  await page.locator(`[data-title="pasteplain"] [data-row-open]`).click();
  await until("pasteplain's terminal again", 20_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  await page.evaluate(
    async ([port, id, paths]) => {
      const r = await fetch(`http://127.0.0.1:${port}/term_drop`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id, paths }),
      });
      if (!r.ok) throw new Error(await r.text());
    },
    [BRIDGE_PORT, await page.locator('[data-title="pasteplain"]').getAttribute("data-row"), dropped],
  );
  //      A space must not split the path into two arguments, and an
  //      apostrophe must not end the quoting early — the two ways a
  //      dropped filename turns into something else.
  await until("the dropped paths arriving quoted", 20_000, async () => {
    const seen = await page.locator("[data-terminal]").innerText();
    return seen.includes("'/tmp/a b.txt'") && seen.includes("'/tmp/it'\\''s.txt'");
  });

  // 24p) 没人看的时候不轮询 (批⑤): a window in the background costs
  //      almost nothing, and comes back current at once.
  //
  //      **Headless Chrome is never hidden**, measured two ways before
  //      settling for a double: a second tab brought to the front leaves
  //      the first one `visible`, and CDP's `Page.setWebLifecycleState`
  //      set to `frozen` does not move it either. So the state is set
  //      the only way left — override the property, fire the real event.
  //
  //      That double covers the whole of what this app owns: the
  //      listener, the pace each poller reads, and the kick on the way
  //      back. What it does not cover is whether a real browser fires
  //      `visibilitychange` when its window goes behind another — which
  //      is the browser's contract, not this app's, and there is nothing
  //      in a headless run that could check it either way.
  //
  //      Counted at the wire rather than by reading a number out of the
  //      app: what is being claimed is that the *calls stop*, and the
  //      calls are the only honest evidence of that.
  const bridgeHits = [];
  const countHits = (r) => {
    if (r.url().includes(`127.0.0.1:${BRIDGE_PORT}`)) bridgeHits.push(Date.now());
  };
  page.on("request", countHits);
  const hitsOver = async (ms) => {
    const from = bridgeHits.length;
    await new Promise((r) => setTimeout(r, ms));
    return bridgeHits.length - from;
  };
  const setHidden = (on) =>
    page.evaluate((hide) => {
      Object.defineProperty(document, "hidden", { value: hide, configurable: true });
      Object.defineProperty(document, "visibilityState", {
        value: hide ? "hidden" : "visible",
        configurable: true,
      });
      document.dispatchEvent(new Event("visibilitychange"));
    }, on);

  //      A terminal is mounted from 24n, so this window is polling at
  //      its fastest — which is what makes the drop below mean anything.
  const busyHits = await hitsOver(2_000);
  if (busyHits < 20) {
    throw new Error(`probe dead: a watched window with a terminal made only ${busyHits} calls in 2s`);
  }
  await setHidden(true);
  if (!(await page.evaluate(() => document.hidden))) {
    throw new Error("probe dead: the page did not take the hidden state at all");
  }
  const quietHits = await hitsOver(3_000);
  //      At the hidden beat the heartbeat's three calls and the
  //      terminal's one land at most once each in this window.
  if (quietHits > 5) {
    throw new Error(`a window nobody is looking at made ${quietHits} calls in 3s (was ${busyHits} in 2s)`);
  }
  //      **And it comes back current, not on the next beat.** Ten
  //      seconds of a stale screen on return is its own failure — the
  //      one the kick exists for, and the half a pause alone would not
  //      have.
  const hitsWhileAway = bridgeHits.length;
  await setHidden(false);
  await until("a fresh answer the moment the window is looked at again", 2_000, async () =>
    bridgeHits.length > hitsWhileAway,
  );
  page.off("request", countHits);

  // 24o) 拖文件到**另一台机器**的终端 (批④ 四笔): the files travel, and
  //      what gets pasted is where they came to rest over there.
  //
  //      **The path is the whole point, and it is the one thing that
  //      cannot be worked out on this side.** beta knows the file as
  //      `<scratch>/drop-me.txt`; typing that into a shell running on
  //      alpha names nothing. So the drop sends the file, has alpha
  //      take it in, and pastes the path alpha answered with — which
  //      is checked here against alpha's actual disk, not against a
  //      path this script also computed.
  //
  //      alpha hosts the session for real (`khor open -d`, a live `cat`
  //      that echoes whatever is pasted into it) and beta's GUI reaches
  //      it the way any far terminal is reached. Nothing here is a
  //      stand-in but the OS drag itself, which 24n explains.
  const farId = cli(envA, "open", "-d", "--title", "farbox", "--", "cat").trim();
  await until("alpha's own session in beta's list", 90_000, async () =>
    (await page.locator(`[data-row="${farId}"]`).count()) === 1,
  );
  await page.locator(`[data-row="${farId}"] [data-row-open]`).click();
  await until("a terminal reached on alpha", 30_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  const dropSrc = join(SCRATCH, "drop-me.txt");
  const dropBody = `farbox-24o-${process.pid}`;
  writeFileSync(dropSrc, dropBody);
  const farFiles = join(A, ".khor/chat/alpha/files");
  await page.evaluate(
    async ([port, id, paths]) => {
      const r = await fetch(`http://127.0.0.1:${port}/term_drop`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id, paths }),
      });
      if (!r.ok) throw new Error(await r.text());
    },
    [BRIDGE_PORT, farId, [dropSrc]],
  );
  //      ① The bytes really crossed. Read off alpha's disk, so this
  //      says the far machine pulled rather than that this one asked.
  const landedName = await until("the file itself on alpha", 60_000, () => {
    const here = existsSync(farFiles) ? readdirSync(farFiles) : [];
    return here.find((f) => f.endsWith("-drop-me.txt") && !f.startsWith("."));
  });
  const landedPath = join(farFiles, landedName);
  if (readFileSync(landedPath, "utf8") !== dropBody) {
    throw new Error(`alpha's copy is not the file that was sent: ${landedPath}`);
  }
  //      ② The paste names *that* file. Whitespace is stripped from
  //      both sides because an 80-column terminal wraps a long path
  //      across rows, and a line break inside a path is the screen's
  //      doing, not the paste's.
  //
  //      The negative half rides with the positive one on purpose: if
  //      the paste never arrived at all, "beta's own path is absent"
  //      would pass while proving nothing (账本: 否定式断言之前先证明
  //      测法还活着). Here the same read has to show alpha's path
  //      present, so an empty screen fails the pair.
  const flat = (t) => t.replace(/\s+/g, "");
  await until("alpha's own path pasted into alpha's terminal", 30_000, async () => {
    const seen = flat(await page.locator("[data-terminal]").innerText());
    if (seen.includes(flat(dropSrc))) {
      throw new Error("the terminal was handed beta's path — that file does not exist on alpha");
    }
    return seen.includes(flat(landedPath));
  });
  //      The session on alpha ends here; the `cat` inside it is a real
  //      process on a real machine and nothing else in this run would
  //      come back for it.
  cli(envA, "close", farId);

  // Put the pane back where 24k expects to find it: this block borrowed
  // the detail to look at two other terminals.
  await page.locator(`[data-row="${tmuxAgentRow}"] [data-row-open]`).click();
  await until("the held agent's pane again", 20_000, async () =>
    (await page.locator("[data-view-chat]").count()) === 1,
  );

  // 24k) 接管 (批C): the read-only face offers it, the confirm warns,
  //      and the go moves the conversation's body — the TUI process
  //      dies (the pane WAS the process, so the tmux session goes with
  //      it), the same row is reborn as a live conversation under the
  //      same vendor uuid, and speaking works: the resumed fake claude
  //      answers. This also ends the viewer host stood up above — the
  //      takeover cleans the bridge before it re-seats the row.
  await page.locator("[data-view-chat]").click();
  await until("the readonly face offering a takeover", 15_000, async () =>
    (await page.locator("[data-takeover]").count()) === 1,
  );
  await page.locator("[data-takeover]").click();
  await until("the confirm with its warning", 10_000, async () =>
    (await page.locator("[data-takeover-warn]").count()) === 1,
  );
  await page.locator("[data-takeover-go]").click();
  await until("the tmux side ended by the takeover", 30_000, async () => {
    try {
      execFileSync("tmux", ["-L", TMUX_SOCK, "has-session", "-t", "agenthome"], {
        timeout: 5_000,
        stdio: "ignore",
      });
      return false;
    } catch {
      return true;
    }
  });
  await until("the same row reborn as a live conversation", 30_000, async () =>
    (await page.locator("[data-chat-input]").count()) === 1,
  );
  await say(page, "hello taken");
  // The pane's own text rides the timeout: "last: false" says only that
  // the assertion never came true, and what the pane *did* hold is the
  // whole difference between "the line never went out" and "the agent
  // answered something else".
  let afterTakeover = "";
  await until("the resumed agent answering from the same id", 20_000, async () => {
    afterTakeover = await page
      .locator("[data-detail-header]")
      .locator("..")
      .innerText()
      .catch(() => "");
    return afterTakeover.includes("echo: hello taken");
  }).catch(async (e) => {
    // The pane says what was painted; the frame log says what arrived.
    // Between them there is no room left for a guess about which half
    // of the chain stopped.
    const frames = await page
      .evaluate(
        async ([port, id]) => {
          const r = await fetch(`http://127.0.0.1:${port}/chat_poll`, {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ id, since: 0 }),
          });
          return await r.text();
        },
        [BRIDGE_PORT, tmuxAgentRow],
      )
      .catch((x) => `frames unreadable: ${x}`);
    throw new Error(
      `${e.message} — the pane held: ${JSON.stringify(afterTakeover.slice(-300))} — frames: ${String(frames).slice(0, 1200)}`,
    );
  });
  rmSync(join(sdir, `${agentPid}.json`), { force: true });
  cli(envB, "close", tmuxAgentRow);
  await until("the taken-over row gone on close", 20_000, async () =>
    (await page.locator(`[data-row="${tmuxAgentRow}"]`).count()) === 0,
  );


  // 25c) the wizard (会话身份批B): a claude session born as a
  //      conversation in a chosen directory, spoken to through khor's
  //      own shim — the fake claude underneath, so the whole chain
  //      (wizard → gui host → _cagent → stream-json → frames → ask
  //      buttons) runs hermetically, no API in sight.
  const wizDir = join(SCRATCH, "wizard-cwd");
  mkdirSync(wizDir, { recursive: true });
  await openLanding("sessions");
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="new"]').click();
  await page.locator("[data-new-session-dir]").fill(wizDir);
  await page.locator("[data-new-session-name]").fill("wizard-one");
  // The conversation form is the default (the ruling); just create.
  await page.locator("[data-new-session-create]").click();
  await until("the wizard dialog closed on success", 40_000, async () => {
    const err = await page.locator("[data-new-session-error]").count();
    if (err) throw new Error(await page.locator("[data-new-session-error]").innerText());
    return (await page.locator("[data-new-session-dialog]").count()) === 0;
  });
  // **A session khor opened itself knows whose it is.** The wizard's
  // 智能体 field is the user's own answer, so the row wears the vendor
  // mark from birth — where before it was the one kind of row that
  // could not say, and waited on a hook that may never be installed.
  await until("the wizard's row wearing its vendor", 20_000, async () =>
    (await page.locator('[data-title="wizard-one"] [data-kind-mark="claude"]').count()) === 1,
  );
  // The fresh conversation is selected and live: speak into it.
  await until("the fresh conversation with an input", 20_000, async () =>
    (await page.locator("[data-chat-input]").count()) === 1,
  );
  await say(page, "hello wizard");
  await until("the reply streamed through the shim", 20_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes("echo: hello wizard");
  });
  // Markdown is the agent's medium, and the fake echoes what it was
  // told — so one line of it makes the round trip and must arrive
  // *rendered*. Two assertions, because one cannot tell the two ways
  // this breaks apart: nothing rendered, or everything rendered. The
  // user's own bubble keeps the source it was typed in (ChatView head:
  // an asterisk they meant as an asterisk).
  // ASCII on purpose: this is test data, not the thing under test
  // (the NEEDLE rule above).
  await say(page, "**bold** and *slant*");
  await until("the agent's markdown arriving rendered", 20_000, async () =>
    (await page.locator("[data-md] strong").count()) > 0,
  );
  const rendered = await page.locator("[data-md]").last().innerText();
  if (rendered.includes("**")) {
    throw new Error(`the agent's marks are still on screen: ${rendered}`);
  }
  const typed = await page.locator("[data-said]").last().innerText();
  if (!typed.includes("**bold**")) {
    throw new Error(`the user's own line must stay verbatim: ${typed}`);
  }
  // A link in an agent's text is a control that carries its address.
  // **Not clicked here**: pressing it hands the page to this machine's
  // browser, and a test must not open one on whoever ran it. What the
  // scheme whitelist refuses is asserted in Rust
  // (`khor_gui_core::web::open_link`), where nothing is launched.
  await say(page, "see [khor](https://example.com/x) here");
  await until("the agent's link wearing its address", 20_000, async () => {
    const links = page.locator('[data-md-link="https://example.com/x"]');
    return (await links.count()) === 1 && (await links.first().innerText()).trim() === "khor";
  });

  // **The agent's own commands, offered by name.** The list arrives on
  // the protocol (the shim turns claude's init frame into one
  // `available_commands_update`), so what is asserted here is the whole
  // chain: the fake's own word for its command, filtered by what was
  // typed, completed into the box, and reaching the agent as a line.
  await page.locator("[data-chat-input]").fill("/comp");
  await until("the agent's own command offered", 20_000, async () =>
    (await page.locator('[data-slash-item="compact"]').count()) === 1,
  );
  await page.locator("[data-chat-input]").press("Enter");
  await until("the command completed into the box, not sent", 10_000, async () => {
    const v = await page.locator("[data-chat-input]").inputValue();
    return v === "/compact " && (await page.locator("[data-slash-menu]").count()) === 0;
  });
  await page.locator("[data-chat-input]").press("Enter");
  await until("the command reaching the agent as a line", 20_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes("echo: /compact");
  });

  // **A turn that will not end has a way out that is not 关闭.** The
  // fake hangs until the stop reaches it as the control protocol's
  // interrupt; the box comes back and the conversation is still there,
  // which is the whole difference from closing the session.
  await say(page, "hang for a while");
  await until("the box closed and a way out beside it", 20_000, async () =>
    (await page.locator("[data-chat-stop]").count()) === 1,
  );
  //      **The frame count before the click is what makes the last two
  //      readings separable.** The backend's list is everything this
  //      conversation ever received — four turns by now — so "a turn
  //      frame is in there" is true whether or not this stop produced
  //      one. What answers the question is the tail after this instant.
  const hangRow = await page.locator('[data-title="wizard-one"]').getAttribute("data-row");
  const framesAtStop = (await frameKinds(page, hangRow).catch(() => [])).length;
  await page.locator("[data-chat-stop]").click();
  //      **This is the wait that flakes** (about two runs in seven), and
  //      the reading is attached here rather than chased with a loop of
  //      its own: the occurrence is already happening on its own in
  //      every ordinary run, so what was missing was never a reproducer,
  //      only a note of which link gave way. See `hangReading`.
  try {
    await until("the turn ended and the box back", 20_000, async () =>
      (await page.locator("[data-chat-stop]").count()) === 0,
    );
  } catch (e) {
    throw new Error(
      `${e.message}\n${await hangReading(page, wizDir, "wizard-one", framesAtStop)}`,
    );
  }
  // The proof that the *turn* ended and not the session: it takes
  // another line. A pane that merely repainted would fail here.
  await say(page, "after the stop");
  await until("the same conversation answering after a stop", 20_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes("echo: after the stop");
  });

  // 25e) 对话面基础 (批①). Four things, all on the live conversation
  //      above — the same fake claude, the same real bridge:
  //      a) the box composes: Shift+Enter breaks the line, Enter sends,
  //         the box grows with what is in it and stops at the ceiling
  //         tokens.css sets, and an Enter that belongs to an IME's
  //         composition is not a send;
  //      b) a running turn takes the *sending*, not the typing: the box
  //         still accepts a draft, the send says so by being
  //         unpressable, a pressed Enter changes nothing, and the draft
  //         is still there — character for character — when the turn is
  //         over;
  //      c) a turn that has produced nothing yet says so in the
  //         conversation, wearing 忙碌's own paint and breath, and stays
  //         legible under prefers-reduced-motion;
  //      d) leaving the tail offers the way back, says when something
  //         landed behind the reader's back, does not yank them, and
  //         goes to zero on arrival.
  const boxSel = "[data-chat-input]";
  const boxHeight = () => page.locator(boxSel).evaluate((el) => el.clientHeight);
  const saidCount = () => page.locator("[data-said]").count();

  // a) One line, then two. The height is the assertion that it is the
  //    *box* that took the break: a textarea that ignored the newline
  //    would hold the same string on one row.
  await page.locator(boxSel).fill("");
  const oneRow = await boxHeight();
  const beforeBreak = await saidCount();
  await page.locator(boxSel).click();
  await page.keyboard.type("first row");
  await page.keyboard.press("Shift+Enter");
  await page.keyboard.type("second row");
  const twoRowText = await page.locator(boxSel).inputValue();
  if (!twoRowText.includes("\n")) {
    throw new Error(`Shift+Enter must break the line: ${JSON.stringify(twoRowText)}`);
  }
  if ((await saidCount()) !== beforeBreak) {
    throw new Error("Shift+Enter must not send");
  }
  const twoRows = await boxHeight();
  if (!(twoRows > oneRow)) {
    throw new Error(`the box must grow with what is typed: ${oneRow} -> ${twoRows}`);
  }
  // …and stops. The ceiling is read off the token, never written here:
  // a number copied into this file would keep passing after the token
  // moved.
  const ceiling = await page.evaluate(() =>
    parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--chat-box")),
  );
  await page.locator(boxSel).fill(Array.from({ length: 40 }, (_, i) => `row ${i}`).join("\n"));
  const capped = await boxHeight();
  if (!(capped > twoRows)) {
    throw new Error(`forty rows must be taller than two: ${twoRows} -> ${capped}`);
  }
  if (!(capped <= ceiling)) {
    throw new Error(`the box must stop at --chat-box (${ceiling}), it reached ${capped}`);
  }

  // The composing Enter belongs to the IME picking a candidate, not to
  // this box. Asserted through a dispatched keydown carrying
  // `isComposing` — and the *same* dispatch is then proven to send when
  // it is not composing, because a negative assertion whose probe was
  // never shown alive is only a spelling of "nothing happened".
  const composed = "composing row";
  await page.locator(boxSel).fill(composed);
  const beforeIme = await saidCount();
  const fireEnter = (composing) =>
    page.locator(boxSel).evaluate((el, isComposing) => {
      el.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, isComposing }));
    }, composing);
  await fireEnter(true);
  await new Promise((r) => setTimeout(r, 500));
  if ((await saidCount()) !== beforeIme) {
    throw new Error("an Enter raised mid-composition must not send");
  }
  if ((await page.locator(boxSel).inputValue()) !== composed) {
    throw new Error("an Enter raised mid-composition must leave the box alone");
  }
  await fireEnter(false);
  await until("the same dispatch sending once it is not composing", 20_000, async () =>
    (await saidCount()) > beforeIme,
  );
  await until("the composed line reaching the agent", 20_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes(`echo: ${composed}`);
  });

  // b) + c) on one hanging turn: the fake answers nothing until the
  //    stop reaches it, which is exactly the window both judgments are
  //    about.
  await say(page, "hang for a while");
  await until("the turn running", 20_000, async () =>
    (await page.locator("[data-chat-stop]").count()) === 1,
  );
  const draft = "written while it was busy";
  await page.locator(boxSel).fill(draft);
  if ((await page.locator(boxSel).inputValue()) !== draft) {
    throw new Error("the box must still take text while a turn runs");
  }
  if (!(await page.locator("[data-chat-send]").isDisabled())) {
    throw new Error("a turn must make the send visibly unavailable, not silently inert");
  }
  const midTurn = await saidCount();
  await page.locator(boxSel).press("Enter");
  await new Promise((r) => setTimeout(r, 600));
  if ((await saidCount()) !== midTurn) {
    throw new Error("Enter must not send while the turn refuses it");
  }
  if ((await page.locator(boxSel).inputValue()) !== draft) {
    throw new Error("a refused line must stay in the box, not be swallowed");
  }

  // c) The waiting mark, in 忙碌's own clothes. The colour is compared
  //    against a probe wearing the token — neither the hue nor the word
  //    is written in this file.
  await until("the waiting mark while the turn has said nothing", 15_000, async () =>
    (await page.locator("[data-chat-thinking]").count()) === 1,
  );
  const breathing = await page.locator("[data-chat-thinking] [data-word-text]").evaluate((el) => {
    const probe = document.createElement("span");
    probe.style.color = "var(--state-busy)";
    document.body.appendChild(probe);
    const want = getComputedStyle(probe).color;
    probe.remove();
    const s = getComputedStyle(el);
    return { color: s.color, want, animation: s.animationName, text: el.textContent };
  });
  if (breathing.color !== breathing.want) {
    throw new Error(`the waiting mark must take busy's paint: ${breathing.color} vs ${breathing.want}`);
  }
  if (breathing.animation === "none") {
    throw new Error("busy breathes — the waiting mark must carry the same keyframe");
  }
  // The word is the state machine's own, not a seventh one invented for
  // this pane: what it says must be what a 忙碌 *row* says. Read off a
  // row rather than off anything in this pane — the mark wears the same
  // `data-word` attribute, so a selector that could match itself would
  // compare the thing to itself and pass on any word at all.
  // …and the detail header says the same thing (批②). **Both read in
  // one call, and that is not tidiness.** These were two reads a moment
  // apart, and the moment was inside a window that closes on its own:
  // when the hanging turn ended between them the header had already
  // moved to 完成 while the row word in hand still said 忙碌, and the
  // run failed reporting a disagreement that never existed at any
  // single instant (账本: 要比较的数字必须一次调用取完).
  //
  // **Retried rather than snapshot, and still not vacuous.** The loop
  // insists the pair agree *and* that the row is the busy one, so a
  // window that has already closed keeps retrying and times out saying
  // so — it cannot pass by finding 完成 on both sides. That matters:
  // the first draft of the header half asserted on a quiet row, where a
  // header hard-coded to 空闲 agreed with it and passed, measured on a
  // break planted to make it fail. A word that only ever matches the
  // default is not a word being read off the row.
  const busyPair = await until(
    "the row and the header saying the same busy word inside one hanging turn",
    20_000,
    async () => {
      const got = await page.evaluate(() => {
        const row = document.querySelector('[data-row][data-word="busy"] [data-word-text]');
        const head = document.querySelector("[data-detail-header] [data-word-text]");
        return {
          row: row?.textContent?.trim() ?? "",
          head: head?.textContent?.trim() ?? "",
        };
      });
      return got.row && got.row === got.head ? got : null;
    },
  );
  const busyWord = busyPair.row;
  if ((breathing.text ?? "").trim() !== busyWord) {
    throw new Error(`the waiting mark must say what a busy row says: ${breathing.text} vs ${busyWord}`);
  }
  await page.emulateMedia({ reducedMotion: "reduce" });
  const stilled = await page.locator("[data-chat-thinking] [data-word-text]").evaluate((el) => {
    const s = getComputedStyle(el);
    return { animation: s.animationName, opacity: s.opacity, w: el.getBoundingClientRect().width };
  });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  if (stilled.animation !== "none") {
    throw new Error("reduced motion must stop the breath");
  }
  if (Number(stilled.opacity) !== 1 || stilled.w === 0) {
    throw new Error(`stopped is not the same as gone: ${JSON.stringify(stilled)}`);
  }

  await page.locator("[data-chat-stop]").click();
  await until("the turn ended", 20_000, async () =>
    (await page.locator("[data-chat-stop]").count()) === 0,
  );
  if ((await page.locator(boxSel).inputValue()) !== draft) {
    throw new Error("the draft must outlive the turn character for character");
  }
  await until("the waiting mark gone with the turn", 10_000, async () =>
    (await page.locator("[data-chat-thinking]").count()) === 0,
  );
  // And it was a draft, not a corpse: it sends.
  await page.locator(boxSel).press("Enter");
  await until("the draft going out once the turn was over", 20_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes(`echo: ${draft}`);
  });

  // d) The tail. First give the conversation a past worth scrolling
  //    into — with the *pane* asked whether it overflows, since a
  //    scroll assertion on a pane that fits is vacuous and green.
  const roll = page.locator("[data-chat-scroll]");
  const fits = () => roll.evaluate((el) => el.scrollHeight <= el.clientHeight + 40);
  for (let i = 0; i < 12 && (await fits()); i += 1) {
    const filler = `filler ${i} ${"x".repeat(200)}`;
    await say(page, filler);
    await until(`filler ${i} answered`, 20_000, async () => {
      const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
      return t.includes(`echo: ${filler}`);
    });
  }
  if (await fits()) {
    throw new Error("the conversation never outgrew its pane — every assertion below would be vacuous");
  }
  if ((await page.locator("[data-chat-bottom]").count()) !== 0) {
    throw new Error("at the tail there is nothing to go back to");
  }
  // Let the stream settle, or a frame landing between the scroll and
  // the read would make "nothing has arrived" a lie about timing.
  await new Promise((r) => setTimeout(r, 1_500));
  await roll.evaluate((el) => {
    el.scrollTop = 0;
  });
  await until("the way back to the tail, once away from it", 10_000, async () =>
    (await page.locator("[data-chat-bottom]").count()) === 1,
  );
  const wordAway = await page.locator("[data-chat-bottom]").innerText();
  if ((await page.locator("[data-chat-bottom]").getAttribute("data-fresh")) !== "false") {
    throw new Error("leaving the tail is the reader's own doing — it is not news");
  }
  await say(page, "arrived while away");
  await until("the control saying something landed", 20_000, async () =>
    (await page.locator('[data-chat-bottom][data-fresh="true"]').count()) === 1,
  );
  const wordFresh = await page.locator("[data-chat-bottom]").innerText();
  if (wordFresh.trim() === wordAway.trim()) {
    throw new Error(`the two reasons to press it must not read the same: ${wordFresh}`);
  }
  // …and the reader is still standing where they were: the whole point
  // of the control is that arriving text does not move anybody.
  const stoodAt = await roll.evaluate((el) => el.scrollTop);
  if (stoodAt > 20) {
    throw new Error(`new frames must not yank a reader who scrolled up (scrollTop ${stoodAt})`);
  }
  await page.locator("[data-chat-bottom]").click();
  await until("the way back gone once it is back", 10_000, async () =>
    (await page.locator("[data-chat-bottom]").count()) === 0,
  );

  // 25f) 变速轮询 (批① d): the pane asks fast while a turn runs and
  //      slows back down when it ends. **Counted off the network, not
  //      off a flag the pane sets about itself**: the polls are real
  //      requests, and a pane that only claimed to have changed pace
  //      would pass an attribute check and fail this one.
  const pollsInASecond = async () => {
    await page.evaluate(() => performance.clearResourceTimings());
    await new Promise((r) => setTimeout(r, 1_000));
    return page.evaluate(
      () =>
        performance
          .getEntriesByType("resource")
          .filter((e) => e.name.includes("/chat_poll")).length,
    );
  };
  await say(page, "hang for a while");
  await until("a turn to be fast for", 20_000, async () =>
    (await page.locator("[data-chat-stop]").count()) === 1,
  );
  const fastPolls = await pollsInASecond();
  await page.locator("[data-chat-stop]").click();
  await until("the turn over", 20_000, async () =>
    (await page.locator("[data-chat-stop]").count()) === 0,
  );
  // One extra beat: the wait already in flight when the turn ended was
  // scheduled at the fast length, so the first second after is a
  // mixture. What is asserted is the pace it settles at.
  await new Promise((r) => setTimeout(r, 1_000));
  const calmPolls = await pollsInASecond();
  if (fastPolls < 10) {
    throw new Error(`a running turn must be followed closely: ${fastPolls} polls in a second`);
  }
  if (calmPolls > 4) {
    throw new Error(`the pace must fall back when the turn ends: ${calmPolls} polls in a second`);
  }
  // …and it must still be polling. Zero would also satisfy the line
  // above, and would mean a conversation that never notices anything
  // again — the failure this cadence could most easily hide.
  if (calmPolls === 0) {
    throw new Error("falling back is not stopping: an idle pane still asks");
  }

  // 25g) 增量 fold 的两条边 (批① d). The picture is no longer refolded
  //      from the whole conversation, so the two things the old fold got
  //      right by construction have to be asserted:
  //
  //      the said line stays where it was said — between the answer
  //      before it and the answer to it — rather than collecting at one
  //      end, which is what an append that ignored order would look
  //      like on a screen and never on a type.
  const chatText = () =>
    page.locator("[data-chat-scroll]").evaluate((el) => el.textContent ?? "");
  await say(page, "anchor one");
  await until("the first anchor answered", 20_000, async () =>
    (await chatText()).includes("echo: anchor one"),
  );
  await say(page, "anchor two");
  await until("the second anchor answered", 20_000, async () =>
    (await chatText()).includes("echo: anchor two"),
  );
  const anchored = await chatText();
  const order = ["anchor one", "echo: anchor one", "anchor two", "echo: anchor two"].map((s) =>
    anchored.indexOf(s),
  );
  if (order.some((i) => i < 0)) {
    throw new Error(`probe dead: an anchor is missing — ${JSON.stringify(order)}`);
  }
  for (let i = 1; i < order.length; i += 1) {
    if (order[i] <= order[i - 1]) {
      throw new Error(`a said line must stay where it was said: ${JSON.stringify(order)}`);
    }
  }

  //      …and a replay replaces the past rather than stacking onto it.
  //      Asked for twice through the real op, because that is what a
  //      reconnect does. The first one must visibly paint something —
  //      otherwise the second's "nothing changed" is the answer a dead
  //      probe gives.
  const replayRow = await page.locator('[data-title="wizard-one"]').getAttribute("data-row");
  if (!replayRow) throw new Error("probe dead: the wizard's row has no id to replay");
  const replay = () =>
    page.evaluate(
      async ([port, id]) => {
        const r = await fetch(`http://127.0.0.1:${port}/chat_replay`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ id }),
        });
        if (!r.ok) throw new Error(await r.text());
      },
      [BRIDGE_PORT, replayRow],
    );
  // The needle is the one line the fake writes into claude's own
  // transcript on its way up, and `session/load` is where the host gets
  // a past from (`gui_host` `replay_to`) — so counting it counts copies
  // of the replayed past and nothing else.
  const RECORDED = "the fake's own past";
  const copiesOfThePast = async () => (await chatText()).split(RECORDED).length - 1;
  // The past is already on screen: this pane asked for a replay when it
  // attached, and that is the first copy. Asserted rather than assumed,
  // because everything below is about that number not moving, and a
  // number that started at zero would never move either.
  const copies = await copiesOfThePast();
  if (copies !== 1) {
    throw new Error(`probe dead: the recorded past should be painted once, it is ${copies}`);
  }
  // Now ask again, the way a reconnect does. Several times, and the
  // count is checked after each: a replay can legitimately arrive with
  // no history at all (`replay_to` drains the agent's load with
  // `try_recv`, so one answered a moment later sends only `HistoryEnd`)
  // and that empties the past — the old fold did the same, its "last
  // complete replay" being an empty one. An empty arrival says nothing
  // about stacking, so what is required is that at least one of these
  // landed, and that none of them ever made a second copy.
  let landedAgain = false;
  let most = copies;
  for (let i = 0; i < 4; i += 1) {
    await replay();
    await new Promise((r) => setTimeout(r, 1_500));
    const now = await copiesOfThePast();
    most = Math.max(most, now);
    if (now >= 1) landedAgain = true;
    if (now > 1) break;
  }
  if (!landedAgain) {
    throw new Error("probe dead: every replay arrived empty, so stacking was never tested");
  }
  if (most > 1) {
    throw new Error(
      `history is a state, not a stream: replaying again left ${most} copies of the past`,
    );
  }

  // The permission round-trip, on the screen's own buttons, in khor's
  // catalog words.
  await say(page, "please ask-permission");
  await until("the ask surfacing with an allow", 20_000, async () =>
    (await page.locator('[data-ask-option="allow"]').count()) === 1,
  );
  // The row wears claude's own uuid (the merge story), and close ends
  // host and agent together.
  const wizId = await page.locator('[data-title="wizard-one"]').getAttribute("data-row");
  if (!wizId || !wizId.startsWith("tui/")) {
    throw new Error(`the wizard row must wear the vendor uuid: ${wizId}`);
  }
  // **A face that was not there when the ask was raised must still be
  // able to answer it.** Leaving the row and coming back is a whole
  // fresh attachment — the pane detaches on unmount, and the host's
  // stream only ever carried "from now on". Without the re-send, this
  // comes back to a conversation whose row says 待批 and whose pane
  // offers nothing to press: a dead end, reachable by opening the
  // session in a second window.
  const elsewhere = (
    await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row))
  ).find((r) => r !== wizId);
  await page.locator(`[data-row="${elsewhere}"]`).click();
  await until("the conversation actually left", 10_000, async () =>
    (await page.locator("[data-chat-ask]").count()) === 0,
  );
  await page.locator(`[data-row="${wizId}"]`).click();
  await until("the ask still answerable from a face that just arrived", 20_000, async () =>
    (await page.locator('[data-ask-option="allow"]').count()) === 1,
  );
  await page.locator('[data-ask-option="allow"]').click();
  // The decision replaces the buttons and stays as the record. This
  // can only come from the host's own `Answered` frame: the clicking
  // face's local memory hides the buttons but paints no word, so a
  // pane that lost the answer would show an empty line here.
  await until("the answered ask wearing what was decided", 20_000, async () => {
    if (await page.locator('[data-ask-option="allow"]').count()) return false;
    return (await page.locator("[data-ask-answer]").count()) === 1;
  });
  await until("the allow reaching the fake claude", 20_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes("verdict:allow");
  });
  // 25d) 接管, the other way (唯一本体 read backwards): the terminal
  //      face of a conversation khor holds is not a terminal — it is
  //      where the conversation moves into one. The row keeps its id
  //      and its past; what changes hands is the form.
  await page.locator("[data-view-term]").click();
  await until("the terminal face offering the move", 15_000, async () =>
    (await page.locator("[data-term-is-a-chat]").count()) === 1 &&
    (await page.locator("[data-takeover-term]").count()) === 1,
  );
  await page.locator("[data-takeover-term]").click();
  await until("the confirm saying which form ends", 10_000, async () =>
    (await page.locator("[data-takeover-term-warn]").count()) === 1,
  );
  await page.locator("[data-takeover-term-go]").click();
  await until("the same row now holding a terminal", 40_000, async () => {
    if (await page.locator("[data-takeover-term-error]").count()) {
      throw new Error(await page.locator("[data-takeover-term-error]").innerText());
    }
    return (await page.locator("[data-terminal]").count()) === 1;
  });
  if ((await page.locator(`[data-row="${wizId}"]`).count()) !== 1) {
    throw new Error("the moved conversation must stay one row, under its own id");
  }
  // Its past came along: the conversation face now reads the vendor's
  // own transcript, and the line the agent wrote on the way up is in it.
  await page.locator("[data-view-chat]").click();
  await until("the conversation surviving the move", 20_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes("the fake's own past");
  });

  // 25h) 详情头的事实 + 关会话入口 (批②). The header says what the row
  //      is, and offers the two things a person does with a session
  //      they are looking at: take its name, or end it.
  //
  //      The state word is asserted up in 25e instead, in the window
  //      where a turn is hanging and the word is provably 忙碌 — here
  //      every row is resting, and a header that always said 空闲 would
  //      pass. That is not a hypothetical: it was this block's first
  //      draft, and planting exactly that break left the run green.
  //
  //      The machine is there only when the row carries one. Both
  //      halves, because "absent" is also what a broken selector says:
  //      this row lives here and names no machine, and a reported row
  //      elsewhere in this same list does.
  if ((await page.locator("[data-detail-device]").count()) !== 0) {
    throw new Error("a session living on this machine must not name one");
  }
  const fromElsewhere = (
    await page.locator("[data-row][data-source]").evaluateAll((els) => els.map((e) => e.dataset.row))
  )[0];
  if (!fromElsewhere) throw new Error("probe dead: no reported row to check the other half with");
  await page.locator(`[data-row="${fromElsewhere}"] [data-row-open]`).click();
  await until("the reported row naming its machine", 15_000, async () =>
    (await page.locator("[data-detail-device]").count()) === 1,
  );
  //      …and wearing that machine's face — **the same picture the row
  //      wears**, compared rather than merely counted: one machine, one
  //      face, wherever it appears.
  const headerFace = page.locator("[data-detail-machine] [data-face]");
  if ((await headerFace.count()) !== 1) throw new Error("probe dead: no face on the detail header");
  if (
    (await faceOf(headerFace)) !==
    (await faceOf(page.locator(`[data-row="${fromElsewhere}"] [data-face]`).first()))
  ) {
    throw new Error("the header and the row paint one session's machine two different ways");
  }
  //      The confirm says what *this kind* of close does. Read off two
  //      rows of different kinds and compared to each other — neither
  //      sentence is spelled here, and a single generic warning would
  //      make these two equal.
  const warningFor = async (rowId) => {
    await page.locator(`[data-row="${rowId}"] [data-row-open]`).click();
    await until("a detail with a close", 15_000, async () =>
      (await page.locator("[data-close-session]").count()) === 1,
    );
    await page.locator("[data-close-session]").click();
    await until("its confirm", 10_000, async () =>
      (await page.locator("[data-close-warn]").count()) === 1,
    );
    const said = (await page.locator("[data-close-warn]").innerText()).trim();
    await page.locator("[data-close-back]").click();
    return said;
  };
  const kindOf = (rowId) =>
    page.locator(`[data-row="${rowId}"]`).evaluate((el) => el.dataset.row.split("/")[0]);
  // **A different kind is not the same thing as a different close.**
  // `shell`, `tui`, `gui` and `borrow` are all live sessions — closing
  // any of them stops a process, so they share a sentence *correctly*,
  // and an assertion that demanded two prefixes differ would call that
  // a bug. What differs is the three `KindSurface::close` bodies, so the
  // comparison is against a device chat, whose close deletes received
  // files instead. Measured: the first spelling of this compared `tui`
  // with `shell` the moment a plain tmux session appeared in the list,
  // and failed on behaviour that is right.
  const otherKind = (
    await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row))
  ).find((r) => r.startsWith("chat/"));
  if (!otherKind) {
    throw new Error("probe dead: no device chat in the list, so the split cannot be seen");
  }
  const [warnWizard, warnOther] = [await warningFor(wizId), await warningFor(otherKind)];
  if (!warnWizard || !warnOther) throw new Error("a confirm must say something");
  if (warnWizard === warnOther) {
    throw new Error(
      `one sentence for two kinds: ${await kindOf(wizId)} and ${await kindOf(otherKind)} both say ` +
        `${JSON.stringify(warnWizard)} — the confirm must say what this close does`,
    );
  }
  // **A close that fails is asserted where a failure is certain**, which
  // is section 26 with the backend taken away — not here on a row from
  // another machine. That was this block's first draft and the premise
  // was wrong: `close_anywhere` *routes* to the machine a session lives
  // on, so closing a reported row is an ordinary close that works, and
  // the assertion sat waiting for a refusal that was never coming.

  //      Back on the wizard's own row: the id is takeable, and the
  //      close asks first.
  await page.locator(`[data-row="${wizId}"] [data-row-open]`).click();
  await until("the wizard row again", 15_000, async () =>
    (await page.locator("[data-copy-id]").count()) === 1,
  );
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  const beforeCopy = await page.locator("[data-copy-id]").innerText();
  await page.locator("[data-copy-id]").click();
  await until("the copy saying it happened", 10_000, async () => {
    const now = await page.locator("[data-copy-id]").innerText();
    return now !== beforeCopy;
  });
  // What actually landed on the clipboard, not what the button claims.
  const pasted = await page.evaluate(() => navigator.clipboard.readText());
  if (pasted !== wizId) {
    throw new Error(`the copy must put the row's own id there: ${JSON.stringify(pasted)}`);
  }
  // The confirm is a real gate: backing out leaves the session running.
  await page.locator("[data-close-session]").click();
  await until("the confirm on the wizard row", 10_000, async () =>
    (await page.locator("[data-close-confirm]").count()) === 1,
  );
  await page.locator("[data-close-back]").click();
  await until("the confirm dismissed", 10_000, async () =>
    (await page.locator("[data-close-confirm]").count()) === 0,
  );
  await settle();
  if ((await page.locator(`[data-row="${wizId}"]`).count()) !== 1) {
    throw new Error("backing out of the confirm must not close the session");
  }
  // …and going through with it ends the session for real — the same
  // ending `khor close` produces, which is what this used to call.
  //
  // **Done on the narrow face on purpose.** Wide, the detail empties by
  // itself the moment the row leaves the list, so nothing here would
  // notice a shell that kept pointing at the dead session. Narrow is
  // one screen at a time: staying on a detail screen with nothing in it
  // and no list behind it is a dead end a person reaches by pressing
  // 关闭, and it is the only face on which that can be seen.
  await page.setViewportSize({ width: 390, height: 720 });
  await until("the narrow detail of the row about to be closed", 10_000, async () =>
    (await page.locator("[data-back]").count()) === 1,
  );
  await page.locator("[data-close-session]").click();
  await until("the confirm again", 10_000, async () =>
    (await page.locator("[data-close-confirm]").count()) === 1,
  );
  await page.locator("[data-close-go]").click();
  await until("the wizard row gone", 20_000, async () => {
    if (await page.locator("[data-close-error]").count()) {
      throw new Error(await page.locator("[data-close-error]").innerText());
    }
    return (await page.locator(`[data-row="${wizId}"]`).count()) === 0;
  });
  //      …and the shell lets go of it: back on the list, with no detail
  //      screen left standing over a session that no longer exists.
  await until("the narrow shell back on the list", 15_000, async () =>
    (await page.locator("[data-list]").count()) === 1 &&
    (await page.locator("[data-back]").count()) === 0,
  );
  //      The pane's controls went with the row, too — a detail still
  //      offering 关闭 and 复制 id for a dead session is a face pointing
  //      at nothing.
  if (
    (await page.locator("[data-close-session]").count()) !== 0 ||
    (await page.locator("[data-copy-id]").count()) !== 0
  ) {
    throw new Error("the detail still offers actions for a session that is gone");
  }
  await page.setViewportSize({ width: 1080, height: 720 });
  await until("back on the wide face", 10_000, async () =>
    (await page.locator("[data-row]").count()) > 0,
  );

  // **A terminal-form session is named before it exists.** The row
  // wears the vendor's own uuid — 26 characters of it, the lossy
  // spelling every agent row uses — rather than a khor-minted leaf
  // that names no vendor file. Without this, the one row khor could
  // not take over was the one khor opened itself.
  const termDir = join(SCRATCH, "wizard-term-cwd");
  mkdirSync(termDir, { recursive: true });
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="new"]').click();
  await page.locator("[data-new-session-dir]").fill(termDir);
  await page.locator("[data-new-session-name]").fill("wizard-term");
  await page.locator("[data-new-session-term]").click();
  await page.locator("[data-new-session-create]").click();
  await until("the terminal-form row born under a vendor uuid", 40_000, async () => {
    const err = await page.locator("[data-new-session-error]").count();
    if (err) throw new Error(await page.locator("[data-new-session-error]").innerText());
    const row = await page.locator('[data-title="wizard-term"]').getAttribute("data-row").catch(() => null);
    // 8-4-4-4 of a uuid is 26 characters, and a khor-minted leaf is not
    // shaped like that at all (`link::fresh_leaf`).
    return !!row && /^tui\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}$/.test(row);
  });
  const termRow = await page.locator('[data-title="wizard-term"]').getAttribute("data-row");
  cli(envB, "close", termRow);
  // Waited for, not fired and forgotten: a row on its way out is still
  // the newest row, and the step after this one picks the first one it
  // sees and presses its pin — which never takes on a row that is
  // being removed. (Measured: that is exactly how it failed.)
  await until("the terminal-form row gone", 20_000, async () =>
    (await page.locator(`[data-row="${termRow}"]`).count()) === 0,
  );


  // 25j) 右下角状态栏 (批④ 三笔): what is happening that nobody is
  //      watching, and nothing else.
  //
  //      Driven by a **real transfer** — alpha sends beta a file over
  //      the real link, beta accepts, and the row walks 待批 → 完成. The
  //      strip is fed off those rows, so this also proves the two
  //      admission rules against something that actually happened
  //      rather than against a fixture.
  await openLanding("sessions");
  if ((await page.locator("[data-statusbar]").count()) !== 0) {
    throw new Error("probe dead: the corner is already occupied before anything happened");
  }
  const sentFile = join(SCRATCH, "corner-note.txt");
  writeFileSync(sentFile, "a file worth a line in the corner\n");
  //      **The id comes from the verb that made it, not from scanning
  //      the list for a transfer.** The list holds more than one by now
  //      — 24o's drop put one there — and "the first row whose id starts
  //      with transfer/" quietly picked that older, already finished one
  //      instead. Accepting it moved nothing, no word changed, and the
  //      corner stayed empty: a failure that reads exactly like the
  //      feature being broken. (Measured: that is how this section first
  //      went red after 24o was added.)
  //      `\S+` and not a hex pattern: a transfer id is
  //      `transfer/<peer>-<ms>-<seq>`, and a character class that
  //      stopped at the first dash produced a real-looking id for a row
  //      that does not exist — which times out reading exactly like the
  //      sync being slow. `transfer/` is a code constant
  //      (`TransferKind::session_id`), so this matches on khor's own
  //      shape rather than on a sentence that gets translated.
  const transferRow = cli(envA, "send", "beta", sentFile).match(/transfer\/\S+/)?.[0];
  if (!transferRow) throw new Error("khor send did not name the transfer it made");
  await until("the transfer row reaching beta", 30_000, async () =>
    (await page.locator(`[data-row="${transferRow}"]`).count()) === 1,
  );
  //      **A row that was already there does not announce itself.** The
  //      strip has now seen this row sitting at 待批 and stayed empty —
  //      which is the half that keeps it from being full at startup.
  await settle();
  if ((await page.locator("[data-statusbar]").count()) !== 0) {
    throw new Error("a transfer nobody touched put itself in the corner");
  }
  //      Now it moves, and the corner says so.
  cli(envB, "accept", transferRow);
  await until("the corner reporting the transfer", 30_000, async () =>
    (await page.locator(`[data-status-item="${transferRow}"]`).count()) === 1,
  );
  //      Every line in there is a process-class fact with a row behind
  //      it — never a click's outcome. Checked as an invariant rather
  //      than by trying to push a click result in, because the rule is
  //      "only these get in", not "that one is kept out".
  const inCorner = await page
    .locator("[data-status-item]")
    .evaluateAll((els) => els.map((e) => e.dataset.statusItem));
  if (inCorner.some((k) => !k.startsWith("transfer/"))) {
    throw new Error(`the corner took something that is not a process: ${JSON.stringify(inCorner)}`);
  }
  //      …and it goes to zero on its own. **Absent, not empty**: the
  //      whole strip stops rendering, so there is no box left saying
  //      nothing.
  await until("the corner emptying itself", 20_000, async () =>
    (await page.locator("[data-statusbar]").count()) === 0,
  );

  // 26) a pin that does not take says so, on the button that was
  //     pressed.
  //
  //     **A real failure, through the real path.** The backend is taken
  //     away and the pin is pressed in the app the way a person presses
  //     it; nothing is stubbed and no call is made behind the UI's back.
  //     This is last because it ends the bridge.
  //
  //     The control comes first: the same button, pressed while the
  //     backend is there, must come back *not* wearing the face. A mark
  //     that is always on is not a failure report.
  const failTarget = (await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row)))[0];
  const failPin = page.locator(`[data-row="${failTarget}"] [data-row-pin]`);
  // **The colour is read after the button's own transitions finish**,
  // not the instant the attribute flips. The button carries
  // `transition-colors`, and a reading taken at t=0 of a transition is
  // bit-for-bit the colour it is *leaving* — which is what one observed
  // failure here reported, with the failure attribute already set and
  // the built stylesheet putting the failure rule last. Waiting on the
  // element's own animations is the "wait for the thing itself" rule; a
  // fixed sleep would measure how busy the machine is, and two equal
  // consecutive reads can both land before the transition starts moving.
  const pinState = () =>
    failPin.evaluate(async (el) => {
      await Promise.all(el.getAnimations().map((a) => a.finished.catch(() => {})));
      return {
        failed: el.dataset.pinFailed,
        name: el.getAttribute("aria-label"),
        color: getComputedStyle(el).color,
      };
    });

  await failPin.click();
  await until("the pin to take while the backend is there", 10_000, async () =>
    (await page.locator(`[data-row="${failTarget}"][data-pinned=true]`).count()) === 1,
  );
  const okState = await pinState();
  if (okState.failed !== "false") {
    throw new Error(`a pin that worked is wearing the failure face: ${JSON.stringify(okState)}`);
  }

  // What the failure colour actually computes to, measured off a probe
  // wearing the token rather than written down here — a hex compared
  // against a browser's `rgb()` never matches, and hard-coding either
  // side would stop tracking the theme.
  const failedColor = await page.evaluate(() => {
    const el = document.createElement("span");
    el.style.color = "var(--state-failed)";
    document.body.appendChild(el);
    const c = getComputedStyle(el).color;
    el.remove();
    return c;
  });

  process.kill(-bridge.pid, "SIGKILL");
  await until("the bridge to be gone", 15_000, async () => {
    const r = await fetch(`http://127.0.0.1:${BRIDGE_PORT}/devices`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: "{}",
    }).catch(() => null);
    return r === null;
  });

  await failPin.click();
  await until("the failure face on the button that was pressed", 10_000, async () =>
    (await pinState()).failed === "true",
  );
  const badState = await pinState();
  // The name changes with it, so the report is not colour alone —
  // compared against the working button's name rather than spelled
  // here, because the words belong to the catalog.
  if (!badState.name || badState.name === okState.name) {
    throw new Error(`the failed pin kept its old name: ${JSON.stringify(badState)}`);
  }
  if (badState.color !== failedColor) {
    throw new Error(
      `the failed pin is not wearing the failure colour: ${badState.color} vs ${failedColor}`,
    );
  }
  // …and it is that row's report, not the whole list going red.
  const others = await page
    .locator("[data-row-pin]")
    .evaluateAll((els) => els.filter((e) => e.dataset.pinFailed === "true").length);
  if (others !== 1) throw new Error(`${others} buttons wear the failure face; exactly one was pressed`);

  // 26b) 关会话失败也就地说 (批②). The same backend-away window, because
  //      this is the only place a close is *certain* to fail: whether a
  //      given row can be closed is `Node::close`'s judgment and the
  //      pane deliberately does not pre-empt it, so with the backend up
  //      there is no row this script can be sure will refuse.
  //
  //      The control is the strip's own absence beforehand: a report
  //      that was already on screen would prove nothing about the
  //      press.
  await page.locator(`[data-row="${failTarget}"] [data-row-open]`).click();
  await until("a detail to close from", 10_000, async () =>
    (await page.locator("[data-close-session]").count()) === 1,
  );
  if ((await page.locator("[data-close-error]").count()) !== 0) {
    throw new Error("a close was reported failed before one was asked for");
  }
  await page.locator("[data-close-session]").click();
  await until("the confirm", 10_000, async () =>
    (await page.locator("[data-close-confirm]").count()) === 1,
  );
  await page.locator("[data-close-go]").click();
  await until("the failure said on the pane that asked", 15_000, async () =>
    (await page.locator("[data-close-error]").count()) === 1,
  );
  if (!(await page.locator("[data-close-error]").innerText()).trim()) {
    throw new Error("the failure must be a sentence, not an empty strip");
  }
  // The confirm goes when it fails — leaving it up would read as the
  // close still being on offer, on a backend that cannot take it.
  if ((await page.locator("[data-close-confirm]").count()) !== 0) {
    throw new Error("a failed close must not leave its confirm standing");
  }

  // 27) the page never threw.
  if (pageErrors.length) throw new Error(`pageerror: ${pageErrors.join(" | ")}`);

  if (process.env.SMOKE_SHOT) {
    await page.setViewportSize({ width: 1080, height: 720 });
    // The rail clicks above leave the pointer on a glyph, and the rail
    // names appear on hover — park it off the rail so the shot shows
    // the resting screen rather than one mid-hover.
    await page.mouse.move(900, 600);
    await new Promise((r) => setTimeout(r, 600));
    await page.screenshot({ path: process.env.SMOKE_SHOT });
  }

  console.log("gui smoke: all green");
} finally {
  await cleanup();
}
