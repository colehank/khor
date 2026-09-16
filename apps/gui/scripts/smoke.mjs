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
//  12. all four landings open; the machine-first pane (browser) lists
//      the same machines the devices pane does — the same set, not
//      merely a list of the same length — and the files landing has no
//      machine column at all, asserted with the same locator that just
//      found machines twice elsewhere;
//  13. each pane's bar carries exactly what that pane can do, and the
//      two machine panes name their search box the same thing (a files
//      pane offering to search files would be naming something it
//      cannot find). Files has no pane bar to describe — its box
//      belongs to a tab (24c);
//  14. the way into the mark's place is at the foot of the rail, really
//      drew something (this machine's face), and **is** a control: a
//      rail button around it and a hover response — with a rail glyph
//      measured the same way as the control, since "nothing changed" is
//      also what a broken probe says.
//      **This line said the opposite of item 14 for some time**: it read
//      「it is not a control: no button around it and no hover response」
//      while the body below asserted the hover. The body was turned over
//      when the mark started opening the map; this summary was not, and
//      a summary that contradicts the thing it summarises is worse than
//      no summary — it is the one a reader trusts without scrolling;
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
//  29. the app's mark opens the mesh, on both faces — **the narrow one
//      is the only way in there at all** — the place replaces the
//      list-and-detail pair rather than filling it, a landing comes back
//      out of it, every face on it is pressable and opens that machine's
//      card, and the session count it prints is the one `khor sessions`
//      lists for the same home, with no number written in this file;
//  31. the workspace landing on the real mesh (工作区批⑥, and the first
//      time this file opens it at all): a workspace declared from
//      beta's face onto **alpha's** disk through the cross-machine
//      picker, arriving in alpha's own `khor ws ls` with nobody telling
//      it; its git corner empty because the directory is no checkout;
//      an mtime alpha answered, re-asked on the way back to the window
//      and changed by a write; the directory listed and walked; a file
//      deleted across the wire really gone from the disk, behind the
//      sheet that says 直接删 and offers no 「不再问」; the sessions
//      column holding the one started under that directory and not the
//      one started beside it; a 400-character line in a real file
//      folding into its column with the numbers still on their lines
//      (工作区批⑨); that column's own 「+」 opening the wizard already
//      answered with alpha and alpha's directory, and a session
//      finishing in that project re-asking git so the file it wrote
//      wears its letter with nobody touching the window (工作区批⑩);
//      and 移出 replicating back while the directory stays;
//  27. zero pageerror throughout.
// Every wait has a deadline; cleanup runs in finally and kills by pid.
//
// No Chinese literal appears below. Words are read off the running app
// (a rail item's aria-label, a row's data-word) and compared against
// what the other face prints — the catalog owns the text, this script
// owns the comparison.
//
// ── Writing a new item: four ways one was got wrong in one sitting ──
//
// All four are already in the repo's 假绿 family; they are written down
// here because this is the file where the next one will be made, and
// each of them **failed loudly only by luck** — three of the four would
// have passed as a green in a slightly different order.
//
//  1. **The list is not the empty state.** 24b waited for
//     `[data-new-tab]` before pinning anything — and a landing with
//     nothing kept paints `[data-no-places]` instead. The assertion was
//     about a thing that by definition did not exist yet. Wait for
//     *either* face of the surface, then assert the fact.
//  2. **Nothing pushes.** A backend op does not reach an open screen.
//     What the app claims is "the pin is the network's, so the next
//     screen that asks sees it" — so the reload is part of the claim,
//     not a way around a flake. Asserting an instant update asserts a
//     feature nobody built.
//  3. **A sibling is not an ancestor.** `[data-places-section] [data-place]`
//     matches nothing, because the heading is the row's sibling. A
//     locator that matches nothing reads exactly like "it never
//     arrived" — the `.nav-badge` lesson, which is in this file's own
//     history twice already.
//  4. **The view before the absence.** "The row is gone" is satisfied
//     by a surface that has not answered yet. Prove the surface drew,
//     *then* assert what is not on it. (This is the repo's own rule —
//     「否定式断言之前,先证明测法还活着」 — violated by the person
//     quoting it.)
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
  statSync,
  symlinkSync,
  utimesSync,
  realpathSync,
} from "node:fs";
import { join, dirname, relative } from "node:path";
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

/** Every `.rs` the two binaries above are actually built from.
    **Named roots, not the whole repo** — and the first spelling of this
    walked the whole repo while its own comment said otherwise. It then
    refused to run because `apps/gui/src-tauri` was newer, which is a
    third binary with its own build that `--bins` does not touch: a
    comment describing an intent the code did not implement, and the
    code was the wrong half.

    Walked rather than globbed because this script has no dependencies.
    `target/` never appears because neither root contains it. */
const RUST_ROOTS = ["crates", join("apps", "gui", "core")];
function rustSources(dir, out = []) {
  if (dir === undefined) {
    for (const r of RUST_ROOTS) rustSources(join(repo, r), out);
    return out;
  }
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.name === "target" || e.name === "node_modules" || e.name.startsWith(".")) continue;
    // **A crate's `tests/` is never linked into a binary**, so a file
    // there can never make one stale — and the guard below refuses to
    // run until the binary is newer than everything here. Editing an
    // integration test therefore used to wedge this script: the rebuild
    // it names finishes in a quarter of a second without touching the
    // binary, so the same refusal comes back for ever. Found by editing
    // `core/tests/kind_tables.rs` (工作区批⑤).
    if (e.name === "tests") continue;
    const full = join(dir, e.name);
    if (e.isDirectory()) rustSources(full, out);
    else if (e.name.endsWith(".rs")) out.push(full);
  }
  return out;
}
const SCRATCH = process.env.SMOKE_DIR ?? join(repo, "target/gui-smoke");
const RUN = join(SCRATCH, "run");
const KHOR = join(RUN, "khor");
const BRIDGE = join(RUN, "bridge");
const GUI_RUN = join(RUN, "gui");
/** What vite needs and nothing else — no `node_modules` (linked), no
    `dist`, no rust crates.

    **Nothing in `src` reaches outside it any more.** There was one
    entry here for the one import that did — the app's own mark, whose
    glyph lives with the tauri icons — and it went with the mark when
    the rail's door became this machine's face (2026-08-29). Removed
    rather than left as a harmless extra file: a path listed here is
    read as「something needs this」, and the next person tracing what
    escapes `src` would find a file nothing imports and go looking for
    the importer.

    **A new escaping import breaks this snapshot, and it says so.** The
    one that used to be listed was found exactly that way: the first
    assertion came back inside a minute with `the rail has nothing …
    console: 500`, and the dev server's log named the file and the line.
    That is the whole reason the rail check is first — and it is why
    this list stays a list rather than becoming `apps/gui`. */
const GUI_PARTS = ["index.html", "vite.config.ts", "tsconfig.json", "package.json", "src"];
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

// **A web port of its own, per home.** `khor serve` stands its web face
// up on `khor_web::DEFAULT_PORT` (5467) unless `KHOR_WEB_PORT` says
// otherwise, and on a developer's machine that port belongs to the
// resident the user is actually using. Without this the run's serve
// prints 「网页脸起不来,5467 口占着了」 — a real refusal, about a face
// nothing here asserts on, in the log a person reads when something
// else went red. It cost one investigation on 2026-09-06. Ports next to
// the run's other two, and a home apiece so the two serves do not take
// each other's.
let webPort = Number(process.env.WEB_PORT ?? 1444);
const homeEnv = (dir, name) => {
  const e = { ...process.env, KHOR_HOME: dir, KHOR_NAME: name, KHOR_WEB_PORT: String(webPort++) };
  delete e.KHOR_SESSION;
  return e;
};
// **The resident runs where the user's does: under launchd's
// environment.** `khor serve` on a real machine is a launchd agent, and
// what launchd hands it was read off the live process on 2026-09-01
// (`ps eww`): `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, `HOME`, `USER`,
// `LOGNAME`, `TMPDIR`, `SHELL=/bin/zsh` — and **no `LANG`, no `LC_*`,
// no `TERM`**. Five times that day something verified green from a
// terminal (npx, tmux, the locale, the unit separator, tmux again) was
// red on that road and nowhere else, because a smoke that starts serve
// from the developer's shell inherits the developer's PATH and locale
// and tests a binary the user never runs. So the resident and the
// bridge (which embeds one) get what launchd gives them, no more and no
// less: `SHELL` stays, because launchd keeps it and the resident's
// PATH repair asks that shell. The CLI calls keep the shell's
// environment, because that is what a person's terminal has.
//
// **A whitelist, not a blacklist.** The first version deleted `LANG`,
// `LC_*` and `TERM` and kept the rest — and the rest included `TMUX`,
// because the smoke was being run from inside a tmux pane; with it in
// the environment tmux printed the unit separator raw and the sweep
// parsed, while the launchd serve on the same machine, with no `TMUX`,
// got `_` and read nothing. A tear-down that should have reddened
// stayed green for exactly as long as that variable was inherited.
// What launchd gives is a short list; anything not on it is the
// developer's, and the developer's is what this must not test.
const launchdEnv = (e) => {
  const out = { PATH: "/usr/bin:/bin:/usr/sbin:/sbin" };
  for (const k of ["HOME", "USER", "LOGNAME", "TMPDIR", "SHELL", "SSH_AUTH_SOCK"]) {
    if (e[k] !== undefined) out[k] = e[k];
  }
  for (const k of Object.keys(e)) {
    if (k.startsWith("KHOR_") || k === "BRIDGE_PORT") out[k] = e[k];
  }
  return out;
};
// alpha gets the fake claude too: 25c-3 starts a conversation *on alpha*
// from beta's face, and what alpha's shim spawns must be the fake, not
// whatever `claude` is on this machine's disk.
// **alpha's opener is `/usr/bin/true`** (工作区第五轮 第 35 条). The
// verb runs somebody else's opener on their own machine, and on this
// mac that is `open` — a smoke run that put a window on the screen
// would be this file spending the operator's attention. Set here rather
// than per call because `khor launch` goes through the **resident
// serve** when there is one (`Node::launch` via_serve), and a serve's
// environment is the one it was started with: `launchdEnv` copies every
// `KHOR_*`, so this reaches it.
const envA = {
  ...homeEnv(A, "alpha"),
  KHOR_CLAUDE: join(SCRATCH, "claude-fake.py"),
  KHOR_LAUNCHER: "/usr/bin/true",
};
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
    // **And it must not be older than what it is meant to be testing.**
    // Existence was the whole check, and a binary from yesterday passes
    // it: a run against one cost an afternoon of debugging a feature
    // that was never in the thing being run — the new bridge command
    // simply did not exist, so the page reported "no record" and three
    // plausible causes were chased before the timestamps were read.
    //
    // Refused rather than rebuilt: building takes a minute and this
    // script starts servers and browsers, so a silent rebuild in the
    // middle of it is a surprise on a machine somebody else may be
    // compiling on. The sentence names the command.
    const sources = rustSources();
    const bin = statSync(built).mtimeMs;
    const newer = sources.filter((f) => statSync(f).mtimeMs > bin);
    if (newer.length > 0) {
      throw new Error(
        `${built} is older than ${newer.length} source file(s) — ` +
          `${relative(repo, newer[0])} and others. ` +
          `cargo build -p khor-cli -p khor-gui-core --bins`,
      );
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
    if m.get("type") == "control_request" and m.get("request", {}).get("subtype") == "set_permission_mode":
        # A mode change, the way the real claude takes one: answered by
        # a control_response naming the request. The mark is what the
        # smoke reads — this process's own account of the mode it was
        # put in, which no assertion on the screen can tell.
        with open("fake.mode", "w") as f:
            f.write(m["request"]["mode"])
        emit({"type": "control_response", "response": {"subtype": "success", "request_id": m["request_id"]}})
        continue
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
    if text.startswith("flood-"):
        # A burst bigger than the frame cap, in one turn and faster than
        # any poll: this is what makes the trim reachable at all (the
        # real shape of it is a streaming agent, one frame per chunk).
        for i in range(int(text.split("-")[1])):
            emit({"type": "assistant", "message": {"content": [{"type": "text", "text": f"flood {i}"}]}})
        emit({"type": "result", "subtype": "success", "session_id": sid})
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
  // home**, which is where its meter reads (`khor_node::usage`).
  //
  // **Kept after the GUI stopped drawing a daily ledger** (2026-08-28:
  // the mark's place shows the map and the subscription arcs, and what
  // was spent per day is `khor usage`'s to print). It stays because the
  // usage machinery still runs behind the app — the shell polls it, and
  // a home with no spending at all would exercise only the empty path.
  // Nothing below reads these numbers off a screen any more.
  //
  // ASCII throughout: this is test data, not the thing under test.
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
  // **alpha's day, planted before alpha wakes** (远端日条批), in the
  // record's own on-disk shape (`khor_node::day`): the minute the
  // window starts, 1440 watched flags, 1440 cells per row. Two things
  // are under test here. That alpha *loads* a record in this shape —
  // a hand-written file `Day::load` accepts is the file format holding
  // — and that whatever alpha's record holds reaches beta's screen
  // through `Request::Days`. The planted cell itself does not have to
  // survive: alpha samples the same row in the same minute and keeps
  // whichever word outranks (`Day::observe`), so by the time beta
  // reads it the cell may well be alpha's own.
  {
    const minute = Math.floor(Date.now() / 60_000);
    const from = minute - 1439;
    mkdirSync(join(A, ".khor"), { recursive: true });
    writeFileSync(
      join(A, ".khor", "day.json"),
      JSON.stringify({
        from,
        watched: "-".repeat(1439) + "+",
        rows: { "tui/cafe1": ".".repeat(1439) + "b" },
      }),
    );
  }
  run(KHOR, ["serve"], launchdEnv(envA), "serve-a");
  await until("alpha endpoint.json", 15_000, () => existsSync(join(A, ".khor/endpoint.json")));
  const ticketA = cli(envA, "invite").trim().split("\n").pop().trim();
  feedHook(envA, "SessionStart");
  feedHook(envA, "UserPromptSubmit");
  if (!cli(envA, "sessions").includes("tui/cafe1")) throw new Error("alpha row missing");

  // beta: the bridge is the app backend — embedded serve pumps sync and
  // holds this key's endpoint, which is what lets the GUI mint and
  // accept tickets without a terminal.
  // **The frame cap, turned down to where a test can reach it.** It is
  // an operations knob (`khor_gui_core::chat::frame_cap`), and this run
  // is its first customer: at the shipped twenty thousand the trim — and
  // the recovery that hangs off it — is code nothing ever executes. Two
  // hundred is still far above any ordinary exchange here, so only the
  // section that deliberately floods crosses it.
  const bridge = run(
    BRIDGE,
    [],
    launchdEnv({ ...envB, BRIDGE_PORT: String(BRIDGE_PORT), KHOR_CHAT_FRAME_CAP: "200" }),
    "bridge",
  );
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
  // Opening a landing, and waiting for the thing that landing *is*.
  //
  // **Three of the four are a list pane and one is not** (标签页一期):
  // the files landing has no machine column any more, so there is no
  // `[data-list]` there to compare a name against, and waiting for one
  // would time out on a landing that came up perfectly. Its own arrival
  // is asserted the same way in spirit — a region this landing and only
  // this landing paints.
  // Which browse shell a landing opens into, when it is one of the two
  // that browse. **Both of them, not just files** — the browser landing
  // stopped having a list pane in 06f5757 (浏览型外壳收成模板) and this
  // went on waiting for one, so every item that opened it timed out on a
  // pane that cannot exist.
  // **Only one landing browses now** (工作区替代文件, 2026-09-11): the
  // disk kind left the shell with the files landing, and what replaced
  // it — 工作区 — is a list pane with a tree under it, not a tab strip
  // over an address bar. A map with one entry is kept rather than
  // inlined because the shape is what says「a landing may or may not
  // browse」, and that is still true of every landing this walks.
  const BROWSE_KIND = { browser: "page" };
  const openLanding = async (tab) => {
    // Re-clicking the open landing now collapses its list (App.tsx 再点
    // 收起, 2026-09-02; the workspaces landing joined on 2026-09-16 —
    // 「再次点击侧边栏工作区icon, 是收起工作区」). This helper is「go
    // here」, so when tab is already open, hop through a sibling first:
    // the return is then a fresh switch, which always leaves the list
    // expanded, and nothing here ever lands on a collapsed pane. (31f
    // went red on exactly this: back to the workspaces landing from the
    // workspaces landing folded the list, and the row's click point was
    // then whatever had spread into its place.)
    if (
      (tab === "sessions" || tab === "devices" || tab === "workspaces") &&
      (await railItem(tab).getAttribute("data-on").catch(() => null)) === "true"
    ) {
      await railItem(tab === "sessions" ? "devices" : "sessions").click();
      await new Promise((r) => setTimeout(r, 300));
    }
    // On the narrow bar only the first four landings have a glyph; the
    // rest live behind「…」(App.tsx `NARROW_SHOWN`), so opening one of
    // those is two presses there. Wait for the rail to settle first —
    // `.count()` does not auto-wait, and reading it a tick before a
    // width change re-rendered the rail would send a wide open down the
    // narrow (fold) path or the reverse.
    await until("the rail to settle", 10_000, async () =>
      (await railItem(tab).count()) + (await page.locator("[data-rail-more]").count()) > 0,
    );
    if ((await railItem(tab).count()) === 0) {
      await page.locator("[data-rail-more]").click();
      await page.locator(`[data-more-landing="${tab}"]`).click();
    } else {
      await railItem(tab).click();
    }
    if (BROWSE_KIND[tab]) {
      await until(`the ${tab} landing`, 10_000, async () =>
        (await page.locator(`[data-browse-landing="${BROWSE_KIND[tab]}"]`).count()) === 1,
      );
      return;
    }
    await until(`the ${tab} pane`, 10_000, async () =>
      (await listPane.getAttribute("aria-label")) === (await railItem(tab).getAttribute("aria-label")),
    );
  };

  // The open machine card's full id.
  //
  // **Through the copy button, because the id is not written on the card
  // any more.** It stepped behind that button in `2ef4750` (the card grew
  // its verbs) and `data-machine-id` went with it — four assertions in
  // this file went on naming that attribute, and a selector that matches
  // nothing reads as "the card did not open" while the card is fine.
  // Three of them were positive and failed honestly; **one was negative
  // and had been passing for free ever since** (「没有 card 被打开」was
  // true of a probe that could never find one). What each of them claims
  // is unchanged — only the way of getting the id is.
  //
  // The clipboard is emptied first: it keeps whatever was put on it
  // last, so a click that did not take would otherwise hand back the
  // previous card's id and every comparison would still pass.
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  const cardId = async () => {
    await page.evaluate(() => navigator.clipboard.writeText(""));
    await page.locator("[data-machine-card] [data-copy-id]").click();
    return until("the card's id on the clipboard", 10_000, async () => {
      const got = await page.evaluate(() => navigator.clipboard.readText());
      return got && got.length > 0 ? got : false;
    });
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
    const missing = ["sessions", "devices", "workspaces", "browser"].filter((k) => !got.includes(k));
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
  //    alpha's own CLI. Two machines, two faces, one message. **Through
  //    the wizard** since 2026-09-06 — 留言 is its first kind, and the
  //    "+" menu's separate door went with the old dialog: pick alpha in
  //    在哪台, pick 留言 in 类型, and the third step is the line itself.
  await assertUntitled("sessions");
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="new"]').click();
  await until("the wizard offering alpha", 10_000, async () =>
    (await page.locator('[data-new-session-machine="alpha"]').count()) === 1,
  );
  await page.locator('[data-new-session-machine="alpha"]').click();
  await page.locator('[data-new-session-kind="tell"]').click();
  //     The kind step is a step: 名字 and 目录 are not asked for a line.
  if ((await page.locator("[data-new-session-name], [data-new-session-dir]").count()) !== 0) {
    throw new Error("留言 still asks for a name or a directory — a line has neither");
  }
  await page.locator("[data-tell-text]").fill(NEEDLE);
  await page.locator("[data-tell-send]").click();
  await until("the wizard to close on a sent line", 20_000, async () => {
    const err = await page.locator("[data-new-session-error]").count();
    if (err) throw new Error(await page.locator("[data-new-session-error]").innerText());
    return (await page.locator("[data-new-session-dialog]").count()) === 0;
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

  // 9b) three channels, three questions (批⑨): the halo says how it is,
  //     the corner mark says what it is, the word says it in letters.
  //     The mark used to say two of those at once, so asserting the
  //     split is what keeps it split.
  //
  //     **Every reading is taken against the row's own word**, never
  //     against a colour written in this file. A halo pinned to one
  //     colour satisfies any check that only asks "is there a halo", and
  //     "every row wears the same ring" is the exact shape that failure
  //     takes — so what is asserted is the *derivation*: this row's ring
  //     is built from this row's word.
  const channels = await sessionRows.evaluateAll((rows) =>
    rows.map((el) => {
      const halo = el.querySelector("[data-halo]");
      const mark = el.querySelector("[data-kind-mark]");
      return {
        word: el.getAttribute("data-word"),
        haloWord: halo ? halo.getAttribute("data-halo") : null,
        // What the halo actually resolved to, beside what this row's own
        // word resolves to. **Not the literal it was written as** —
        // `getComputedStyle` substitutes `var()` inside custom
        // properties, so a computed `--halo-color` comes back as a
        // colour and never as the text `var(--state-idle)`.
        from: halo ? getComputedStyle(halo).getPropertyValue("--halo-color").trim() : null,
        wants: getComputedStyle(document.documentElement)
          .getPropertyValue(`--state-${el.getAttribute("data-word")}`)
          .trim(),
        shadow: halo ? getComputedStyle(halo).boxShadow : null,
        markColor: mark ? getComputedStyle(mark).color : null,
        muted: getComputedStyle(document.documentElement).getPropertyValue("--muted-foreground").trim(),
      };
    }),
  );
  if (channels.length === 0) throw new Error("probe dead: no session rows to read channels off");
  for (const c of channels) {
    if (c.from === null) throw new Error(`the ${c.word} row wears no halo`);
    if (c.haloWord !== c.word) {
      throw new Error(`a row says ${c.word} at its end and ${c.haloWord} in its halo`);
    }
    if (c.from !== c.wants) {
      throw new Error(`the ${c.word} row's halo paints ${c.from}, but its own word is ${c.wants}`);
    }
    if (!c.shadow || c.shadow === "none") throw new Error(`the ${c.word} row's halo paints nothing`);
    // Neutral, and measured rather than assumed absent: the mark carried
    // an inline state colour until the halo landed, and a mark that went
    // back to it would still pass every count in this file.
    if (c.markColor !== c.muted) {
      throw new Error(`the ${c.word} row's kind mark paints ${c.markColor}, not the neutral ${c.muted}`);
    }
  }
  //     And the rings really are different where the words are: the
  //     check above would still pass if every `--state-*` resolved to
  //     one colour.
  const byWord = new Map(channels.map((c) => [c.word, c.shadow]));
  if (byWord.size > 1 && new Set(byWord.values()).size === 1) {
    throw new Error(`${byWord.size} different words all wear the same ring`);
  }

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
  //    **The reading stack left the rows on 2026-09-02** (「这个环很
  //    抽象,看不明白。取消」) — a machine row is the face, the name, and
  //    how the machine is reached. Both halves asserted: the stack has
  //    not come back, and the connection line really took its slot —
  //    the self row always has one, because a machine reaches itself
  //    over nothing and that fact has a word of its own
  //    (`machine-here`). The road word's agreement with the terminal
  //    is asserted in the card block below, where the CLI reader lives.
  if ((await page.locator("[data-device] [data-vitals-stack]").count()) !== 0) {
    throw new Error("the reading stack is back on a machine row");
  }
  const selfConnection = page.locator('[data-device="beta"] [data-row-connection]');
  if ((await selfConnection.count()) !== 1 || !(await selfConnection.innerText()).trim()) {
    throw new Error("the self row draws no connection word");
  }

  //    **`[data-rail-mark]`, not `nav > [data-face]`.** The face at the
  //    foot of the rail stopped being a direct child of the nav on
  //    2026-08-29, when it became the door into the mark's place and
  //    moved inside a rail button. The old locator was a path through
  //    the layout; this one names the thing, and the thing is what this
  //    assertion is about — one machine, two places, one picture.
  const railFace = page.locator("nav [data-rail-mark] [data-face]");
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

  // 12) four landings: one watches machines, two browse, one lists
  //     sessions — and that difference is the assertion.
  //
  //     Machine *names* are collected as a set, not counted: three rows
  //     prove nothing if they are three of something else.
  //
  //     **Both browsing landings are the counter-example on purpose**
  //     (标签页一期 for files, 06f5757 for browser). Either used to be
  //     checked here for listing the same machines as the devices pane;
  //     neither has a machine column any more, because which machine is
  //     a step on the way to a place rather than a thing to watch
  //     (docs/UX.md 浏览型), and asserting the absence is the point
  //     rather than a gap left by the deletion.
  const LANDING_TABS = ["sessions", "devices", "workspaces", "quota"];
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
  //     …and the negative half, after a probe that proves the selector
  //     still finds machines somewhere (the line above did that with the
  //     same locator, on the one landing that watches machines). Absent
  //     that, "no machine rows" and "the attribute got renamed" are one
  //     reading.
  for (const tab of ["browser"]) {
    await openLanding(tab);
    if ((await page.locator("[data-list]").count()) !== 0) {
      throw new Error(`the ${tab} landing still has a list pane`);
    }
    if ((await page.locator("[data-device]").count()) !== 0) {
      throw new Error(`the ${tab} landing still lists machines`);
    }
    if ((await page.locator("[data-address-bar]").count()) !== 1) {
      throw new Error(`the ${tab} landing has no address bar, so it has no second step at all`);
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
  // **Neither browsing landing has a pane bar to describe**, because
  // neither has a pane: the box belongs to a tab (24c). Files was
  // already out for that reason; browser was still in this list a batch
  // after it stopped having one.
  const BAR_TABS = ["sessions", "devices"];
  for (const tab of BAR_TABS) bars[tab] = await barOf(tab);
  const want = {
    sessions: { search: 1, filter: 1, plus: 1 },
    devices: { search: 1, filter: 0, plus: 1 },
  };
  for (const [tab, w] of Object.entries(want)) {
    for (const [what, n] of Object.entries(w)) {
      if (bars[tab][what] !== n) {
        throw new Error(`the ${tab} bar has ${bars[tab][what]} ${what}, expected ${n}`);
      }
    }
    if (!bars[tab].label) throw new Error(`probe dead: the ${tab} search box has no name`);
  }
  // The sessions pane searches something other than machines, and says
  // so. Read off the app, never spelled here — the catalog owns the
  // words, this owns the comparison. (The other half of this comparison
  // used to be the browser pane's box, which was the same box named the
  // same way; it is a tab's address bar now, and 24c reads that.)
  if (bars.sessions.label === bars.devices.label) {
    throw new Error("the sessions pane borrowed the machine pane's search label");
  }

  // 14) the way into the mark's place — **and it answers.**
  //
  //     This item used to assert the opposite, in as much detail: no
  //     button, no response to the pointer. That was right while the
  //     mark opened nothing, and it went false when it started to — so
  //     it was turned over rather than deleted, because the new fact is
  //     the one worth guarding. What it opens is asserted below (the
  //     map, and the cards its nodes open); what is asserted here is
  //     that it looks like something that opens anything at all.
  //
  //     **The door moved and changed its picture on 2026-08-29**: it was
  //     the app's own logo at the head of the rail, and it is this
  //     machine's face at the foot. `[data-rail-mark]` names the door
  //     rather than the artwork, so everything below is untouched by
  //     that — the one line that was not is the one that reached for an
  //     `<img>`'s `naturalWidth`, and what replaced it is the same claim
  //     in the new medium: **something was really drawn there.** A door
  //     that lost its picture passes every other assertion here, because
  //     a hover is a hover and a box is a box.
  const mark = page.locator("[data-rail-mark]");
  if ((await mark.count()) !== 1) throw new Error("no way into the mark's place in the rail");
  //     Either form of a face counts: the derived SVG, or the blank that
  //     stands for「no identity to draw one from」. Which of the two it
  //     should be is 12b's question, and it answers it against beta's
  //     own row — one machine, two places, one picture.
  if ((await mark.locator("[data-face], [data-face-blank]").count()) !== 1) {
    throw new Error("the door into the mark's place is not wearing this machine's face");
  }
  const doorBox = await mark.boundingBox();
  if (!doorBox || doorBox.width < 1 || doorBox.height < 1) {
    throw new Error("the door into the mark's place has been drawn to nothing");
  }
  const markButton = page.locator("[data-rail-item]", { has: mark });
  if ((await markButton.count()) !== 1) {
    throw new Error("the door is not a rail item, so it is not reachable by keyboard either");
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
  const glyph = railItem("workspaces");
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
  // Pinning and closing a session moved off the row and into a context
  // menu (right-click on a pointer, long-press on touch). Three
  // mechanics decide how every assertion below is written, and each one
  // fails in a way that reads like a broken product rather than a
  // broken test:
  //
  //   1. **The menu is portalled to `body`.** `[data-row="x"] [data-row-menu]`
  //      matches nothing, ever. Menu items are found globally.
  //   2. **The menu is modal** — Radix marks the rest of the page
  //      `aria-hidden`, so playwright judges every row unactionable
  //      while it is open. Press Escape before touching the list again.
  //   3. **Success closes it; failure does not.** That is the design
  //      (a refusal has to be said on the thing that was pressed), so
  //      whether an Escape is owed depends on which path was taken.
  //   4. **Close is two presses.** The first only arms the item
  //      (`[data-menu-item="close"][data-armed="true"]`); nothing has
  //      happened yet. "Press close, wait for the row to go" hangs, and
  //      the timeout says the row never left — which reads as closing
  //      being broken rather than as a press being owed. Assert the
  //      armed state between them, and the two-step is written into the
  //      test instead of having to be learned from `RowMenu`.
  //
  // The trigger is the row's own open button — it now spans the row,
  // and it is a button rather than the surrounding div so that a
  // keyboard's Menu key lands somewhere that can receive it.
  const openRowMenu = async (id) => {
    await page.locator(`[data-row="${id}"] [data-row-open]`).click({ button: "right" });
    await until(`the menu on ${id}`, 5_000, async () =>
      (await page.locator('[data-menu-item="pin"]').count()) === 1,
    );
  };
  const menuItemWord = async (which) =>
    (await page.locator(`[data-menu-item="${which}"]`).innerText()).trim();
  const closeRowMenu = async () => {
    await page.keyboard.press("Escape");
    await until("the menu to close", 5_000, async () =>
      (await page.locator("[data-row-menu]").count()) === 0,
    );
  };

  //     a. the control: nothing is pinned, and **every row opens a menu**
  //     — "every row" is the reachability claim, and walking the rows
  //     themselves is what makes it one.
  //
  //     **Not "every row has a `[data-row-open]`"**: that was already
  //     true before the menu existed, so it would assert nothing. What
  //     is new is that pressing it the other way yields the actions.
  if ((await page.locator("[data-row][data-pinned=true]").count()) !== 0) {
    throw new Error("something is pinned before the test pinned anything — the control is void");
  }
  for (const id of await sessionIds()) {
    await openRowMenu(id).catch(() => {
      throw new Error(`${id} opens no menu — its actions are unreachable`);
    });
    await closeRowMenu();
  }
  //     …and the row no longer carries a pin of its own. Asserted after
  //     the walk above, which is the proof that the selectors here are
  //     alive: a "there is no pin" read off a list that never drew is
  //     the same string as a passing test.
  if ((await page.locator("[data-row] [data-row-pin]").count()) !== 0) {
    throw new Error("a session row still carries its own pin button beside the menu");
  }

  //     b. GUI → CLI. Pin the second row (not the first: a row already on
  //     top proves nothing about floating), then read beta's own CLI.
  const idsBefore = await sessionIds();
  const target = idsBefore[1];
  const cliLineOf = (env, id) =>
    cli(env, "sessions").split("\n").find((l) => l.startsWith(`${id}\t`)) ?? "";
  const plainLine = cliLineOf(envB, target);
  if (!plainLine) throw new Error(`probe dead: ${target} is not in beta's CLI list`);

  //     …and the word on the menu item is read before pressing it, so
  //     that c) below can compare it against the other state's.
  await openRowMenu(target);
  const restingWord = await menuItemWord("pin");
  await page.locator('[data-menu-item="pin"]').click();
  //     No Escape: a pin that took closes the menu itself. If it did not
  //     take, the menu stays open wearing the failure — and the next
  //     assertion times out saying the row never floated, which is the
  //     honest reading of that state.
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

  //     c. the two states are two different sentences.
  //
  //     **This is the old "two shapes, not one shape in two colours"
  //     claim, kept and moved rather than dropped.** What it guarded is
  //     that a control may not report its state by position alone — the
  //     check "did the row jump to the top" is exactly the one that
  //     fails when the row was already at the top. A session row has no
  //     glyph any more, so here the state is carried by language: the
  //     item says what pressing it will do, which is precisely the
  //     negation of what is true now. The glyph half of the claim still
  //     has a home and is asserted where the glyph still lives — on the
  //     device list, in f) below.
  await openRowMenu(target);
  const pinnedWord = await menuItemWord("pin");
  await closeRowMenu();
  if (pinnedWord === restingWord) {
    throw new Error(`pinned and unpinned offer the same word: ${pinnedWord}`);
  }
  if (!pinnedWord || !restingWord) {
    throw new Error(`probe dead: a menu item printed nothing (${restingWord} / ${pinnedWord})`);
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
  await openRowMenu(target);
  await page.locator('[data-menu-item="pin"]').click();
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

  //     f. machines pin too, and the same way — through the same menu
  //     (2026-09-06:「机器左侧 list,与 session 一样,是右键,置顶或删除,
  //     取消现在的置顶 icon」). The row carries no pin of its own any
  //     more; the menu's first item says 置顶 or 取消置顶, the table
  //     sorts, the app paints. gamma is last by name, so its arrival at
  //     the top is not something the previous order could have
  //     produced.
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) === 3);
  const deviceNames = () =>
    page.locator("[data-device]").evaluateAll((els) => els.map((e) => e.dataset.device));
  if ((await deviceNames())[0] === "gamma") throw new Error("gamma already leads — the control is void");
  //     Asserted after a) walked the session rows' menus, which is the
  //     proof the selector is alive: a "there is no pin" read off a
  //     list that never drew is the same string as a passing test.
  if ((await page.locator("[data-device] [data-row-pin]").count()) !== 0) {
    throw new Error("a machine row still carries its own pin button beside the menu");
  }
  const gammaRow = await page.locator('[data-device="gamma"]').getAttribute("data-row");
  await openRowMenu(gammaRow);
  const gammaResting = await menuItemWord("pin");
  await page.locator('[data-menu-item="pin"]').click();
  await until("gamma to lead beta's machine list", 10_000, async () => (await deviceNames())[0] === "gamma");
  await until("beta's CLI to lead with gamma", 10_000, () =>
    cli(envB, "devices").split("\n")[0]?.startsWith("gamma"),
  );
  //     …and **the state half of c) lives here too**: the item's word
  //     is the opposite of the row's state, so a pinned row's menu
  //     wears a different word from the resting one. Compared against
  //     the word this same item wore a moment ago rather than spelled
  //     here, because the words belong to the catalog.
  await openRowMenu(gammaRow);
  const gammaPinnedWord = await menuItemWord("pin");
  if (!gammaPinnedWord || gammaPinnedWord === gammaResting) {
    throw new Error(`pinned and unpinned read the same on the menu: ${JSON.stringify(gammaPinnedWord)}`);
  }
  await page.locator('[data-menu-item="pin"]').click();
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
  // **The strip under alpha's row is alpha's own day, not 没在看.**
  // Alpha's record has words on this row (its sampler has been running
  // for minutes, and a cell was planted before it woke); they reach
  // beta's screen through `Request::Days`, and a beta that still
  // painted every reported row as unwatched shows one grey run here.
  // **Any word, not a particular one**: which word a screen cell wears
  // when several minutes merge into it is the node's ranking, and the
  // first version of this check asked for 忙碌 by name and reddened on
  // a strip that was alpha's all along — its busy minutes merged
  // under a 完成. Waited on rather than asserted: beta asks for a
  // peer's day on its report beat.
  // The band sits under the open session's header now, so the row is
  // opened first and the band read there.
  await page.locator('[data-row="tui/cafe1"] [data-row-open]').click();
  await until("a word from alpha's own day on beta's band for tui/cafe1", 60_000, async () =>
    (await page.locator('[data-detail-day] [data-day-mark]:not([data-day-mark="unwatched"])').count()) >= 1,
  );
  // 16-0) open sessions are tabs. Opening a second row adds a tab and
  //       brings it to the front; closing that tab leaves its row on
  //       the list and its session alive (a view left, nothing ended),
  //       and the first tab is in front again.
  await until("the first tab", 10_000, async () =>
    (await page.locator('[data-session-tabs] [data-tab="tui/cafe1"][data-on="true"]').count()) === 1,
  );
  const secondRow = await page
    .locator('[data-row]:not([data-row="tui/cafe1"])')
    .first()
    .getAttribute("data-row");
  await page.locator(`[data-row="${secondRow}"] [data-row-open]`).click();
  await until("a second tab, in front", 10_000, async () =>
    (await page.locator(`[data-session-tabs] [data-tab="${secondRow}"][data-on="true"]`).count()) === 1 &&
    (await page.locator('[data-session-tabs] [data-tab]').count()) === 2,
  );
  await page.locator(`[data-session-tabs] [data-tab-close="${secondRow}"]`).click();
  await until("the second tab gone, the first in front, the row still listed", 10_000, async () =>
    (await page.locator('[data-session-tabs] [data-tab]').count()) === 1 &&
    (await page.locator('[data-session-tabs] [data-tab="tui/cafe1"][data-on="true"]').count()) === 1 &&
    (await page.locator(`[data-row="${secondRow}"]`).count()) === 1,
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
    // 工作区 (工作区批): the sessions khor opened here place themselves
    // in this checkout, and a conversation with another machine places
    // nowhere — so this mode has two groups at this point for two
    // different reasons, which is exactly what it is for.
    ["place", "at:"],
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
  // Waited for the headings to *change*, the loop's own rule: the
  // list keeps the place headings until the rows come back under the
  // state mode, so "two or more headings" is true at once and reads the
  // mode before — seen live under a cargo build's load, where the
  // regroup took longer than the read.
  const beforeState = (await groupKeys()).join();
  await chooseArrange("state");
  await until("the state grouping to settle", 15_000, async () => {
    const now = await groupKeys();
    return now.length >= 2 && now.join() !== beforeState;
  });
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
  await openRowMenu(pinTarget);
  await page.locator('[data-menu-item="pin"]').click();
  for (const mode of ["recent", "category", "device", "state"]) {
    await chooseArrange(mode);
    await until(`${pinTarget} leading in ${mode}`, 15_000, async () => {
      const now = await arrangeIds();
      return now[0] === pinTarget;
    });
  }
  await openRowMenu(pinTarget);
  await page.locator('[data-menu-item="pin"]').click();
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
  // …and the landings past the fourth are one fold away (2026-09-02:
  // 「窄屏的时候…最多只显示四个icon,多余的折叠为...」):「…」is there,
  // and `openLanding` goes through it below.
  if ((await page.locator("[data-rail-more]").count()) !== 1) {
    throw new Error("no fold on the narrow rail — the landings past the fourth are unreachable");
  }
  if ((await page.locator("[data-rail-mark]").count()) !== 1) {
    throw new Error("the mark is missing from the narrow rail, so the mesh is unreachable here");
  }
  // Opening it is the assertion: `openLanding` waits for this landing's
  // own shell, and a browsing landing has no machine rows to count on
  // any width (12). This waited for exactly that count, which is why the
  // narrow half of this item stopped being reachable a batch before
  // anybody ran it.
  await openLanding("browser");
  if ((await page.locator("[data-address-bar]").count()) !== 1) {
    throw new Error("the narrow browser landing has no address bar, so it has no way in");
  }

  // 18) the row's actions are reachable on the narrow face — the one
  //     where there is no pointer at all. A hover-only row action is
  //     simply absent on a phone, and it looks identical to a working
  //     one in every screenshot taken with a mouse present.
  //
  //     The actions now live behind a long press, which is the touch
  //     half of the same gesture the pointer spells as right-click. So
  //     what has to be proven here is the *press*, and playwright's
  //     mouse and `touchscreen.tap()` produce neither: the trigger wants
  //     a `pointerdown` whose `pointerType` is touch, held past its own
  //     timer. That takes CDP.
  //
  //     **The premise was measured before anything was asserted on it**
  //     (a listener-only probe on the real page confirmed this path
  //     yields `pointerType: "touch"` held ~900ms). Without that, a menu
  //     that appeared for some other reason would read as a pass.
  await openLanding("sessions");
  await until("rows on the narrow session list", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  await pointerAway();
  await settle();
  const narrowIds = () =>
    page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const narrowTarget = (await narrowIds())[1];
  //     **Touch is switched on for this item and off again after it**,
  //     rather than for the whole run. Measured, after the whole-run
  //     version broke §14: a context with `hasTouch` reports
  //     `(hover: hover)` and `(pointer: fine)` as **false**, and the
  //     built stylesheet gates every hover rule behind `@media
  //     (hover:hover)` — so every hover assertion in this file silently
  //     stops being about anything. One flag for one item cost five
  //     others; the scope is the fix.
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
  const pressFor = async (id, ms) => {
    const b = await page.locator(`[data-row="${id}"] [data-row-open]`).boundingBox();
    if (!b) throw new Error(`probe dead: ${id} has no box to press`);
    const x = Math.round(b.x + b.width / 2);
    const y = Math.round(b.y + b.height / 2);
    await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x, y }] });
    await new Promise((r) => setTimeout(r, ms));
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  };
  //     The negative half first, and it is the half that keeps this from
  //     being a free pass: a short tap must **not** open it. Without
  //     this, an implementation that opened the menu on any touch at all
  //     would satisfy everything below.
  await pressFor(narrowTarget, 120);
  await settle();
  if ((await page.locator("[data-row-menu]").count()) !== 0) {
    throw new Error("a short tap opened the row menu — every tap would");
  }
  //     …and a short tap is not a no-op: it is a tap, so it opens the
  //     row. On this face that replaces the list with the detail, which
  //     is why the walk back is here rather than treated as a surprise.
  //     (Measured the hard way: without it the next press looks for a
  //     row that is no longer on screen, and the timeout reads as the
  //     long press being broken.)
  if ((await page.locator("[data-back]").count()) === 1) {
    await page.locator("[data-back]").click();
    await until("the list back after the tap", 10_000, async () =>
      (await page.locator("[data-row]").count()) > 1,
    );
  } else if ((await page.locator("[data-row]").count()) === 0) {
    throw new Error("the short tap left neither a list nor a way back to one");
  }
  await pressFor(narrowTarget, 900);
  await until("the long press to open the row menu", 5_000, async () =>
    (await page.locator('[data-menu-item="pin"]').count()) === 1,
  );
  //     …and it still works down here, which is the half a visibility
  //     check cannot cover: a control can be perfectly visible and out
  //     of reach.
  await page.locator('[data-menu-item="pin"]').click();
  await until("the narrow pin to float its row", 10_000, async () => (await narrowIds())[0] === narrowTarget);
  await pressFor(narrowTarget, 900);
  await until("the menu again", 5_000, async () =>
    (await page.locator('[data-menu-item="pin"]').count()) === 1,
  );
  await page.locator('[data-menu-item="pin"]').click();
  await until("the narrow row to go back", 10_000, async () => (await narrowIds())[0] !== narrowTarget);
  //     …and the pointer comes back, or everything downstream that asks
  //     about a hover is asking a page that has no pointer.
  await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: false });
  await until("the pointer to come back", 5_000, async () =>
    page.evaluate(() => matchMedia("(hover: hover)").matches),
  );

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
    // **The probe's own verb, asserted before its reading is believed.**
    // The rule under test is `[data-slot="button"]:active`, so a target
    // that is not in `:active` while held is a reading about something
    // else — and the red it produces points at the press rule, which is
    // fine. Measured 2026-09-06 on the pane's 「+」: scale 1, opacity 1,
    // and its menu already open. A Radix trigger acts on `pointerdown`
    // and the open menu takes the pointer off it, so `:active` is gone
    // before this line runs. 「动词属于探针,不属于产品」.
    const live = await locator.evaluate((el) => el.matches(":active"));
    if (!live) {
      throw new Error(
        "probe dead: the control is not :active while held — a menu trigger " +
          "acts on pointerdown and cannot be measured here",
      );
    }
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
  //      **Measured on the machine card's back**, because it is the
  //      `Button` on this landing that is a button and nothing else:
  //      the session row's pin moved into a menu (§15) and the machine
  //      row's followed it on 2026-09-06, and a menu item is not this
  //      component — it is not a `Button`, so the give this item asserts
  //      does not apply to it and asserting it there would be inventing
  //      a rule.
  //
  //      **The pane's 「+」 was the target for one day and could not be
  //      one** (2026-09-06, a red): it is a `DropdownMenuTrigger`, and a
  //      trigger acts on `pointerdown` — by the time a held probe reads
  //      it the menu is open and the pointer is off the trigger, so
  //      `:active` never applies and the reading is 1. Nothing was wrong
  //      with the press rule; the target was wrong. The `:active` check
  //      inside `pressing` is what says so now, in one line, instead of
  //      pointing this assertion at a rule that is fine.
  //
  //      The card's back is a ghost icon `Button` like the 「+」, opens
  //      nothing, and on the narrow face it is the one control the card
  //      must always have.
  //
  //      **The row is read first, then the walk into the card.** A row
  //      is on the list screen and the back is on the detail screen, and
  //      the reduced-motion pass at the end of this section reads the
  //      back again — so the section ends on the screen that has it,
  //      rather than walking back and leaving that locator pointing at a
  //      screen that is gone (the note further down is the scar from the
  //      last time this order was got wrong).
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) > 0);
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
  await page.locator("[data-device] [data-row-open]").first().click();
  await until("a machine's card", 10_000, async () => (await page.locator("[data-back]").count()) === 1);
  const back = page.locator("[data-back]");
  const press = await pressing(back);
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
  //      Released off the control, so the card never went back.
  if ((await page.locator("[data-back]").count()) !== 1) {
    throw new Error("the press probe pressed something for real — the card went back");
  }

  //      …and under reduced motion the give goes while the dim stays.
  //      Both halves: "nothing moved" is also what a broken probe says,
  //      and a press that answered with nothing at all would be the
  //      做了但没变化 failure arrived at through an accessibility
  //      setting.
  await page.emulateMedia({ reducedMotion: "reduce" });
  const still = await pressing(back);
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
  //      Back to the sessions list, and **after the last reader of the
  //      card's back** rather than straight after the first: the reduced
  //      motion pass above uses the same locator, and moving the walk
  //      back up here left it pointing at a screen that was no longer up
  //      — a timeout that reads as the control having vanished. It was
  //      the device pin's list when this was first written; the same
  //      order is what the walk into the card has to keep.
  await openLanding("sessions");
  await until("rows again", 10_000, async () => (await page.locator("[data-row]").count()) > 0);

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
  //      **The subject is chosen by what it is, not by where it sits.**
  //      This used to take `rowIds()[2]` — a row with something above it
  //      to displace, which is the property this item needs — and index
  //      2 is not that property, it is a guess at it. Earlier items
  //      leave pins behind and the list is ordered by the node, so on
  //      some runs index 2 was **already pinned**: the press then
  //      correctly unpinned it, the row correctly did not lead, and the
  //      wait below timed out with 「last: false」. Two runs out of four
  //      on 2026-08-29, and both read as「pinning is broken」 while the
  //      product was doing exactly the right thing.
  //
  //      So: the first row that is not pinned and is not already at the
  //      top. Same property, stated instead of guessed.
  const rowPins = await page
    .locator("[data-row]")
    .evaluateAll((els) => els.map((e) => [e.dataset.row, e.dataset.pinned]));
  const flipTarget = rowPins.slice(1).find(([, pinned]) => pinned !== "true")?.[0];
  if (!flipTarget) {
    throw new Error(
      `probe dead: no unpinned row below the top to float — ${JSON.stringify(rowPins)}`,
    );
  }
  const wasAt = rowPins.findIndex(([id]) => id === flipTarget);
  //      **The reordering action is still the pin; only its doorway
  //      moved.** The menu is opened first and the frame counter starts
  //      after it is up: the right-click itself reorders nothing, so
  //      counting across it would spend frames on an event that cannot
  //      move a row. The menu is portalled outside `[data-row]`, so it
  //      cannot contribute a transform to the count either.
  await openRowMenu(flipTarget);
  await watchTransforms();
  await page.locator('[data-menu-item="pin"]').click();
  //      **What this waits for is that the row moved up and that the
  //      screen agrees with the node — not that it leads.**
  //
  //      「It leads」 was the third wrong spelling of this in one day and
  //      the other two are worth keeping, because each was a smaller
  //      version of the same mistake — *asserting a position when the
  //      property wanted is a relation*:
  //
  //        1. `rowIds()[2]` as the subject: index 2 is not「a row with
  //           something above it」, it is a guess at one, and on some
  //           runs it was already pinned — the press unpinned it and the
  //           row correctly did not move up;
  //        2. 「the pinned row leads」 with an unpinned subject: pinning
  //           floats a row into the pinned **group**, and it only
  //           reaches index 0 when that group was empty. Measured on the
  //           run that caught this: `data-pinned=true`, order
  //           `[tui/cafe1 chat/alpha chat/beta chat/gamma]`, and
  //           `khor sessions` led with `tui/cafe1` too — **the product
  //           and the CLI agreed with each other and disagreed with the
  //           test**. That is the shape of a false red: it read as
  //          「pinning is broken」 while nothing was.
  //
  //      So the claim is the one this layer is allowed to make (§15's
  //      own words): the row the node put where it put it *is* there.
  //      Two halves — it climbed, and the DOM order is the node's order
  //      — and the first would pass on a list that reshuffled at random
  //      while the second would pass on a list that never moved.
  await until("the pinned row to climb, and the screen to agree with the node", 20_000, async () => {
    const now = await rowIds();
    const nowAt = now.indexOf(flipTarget);
    const backend = cli(envB, "sessions")
      .split("\n")
      .filter((l) => l.includes("\t"))
      .map((l) => l.split("\t")[0]);
    // **The target's seat, not the whole order.** This used to demand
    // the screen's full order equal the node's — two reads of a list
    // that moves on a 2s poll, taken one after the other — and it
    // reddened on 2026-09-01 with both sides right and merely a beat
    // apart (its own history above holds three earlier false reds of
    // the same family). The claim §15 allows is about *this row*: the
    // node put it somewhere, and the screen shows it there. Rows
    // shuffling around it are the list being alive, not a disagreement.
    if (nowAt >= 0 && nowAt < wasAt && nowAt === backend.indexOf(flipTarget)) return true;
    const pinnedAttr = await page.locator(`[data-row="${flipTarget}"]`).getAttribute("data-pinned");
    throw new Error(
      `target=${flipTarget} data-pinned=${pinnedAttr} was #${wasAt} now #${nowAt}; ` +
        `screen=[${now.join(" ")}] node=[${backend.join(" ")}]`,
    );
  });
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
  await openRowMenu(flipTarget);
  await watchTransforms();
  await page.locator('[data-menu-item="pin"]').click();
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
  //     with nothing picked the space holds the map instead, so the
  //     card's own section is what proves a machine was opened — it is
  //     not rendered at all until a row is clicked.
  //
  //     **This is the one of the four that was passing for free.** It
  //     named `data-machine-id`, which has not existed since `2ef4750`,
  //     so "no card is open" was being answered by a probe that could
  //     never have found one. The live half is the map assertion right
  //     below, which finds real elements in this very state.
  await page.setViewportSize({ width: 1080, height: 720 });
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) >= 2);
  if ((await page.locator("[data-machine-card]").count()) !== 0) {
    throw new Error("a machine card is open before any machine was opened");
  }

  //     **And what that space holds instead is the mandala map** — one
  //     seat per machine, exactly one of them the middle, the ring a
  //     real ring, and every line on it starting at that middle.
  //
  //     **The last one is turned over rather than deleted.** It used to
  //     read "nothing drawn between any two of them", on the judgment
  //     that khor knows who is in the mesh and not who can reach whom.
  //     The hop reading overturned exactly half of that: khor does know
  //     one thing about reachability — the road *this* machine is using
  //     to reach each other one, first-hand, one edge's worth per
  //     machine (`MandalaMap`'s head says the same). What two *other*
  //     machines have between them it still does not know, so what must
  //     be asserted now is not the absence of lines but their shape: a
  //     star. Every edge shares one endpoint, and that endpoint is the
  //     middle seat.
  //
  //     The positive half is counted first, so a zero means zero rather
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
  //     One edge per machine that is not this one, and not one more:
  //     a second-hand edge would be khor drawing a road it was never
  //     told about.
  const edges = await page.locator("[data-mandala-map] [data-edge]").evaluateAll((els) =>
    els.map((e) => ({
      road: e.dataset.road,
      from: `${e.getAttribute("x1")},${e.getAttribute("y1")}`,
      to: `${e.getAttribute("x2")},${e.getAttribute("y2")}`,
    })),
  );
  if (edges.length !== machines - 1) {
    throw new Error(`the map draws ${edges.length} edges for ${machines - 1} other machines`);
  }
  const ends = new Set(edges.map((e) => e.from));
  if (ends.size !== 1) {
    throw new Error(
      `the map's edges start from ${ends.size} different points — a line between two other ` +
        "machines is a road khor was never told about",
    );
  }
  //     …and that shared end really is the middle seat, not merely a
  //     point they happen to agree on.
  const middle = await page
    .locator('[data-mandala-map] [data-seat][data-seat-me="true"]')
    .evaluate((el) => el.style.left + "|" + el.style.top);
  const [mLeft, mTop] = middle.split("|").map((v) => parseFloat(v));
  const [eLeft, eTop] = [...ends][0].split(",").map(Number);
  if (Math.abs(mLeft - eLeft) > 0.01 || Math.abs(mTop - eTop) > 0.01) {
    throw new Error(`edges meet at ${eLeft},${eTop} and the middle seat sits at ${mLeft},${mTop}`);
  }
  //     Every road wears one of the three strokes, and the word beside
  //     it is the same one the machine's own row prints (24 reads that);
  //     an unknown key would paint a faint line with no word at all.
  for (const e of edges) {
    if (!["direct", "relay", "none"].includes(e.road)) {
      throw new Error(`an edge wears the road ${e.road}, which is not one of the three`);
    }
  }
  //     The faces are still the only other svg on the map: one per seat,
  //     plus the single layer the edges live in.
  const drawn = await page.locator("[data-mandala-map] svg").count();
  if (drawn !== faces + 1) {
    throw new Error(`the map draws ${drawn} svg elements for ${faces} faces and one edge layer`);
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
  //
  //     The row is `name \t id \t road \t readings` — the readings are
  //     **the last cell**, read that way rather than by number so that a
  //     column added before them (the road one was, 批⑦) moves this
  //     probe instead of silently making it read the wrong thing.
  const cliRow = (machine, env) => {
    const line = cli(env, "devices")
      .split("\n")
      .find((l) => l.startsWith(`${machine}\t`));
    if (!line) throw new Error(`probe dead: no ${machine} row printed by \`khor devices\``);
    const cells = line.split("\t");
    if (cells.length < 4) throw new Error(`probe dead: ${machine}'s row is ${cells.length} cells`);
    return { road: cells[2], readings: cells[cells.length - 1].split("  ").filter(Boolean) };
  };
  const cliReadings = (machine, env) => cliRow(machine, env).readings;
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

  //     The row's connection line is the same road word the card and
  //     the terminal print — one fact, three faces (2026-09-02: the
  //     row's reading stack left and the road took its slot).
  const rowRoad = page.locator('[data-device="alpha"] [data-row-connection]');
  if ((await rowRoad.count()) !== 1) throw new Error("alpha's row draws no connection line");
  const rowRoadWord = (await rowRoad.innerText()).trim();
  if (rowRoadWord !== cliRow("alpha", envB).road.trim()) {
    throw new Error(`alpha's row road is "${rowRoadWord}", the terminal prints something else`);
  }
  //     Clicked the way a person clicks it: the row's own button.
  const shortId = await page
    .locator('[data-device="alpha"]')
    .evaluate((el) => el.dataset.row.slice(0, 12));
  await page.locator('[data-device="alpha"] [data-row-open]').click();
  await until("alpha's card", 10_000, async () => (await page.locator("[data-machine-card]").count()) === 1);

  //     …and it is alpha's card, not merely *a* card. The row carries the
  //     first twelve characters of the id and the card can hand over all
  //     of it, so one being a prefix of the other ties the two together
  //     without this script knowing either value (`cardId` says how it
  //     is fetched now, and why).
  const openCard = await cardId();
  if (!openCard.startsWith(shortId) || openCard.length <= shortId.length) {
    throw new Error(`the card hands over ${openCard}, which is not the full id behind ${shortId}`);
  }

  //     The prefix on the card's own face is the same twelve characters
  //     (2026-09-02:「序列号写在机器页面内」) — tied to the full id the
  //     card hands over, the way the row used to be.
  if ((await page.locator("[data-machine-card] [data-card-id]").innerText()).trim() !== shortId) {
    throw new Error("the card's id prefix is not the id it hands over");
  }

  //     **改名 (2026-09-11)**: double-click the name, type, Enter — the
  //     same gesture and the same component a session title uses
  //     (`EditableName`), because the machine page growing its own copy
  //     of what the rest of the app has is a standing complaint, not a
  //     one-off (docs/handoff/001.md).
  //
  //     Three things asserted, and the third is the one with teeth:
  //
  //       1. the card wears the new name;
  //       2. so does the list row — a rename is network-wide, not a
  //          nickname on the screen that typed it;
  //       3. **`[data-device="alpha"]` is still there.** That attribute
  //          is the *channel*, and the channel is what `chat/<name>`,
  //          the read watermark and the pin are keyed on. A rename
  //          written into the channel would move all three, and this
  //          line is the only one that would notice. Write the label
  //          into `F_NAME` and it goes red; write it into `F_LABEL` and
  //          nothing on disk moves at all.
  const cardName = page.locator("[data-machine-card] [data-machine-name]");
  if ((await cardName.count()) !== 1) throw new Error("probe dead: the card has no editable name");
  if ((await cardName.innerText()).trim() !== "alpha") {
    throw new Error(`the card names this machine "${(await cardName.innerText()).trim()}", not alpha`);
  }
  await cardName.dblclick();
  const nameBox = page.locator("[data-machine-card] [data-title-input]");
  await until("the name editor", 5_000, async () => (await nameBox.count()) === 1);
  await nameBox.fill("北京服务器");
  await nameBox.press("Enter");
  await until("the card to wear the new name", 10_000, async () =>
    (await page.locator("[data-machine-card] [data-machine-name]").innerText()).trim() === "北京服务器",
  );
  await until("the list row to wear it too", 15_000, async () =>
    (await page.locator('[data-device="alpha"]').innerText()).includes("北京服务器"),
  );
  if ((await page.locator('[data-device="alpha"]').count()) !== 1) {
    throw new Error("renaming a machine moved its channel — chat, seen and pins are all keyed on that");
  }
  //     And the eraser: an emptied box gives the machine its reported
  //     name back. Without this arm a rename that could not be undone
  //     would pass everything above.
  await page.locator("[data-machine-card] [data-machine-name]").dblclick();
  await until("the name editor again", 5_000, async () => (await nameBox.count()) === 1);
  await nameBox.fill("");
  await nameBox.press("Enter");
  await until("the machine to wear its own name again", 10_000, async () =>
    (await page.locator("[data-machine-card] [data-machine-name]").innerText()).trim() === "alpha",
  );

  //     **And the list row renames too** (2026-09-11:「session 现在列表
  //     双击名称即可重命名,我需要机器也一样」). The card above proves
  //     the write works; this proves the *gesture* reaches the row —
  //     two different things, and the row is the one with a trap in it:
  //     the name sits inside the button that opens the row, so a
  //     double-click that did not stop propagating would open the row
  //     over and over instead of opening an editor.
  const rowName = page.locator('[data-device="alpha"] [data-machine-name]');
  if ((await rowName.count()) !== 1) throw new Error("probe dead: alpha's row has no editable name");
  await rowName.dblclick();
  const rowBox = page.locator('[data-device="alpha"] [data-title-input]');
  await until("the row's name editor", 5_000, async () => (await rowBox.count()) === 1);
  await rowBox.fill("从列表改的");
  await rowBox.press("Enter");
  await until("the row to wear it", 10_000, async () =>
    (await page.locator('[data-device="alpha"]').innerText()).includes("从列表改的"),
  );
  //     The card is the other half of the same fact — one machine, one
  //     name, wherever it is drawn.
  await until("the card to agree", 10_000, async () =>
    (await page.locator("[data-machine-card] [data-machine-name]").innerText()).trim() === "从列表改的",
  );
  await page.locator('[data-device="alpha"] [data-machine-name]').dblclick();
  await until("the row's editor again", 5_000, async () => (await rowBox.count()) === 1);
  await rowBox.fill("");
  await rowBox.press("Enter");
  await until("alpha back to its own name", 10_000, async () =>
    (await page.locator("[data-machine-card] [data-machine-name]").innerText()).trim() === "alpha",
  );

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
  //     Named for what it is: the filled arc of a ring. It was
  //     `data-vitals-bar` while the readings were bars, and the shape
  //     changed (批⑨) — the fact asserted did not. It points at the arc
  //     itself rather than at a wrapper, so "a gauge drew" cannot be
  //     answered by a box that would have been there either way.
  if ((await page.locator("[data-ring-fill]").count()) === 0) {
    throw new Error("no reading drew its gauge");
  }
  await sameNumberOfReadings("alpha", envB);

  //     **A directory watched from here shows up on alpha's card, read
  //     by alpha** (2026-09-06 关注目录). The list is written into
  //     alpha's row of the device table from beta's face; it reaches
  //     alpha on the next sync round, alpha answers `Watched` on beta's
  //     next visit, and the card grows one row per directory. Two
  //     directories, two shapes — alpha's own home (on a listed volume)
  //     and a path that is not there — so 「missing」 is a sentence and
  //     not a zero. The CLI witness runs again after: `khor devices`
  //     prints one sentence per row here too, and the count has to
  //     still hold with the rows in. Then `khor watch alpha` with no
  //     directories clears the list from the terminal, and the rows go.
  await page.locator('[data-machine-card] [data-dial="disk"]').click();
  await until("the disk drawer's editor", 5_000, async () =>
    (await page.locator("[data-machine-card] [data-watch-editor]").count()) === 1,
  );
  if ((await page.locator("[data-machine-card] [data-watch-row]").count()) !== 0) {
    throw new Error("alpha's card already draws watched rows before anything was asked — the control is void");
  }
  for (const p of [A, "/khor-smoke-no-such-dir"]) {
    await page.locator("[data-machine-card] [data-watch-input]").fill(p);
    await page.locator("[data-machine-card] [data-watch-add]").click();
  }
  await page.locator("[data-machine-card] [data-watch-save]").click();
  await until("the save to rest", 10_000, async () => {
    const err = await page.locator("[data-machine-card] [data-watch-error]").count();
    if (err) throw new Error(await page.locator("[data-machine-card] [data-watch-error]").innerText());
    return await page.locator("[data-machine-card] [data-watch-save]").isDisabled();
  });
  await until("alpha's answer about the two directories to reach this card", 40_000, async () =>
    (await page.locator('[data-machine-card] [data-watch-row][data-watch-shape="on"]').count()) === 1 &&
    (await page.locator('[data-machine-card] [data-watch-row][data-watch-shape="missing"]').count()) === 1,
    1_000,
  );
  //     One number on both faces, from one answer: the sentence the row
  //     carries is the sentence the terminal prints.
  const watchedOnCard = await page.locator('[data-machine-card] [data-vitals-detail="watch"]').allInnerTexts();
  const watchedPrinted = cliReadings("alpha", envB).filter((s) => watchedOnCard.some((c) => c.trim() === s.trim()));
  if (watchedPrinted.length !== 2) {
    throw new Error(`the card's two watched sentences ${JSON.stringify(watchedOnCard)} are not what \`khor devices\` prints: ${cliReadings("alpha", envB).join(" | ")}`);
  }
  await sameNumberOfReadings("alpha", envB);
  cli(envB, "watch", "alpha");
  await until("the rows to go once the list is cleared from the terminal", 40_000, async () =>
    (await page.locator("[data-machine-card] [data-watch-row]").count()) === 0,
    1_000,
  );
  await page.locator('[data-machine-card] [data-dial="disk"]').click();

  //     **The road reading is one fact, not two** (批⑦): the card and
  //     `khor devices` are two faces on one `Node::hops` answer, and a
  //     screen that worked it out for itself is a screen that can
  //     disagree with the terminal. Compared as the word, which is the
  //     only form both ends show — neither side spells it here.
  const cardRoad = page.locator("[data-machine-card] [data-hop]");
  if ((await cardRoad.count()) !== 1) {
    throw new Error("alpha's card draws no road reading");
  }
  const printedRoad = cliRow("alpha", envB).road;
  if ((await cardRoad.innerText()).trim() !== printedRoad.trim()) {
    throw new Error(
      `alpha's road is "${await cardRoad.innerText()}" on the card and "${printedRoad}" in the terminal`,
    );
  }

  //     **The offline axis reaches the screen.** alpha's reading was
  //     taken on alpha and carried here, so it has an age; beta samples
  //     itself to answer the very call that painted this, so it has
  //     none. Both halves are asserted because either alone passes for
  //     the wrong reason — always showing an age, or never showing one.
  if ((await page.locator("[data-vitals-age]").count()) !== 1) {
    throw new Error("a reading that travelled here shows no age");
  }
  const betaShort = await page
    .locator('[data-device="beta"]')
    .evaluate((el) => el.dataset.row.slice(0, 12));
  await page.locator('[data-device="beta"] [data-row-open]').click();
  //     Waited for, then read once. **Not read inside the wait**: the
  //     fetch is a click on the copy button, and a loop would press it
  //     once per turn on whichever card happened to be up.
  await until("beta's card", 10_000, async () =>
    (await page.locator("[data-machine-card]").count()) === 1,
  );
  const betaCard = await cardId();
  if (!betaCard.startsWith(betaShort)) {
    throw new Error(`the card hands over ${betaCard}, which is not the id behind ${betaShort}`);
  }
  //     **And this machine's own card carries no road at all.** A
  //     machine does not reach itself over the network, so the row holds
  //     no reading rather than an unreadable one — the negative half of
  //     the check above, and the one that would go quietly wrong if the
  //     field were a word every face had to remember to hide.
  if ((await page.locator("[data-machine-card] [data-hop]").count()) !== 0) {
    throw new Error("beta's own card claims a road to itself");
  }
  //     What the slot holds instead (2026-09-02): the machine's own
  //     word — it reaches itself over nothing — with the id prefix
  //     beside it.
  if ((await page.locator("[data-machine-card] [data-machine-here]").count()) !== 1) {
    throw new Error("beta's own card does not carry the here-word in the connection slot");
  }
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
  //     **The sentence is on the back of the reading now** (批⑨): the
  //     front is a gauge, and the exact quantity turns over. So it is
  //     turned over the way a person turns it over — which asserts one
  //     thing more than the old reading of the whole unit did, that the
  //     sentence is *reachable* rather than merely present in the
  //     markup. And it is read off the element that holds only the
  //     sentence, instead of off a box whose text happened to be the
  //     sentence and nothing else.
  //     **The card's parts live in a drawer now**, and the drawer that
  //     opens on arrival is the leftmost one (CPU) — so the way a
  //     person reaches a card's row is to press the GPU dial first.
  //     That is one more thing asserted, not one less: the drawer is
  //     now on the path between somebody and this sentence, and a
  //     drawer that would not open would red here.
  const gpuDial = page.locator('[data-machine-card] [data-dial="gpu"]');
  if ((await gpuDial.count()) === 1 && (await gpuDial.getAttribute("data-open")) !== "true") {
    await gpuDial.click();
  }
  await gpuRow.click();
  const gpuDetail = page.locator('[data-vitals-detail="gpu"]');
  await until(
    "the GPU reading turned over",
    5_000,
    async () => (await gpuDetail.innerText()).trim().length > 0,
  );
  const sentence = (s) => s.replace(/\d+/g, "#").replace(/\s+/g, " ").trim();
  const cardCount = (s) => s.match(/\d+/g)?.[1];
  const onCard = await gpuDetail.innerText();
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
  //     devices card, the borrowed network. The pane that led nowhere
  //     was the bug this once caught; there is none left, so the
  //     assertion inverts to "both lead somewhere", read off the same
  //     three machines each pane lists. (Files was a third of these
  //     until it stopped having machine rows at all — 12 asserts that,
  //     and 24c asserts what replaced them.)
  const openersOn = async (tab) => {
    await openLanding(tab);
    await until(`machines on the ${tab} pane`, 10_000, async () =>
      (await page.locator("[data-device]").count()) > 0,
    );
    return page.locator("[data-device] [data-row-open]").count();
  };
  // Devices only: the browsing landings reach their second step through
  // a tab's address bar rather than a row (24c reads the files one, and
  // `check-tabs` drives the pages one), so asking them for machine rows
  // asks for the pane 12 just proved they do not have.
  {
    const n = await openersOn("devices");
    if (n < 3) {
      throw new Error(`only ${n} of three machine rows open their second step on the devices pane`);
    }
  }

  // 24c) ⚠️ **Tombstone: the files landing's address bar, 2026-09-11**
  //      (工作区批3). The whole section is gone because its subject is:
  //      工作区替代文件 took the disk kind out of the browsing shell, so
  //      there is no pane with a path in a box any more, and the hook
  //      that fed it (`hooks/use-path-candidates`) has no consumer left
  //      in the product.
  //
  //      **What went with it, named rather than quietly dropped:**
  //
  //      - **Typing a path opens it.** khor's faces can no longer reach
  //        an arbitrary directory on a far machine — only a workspace.
  //        `khor ls` still does it; no screen does. (**Updated
  //        2026-09-14**: a workspace is now *declared* rather than
  //        derived — `khor ws add` names one, and the cross-machine
  //        directory picker behind the landing's 「+」 is 工作区批②,
  //        which is where typing a path comes back.)
  //      - **The anti-flicker measurement.** Candidates are keyed by
  //        directory, so narrowing inside one asked the machine nothing;
  //        that was counted off `performance.getEntriesByType`, four
  //        keystrokes for zero calls, and it is now checked by nobody.
  //      - **The real-`ls` proof.** A marker file written on beta's disk
  //        and then found on screen was this run's one end-to-end
  //        assertion that a typed path really crosses the wire.
  //
  //      A 「打开一个目录」 button was drafted for the workspace landing
  //      and never built, and its catalog key was deleted rather than
  //      left waiting. **2026-09-14 reversed the reason, not the
  //      deletion**: a workspace *is* something a person makes, so the
  //      landing has a 「+」 again — disabled until 工作区批② builds the
  //      picker behind it. **When it lands, the box belongs here again
  //      and so does the hook.**

  // 24c-bis) git across the wire (工作区批3). **The only assertion in
  //      this repo that the two git ops reach another machine at all**:
  //      `khor_node::git`'s own tests read the checkout this process is
  //      compiled inside, and `check-face` paints a fixture. Neither
  //      puts a frame on a connection.
  //
  //      A real repository, made with real `git` — the point is that
  //      khor's reader and git agree, so a fixture khor wrote itself
  //      would be khor agreeing with khor. Read from **alpha**, about
  //      **beta's** disk, so the answer crosses two nodes.
  const gitDir = join(SCRATCH, "git-wire");
  mkdirSync(gitDir, { recursive: true });
  const git = (...a) =>
    execFileSync("git", a, {
      cwd: gitDir,
      encoding: "utf8",
      // Named here rather than left to whoever runs this: a machine
      // with no `user.email` configured cannot commit, and the failure
      // would read as khor's.
      env: { ...process.env, GIT_AUTHOR_NAME: "smoke", GIT_AUTHOR_EMAIL: "smoke@example.invalid", GIT_COMMITTER_NAME: "smoke", GIT_COMMITTER_EMAIL: "smoke@example.invalid" },
    });
  git("init", "-q", "-b", "main");
  writeFileSync(join(gitDir, "kept.txt"), "one\ntwo\nthree\n");
  //      Committed and then left alone. **Its absence from the answer is
  //      an assertion, so it has to exist** — checking that a file khor
  //      was never given is missing would pass against a reader that
  //      answers nothing at all.
  writeFileSync(join(gitDir, "untouched.txt"), "still\n");
  git("add", "-A");
  git("commit", "-q", "-m", "first");
  //      One file edited in the middle and one file git has never seen:
  //      two of the four letters, and the two hunk shapes that carry
  //      old lines and no old lines.
  writeFileSync(join(gitDir, "kept.txt"), "one\nTWO\nthree\n");
  writeFileSync(join(gitDir, "new.txt"), "x\n");

  const gitStatus = cli(envA, "git", "status", "beta", gitDir).split("\n");
  if (gitStatus[0] !== "main") {
    throw new Error(`the branch did not come back first: ${JSON.stringify(gitStatus.slice(0, 3))}`);
  }
  for (const want of ["M\tkept.txt", "U\tnew.txt"]) {
    if (!gitStatus.includes(want)) {
      throw new Error(`status is missing ${JSON.stringify(want)}: ${JSON.stringify(gitStatus)}`);
    }
  }
  //      …and the file nobody touched is absent, which is the half that
  //      says these are marks and not a listing.
  if (gitStatus.some((l) => l.endsWith("\tuntouched.txt"))) {
    throw new Error("status marked a file that matches HEAD");
  }

  //      The hunk's coordinates are the file's **current** line numbers,
  //      and the old line rides with it. Both halves asserted, because
  //      a reader that found the change and lost what was there is the
  //      failure the gutter's expansion exists to rule out.
  const gitDiff = cli(envA, "git", "diff", "beta", join(gitDir, "kept.txt")).split("\n");
  if (!gitDiff.includes("@@ 2,1 @@") || !gitDiff.includes("-two")) {
    throw new Error(`the diff did not find line two: ${JSON.stringify(gitDiff)}`);
  }
  //      A file git has never seen is one insertion with nothing
  //      removed — not an error, and not an empty answer either.
  const gitNew = cli(envA, "git", "diff", "beta", join(gitDir, "new.txt")).split("\n").filter(Boolean);
  if (gitNew.length !== 1 || gitNew[0] !== "@@ 1,1 @@") {
    throw new Error(`an untracked file did not read as one whole insertion: ${JSON.stringify(gitNew)}`);
  }

  // 24c-ter) writing a file across the wire (工作区批4), and the
  //      refusal that protects it. Same repository 24c-bis made, so the
  //      two halves of a workspace — reading git and writing a file —
  //      are proved against one real checkout on beta's disk.
  //
  //      **The refusal is the half worth the assertion.** That a write
  //      writes is easy to believe; that a *stale* write is refused
  //      whole, with the other person's bytes still on disk afterwards,
  //      is the claim the whole digest exists for, and it is the one
  //      that would rot silently.
  const mine = join(SCRATCH, "mine.txt");
  writeFileSync(mine, "edited over here\n");
  cli(envA, "put", "beta", mine, join(gitDir, "kept.txt"));
  if (readFileSync(join(gitDir, "kept.txt"), "utf8") !== "edited over here\n") {
    throw new Error("put did not land the bytes it was given");
  }
  //      …and nothing is left beside it. The dot-name is a crash
  //      artefact, never a resting state (`khor_node::files`).
  const beside = readdirSync(gitDir).filter((n) => n.startsWith(".khor-save-"));
  if (beside.length) {
    throw new Error(`put left ${JSON.stringify(beside)} beside the file it wrote`);
  }
  //      Now somebody changes it under the writer. `put` reads first
  //      and pins to what it read, so the window is small — it is
  //      opened by hand here, by pinning to a digest that is already
  //      wrong: `--new` claims the path is free, and it is not.
  writeFileSync(mine, "and mine on top\n");
  let refused = null;
  try {
    cli(envA, "put", "--new", "beta", mine, join(gitDir, "kept.txt"));
  } catch (e) {
    refused = String(e.stderr || e);
  }
  if (refused === null) {
    throw new Error("a write pinned to the wrong digest was allowed — nothing here protects a file");
  }
  if (readFileSync(join(gitDir, "kept.txt"), "utf8") !== "edited over here\n") {
    throw new Error("the refused write changed the file anyway, which is the worst of both");
  }

  // 24c-quater) VS Code across the wire (工作区批5).
  //
  //      **Detection is asserted unconditionally; the launch only when
  //      detection says there is something to launch.** A machine with
  //      no `code` is a legitimate machine, and a run on one must not
  //      fail — but it must not silently skip either, so which branch
  //      ran is printed.
  let vs = null;
  try {
    vs = cli(envA, "code", "beta", gitDir);
  } catch (e) {
    const why = String(e.stderr || e);
    // The one refusal that is a fact about the machine rather than a
    // failure of this run.
    if (!why.includes("没有 VS Code")) throw new Error(`khor code failed: ${why}`);
    console.log("vscode: beta has no `code`, so the launch half of this section did not run");
  }
  if (vs !== null) {
    const url = vs.split("\n")[0].trim();
    if (!/^http:\/\/127\.0\.0\.1:\d+\/\?tkn=[0-9a-f]+&folder=/.test(url)) {
      throw new Error(`khor code printed something that is not a token-carrying link: ${url}`);
    }
    //    The link is the assertion. **Both halves**: without the token
    //    the server must refuse, with it it must let you in — one of
    //    those alone passes against a server that answers everything or
    //    nothing.
    const bare = url.slice(0, url.indexOf("?"));
    const shut = await fetch(bare).then((r) => r.status).catch(() => 0);
    if (shut !== 403) {
      throw new Error(`VS Code answered ${shut} without a token, wanted 403 — the token gates nothing`);
    }
    const jar = [];
    const first = await fetch(url, { redirect: "manual" });
    for (const c of first.headers.getSetCookie?.() ?? []) jar.push(c.split(";")[0]);
    if (first.status !== 302) {
      throw new Error(`the token link answered ${first.status}, wanted 302`);
    }
    const open = await fetch(bare, { headers: { cookie: jar.join("; ") } });
    if (open.status !== 200) {
      throw new Error(`after the token, the editor answered ${open.status}, wanted 200`);
    }
    console.log(`vscode: ${bare} refuses without a token (403), redirects with one (302), then serves (200)`);
    //    …and it stops, whole. `code` is a shell script that execs a
    //    launcher that spawns the server, so「it stopped」 has to be
    //    asked of the port rather than of the handle khor holds.
    cli(envA, "code", "--stop", "beta");
    await new Promise((r) => setTimeout(r, 1500));
    const gone = await fetch(bare).then((r) => r.status).catch(() => 0);
    if (gone !== 0) {
      throw new Error(`after --stop the editor still answered ${gone} — the process tree outlived the handle`);
    }
    console.log("vscode: --stop took the whole tree with it");
  }

  // 24c-quinquies) documents and pictures across the wire (工作区批6).
  //
  //      **The read is the assertion, not the render.** What is new
  //      here is that a file bigger than one slice arrives whole — the
  //      slice is 256 KB and khor loops to 4 MB — so the file written
  //      below is deliberately over that line. A preview that silently
  //      handed back the first 256 KB of a picture would draw a
  //      half-decoded image, and the failure would look like a corrupt
  //      file rather than like khor.
  //      **Few long rows, not many short ones**, and the first attempt
  //      got that wrong: a 400 KB csv of twelve-byte rows is 33 000
  //      rows, and the table's own 2000-row cap cut it long before the
  //      slice did — so the assertion below went red about the read
  //      while the read was fine. 1500 rows of 300 bytes is over one
  //      slice, so only one thing is being measured.
  //
  //      ⚠️ **The table is gone** (工作区批⑤, 2026-09-15). A csv is a
  //      text file now: `khor read` prints it, and the screen builds
  //      the table from the buffer it is showing. So the row cap the
  //      paragraph above worked around no longer exists, and what this
  //      asserts is what it was always really about — **a file over one
  //      slice arrives whole**. Kept as a csv rather than moved to some
  //      other format because the two ends being checked below are the
  //      first line and the last, and a csv makes both nameable.
  const bigCsv = join(SCRATCH, "wide.csv");
  {
    const pad = "x".repeat(290);
    let rows = "a,b,c\n";
    for (let i = 0; i < 1500; i++) rows += `${i},${pad},y\n`;
    writeFileSync(bigCsv, rows);
  }
  const csvOut = cli(envA, "read", "beta", bigCsv);
  if (!csvOut.startsWith("a,b,c\n")) {
    throw new Error(`a csv over one slice did not come back as itself: ${csvOut.slice(0, 80)}`);
  }
  //      …and the far end of it arrived too, which is the half that a
  //      one-slice read would have failed.
  const lastRow = readFileSync(bigCsv, "utf8").trim().split("\n").pop().split(",")[0];
  if (!csvOut.includes(`\n${lastRow},`)) {
    throw new Error(`the last row of a ${Math.round(statSync(bigCsv).size / 1024)} KB csv is missing — the read stopped at one slice`);
  }
  //      The cut is said rather than silent, on a file past the ceiling.
  //      **Written here rather than assumed**: 4 MB is the number the
  //      app and the verb share, and a ceiling nobody tests is a
  //      ceiling that moves.
  const huge = join(SCRATCH, "huge.csv");
  writeFileSync(huge, `h\n${"0123456789,\n".repeat(400_000)}`);
  // **`maxBuffer`, and it is not a nicety.** node caps a captured
  // stdout at 1 MB and kills the child when it goes over — so the
  // moment a csv stopped being folded into a 2000-row table (工作区
  // 批⑤) this call started reading 4 MB, the process was killed before
  // it could print the cut line, and the assertion below went red about
  // khor for node's buffer. The ceiling is khor's own `LOOK`; the
  // buffer has to be over it.
  const big = { env: envA, encoding: "utf8", timeout: 60_000, maxBuffer: 16 * 1024 * 1024 };
  let cutWord = "";
  try {
    execFileSync(KHOR, ["read", "beta", huge], big);
  } catch {
    /* a big read may exit non-zero on a broken pipe; the word is what matters */
  }
  cutWord = spawnSync(KHOR, ["read", "beta", huge], big).stderr ?? "";
  if (!cutWord.includes("只读了前")) {
    throw new Error(`a file past the 4 MB ceiling did not say it was cut: ${JSON.stringify(cutWord.slice(0, 120))}`);
  }
  console.log(`preview: a ${Math.round(statSync(bigCsv).size / 1024)} KB csv came back whole, and a 5 MB one says it was cut`);

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
  //      ⚠️ **Tombstone: the exit chip's three assertions, 2026-09-11.**
  //      Pressing Enter here and then checking for
  //      `[data-chip="dev:beta"]`, and then removing it — all three
  //      named a chip that was taken out of this box on 2026-09-09
  //      together with the files landing's (`AddressBar`'s
  //      `chosen={[]}` carries the reasoning: with one exit per landing
  //      the machine is not a property of the address being typed).
  //
  //      **Both sections went red at the same commit and neither was
  //      noticed, which is the three-step rule in this repo's CLAUDE.md
  //      with the third step missed**: the chip was removed, its
  //      readers in the product were removed, and nobody grepped for
  //      who else was pointing at it. A gate is a reader.
  //
  //      What survives is the assertion above — the box offers the
  //      machine as an exit — and that is the half this section can
  //      still prove.

  // 24b) the browser landing keeps pages: a pin made against the real
  //      webpins table shows up among this landing's places, survives a
  //      reload (it is the network's, not this screen's), and goes when
  //      it is unpinned.
  //
  //      **Made through the backend op, not by clicking pin.** The pin
  //      control (`[data-pin-web]`) lives on the *opened* page now, and
  //      opening dials a real borrow through somebody's actual browser —
  //      the same reason 24d stops short of pressing Enter. So this item
  //      calls the very op that button calls (`pin_web`, over the bridge
  //      that is one HTTP skin on the real node) and then asks the
  //      screen what it shows. Nothing here is mocked; what is skipped
  //      is the dial.
  //
  //      **It replaces an item that could not run.** The old one was
  //      written against the pre-06f5757 browser pane — machine rows, a
  //      `[data-web-address]` box, a `[data-web-pin-open]` list — and
  //      two of those three attributes no longer exist anywhere in the
  //      source. A locator that matches nothing reads as "the app is
  //      broken" when it is positive and passes for free when it is
  //      negative; either way this feature went unguarded from the day
  //      the shell landed.
  const bridgeOp = async (cmd, args) => {
    const r = await fetch(`http://127.0.0.1:${BRIDGE_PORT}/${cmd}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(args),
    });
    if (!r.ok) throw new Error(`${cmd} answered ${r.status}`);
    return r.json().catch(() => null);
  };
  const webUrl = "https://smoke.example/page";
  await openLanding("browser");
  //      **Either marker proves the places view drew**, and it has to be
  //      either: `[data-new-tab]` is the list, and a landing with
  //      nothing kept yet paints `[data-no-places]` instead — which is
  //      exactly the state this item starts in. Waiting for the list
  //      alone is an assertion that cannot hold before the thing it is
  //      about exists (it did not, first time out).
  await until("the browser landing's places", 10_000, async () =>
    (await page.locator("[data-new-tab], [data-no-places]").count()) >= 1,
  );
  //      The control: not there before it is pinned, so what follows is
  //      about the pin rather than about the selector.
  if ((await page.locator(`[data-place="${webUrl}"]`).count()) !== 0) {
    throw new Error("that page was already pinned before this item pinned it");
  }
  await bridgeOp("pin_web", { machine: "beta", url: webUrl, on: true });
  //      **Reload, and that is the claim rather than a workaround.**
  //      This landing asks for what is kept when it mounts (and again
  //      after it visits somewhere); nothing pushes a pin at an open
  //      screen, and asserting that one appeared without a reload would
  //      be asserting a live update the app does not do and has never
  //      claimed. What it does claim is that the pin belongs to the
  //      network and not to this screen — so the next time any screen
  //      asks, it is there. (Reload keeps the ?bridge= query.)
  await page.reload();
  await openLanding("browser");
  //      **Two locators, because the heading is the row's sibling and
  //      not its parent** — `[data-places-section] [data-place]` matches
  //      nothing at all, which reads exactly like "the pin never
  //      arrived" (`.nav-badge`'s lesson, again).
  await until("the pinned page among the places", 15_000, async () =>
    (await page.locator(`[data-place="${webUrl}"]`).count()) === 1,
  );
  if ((await page.locator('[data-places-section="pinned"]').count()) !== 1) {
    throw new Error("the pinned page arrived without the heading that says it is pinned");
  }
  await bridgeOp("pin_web", { machine: "beta", url: webUrl, on: false });
  await page.reload();
  await openLanding("browser");
  //      **The view first, the absence second, and that order is the
  //      assertion's spine.** While nobody has answered about the pins
  //      this landing paints nothing at all (`usePlaces.asked` — "no
  //      answer yet" is not "nothing kept"), and an empty screen
  //      satisfies "the pin is gone" for a reason that has nothing to do
  //      with the pin. Written the other way round it read the loading
  //      frame as proof and then tripped over its own probe.
  await until("the places view again", 15_000, async () =>
    (await page.locator("[data-new-tab], [data-no-places]").count()) >= 1,
  );
  if ((await page.locator(`[data-place="${webUrl}"]`).count()) !== 0) {
    throw new Error("the page is still among the places after it was unpinned");
  }

  // 29) the mark opens the mesh and what it cost — on both faces.
  //
  //     The narrow half is the point of it: with the map living in the
  //     devices pane's detail, a phone can never see it (that pane shows
  //     the list, and picking a machine shows that machine's card). This
  //     is the only way in from there, so it is checked there first.
  //
  //     What the map says is compared against the node's own listing —
  //     the independent witness, and **no number is written in this
  //     file**: both sides are read and set against each other.
  await page.setViewportSize({ width: 390, height: 720 });
  await page.locator("[data-rail-mark]").click();
  await until("the mesh on the narrow face", 15_000, async () =>
    (await page.locator("[data-mark-place] [data-mandala-map]").count()) === 1,
  );
  //     **The daily ledger is gone from this place and is not asserted
  //     anywhere else.** It used to sit beside the map and be compared,
  //     day by day, against `khor usage` — the independent witness this
  //     item was built around. That screen no longer exists in the GUI
  //     (`khor usage` still prints it), and moving the assertion
  //     somewhere else would be leaving a gate on a thing that is not
  //     there. What replaces the witness is below: the session count on
  //     the map is compared against the node's own list.
  //
  //     The anchor stays `[data-mandala-map]` rather than the quota
  //     panel: these homes have no Claude login, quota answers nothing,
  //     and that section does not draw at all — waiting on it would fail
  //     as "probe dead" for a reason that has nothing to do with this.
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
  //     **Every face here is pressable, and this assertion is the old
  //     one turned over rather than deleted.** It used to demand that
  //     none of them were: there was no card in this space to open into,
  //     and an affordance that answers nothing teaches people to stop
  //     trying the ones that do. There is a card now, so the fact worth
  //     guarding is the opposite one — and a fact that has been
  //     overturned is guarded by asserting the new one, not by removing
  //     the old.
  const seatsHere = await page
    .locator("[data-mark-place] [data-seat]")
    .evaluateAll((els) => els.map((e) => e.tagName));
  if (seatsHere.length === 0) {
    throw new Error("probe dead: no seats on the mark's place to press");
  }
  if (seatsHere.some((t) => t !== "BUTTON")) {
    throw new Error(`a face here answers nothing: ${JSON.stringify(seatsHere)}`);
  }
  //     …and pressing one really opens **that** machine's card, and the
  //     card carries what that machine is holding. Both halves are read
  //     off the screen: the name the card prints against the name on the
  //     seat that was pressed, so "a card opened" and "the right card
  //     opened" are not the same pass; and the 全部 count, which is the
  //     four counts that used to hang off one node standing for the
  //     whole mesh, after they were moved onto the machine they are
  //     about (「会话是在机器节点点击后看的,不是在地图外面」 — the
  //     user, 2026-08-28).
  //
  //     **The independent witness is the sum.** Each card counts one
  //     machine's rows; `khor sessions --all` on the same home counts
  //     every row from the other side. No number is written in this
  //     file — both sides are read, and a scoping bug in either
  //     direction (a card showing the whole table, or a machine's rows
  //     landing on nobody) moves the sum off the total.
  const seatIds = await page
    .locator("[data-mark-place] [data-seat]")
    .evaluateAll((els) => els.map((e) => e.getAttribute("data-seat")));
  let summed = 0;
  for (const id of seatIds) {
    const seatName = (
      await page.locator(`[data-mark-place] [data-seat="${id}"] [data-seat-name]`).innerText()
    ).trim();
    await page.locator(`[data-mark-place] [data-seat="${id}"]`).click();
    await until(`the card for ${seatName}`, 10_000, async () =>
      (await page.locator("[data-node-card]").count()) === 1,
    );
    const cardName = (await page.locator("[data-node-card-name]").innerText()).trim();
    if (!cardName.startsWith(seatName)) {
      throw new Error(`pressed ${seatName} and got the card for ${cardName}`);
    }
    //     A machine with nothing on it says so in one sentence rather
    //     than in four zeros, so the two shapes are read as the two
    //     shapes — and exactly one of them may be up.
    const none = (await page.locator("[data-node-card] [data-count-none]").count()) === 1;
    const counted = (await page.locator('[data-node-card] [data-count="all"]').count()) === 1;
    if (none === counted) {
      throw new Error(`${seatName}'s card shows ${none ? "both" : "neither"} the counts and the empty word`);
    }
    if (counted) {
      const n = Number(
        (await page.locator('[data-node-card] [data-count="all"]').innerText()).replace(/\D+/g, ""),
      );
      if (!Number.isFinite(n)) {
        throw new Error(`${seatName}'s card counts something unreadable`);
      }
      summed += n;
    }
    await page.keyboard.press("Escape");
    await until("the card to close", 5_000, async () =>
      (await page.locator("[data-node-card]").count()) === 0,
    );
  }
  const cliRows = cli(envB, "sessions", "--all").split("\n").filter((l) => l.includes("\t")).length;
  if (cliRows === 0) {
    throw new Error("probe dead: khor lists no sessions, so the sum has nothing to disagree with");
  }
  if (summed !== cliRows) {
    throw new Error(`the machine cards count ${summed} sessions between them and \`khor sessions\` lists ${cliRows}`);
  }

  //     **Waiting is said out on the map, without a press.** The counts
  //     went behind a press when they moved onto the machines; this one
  //     fact did not, and may not — it is the one state of the six with
  //     a person on the other end of it, and a screen that has to be
  //     searched for an alarm has no alarm. The rail's badge cannot
  //     stand in: it counts waiting *or* unread and says which of the
  //     two it means nowhere.
  //
  //     **Both directions, on every seat.** An implementation that never
  //     drew the line would satisfy the absent half on its own, so what
  //     is asserted is the agreement — a seat draws the line exactly
  //     when its count is above zero. Whether anything is waiting on
  //     this particular run is the run's business; 26 below creates the
  //     state on purpose and checks the drawn half there.
  const seatMarks = await page.locator("[data-mark-place] [data-seat]").evaluateAll((els) =>
    els.map((e) => ({
      name: e.querySelector("[data-seat-name]")?.textContent.trim() ?? "",
      blocked: Number(e.getAttribute("data-blocked") ?? "-1"),
      drawn: e.querySelector("[data-seat-blocked]") !== null,
    })),
  );
  if (seatMarks.length === 0) {
    throw new Error("probe dead: no seats to read the waiting mark off");
  }
  for (const m of seatMarks) {
    if (m.blocked < 0) {
      throw new Error(`${m.name}'s seat does not carry a waiting count at all`);
    }
    if (m.drawn !== m.blocked > 0) {
      throw new Error(
        `${m.name} counts ${m.blocked} waiting and ${m.drawn ? "draws" : "does not draw"} the line`,
      );
    }
  }
  if (seatMarks.every((m) => m.blocked === 0)) {
    console.log("  note: nothing is waiting on this run — the drawn half of the seat's mark was not exercised here (26 does it)");
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
    (await page.locator("[data-machine-card]").count()) === 1,
  );
  //     …and it is *that* seat's machine. Read once, outside the wait
  //     (`cardId`).
  const seatCard = await cardId();
  if (seatCard !== guest) {
    throw new Error(`pressing seat ${guest} opened the card for ${seatCard}`);
  }
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
    (await page.locator("[data-machine-card] [data-copy-id]").count()) === 1,
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

  // Reading the terminal through its own buffer (`__xterm`, the pane's
  // gate hook): the webgl renderer paints no DOM text, so the buffer is
  // the one honest place the screen's characters live. `termText` is
  // the whole buffer, history included; `termView` is the viewport rows
  // alone — the difference is the whole subject of 24s. Lines arrive
  // right-trimmed (`translateToString(true)`), which is also what a
  // copy hands over.
  const termText = () =>
    page.evaluate(() => {
      const t = document.querySelector("[data-terminal]")?.__xterm;
      if (!t) return "";
      const b = t.buffer.active;
      const out = [];
      for (let i = 0; i < b.length; i += 1) out.push(b.getLine(i)?.translateToString(true) ?? "");
      return out.join("\n");
    });
  const termView = () =>
    page.evaluate(() => {
      const t = document.querySelector("[data-terminal]")?.__xterm;
      if (!t) return "";
      const b = t.buffer.active;
      const out = [];
      for (let i = b.viewportY; i < b.viewportY + t.rows; i += 1)
        out.push(b.getLine(i)?.translateToString(true) ?? "");
      return out.join("\n");
    });
  // Focus lives on xterm's own textarea now — the element an input
  // method composes in, and the one `useKeysTo` aims at.
  const termFocused = () =>
    page.evaluate(
      () => document.activeElement?.classList?.contains("xterm-helper-textarea") === true,
    );

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
  // The terminal's half of 25f: arriving focuses the field, and a word
  // typed at a rail button afterwards reaches the program — its first
  // character through the field's `onInput` road and the rest through
  // the grid's keys, which is why it is a word and not a letter. `cat`
  // echoes it, so the screen is the proof. Waited for, not asserted at
  // once: the dev double-mount tears the pane down and back up, and the
  // second mount is the one that keeps the focus.
  await until("the field focused on arrival", 5_000, termFocused);
  await railItem("sessions").focus();
  if (await termFocused()) {
    throw new Error("probe dead: focus never left the field, so the stray key has nothing to prove");
  }
  // Typed inside the wait, re-parking the focus each round, for the
  // marker loop's reason below: the dev double-mount tears the pane
  // down and back up, and a word whose first letter lands in that gap
  // is a word that never paints whole.
  await until("the word typed at the rail echoed by the terminal", 15_000, async () => {
    await railItem("sessions").focus();
    await page.keyboard.type("stray7");
    await page.keyboard.press("Enter");
    await new Promise((r) => setTimeout(r, 300));
    return (await termText()).includes("stray7");
  });
  // Type inside the wait, re-focusing each round: React's dev double-mount
  // briefly tears the attachment down and back up (chat's hold-count, and
  // the same window), and a keystroke that lands in that gap is dropped.
  // A real user types once the pane has settled; the smoke retries until a
  // keystroke sticks. `cat` echoes each attempt, so one landing paints it.
  const marker = "marker9";
  await until("the typed line painted by the terminal", 15_000, async () => {
    // A click, not `.focus()`: the container is not focusable any more
    // — xterm focuses its own textarea on a press, the way a person
    // lands in a terminal.
    await page.locator("[data-terminal]").click();
    await page.keyboard.type(marker);
    await page.keyboard.press("Enter");
    await new Promise((r) => setTimeout(r, 300));
    return (await termText()).includes(marker);
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
  // **The pane shows the form the host is running, and there is
  // nothing to switch.** This row is hosted in a terminal, so the
  // terminal is what the pane paints — the 终端/对话 strip that used to
  // stand here is gone, and with it the "default face" question it
  // existed to answer.
  await page.locator(`[data-row="${agentId}"] [data-row-open]`).click();
  await until("the terminal, with no switch to press", 15_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  // The negative assertion, and the live probe that earns it: the pane
  // *is* on screen (the line above), so a missing switch is a missing
  // switch and not a page that never rendered.
  if ((await page.locator("[data-view-term]").count()) !== 0) {
    throw new Error("a session has one form; the pane must not offer a face switch");
  }
  if ((await page.locator("[data-view-chat]").count()) !== 0) {
    throw new Error("a session has one form; the pane must not offer a face switch");
  }
  // Leaving and coming back lands on the same thing, because the thing
  // is the host and not a remembered click.
  await page.locator(`[data-row="chat/alpha"] [data-row-open]`).click();
  await until("another detail in between", 10_000, async () =>
    (await page.locator("[data-terminal]").count()) === 0,
  );
  await page.locator(`[data-row="${agentId}"] [data-row-open]`).click();
  await until("the terminal again, unasked", 15_000, async () =>
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
  await until("the tmux screen painted in the app", 20_000, async () =>
    (await termText()).includes("tmux-bridge-marker"),
  );
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
  // **The row says its process sits in tmux** (`Session::mux`, 面跟
  // 宿主批): the fact the detail routes its default face on, ahead of
  // the row's kind. Read off the row before opening it, so a face that
  // opened the terminal for some other reason could not stand in for
  // the wire carrying the fact.
  await until("the tmux-held agent row carrying mux", 20_000, async () =>
    (await page.locator(`[data-row="${tmuxAgentRow}"]`).getAttribute("data-mux")) === "true",
  );
  await page.locator(`[data-row="${tmuxAgentRow}"] [data-row-open]`).click();
  await until("the pane's terminal painted", 20_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  const paneMarker = "pane-marker4";
  await until("a keystroke reaching the real pane", 20_000, async () => {
    await page.locator("[data-terminal]").click();
    await page.keyboard.type(paneMarker);
    await page.keyboard.press("Enter");
    await new Promise((r) => setTimeout(r, 300));
    return (await termText()).includes(paneMarker);
  });

  // **Committed text reaches the program whole.** The composition
  // machinery is xterm's own now — the 229 guards, the half-built-word
  // suppression, the candidate placement all live upstream, where every
  // VS Code user exercises them daily; re-driving its internals with
  // synthetic composition events would be testing the library through
  // the keyhole. What khor still owes a test is its half of the wire:
  // the text an input method finally commits goes out the `term_key`
  // road intact and the echo comes back. `insertText` is CDP's real
  // committed-text input event — the same shape dictation and emoji
  // input arrive as.
  await page.locator("[data-terminal]").click();
  await until("the committed word out and echoed back", 15_000, async () => {
    if (!(await termFocused())) await page.locator("[data-terminal]").click();
    await page.keyboard.insertText("中文");
    await new Promise((r) => setTimeout(r, 300));
    return (await termText()).includes("中文");
  });
  // The input method's side of 25f. The first `keydown` of a syllable
  // (`keyCode 229`, before any `compositionstart`) pressed while a
  // button holds focus must move the focus to the field *during that
  // key*, because it is the one event before the composition picks
  // its target. Synthetic, like the block above, and for the same
  // reason: this proves khor moves the focus on that key; whether
  // Chrome then composes in the newly focused field is Chrome's
  // contract, and it is the same contract the pointer road already
  // leans on.
  const imeAimed = await page.evaluate(() => {
    const field = document.querySelector("[data-terminal] .xterm-helper-textarea");
    const button = document.querySelector('[data-rail-item][data-landing="sessions"]');
    if (!field || !button) return { error: "no field or no rail button to press the key at" };
    button.focus();
    if (document.activeElement !== button) return { error: "probe dead: the button did not take focus" };
    const first = new KeyboardEvent("keydown", { key: "Process", bubbles: true, cancelable: true });
    Object.defineProperty(first, "keyCode", { get: () => 229 });
    button.dispatchEvent(first);
    return { landed: document.activeElement === field };
  });
  if (imeAimed.error) throw new Error(imeAimed.error);
  if (!imeAimed.landed) {
    throw new Error("an input method's opening key pressed at a button must aim the composition at the field");
  }
  // 24m) bracketed paste (批④ 前置, 重写 2026-09-03): a paste is wrapped
  //      when — and only when — the running program asked for it, and
  //      since the xterm pivot the wrapping is xterm's own (DECSET 2004
  //      flows up the byte stream and xterm brackets on `onData`). What
  //      this proves is that khor's pipe carries the mode up and the
  //      paste back down whole.
  //
  //      **Both halves, on two khor-hosted terminals — not tmux.** The
  //      old rig ran these under tmux, and tmux owns the outer screen:
  //      it does not forward a pane's DECSET 2004 to *this* terminal, so
  //      xterm never entered bracketed mode and the positive half could
  //      not be seen (it was marked 「NOT red-proven」for exactly that).
  //      Hosted directly, `printf '\\033[?2004h'` reaches xterm with
  //      nothing in between.
  //
  //      `cat -v` is the reader: it renders every control byte as caret
  //      notation (`ESC`→`^[`) in its *output*, so the marks are visible
  //      deterministically rather than depending on the tty's echo
  //      flags; `stty -echo` silences that echo so only `cat -v`'s
  //      conversion is on screen. The negative half is the one that
  //      matters — a program that never asked must not be handed the
  //      escape — and it is red-provable here: a break that wrapped
  //      unconditionally would put the marks on the plain pane.
  const pasteText = "paste-marker-24m";
  const plainCatId = cli(envB, "open", "-d", "--title", "plaincat", "--", "sh", "-c", "stty echoctl; exec cat").trim();
  const brackCatId = cli(envB, "open", "-d", "--title", "brackcat", "--", "sh", "-c", "printf '\\033[?2004h'; stty echoctl; exec cat").trim();
  for (const [name, cmd] of [["pasteplain", "cat"]]) {
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
  const pasteInto = async (title, wantBracket) => {
    await until(`the ${title} row`, 25_000, async () =>
      (await page.locator(`[data-title="${title}"] [data-row-open]`).count()) === 1,
    );
    await page.locator(`[data-title="${title}"] [data-row-open]`).click();
    await until(`${title}'s terminal`, 20_000, async () =>
      (await page.locator("[data-terminal]").count()) === 1,
    );
    // **Wait for the mode to actually arrive before pasting.** In the
    // byte-stream world the program's DECSET 2004 is one byte in the
    // feed, and the pane writes the attach + the stream a beat after the
    // element mounts — a paste fired the instant `[data-terminal]`
    // exists can beat it (the old cell-poll carried the mode in every
    // whole-screen answer, so this race did not exist). Waiting on
    // xterm's own `modes.bracketedPasteMode` is the honest gate: it is
    // exactly the fact that had to travel up the pipe, so a dropped
    // 2004h times out here instead of failing the mark check below with
    // a confusing message. `wantBracket === false` proves the other
    // way — a program that never asked must never leave the mode set.
    if (wantBracket !== undefined) {
      await until(`${title} reaching bracketed=${wantBracket}`, 20_000, async () =>
        (await page.evaluate(() => document.querySelector("[data-terminal]").__xterm.modes.bracketedPasteMode)) ===
        wantBracket,
      );
    }
    // Driven through xterm's own `paste()` — the public method a real
    // paste event lands in, which applies bracketing per the mode the
    // program set through the byte stream. That the mode was carried up
    // (feed→write) and the wrapped bytes go back down (onData→term_key)
    // is khor's half of the contract and the thing under test.
    await page.evaluate((text) => {
      document.querySelector("[data-terminal]").__xterm.paste(text);
    }, pasteText);
    await until(`${title} echoing the paste`, 20_000, async () =>
      (await termText()).includes(pasteText),
    );
    return termText();
  };
  //      The tty's own echo (immediate, per-character — no stdout
  //      buffer to flush) renders the bytes, and `echoctl` makes ESC
  //      show as `^[`. So a wrapped paste reads as `^[[200~…^[[201~`
  //      and an unwrapped one does not — observable now, unlike the
  //      tmux rig this replaced, and red-provable in both directions.
  const plainEcho = await pasteInto("plaincat", false);
  if (plainEcho.includes("[200~")) {
    throw new Error(`a program that never asked for bracketed paste was sent the marks: ${plainEcho.slice(-120)}`);
  }
  const brackEcho = await pasteInto("brackcat", true);
  if (!brackEcho.includes("[200~") || !brackEcho.includes("[201~")) {
    throw new Error(
      `a program that asked for bracketed paste did not get the marks: ${brackEcho.slice(-120)}`,
    );
  }
  cli(envB, "close", plainCatId);
  cli(envB, "close", brackCatId);
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
    const seen = await termText();
    return seen.includes("'/tmp/a b.txt'") && seen.includes("'/tmp/it'\\''s.txt'");
  });

  // 24q) F 键 (批⑤): a function key reaches the program as the bytes
  //      that program expects.
  //
  //      The terminal in view is a `cat`, so whatever arrives at the tty
  //      is echoed straight back — which makes the assertion about what
  //      **left this app**, not about what it drew. `ESC` echoes as
  //      `^[`, so an F5 that arrived whole reads as `^[[15~`.
  await page.locator("[data-terminal]").click();
  await page.keyboard.press("F5");
  await until("F5 arriving as its own escape sequence", 10_000, async () =>
    (await termText()).includes("^[[15~"),
  );

  // 24r) 选择复制 (批⑤): what lands on a copy is the text, not the
  //      padding. The selection machinery is xterm's now — webgl paints
  //      no DOM text to range over — so the claim is read through its
  //      own selection: select a whole buffer row, the full grid width,
  //      and what `getSelection()` answers must be the line without the
  //      eighty columns a cell grid keeps to its right. The control is
  //      the row having text at all: "no trailing spaces" is otherwise
  //      true of a selection that got nothing.
  const sel = await page.evaluate(() => {
    const t = document.querySelector("[data-terminal]")?.__xterm;
    if (!t) return null;
    const b = t.buffer.active;
    let at = -1;
    for (let i = 0; i < b.length; i += 1) {
      if ((b.getLine(i)?.translateToString(true) ?? "").trim().length > 0) {
        at = i;
        break;
      }
    }
    if (at < 0) return null;
    t.select(0, at, t.cols);
    const picked = t.getSelection();
    t.clearSelection();
    return { picked, want: b.getLine(at).translateToString(true) };
  });
  if (!sel || !sel.picked.trim()) {
    throw new Error("probe dead: nothing selectable reached the terminal at all");
  }
  if (/[ \t]$/.test(sel.picked)) {
    throw new Error(`the copy kept the terminal's padding: ${JSON.stringify(sel.picked)}`);
  }
  if (sel.picked.trimEnd() !== sel.want.trimEnd()) {
    throw new Error(
      `the copy changed more than the padding: ${JSON.stringify(sel.picked)} vs ${JSON.stringify(sel.want)}`,
    );
  }

  // 24s) 回看 (批⑤): what scrolled past is still there, the reader
  //      keeps their place, and there is a way back down.
  //
  //      **On a khor-hosted terminal, and that is not incidental.** The
  //      panes above are tmux sessions, and tmux runs on the *alternate
  //      screen* — which `vt100` gives no scrollback at all
  //      (`Grid::new(size, 0)` for the alternate grid). That is the
  //      right behaviour and matches every real terminal: you do not
  //      scroll a terminal back into tmux, tmux has its own copy mode.
  //      It is also exactly how the first draft of this section fooled
  //      itself — it drove the tmux pane and read "nothing moved" as
  //      the feature being broken. The negative half at the end pins
  //      the tmux case so nobody later "fixes" it.
  const sbRow = cli(envB, "open", "-d", "--title", "sbbox", "--", "cat").trim();
  await until("the khor-hosted terminal in the list", 30_000, async () =>
    (await page.locator(`[data-row="${sbRow}"]`).count()) === 1,
  );
  await page.locator(`[data-row="${sbRow}"] [data-row-open]`).click();
  await until("its terminal open", 30_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  //      Two hundred numbered lines pasted into the `cat` it is running:
  //      it echoes them, so the terminal has more history than screen —
  //      the only state in which any of this means anything.
  const sbLines = Array.from({ length: 200 }, (_, i) => `sb-line-${i + 1}`).join("\n") + "\n";
  // Fed as raw input (`term_key`) — `term_paste` left with the pivot;
  // the reader is a bare `cat`, which never asked for bracketing, so
  // bytes are bytes.
  const pasteVia = async (id, text) =>
    page.evaluate(
      async ([port, sid, body]) => {
        const r = await fetch(`http://127.0.0.1:${port}/term_key`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ id: sid, keys: body }),
        });
        if (!r.ok) throw new Error(await r.text());
      },
      [BRIDGE_PORT, id, text],
    );
  await pasteVia(sbRow, sbLines);
  //      Which of the block's lines are on screen, as numbers — asking
  //      "does it contain sb-line-1" is how the first draft of this
  //      fooled itself a second time: after four pages back the visible
  //      lines were in the fifties, and a pattern written for one- and
  //      two-digit numbers called that "no history". `termView`, not
  //      `termText`: the whole buffer holds every line by design, and
  //      reading it here would call "scrolled" and "not scrolled" one
  //      thing — the viewport is the thing that moves.
  const onScreen = async () =>
    [...(await termView()).matchAll(/sb-line-(\d+)/g)].map((m) => Number(m[1]));
  await until("the tail of the pasted block on screen", 20_000, async () =>
    (await onScreen()).includes(200),
  );
  //      **The control for everything below**: the top of that block is
  //      off the screen. Without it, "scrolling back showed an earlier
  //      line" would also pass on a terminal that never scrolled.
  const atBottom = await onScreen();
  if (atBottom.includes(1)) {
    throw new Error("probe dead: the whole block fits on screen, so there is nothing to scroll back to");
  }
  if ((await page.locator("[data-term-bottom]").count()) !== 0) {
    throw new Error("probe dead: the way-back-down shows while the view is at the bottom");
  }
  await page.locator("[data-terminal]").click();
  for (let i = 0; i < 4; i++) await page.keyboard.press("Shift+PageUp");
  await until("history above the live screen", 20_000, async () => {
    const seen = await onScreen();
    // Moved, and moved *upward*: the newest line is gone and what shows
    // is older than anything that showed before.
    return seen.length > 0 && !seen.includes(200) && Math.min(...seen) < Math.min(...atBottom);
  });
  await until("the way back down offered", 10_000, async () =>
    (await page.locator("[data-term-bottom]").count()) === 1,
  );
  await page.locator("[data-term-bottom]").click();
  await until("the live screen again, and the control gone", 20_000, async () => {
    const seen = await onScreen();
    return seen.includes(200) && (await page.locator("[data-term-bottom]").count()) === 0;
  });

  // 24u) 全功能面 (完整终端 T2, 2026-09-03): the things a terminal
  //      emulator leaves to the window around it — search, links, the
  //      title a program sets, the bell, copy-on-select — on the same
  //      `cat` with its four hundred lines (every pasted line is on the
  //      screen twice: the tty's echo and cat's own output). Each one
  //      is driven through the real road: keys into xterm's textarea,
  //      bytes out of the pty, a pointer on a cell. None through the
  //      addon's API, which is what a gate that could not fail would
  //      read.
  //
  //      (1) ⌘F is the terminal's search. The bar opens with the keys in
  //      it, the word is found, the count is *the* count (compared to
  //      the buffer's own occurrences, so a search that always says
  //      "1 / 1" fails), ⏎ steps, ⎋ hands the keys back.
  const hitsText = () => page.locator("[data-term-find-hits]").innerText();
  await page.locator("[data-terminal]").click();
  await page.keyboard.press("Meta+f");
  await until("the search bar", 5_000, async () => (await page.locator("[data-term-find-input]").count()) === 1);
  if (!(await page.evaluate(() => document.activeElement?.hasAttribute("data-term-find-input")))) {
    throw new Error("the search bar opened without the keys in it");
  }
  await page.keyboard.type("sb-line-42");
  const wantHits = ((await termText()).match(/sb-line-42(?!\d)/g) ?? []).length;
  if (wantHits < 2) throw new Error(`probe dead: sb-line-42 is on the buffer ${wantHits} times`);
  await until(`the count 1 / ${wantHits}`, 10_000, async () => (await hitsText()).trim() === `1 / ${wantHits}`);
  const pickedByFind = await page.evaluate(() => document.querySelector("[data-terminal]").__xterm.getSelection());
  if (pickedByFind !== "sb-line-42") {
    throw new Error(`the search found ${JSON.stringify(pickedByFind)}, not the word`);
  }
  if (!(await onScreen()).includes(42)) {
    throw new Error("the search counted a match the view did not move to");
  }
  await page.keyboard.press("Enter");
  await until(`the count 2 / ${wantHits}`, 10_000, async () => (await hitsText()).trim() === `2 / ${wantHits}`);
  await page.keyboard.press("Shift+Enter");
  await until(`back to 1 / ${wantHits}`, 10_000, async () => (await hitsText()).trim() === `1 / ${wantHits}`);
  //      The control: a word that is not there says so in words, not in
  //      a number and not in nothing — an empty count is what a search
  //      that never ran would show.
  await page.keyboard.press("Meta+a");
  await page.keyboard.type("sb-line-999");
  await until("a no-matches answer", 10_000, async () => {
    const t = (await hitsText()).trim();
    return t.length > 0 && !/\d/.test(t);
  });
  await page.keyboard.press("Escape");
  await until("the bar gone and the keys back", 5_000, async () =>
    (await page.locator("[data-term-find]").count()) === 0 && (await termFocused()),
  );
  //      The search left the view up at the match (iTerm2 leaves it
  //      there too: closing a find is not a scroll), so the way back
  //      down is offered — and everything below reads the *viewport*,
  //      so it is taken first. The first run of this stop timed out on
  //      an address that had arrived fine, four hundred lines under
  //      the screen.
  await until("the way back down after the search", 10_000, async () =>
    (await page.locator("[data-term-bottom]").count()) === 1,
  );
  await page.locator("[data-term-bottom]").click();
  await until("the live screen again", 20_000, async () => (await onScreen()).includes(200));

  //      (2) the title a program sets (OSC 0) is shown; (3) a bell
  //      flashes the frame. Both arrive as bytes through `cat` — the tty
  //      echoes the escape as `^[` and cat writes the real one — and the
  //      bell is caught by an observer armed *before* the byte is sent,
  //      because the flash is shorter than a poll.
  await pasteVia(sbRow, "\x1b]0;smoke-title\x07\n");
  await until("the title on the pane", 10_000, async () =>
    (await page.locator("[data-term-title]").count()) === 1 &&
    (await page.locator("[data-term-title]").innerText()).trim() === "smoke-title",
  );
  await page.evaluate(() => {
    const box = document.querySelector("[data-terminal]");
    window.__bellSeen = false;
    new MutationObserver(() => {
      if (box.getAttribute("data-term-bell") === "true") window.__bellSeen = true;
    }).observe(box, { attributes: true, attributeFilter: ["data-term-bell"] });
  });
  await pasteVia(sbRow, "\x07\n");
  await until("the bell's flash", 10_000, async () => page.evaluate(() => window.__bellSeen === true));

  //      (4) a link opens on ⌘-click and not on a click. The pointer is
  //      put on the URL's own cell (the screen's box divided by the
  //      grid: xterm sizes it to exactly cols × rows cells) — a click
  //      anywhere else is a click on nothing. In a browser the link is
  //      a new page; the control is the plain click opening none.
  await pasteVia(sbRow, "see https://example.com/khor-smoke now\n");
  await until("the address on screen", 10_000, async () => (await termView()).includes("https://example.com/khor-smoke"));
  const urlAt = await page.evaluate(() => {
    const t = document.querySelector("[data-terminal]").__xterm;
    const b = t.buffer.active;
    const screen = document.querySelector("[data-terminal] .xterm-screen").getBoundingClientRect();
    for (let r = 0; r < t.rows; r += 1) {
      const line = b.getLine(b.viewportY + r)?.translateToString(true) ?? "";
      const c = line.indexOf("https://example.com");
      if (c >= 0) {
        return {
          x: screen.left + ((c + 4) * screen.width) / t.cols,
          y: screen.top + ((r + 0.5) * screen.height) / t.rows,
        };
      }
    }
    return null;
  });
  if (!urlAt) throw new Error("probe dead: the address is on screen but on no row");
  //      The pointer is on the link only once xterm says so (it marks
  //      the screen `xterm-cursor-pointer` when a link is under the
  //      pointer — the underline's own class). Waited for before *both*
  //      clicks: without it the plain click below is a click on nothing
  //      and passes for the wrong reason, and the ⌘-click races the
  //      link detection, which is asynchronous.
  const onLink = () =>
    page.evaluate(() => document.querySelector("[data-terminal] .xterm-screen")?.classList.contains("xterm-cursor-pointer") === true);
  const pagesBefore = page.context().pages().length;
  await page.mouse.move(urlAt.x, urlAt.y);
  await until("the pointer on the link", 10_000, onLink);
  await page.mouse.click(urlAt.x, urlAt.y);
  await settle();
  if (page.context().pages().length !== pagesBefore) {
    throw new Error("a plain click on a link opened it — that click is the start of a selection");
  }
  //      ⌘ held through the keyboard, not `click({ modifiers })`: this
  //      playwright's modifier option never reached the mouse events
  //      (a document-level `mouseup` read `metaKey=false` under it,
  //      measured with a probe), which read as "the link ignores ⌘"
  //      until the probe said whose fault it was.
  //      Two halves, told apart on purpose: the pane asking the browser
  //      to open the address (its own `window.open`, wrapped here so
  //      the ask is recorded), and the browser opening it. A blocked
  //      popup and a link that never fired look the same from the
  //      outside and are not the same fault — and only the first half
  //      is the pane's. **The second is not asserted**: measured twice,
  //      headless Chrome opens no page for this `window.open` on the
  //      face (the same call on a bare xterm page does open one), while
  //      headed Chrome on the served face opened the tab on ⌘-click
  //      (2026-09-03, live, `tabs_context` listed it). The popup policy
  //      is the browser's; the ask with the right address is what this
  //      stop can honestly hold.
  await page.evaluate(() => {
    const real = window.open.bind(window);
    window.__opened = null;
    window.open = (u, ...rest) => {
      window.__opened = String(u);
      return real(u, ...rest);
    };
  });
  await page.keyboard.down("Meta");
  await page.mouse.move(urlAt.x + 1, urlAt.y);
  await until("the pointer on the link, ⌘ held", 10_000, onLink);
  const opened = page.context().waitForEvent("page", { timeout: 3_000 }).catch(() => null);
  await page.mouse.click(urlAt.x, urlAt.y);
  await page.keyboard.up("Meta");
  await until("the pane asking to open the address", 10_000, async () =>
    (await page.evaluate(() => window.__opened)) === "https://example.com/khor-smoke",
  );
  const popup = await opened;
  if (popup) await popup.close();

  //      (5) a selection is a copy. The clipboard is seeded first so a
  //      copy that never happened cannot pass on an earlier one.
  await page.evaluate(() => navigator.clipboard.writeText("not-the-selection"));
  await page.evaluate(() => {
    const t = document.querySelector("[data-terminal]").__xterm;
    const b = t.buffer.active;
    for (let i = 0; i < b.length; i += 1) {
      if ((b.getLine(i)?.translateToString(true) ?? "") === "sb-line-42") {
        t.select(0, i, t.cols);
        return;
      }
    }
    throw new Error("probe dead: no row reads sb-line-42 exactly");
  });
  await until("the selection on the clipboard", 10_000, async () =>
    (await page.evaluate(() => navigator.clipboard.readText())).trim() === "sb-line-42",
  );
  await page.evaluate(() => document.querySelector("[data-terminal]").__xterm.clearSelection());

  //      (6) 标签宽度 (2026-09-02:「未选中固定长度,选中了变宽来显示
  //      完整标签」): a second hosted session with a name longer than a
  //      stub. In front, its tab is wider than a stub and its whole
  //      name shows; the tab that stepped back is exactly a stub.
  const longName = "stretch-the-tab-well-past-the-stub-width";
  const stretchRow = cli(envB, "open", "-d", "--title", longName, "--", "cat").trim();
  await until("the long-named session in the list", 30_000, async () =>
    (await page.locator(`[data-row="${stretchRow}"]`).count()) === 1,
  );
  await page.locator(`[data-row="${stretchRow}"] [data-row-open]`).click();
  await until("its tab in front", 20_000, async () =>
    (await page.locator(`[data-tab="${stretchRow}"][data-on="true"]`).count()) === 1,
  );
  const tabW = await page.evaluate(
    ([onId, offId]) => {
      const w = (id) => document.querySelector(`[data-tab="${id}"]`).getBoundingClientRect().width;
      const label = document.querySelector(`[data-tab="${onId}"] [data-tab-label]`);
      const stub = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--tab-w"));
      return { on: w(onId), off: w(offId), stub, clipped: label.scrollWidth > label.clientWidth };
    },
    [stretchRow, sbRow],
  );
  if (Math.abs(tabW.off - tabW.stub) > 1) {
    throw new Error(`a tab not in front is ${tabW.off} wide, not the stub's ${tabW.stub}`);
  }
  if (tabW.on <= tabW.stub + 40) {
    throw new Error(`the tab in front is ${tabW.on} wide for a ${longName.length}-character name — it did not grow`);
  }
  if (tabW.clipped) throw new Error("the tab in front still truncates its name");

  //      (6b) 标签页改名 (2026-09-11:「session 标签页不可双击重命名,
  //      我需要和机器一样」). Renaming from the tab, and read back from
  //      the **list row** — one session, one name, and the list is the
  //      surface that did not take the gesture, so agreeing there is
  //      what says the write went to the row rather than to the strip's
  //      own memory of a label.
  //
  //      **`data-tab-label` is still the hook above**, and that is the
  //      arm with teeth: it now sits on the editor's outer box because
  //      that is what clips. Put it on a wrapper instead and (6)'s
  //      truncation check goes quietly dead — two nested truncating
  //      boxes clip at the same width, so the outer can never report
  //      clipping.
  const tabName = page.locator(`[data-tab="${stretchRow}"] [data-session-title]`);
  if ((await tabName.count()) !== 1) throw new Error("probe dead: the tab in front has no editable name");
  await tabName.dblclick();
  const tabBox = page.locator(`[data-tab="${stretchRow}"] [data-title-input]`);
  await until("the tab's name editor", 5_000, async () => (await tabBox.count()) === 1);
  await tabBox.fill("从标签页改的");
  await tabBox.press("Enter");
  await until("the list row to wear the tab's new name", 15_000, async () =>
    (await page.locator(`[data-row="${stretchRow}"]`).innerText()).includes("从标签页改的"),
  );
  //      **The control arm, and it is proven alive before it is
  //      believed.** A browsing tab hands in no rename, so it must not
  //      grow an editor — the arm against 「tabs are renamable」, which
  //      would put one on a name that is the page's rather than
  //      anybody's to override. Asserted on the browser landing, where
  //      the tabs are browsing tabs; asserting it here, on a strip made
  //      only of session tabs, would be a count of zero for a reason
  //      that has nothing to do with the rule.
  //
  //      **It read the files landing until 2026-09-11** (工作区替代文件).
  //      That landing left the rail with the shell's disk kind, and the
  //      browser is now the only one that browses — the rule being
  //      checked is the browsing tab's, not the disk's, so it moved with
  //      the tabs rather than going on the ledger.
  //
  //      First that there *are* tabs there — a landing that drew none
  //      answers 「no editors」 the same way a correct one does, and
  //      that is the shape this file has been bitten by before.
  await openLanding("browser");
  const browseTabs = await page.locator("[data-browse-landing] [data-tab]").count();
  if (browseTabs === 0) throw new Error("probe dead: the browser landing drew no tabs to check");
  const strays = await page.locator("[data-browse-landing] [data-tab] [data-session-title]").count();
  if (strays !== 0) throw new Error(`${strays} of ${browseTabs} browsing tab(s) grew a rename editor`);
  await openLanding("sessions");
  await until("back on the session strip", 15_000, async () =>
    (await page.locator(`[data-tab="${stretchRow}"]`).count()) === 1,
  );

  cli(envB, "close", stretchRow);
  await until("the stretched tab gone", 20_000, async () =>
    (await page.locator(`[data-tab="${stretchRow}"]`).count()) === 0,
  );
  cli(envB, "close", sbRow);

  //      …and the tmux pane has none of this, on purpose. Asserted so
  //      that the difference stays a decision rather than becoming a
  //      bug report — and proven live first, because "the control did
  //      not appear" is also what a terminal that never got the key
  //      would show.
  await page.locator(`[data-title="pasteplain"] [data-row-open]`).click();
  await until("the tmux pane's terminal", 20_000, async () =>
    (await page.locator("[data-terminal]").count()) === 1,
  );
  await pasteVia(await page.locator('[data-title="pasteplain"]').getAttribute("data-row"), sbLines);
  await until("the tmux pane holding the same block", 20_000, async () =>
    (await termText()).includes("sb-line-200"),
  );
  await page.locator("[data-terminal]").click();
  for (let i = 0; i < 4; i++) await page.keyboard.press("Shift+PageUp");
  await settle();
  if ((await page.locator("[data-term-bottom]").count()) !== 0) {
    throw new Error("a terminal on the alternate screen must not offer a scrollback it does not have");
  }
  if (!(await termText()).includes("sb-line-200")) {
    throw new Error("the tmux pane's view moved — it has no history to move into");
  }

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
    const seen = flat(await termText());
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
    (await page.locator("[data-terminal]").count()) === 1,
  );

  // 24k) 转成对话 (接管, 批C): **the row's menu**, not a button inside
  //      the pane. Changing what a session *is* belongs beside 置顶 and
  //      关闭, which also change a row rather than look at one — the
  //      pane now paints the form the host runs and offers nothing.
  //
  //      The confirm warns, and the go moves the conversation's body:
  //      the TUI process dies (the pane WAS the process, so the tmux
  //      session goes with it), the same row is reborn as a live
  //      conversation under the same vendor uuid, and speaking works —
  //      the resumed fake claude answers. This also ends the viewer host
  //      stood up above: the takeover cleans the bridge before it
  //      re-seats the row.
  await openRowMenu(tmuxAgentRow);
  await until("转成对话 on a hosted claude's menu", 10_000, async () =>
    (await page.locator('[data-menu-item="to-a-chat"]').count()) === 1,
  );
  // Two presses, and the first one must not act: this ends a process on
  // another machine's terms, so the sentence has to be read first
  // (`RowMenu`'s confirm). The armed state is the proof the first press
  // armed rather than ran.
  await page.locator('[data-menu-item="to-a-chat"]').click();
  await until("the confirm with its warning", 10_000, async () =>
    (await page.locator('[data-menu-confirm="to-a-chat"]').count()) === 1,
  );
  if ((await page.locator('[data-menu-item="to-a-chat"][data-armed="true"]').count()) !== 1) {
    throw new Error("the first press must arm the item, not run it");
  }
  await page.locator('[data-menu-item="to-a-chat"]').click();
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
  // 25f) the keys go where the attention is (`useKeysTo`, 2026-09-01:
  //      「每次打字，都在选中 session 列表或侧边栏」). Two roads, both
  //      asserted on *where the character landed* rather than on
  //      `activeElement` alone: arriving focuses the box, and a key
  //      pressed at a rail button afterwards lands in the box — the
  //      real key path, playwright's keyboard through Chrome's own
  //      default action, not a synthetic event aimed at the handler.
  //      Parked on a rail button because that is where the complaint
  //      said focus sat, and because a button is not somewhere text
  //      could have gone by itself.
  await until("the box focused on arrival", 5_000, () =>
    page.evaluate(() => {
      if (document.activeElement?.hasAttribute("data-chat-input") === true) return true;
      // Name who has it instead, and what stands on top — the same
      // two facts the miss below reports, because「not the box」on its
      // own says nothing about which pane took the keys.
      const tell = (el) =>
        el.tagName + [...el.attributes].map((x) => `[${x.name}=${x.value.slice(0, 40)}]`).join("");
      const a = document.activeElement;
      const onTop = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]')].map(tell);
      throw new Error(
        `focus is on ${a ? tell(a) : "nothing"}; on top: ${JSON.stringify(onTop)}; ` +
          `fine pointer: ${window.matchMedia("(hover: hover) and (pointer: fine)").matches}; ` +
          `boxes: ${document.querySelectorAll("[data-chat-input]").length}`,
      );
    }),
  );
  await railItem("sessions").focus();
  if (await page.evaluate(() => document.activeElement?.hasAttribute("data-chat-input"))) {
    throw new Error("probe dead: focus never left the box, so the stray key has nothing to prove");
  }
  await page.keyboard.type("q");
  {
    const landed = await page.evaluate(() => ({
      focused: document.activeElement?.hasAttribute("data-chat-input") === true,
      value: document.querySelector("[data-chat-input]")?.value ?? null,
      // Who has the keys instead, and what is standing on top: the
      // hook gives way to any dialog, menu or listbox in the document
      // (`use-keys-to` ON_TOP), so a stray one left mounted by another
      // pane is the first thing to name when this goes red.
      active: (() => {
        const a = document.activeElement;
        if (!a) return null;
        return a.tagName + [...a.attributes].map((x) => `[${x.name}=${x.value.slice(0, 40)}]`).join("");
      })(),
      onTop: [...document.querySelectorAll('[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]')].map(
        (el) => el.tagName + [...el.attributes].map((x) => `[${x.name}=${x.value.slice(0, 40)}]`).join(""),
      ),
      boxes: document.querySelectorAll("[data-chat-input]").length,
    }));
    if (!landed.focused || landed.value !== "q") {
      throw new Error(`a key typed at the rail did not land in the box: ${JSON.stringify(landed)}`);
    }
  }
  await page.locator("[data-chat-input]").fill("");
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


  // 25b-2) 常去的地方 (工作区批, 2026-09-01): the wizard's first
  //        question, answered before it is asked. The list is
  //        **derived** from the vendors' own records — nobody creates
  //        or names a place — so this plants records and reads the
  //        dialog, which is the only way to tell a derivation from a
  //        list somebody typed.
  //
  //        Three claims, and the middle one is the design:
  //
  //         1. a recorded working directory shows up as a place;
  //         2. **two depths of one checkout are ONE place**, with the
  //            counts summed — the reason the key is the project root
  //            and not the cwd (measured on the author's machine: the
  //            same tree was open at two depths at once);
  //         3. a place whose directory is gone is not offered, because
  //            starting a session there cannot work.
  //
  //        Claim 3 is also the control arm for claims 1 and 2: without
  //        it, a section that simply printed every recorded cwd would
  //        pass them both.
  const placeRepo = join(SCRATCH, "place-repo");
  mkdirSync(join(placeRepo, ".git"), { recursive: true });
  mkdirSync(join(placeRepo, "apps/deep"), { recursive: true });
  const placeGone = join(SCRATCH, "place-that-was-deleted");
  const plantPlace = (slug, uuid, cwd) => {
    const dir = join(B, ".claude", "projects", slug);
    mkdirSync(dir, { recursive: true });
    writeFileSync(
      join(dir, `${uuid}.jsonl`),
      JSON.stringify({ type: "user", cwd, message: { role: "user", content: "x" } }) + "\n",
    );
  };
  plantPlace("pl-root", "aaaaaaa1-0000-4000-8000-000000000001", placeRepo);
  plantPlace("pl-deep", "aaaaaaa1-0000-4000-8000-000000000002", join(placeRepo, "apps/deep"));
  plantPlace("pl-gone", "aaaaaaa1-0000-4000-8000-000000000003", placeGone);

  await openLanding("sessions");
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="new"]').click();
  await until("常去的地方 offering the planted checkout", 20_000, async () =>
    (await page.locator(`[data-place="${placeRepo}"]`).count()) === 1,
  );
  const placesSaid = await page.evaluate(() => ({
    rows: [...document.querySelectorAll("[data-place]")].map((e) => ({
      path: e.getAttribute("data-place"),
      text: e.innerText.replace(/\s*\n\s*/g, " | "),
    })),
  }));
  const deep = placesSaid.rows.filter((r) => r.path.startsWith(placeRepo));
  if (deep.length !== 1) {
    throw new Error(
      `two depths of one checkout must be one place, got ${JSON.stringify(deep)}`,
    );
  }
  if (!deep[0].text.includes("2 ")) {
    throw new Error(`both depths must be counted into it: ${deep[0].text}`);
  }
  if (placesSaid.rows.some((r) => r.path === placeGone)) {
    throw new Error("a place whose directory is gone must not be offered to start in");
  }
  // Picking one is the whole point: it fills the box the create button
  // reads, absolute, so nothing downstream has to expand a `~`.
  await page.locator(`[data-place="${placeRepo}"]`).click();
  const picked = await page.locator("[data-new-session-dir]").inputValue();
  if (picked !== placeRepo) {
    throw new Error(`picking a place must fill 目录 with it: ${picked}`);
  }
  await page.keyboard.press("Escape");
  await until("the places dialog closed", 10_000, async () =>
    (await page.locator("[data-new-session-dialog]").count()) === 0,
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
  // 25c-2) what can be set, through the shim: the menu lists claude's
  //        four modes, picking one crosses the wire as the control
  //        request the fake answers (`set_permission_mode`), and the
  //        menu follows claude's answer. Asserted on the fake's own
  //        mark — what it was put in — and on the menu's face after
  //        the round trip, never on the press: a menu that flipped for
  //        its own click would pass a test that only read the menu.
  await until("the mode menu on the wizard's conversation", 10_000, async () =>
    (await page.locator("[data-chat-mode-menu]").count()) === 1,
  );
  if ((await page.locator("[data-chat-mode-menu]").getAttribute("data-mode-id")) !== "default") {
    throw new Error("probe dead: the session did not open in claude's default mode, so a change proves nothing");
  }
  await page.locator("[data-chat-mode-menu]").click();
  await until("claude's four modes listed", 5_000, async () =>
    (await page.locator("[data-mode-option]").count()) === 4,
  );
  await page.locator('[data-mode-option="plan"]').click();
  await until("the fake put in plan mode, by its own mark", 10_000, async () =>
    existsSync(join(wizDir, "fake.mode")) && readFileSync(join(wizDir, "fake.mode"), "utf8") === "plan",
  );
  await until("the menu wearing 只看不改 after the round trip", 10_000, async () => {
    const el = page.locator("[data-chat-mode-menu]");
    return (await el.getAttribute("data-mode-id")) === "plan" && (await el.innerText()).trim() === "只看不改";
  });
  if ((await page.locator("[data-chat-mode]").count()) !== 0) {
    throw new Error("the marker drew beside a menu wearing the same word");
  }

  // 25c-3) 在哪台: the same wizard, a session born on **alpha**, spoken
  //        to from beta's face. Four claims on one road:
  //        1. the machine row offers every paired machine, this one
  //           marked by default;
  //        2. picking alpha lists *alpha's* places — planted on alpha,
  //           and beta's own place must not be on that list (the
  //           control arm: a wizard that kept reading this machine's
  //           records would pass claim 2 with the wrong list);
  //        3. 建立 answers a row alpha owns (alpha's own `sessions`
  //           lists it) — the four answers crossed as a name and were
  //           resolved over there (`Request::OpenAgent`);
  //        4. opening that row from here attaches through the resident's
  //           pipe onto alpha's ghost, and a line goes round: beta's
  //           face → the pipe → alpha's shim → alpha's fake claude →
  //           back. The echo is the proof; a chat that opened but could
  //           not speak would paint an empty conversation, which is
  //           what an unreachable one looks like too.
  const farDir = join(SCRATCH, "far-cwd");
  mkdirSync(farDir, { recursive: true });
  const farPlace = join(SCRATCH, "far-place-repo");
  mkdirSync(join(farPlace, ".git"), { recursive: true });
  {
    const dir = join(A, ".claude", "projects", "far-pl");
    mkdirSync(dir, { recursive: true });
    writeFileSync(
      join(dir, "bbbbbbb1-0000-4000-8000-000000000001.jsonl"),
      JSON.stringify({ type: "user", cwd: farPlace, message: { role: "user", content: "x" } }) + "\n",
    );
  }
  await openLanding("sessions");
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="new"]').click();
  await until("the machine row offering alpha", 10_000, async () =>
    (await page.locator('[data-new-session-machine="alpha"]').count()) === 1,
  );
  if ((await page.locator('[data-new-session-machine="beta"]').getAttribute("data-on")) !== "true") {
    throw new Error("this machine must be the wizard's default");
  }
  await page.locator('[data-new-session-machine="alpha"]').click();
  await until("alpha's own places listed for alpha", 20_000, async () =>
    (await page.locator(`[data-place="${farPlace}"]`).count()) === 1,
  );
  if ((await page.locator(`[data-place="${placeRepo}"]`).count()) !== 0) {
    throw new Error("beta's own place listed under alpha — the wizard is still reading this machine's records");
  }
  await page.locator("[data-new-session-dir]").fill(farDir);
  await page.locator("[data-new-session-name]").fill("far-one");
  await page.locator("[data-new-session-create]").click();
  await until("the far wizard dialog closed on success", 60_000, async () =>
    (await page.locator("[data-new-session-dialog]").count()) === 0,
  );
  await until("the far row on beta's list", 30_000, async () =>
    (await page.locator('[data-title="far-one"]').count()) === 1,
  );
  const farRowId = await page.locator('[data-title="far-one"]').getAttribute("data-row");
  if (!cli(envA, "sessions").includes("far-one")) {
    throw new Error("alpha's own sessions do not list far-one — the row was not born there");
  }
  await page.locator(`[data-row="${farRowId}"] [data-row-open]`).click();
  await until("the far conversation's box", 30_000, async () =>
    (await page.locator("[data-chat-input]").count()) === 1,
  );
  await say(page, "hello far");
  await until("the far reply, round through alpha's shim and the pipe", 40_000, async () => {
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes("echo: hello far");
  });
  // Back to the wizard's own conversation: what follows replays it
  // through the bridge, which only knows the chat the pane holds.
  const wizardRow = await page.locator('[data-title="wizard-one"]').getAttribute("data-row");
  await page.locator(`[data-row="${wizardRow}"] [data-row-open]`).click();
  await until("the wizard's conversation back on the pane", 20_000, async () =>
    (await page.locator("[data-chat-input]").count()) === 1 &&
    (await page.locator("[data-detail-header]").innerText().catch(() => "")).includes("wizard-one"),
  );
  // Markdown is the agent's medium, and the fake echoes what it was
  // told — so one line of it makes the round trip and must arrive
  // *rendered*. Two assertions, because one cannot tell the two ways
  // this breaks apart: nothing rendered, or everything rendered.
  //
  // **Both sides render now, and the second assertion is a tombstone.**
  // It read: the user's own bubble keeps the source it was typed in
  // (ChatView head — an asterisk they meant as an asterisk). That was
  // overruled on 2026-08-31 by the person using this app, after a user
  // line turned out to be where a **message from another agent
  // session** arrives as well: one side of a conversation showing raw
  // markdown while the other showed rendered text read as a bug, and
  // half the time it was one.
  //
  // So the claim flipped rather than went away. It is still two
  // assertions and they still catch different halves — nothing
  // rendered, versus the marks left on screen — only now the bubble is
  // held to the same standard as the agent's side instead of the
  // opposite one.
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
  const typed = await page.locator("[data-said]").last();
  if ((await typed.locator("strong").count()) === 0) {
    throw new Error(
      `the user's own line did not render: ${await typed.innerText()}`,
    );
  }
  const typedText = await typed.innerText();
  if (typedText.includes("**")) {
    throw new Error(`the user's own marks are still on screen: ${typedText}`);
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
  // **The same wait `say` does, and for the same reason.** This block
  // presses Enter itself rather than going through the helper — it has
  // to, because the first press must *complete* rather than send — and
  // in doing so it stepped outside the one thing the helper is for: a
  // send during a running turn is refused, silently and by design (批①).
  //
  // The turn still open here is the link's, from a few lines up: `say`
  // waits for the turn *before* its own send, not after, and the
  // assertion in between is satisfied by the agent's text, which lands
  // well before its `Turn` frame does. So the second press below could
  // land inside that turn, go nowhere, and time out here — which is
  // what happened, and what read as "the command never reached the
  // agent" when the truth was "the line was never sent".
  //
  // **Not the send rule being softened**: that rule has its own
  // assertions further down, where a line is pressed into a turn on
  // purpose and must stay in the box.
  await page.waitForFunction(() => !document.querySelector("[data-chat-stop]"), null, {
    timeout: 60_000,
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

  // c) The running mark, in 忙碌's own clothes, **with how long it has
  //    been running**. The colour is compared against a probe wearing
  //    the token — neither the hue nor the word is written in this file.
  //
  //    The breath moved off the words and onto the mark beside them
  //    (`ToolDot`): a title people are reading should not pulse, and a
  //    dot that says nothing but "still going" costs nothing to animate.
  await until("the running mark while the turn is open", 15_000, async () =>
    (await page.locator("[data-chat-thinking]").count()) === 1,
  );
  const breathing = await page.locator("[data-chat-thinking]").evaluate((box) => {
    const el = box.querySelector("[data-chat-busy-for]");
    const dot = box.querySelector("[data-tool-dot]");
    const probe = document.createElement("span");
    probe.style.color = "var(--state-busy)";
    document.body.appendChild(probe);
    const want = getComputedStyle(probe).color;
    probe.remove();
    const s = getComputedStyle(el);
    return {
      color: s.color,
      want,
      animation: getComputedStyle(dot).animationName,
      text: el.textContent,
    };
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
  // …and the front tab's halo says the same thing (批②; the header
  // stopped carrying the word with 2026-09-02's 标题瘦身 — the tab is
  // the second place now, key against key). **Both read in one call,
  // and that is not tidiness.** These were two reads a moment apart,
  // and the moment was inside a window that closes on its own: when
  // the hanging turn ended between them one side had already moved to
  // 完成 while the other in hand still said 忙碌, and the run failed
  // reporting a disagreement that never existed at any single instant
  // (账本: 要比较的数字必须一次调用取完).
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
    "the row and its tab wearing the same busy word inside one hanging turn",
    20_000,
    async () => {
      const got = await page.evaluate(() => {
        const row = document.querySelector('[data-row][data-word="busy"]');
        const halo = row
          ? document.querySelector(`[data-session-tabs] [data-tab="${row.dataset.row}"] [data-halo]`)
          : null;
        return {
          row: row?.querySelector("[data-word-text]")?.textContent?.trim() ?? "",
          key: row?.dataset.word ?? "",
          tab: halo?.dataset.halo ?? "",
        };
      });
      return got.row && got.key === got.tab ? got : null;
    },
  );
  const busyWord = busyPair.row;
  if (!(breathing.text ?? "").trim().startsWith(busyWord)) {
    throw new Error(`the running mark must say what a busy row says: ${breathing.text} vs ${busyWord}`);
  }
  // **The duration is the whole reason this mark stands for the whole
  // turn**, so it has to be a clock and not a decoration. Read twice,
  // far enough apart that the seconds rung must have moved; the fixture
  // is hanging on purpose, so nothing else can end the turn in between.
  //
  // The wording is not asserted — it is `cli.age_seconds`, and this
  // file spells no words. What is asserted is that it *changed*, which
  // is the claim a frozen `Date.now()` would fail and a static string
  // would fail too.
  const firstRead = (breathing.text ?? "").trim();
  await new Promise((r) => setTimeout(r, 2_500));
  const secondRead = (
    await page.locator("[data-chat-thinking] [data-chat-busy-for]").innerText()
  ).trim();
  if (secondRead === firstRead) {
    throw new Error(
      `the running mark's duration never moved: still ${JSON.stringify(firstRead)} after 2.5s — ` +
        `a clock that does not tick is a label`,
    );
  }
  if (!secondRead.startsWith(busyWord)) {
    throw new Error(`and it must still be 忙碌 while it counts: ${secondRead}`);
  }
  await page.emulateMedia({ reducedMotion: "reduce" });
  const stilled = await page.locator("[data-chat-thinking]").evaluate((box) => {
    const dot = box.querySelector("[data-tool-dot]");
    const el = box.querySelector("[data-chat-busy-for]");
    const s = getComputedStyle(dot);
    return {
      animation: s.animationName,
      opacity: getComputedStyle(el).opacity,
      w: el.getBoundingClientRect().width,
    };
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
  // 25k) 帧上限的恢复 (批⑤): a conversation that overflowed one
  //      attachment's frame list comes back whole instead of coming
  //      back with a hole.
  //
  //      **This is the path that had never run once.** The cap ships at
  //      twenty thousand, so nothing in ordinary use reaches it, and the
  //      three lines that recover from it were written and never
  //      executed — the kind of code whose failure is silence (a
  //      conversation quietly missing its middle, which nobody reports
  //      as a bug, they just stop trusting the app). The knob turned
  //      down at startup is what makes it reachable; the burst below is
  //      what crosses it.
  //
  //      The evidence is a **request the face would not otherwise
  //      make**: a replay is asked for on attach and never again by
  //      itself, so one arriving now can only be the recovery deciding
  //      its own picture is no longer backed by anything.
  const replaysAsked = [];
  const countReplays = (r) => {
    if (r.url().endsWith("/chat_replay")) replaysAsked.push(Date.now());
  };
  page.on("request", countReplays);
  await say(page, "flood-600");
  await until("the far end of the burst on screen", 40_000, async () =>
    (await chatText()).includes("flood 599"),
  );
  if (replaysAsked.length === 0) {
    throw new Error("the cap was crossed and nothing asked for the past back — the recovery never ran");
  }
  //      …and what it recovered is a conversation, not an empty pane.
  //
  //      **This second half does not test the clear, and saying so is
  //      the point.** The obvious break — recover but skip
  //      `setPaint(BLANK)` — was run, and this stayed green. Two
  //      reasons, both measured: `absorb` *replaces* the past when a
  //      `history_end` lands rather than appending, so a second replay
  //      can never stack; and this stand-in does not replay its own
  //      turns (`session/load` returns only the line it wrote on its
  //      way up), so the clear has nothing visible to take away. The
  //      clear's real work is on the live half, and with an agent that
  //      replays for real it would be the difference between a middle
  //      restored and a middle missing. What is gated here is that the
  //      recovery *runs* and leaves a conversation behind; the content
  //      half waits for a stand-in that answers a load.
  await until("the recorded past back on screen, once", 30_000, async () => {
    const t = await chatText();
    return t.split(RECORDED).length - 1 === 1;
  });
  if (!(await chatText()).includes("flood 599")) {
    throw new Error("the recovery took the newest frames with it — those were never lost");
  }
  page.off("request", countReplays);

  // 25d) 转成终端 (接管, 唯一本体 read backwards), from the row's menu
  //      like its opposite in 24k. The row keeps its id and its past;
  //      what changes hands is the form.
  //
  //      **The row must offer exactly one direction**, and that is the
  //      claim this block opens with: a menu carrying both would mean
  //      the app does not know which form the session is in. Right now
  //      khor holds it as a conversation, so 转成终端 is there and
  //      转成对话 is not — and the pane, which paints the host's form,
  //      is showing the chat rather than a card about a terminal that
  //      does not exist.
  await openRowMenu(wizId);
  await until("转成终端 on a conversation's menu", 10_000, async () =>
    (await page.locator('[data-menu-item="to-a-terminal"]').count()) === 1,
  );
  if ((await page.locator('[data-menu-item="to-a-chat"]').count()) !== 0) {
    throw new Error("a session is in one form; its menu offers the other one, not both");
  }
  await page.locator('[data-menu-item="to-a-terminal"]').click();
  await until("the confirm saying which form ends", 10_000, async () =>
    (await page.locator('[data-menu-confirm="to-a-terminal"]').count()) === 1,
  );
  await page.locator('[data-menu-item="to-a-terminal"]').click();
  await until("the same row now holding a terminal", 40_000, async () => {
    const why = await page.locator('[data-menu-note="to-a-terminal"]').innerText().catch(() => "");
    if (why) throw new Error(why);
    return (await page.locator("[data-terminal]").count()) === 1;
  });
  if ((await page.locator(`[data-row="${wizId}"]`).count()) !== 1) {
    throw new Error("the moved conversation must stay one row, under its own id");
  }
  // Its past came along, and the round trip proves it: moved back, the
  // conversation khor reads from the vendor's own transcript still has
  // the line the agent wrote on the way up. **Going back is also the
  // only way to look**, now that a session shows the form it is in —
  // which is the point rather than a cost: the past was never a second
  // face of a terminal, it was the other form of the same session.
  await openRowMenu(wizId);
  await page.locator('[data-menu-item="to-a-chat"]').click();
  await page.locator('[data-menu-item="to-a-chat"]').click();
  await until("the conversation surviving the move", 40_000, async () => {
    const why = await page.locator('[data-menu-note="to-a-chat"]').innerText().catch(() => "");
    if (why) throw new Error(why);
    const t = await page.locator("[data-detail-header]").locator("..").innerText().catch(() => "");
    return t.includes("the fake's own past");
  });

  // 25h) 行的事实与动作,标题瘦身之后 (批② + 2026-09-02「都和标签内容
  //      重复了」). The header keeps the name and its two quiet facts
  //      (workspace · kind); the state rides the tab's halo, the origin
  //      rides the row, and taking the id or ending the session are the
  //      row menu's — so that is where each is asserted now.
  //
  //      The state is asserted up in 25e instead, in the window where a
  //      turn is hanging and the word is provably 忙碌 — here every row
  //      is resting, and a face that always said 空闲 would pass. That
  //      is not a hypothetical: it was this block's first draft, and
  //      planting exactly that break left the run green.
  //
  //      The machine's name is on the row only when the row carries
  //      one. Both halves, because "absent" is also what a broken
  //      selector says: this row lives here and names no machine, and a
  //      reported row in this same list does — the half that proves the
  //      finder.
  if ((await page.locator(`[data-row="${wizId}"] [data-row-source]`).count()) !== 0) {
    throw new Error("a session living on this machine must not name one");
  }
  const fromElsewhere = (
    await page.locator("[data-row][data-source]").evaluateAll((els) => els.map((e) => e.dataset.row))
  )[0];
  if (!fromElsewhere) throw new Error("probe dead: no reported row to check the other half with");
  if ((await page.locator(`[data-row="${fromElsewhere}"] [data-row-source]`).count()) !== 1) {
    throw new Error("a reported row must name the machine it came from");
  }
  //      …and its tab wears that machine's face with the row's own
  //      state as the halo — the same assembly the row wears
  //      (`SessionFace`, both places), compared rather than merely
  //      counted: one machine, one face; one word, one halo.
  await page.locator(`[data-row="${fromElsewhere}"] [data-row-open]`).click();
  const tabFace = page.locator(`[data-session-tabs] [data-tab="${fromElsewhere}"] [data-face]`);
  await until("the reported row's tab wearing a face", 15_000, async () =>
    (await tabFace.count()) === 1,
  );
  if (
    (await faceOf(tabFace)) !==
    (await faceOf(page.locator(`[data-row="${fromElsewhere}"] [data-face]`).first()))
  ) {
    throw new Error("the tab and the row paint one session's machine two different ways");
  }
  const wornPair = await page.evaluate((id) => {
    const row = document.querySelector(`[data-row="${id}"]`);
    const halo = document.querySelector(`[data-session-tabs] [data-tab="${id}"] [data-halo]`);
    return { row: row?.dataset.word ?? "", tab: halo?.dataset.halo ?? "" };
  }, fromElsewhere);
  if (!wornPair.row || wornPair.row !== wornPair.tab) {
    throw new Error(`the tab's halo must be the row's word: ${JSON.stringify(wornPair)}`);
  }
  //      The confirm says what *this kind* of close does. Read off two
  //      rows of different kinds and compared to each other — neither
  //      sentence is spelled here, and a single generic warning would
  //      make these two equal.
  const warningFor = async (rowId) => {
    await openRowMenu(rowId);
    await page.locator('[data-menu-item="close"]').click();
    await until("its confirm", 10_000, async () =>
      (await page.locator('[data-menu-confirm="close"]').count()) === 1,
    );
    const said = (await page.locator('[data-menu-confirm="close"]').innerText()).trim();
    await closeRowMenu();
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

  //      Back on the wizard's own row: the id is takeable from the
  //      row's menu (2026-09-02, with the header diet:「id用户知道也没
  //      用」— its one reader wires a session up from another machine,
  //      and finds it beside the row's other acts), and the close asks
  //      first.
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.evaluate(() => navigator.clipboard.writeText(""));
  await openRowMenu(wizId);
  await page.locator('[data-menu-item="copy-id"]').click();
  // What actually landed on the clipboard, not what anything claims —
  // and the menu closing is the success face (`RowMenu`: rejecting is
  // how an action says it did not happen).
  await until("the row's id on the clipboard", 10_000, async () => {
    const got = await page.evaluate(() => navigator.clipboard.readText());
    return got === wizId;
  });
  await until("the menu closing on a copy that took", 10_000, async () =>
    (await page.locator("[data-row-menu]").count()) === 0,
  );
  // The confirm is a real gate: backing out leaves the session running.
  await openRowMenu(wizId);
  await page.locator('[data-menu-item="close"]').click();
  await until("the confirm on the wizard row", 10_000, async () =>
    (await page.locator('[data-menu-confirm="close"]').count()) === 1,
  );
  await closeRowMenu();
  await settle();
  if ((await page.locator(`[data-row="${wizId}"]`).count()) !== 1) {
    throw new Error("backing out of the confirm must not close the session");
  }
  // …and going through with it ends the session for real — the same
  // ending `khor close` produces, which is what this used to call.
  //
  // **Done on the narrow face on purpose.** The close lives in the
  // row's menu now, so the act happens on the list — what narrow must
  // still prove is that the shell lets go: the dead row's tab drops
  // (the gone-row rule) instead of a detail screen standing over a
  // session that no longer exists.
  await page.setViewportSize({ width: 390, height: 720 });
  await until("the narrow detail of the row about to be closed", 10_000, async () =>
    (await page.locator("[data-back]").count()) === 1,
  );
  await page.locator("[data-back]").click();
  await until("the narrow list", 10_000, async () =>
    (await page.locator("[data-list]").count()) === 1,
  );
  await openRowMenu(wizId);
  await page.locator('[data-menu-item="close"]').click();
  await until("the confirm again", 10_000, async () =>
    (await page.locator('[data-menu-confirm="close"]').count()) === 1,
  );
  await page.locator('[data-menu-item="close"]').click();
  // Closing a live GUI-hosted session does real work — kill the host,
  // drop its pty, reap the agent, record the exit, remove the dir — then
  // the row leaves on the next 2s list poll. Measured end to end at
  // 1.2s (internal disk) and 2.4s (external) on 2026-09-03; the backend
  // `close_session` alone is 0.3-2.1s. 20s is ~10x that. (A one-off 30s
  // was seen while the external disk was 97% full and four cargo builds
  // were running: the dev bridge serves requests on one thread, so a
  // starved machine serialises the poll behind other ops. That is the
  // harness, not the product — `khor serve` runs a four-worker pool.)
  // 60s, not 20: the same serialisation put it past 20s again while four
  // builds ran beside it, and a harness wait is for the harness's load.
  await until("the wizard row gone", 60_000, async () => {
    const why = await page.locator('[data-menu-note="close"]').innerText().catch(() => "");
    if (why) throw new Error(why);
    return (await page.locator(`[data-row="${wizId}"]`).count()) === 0;
  });
  //      …and the shell lets go of it: back on the list, with no detail
  //      screen left standing over a session that no longer exists.
  await until("the narrow shell back on the list", 15_000, async () =>
    (await page.locator("[data-list]").count()) === 1 &&
    (await page.locator("[data-back]").count()) === 0,
  );
  await page.setViewportSize({ width: 1080, height: 720 });
  await until("back on the wide face", 10_000, async () =>
    (await page.locator("[data-row]").count()) > 0,
  );
  //      The tab went with the row (the gone-row rule): a strip keeping
  //      a dead session's tab would be a face pointing at nothing.
  if ((await page.locator(`[data-session-tabs] [data-tab="${wizId}"]`).count()) !== 0) {
    throw new Error("the strip still holds a tab for a session that is gone");
  }

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
  //      Driven by a **real transfer**, which since 传输并入对话批 is a
  //      file message in the machine's conversation — no transfer/ row
  //      exists any more; the pressure lands on the machine's chat row
  //      (`TransferKind::urgency`), and the strip is fed off exactly
  //      that (`use-status-bar`'s PRESS words). So this section watches
  //      the row the fact actually lives on now: beta's own window,
  //      chat/beta. What it proves is unchanged — the two admission
  //      rules, against something that really happened.
  await openLanding("sessions");
  if ((await page.locator("[data-statusbar]").count()) !== 0) {
    throw new Error("probe dead: the corner is already occupied before anything happened");
  }
  const sentFile = join(SCRATCH, "corner-note.txt");
  writeFileSync(sentFile, "a file worth a line in the corner\n");
  //      **The accept handle comes from the verb that made it.** Rows
  //      retired; the address scheme did not (`TransferKind::session_id`
  //      still mints transfer/<msg>, and `khor accept` still takes it).
  //      `\S+` and not a hex pattern: a character class that stopped at
  //      the first dash once produced a real-looking id for a transfer
  //      that does not exist, which timed out reading exactly like the
  //      sync being slow.
  const transferRow = cli(envA, "send", "beta", sentFile).match(/transfer\/\S+/)?.[0];
  if (!transferRow) throw new Error("khor send did not name the transfer it made");
  //      The offer arriving presses 待批 onto beta's own window row —
  //      the merged shape's way of saying "the summary reached beta".
  await until("the offer pressing 待批 onto beta's window row", 30_000, async () =>
    (await page.locator('[data-row="chat/beta"][data-word="blocked"]').count()) === 1,
  );
  //      **And with something provably waiting, the mark's place has to
  //      say so without being pressed.** This is the half 29 above
  //      cannot guarantee — whether anything is waiting when that item
  //      runs is the run's business, and a gate whose positive case
  //      depends on the weather is a gate that reports "both directions
  //      asserted" having exercised one. Here the state was just
  //      created, so the assertion holds every time.
  //
  //      **And it has to say so on the right machine.** The mark moved
  //      from one node standing for the whole mesh onto the seat of the
  //      machine that owns the row, so "somebody is waiting" and "this
  //      one is" stopped being the same claim — a mark on the wrong
  //      seat sends a person to the wrong card and would still pass
  //      "something says so". Which machine that is, is not written
  //      here: it is read off the row itself (`SessionRow::home`, the
  //      node's own join), and the map has to agree with it.
  const waitingHome = await page.locator('[data-row="chat/beta"]').getAttribute("data-home");
  if (!waitingHome) {
    throw new Error("probe dead: the waiting row does not say which machine it is on");
  }
  await page.locator("[data-rail-mark]").click();
  await until("the mark's place", 15_000, async () =>
    (await page.locator("[data-mark-place] [data-mandala-map]").count()) === 1,
  );
  //      A poll apart: the list saw 待批 first, and the map is fed by
  //      the same table one round later.
  await until(`${waitingHome}'s seat to say something is waiting`, 30_000, async () => {
    const seat = await page.locator("[data-mark-place] [data-seat]").evaluateAll(
      (els, name) =>
        els
          .filter((e) => e.querySelector("[data-seat-name]")?.textContent.trim() === name)
          .map((e) => ({
            blocked: Number(e.getAttribute("data-blocked") ?? "0"),
            drawn: e.querySelector("[data-seat-blocked]") !== null,
          }))[0] ?? null,
      waitingHome,
    );
    return seat !== null && seat.blocked >= 1 && seat.drawn;
  });
  //      …read at rest: nothing here was pressed open, which is the
  //      whole point — the count has to be legible before anyone asks.
  if ((await page.locator("[data-node-card]").count()) !== 0) {
    throw new Error("probe void: a card is open, so this was not read at rest");
  }
  await openLanding("sessions");
  await until("the list back", 15_000, async () => (await page.locator("[data-row]").count()) > 0);

  //      **待批 does not announce itself.** The strip has watched the
  //      row arrive at 待批 and stayed empty: an offer waiting on THIS
  //      machine already has the badge and the top of the list, and the
  //      corner carries only what is happening — the same half that
  //      used to keep a first-seen 待批 transfer row out of it, worn by
  //      the new carrier.
  await settle();
  if ((await page.locator("[data-statusbar]").count()) !== 0) {
    throw new Error("an offer nobody touched put itself in the corner");
  }
  //      Now it moves — the accept pulls the bytes, the row leaves 待批
  //      (through 忙碌 to 完成, and a small file may cross both between
  //      two polls; leaving 待批 is itself an admission, so the strip
  //      catches it either way) — and the corner says so, keyed by the
  //      row the fact lives on.
  cli(envB, "accept", transferRow);
  await until("the corner reporting the movement", 30_000, async () =>
    (await page.locator('[data-status-item="chat/beta"]').count()) === 1,
  );
  //      Every line in there is a process-class fact with a row behind
  //      it — never a click's outcome, never mere chatter (a message
  //      arriving moves 完成/空闲 and is deliberately inadmissible).
  //      Checked as an invariant rather than by trying to push a click
  //      result in, because the rule is "only these get in", not "that
  //      one is kept out".
  const inCorner = await page
    .locator("[data-status-item]")
    .evaluateAll((els) => els.map((e) => e.dataset.statusItem));
  if (inCorner.some((k) => k !== "chat/beta")) {
    throw new Error(`the corner took something beyond the pressed row: ${JSON.stringify(inCorner)}`);
  }
  //      …and it goes to zero on its own. **Absent, not empty**: the
  //      whole strip stops rendering, so there is no box left saying
  //      nothing.
  await until("the corner emptying itself", 20_000, async () =>
    (await page.locator("[data-statusbar]").count()) === 0,
  );

  // 25e) 额度: a registered API account is a ring, wherever its key
  //      lives. A fake new-api gateway is started here and registered
  //      **on alpha**; beta's page draws it from what alpha carried over
  //      (`Request::Quotas`), wearing alpha's name — so the key never
  //      left alpha (the fake records the bearer it was shown, which is
  //      the proof the number came from alpha's key and not from a
  //      copy). Three arms on one road: the good token (45% of a $2
  //      quota), a wrong key on the same gateway (钥匙被拒, carried as a
  //      trouble, not as a zero), and a gateway nobody answers at,
  //      registered on beta itself (连不上, read here and now).
  const gwPort = 17321;
  const gwDir = join(SCRATCH, "fake-newapi");
  mkdirSync(gwDir, { recursive: true });
  const gwScript = join(gwDir, "newapi.py");
  writeFileSync(
    gwScript,
    `#!/usr/bin/env python3
import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def send(self, code, body):
        data = json.dumps(body).encode()
        self.send_response(code); self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data))); self.end_headers(); self.wfile.write(data)
    def do_GET(self):
        if self.path == "/api/status":
            return self.send(200, {"success": True, "data": {"quota_per_unit": 500000, "quota_display_type": "USD"}})
        if self.path == "/api/usage/token/":
            auth = self.headers.get("Authorization", "")
            with open("bearers.log", "a") as f: f.write(auth + "\\n")
            # **The gateway's own error convention: HTTP 200 with the
            # failure in the body.** new-api answers every refusal this
            # way (common/gin.go's ApiError / ApiErrorMsg / ApiErrorI18n
            # all use http.StatusOK), and khor used to read the 200 as a
            # yes, find no data, and say 读不懂它的回答 — pointing at a
            # thing that was not broken. This arm is that exact shape.
            if auth == "Bearer sk-two-hundred-no":
                return self.send(200, {"success": False, "message": "无效的令牌"})
            if auth != "Bearer sk-good-token":
                return self.send(401, {"success": False, "message": "无效的令牌"})
            return self.send(200, {"success": True, "data": {"object": "token_usage",
                "total_granted": 1000000, "total_used": 450000, "total_available": 550000, "unlimited_quota": False}})
        self.send(404, {"success": False})
HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
`,
  );
  run("python3", [gwScript, String(gwPort)], process.env, "fake-newapi", gwDir);
  await until("the fake gateway listening", 10_000, async () => {
    try {
      const r = await fetch(`http://127.0.0.1:${gwPort}/api/status`);
      return r.ok;
    } catch {
      return false;
    }
  });
  const goodId = cli(envA, "accounts", "add", "gw-good", "--provider", "newapi",
    "--base-url", `http://127.0.0.1:${gwPort}`, "--key", "sk-good-token").trim();
  cli(envA, "accounts", "add", "gw-bad", "--provider", "newapi",
    "--base-url", `http://127.0.0.1:${gwPort}`, "--key", "sk-wrong");
  // **The fourth arm, and it is the one no HTTP status can catch**: a
  // refusal that arrives as 200 (2026-09-07). `gw-bad` above is refused
  // with a 401 — an implementation that only reads statuses passes that
  // one and fails this, which is why both are here.
  cli(envA, "accounts", "add", "gw-200no", "--provider", "newapi",
    "--base-url", `http://127.0.0.1:${gwPort}`, "--key", "sk-two-hundred-no");
  cli(envB, "accounts", "add", "gw-dead", "--provider", "newapi", "--base-url", "http://127.0.0.1:1", "--key", "sk-x");
  if (!/^[0-9a-f]{32}$/.test(goodId)) throw new Error(`accounts add did not answer an id: ${goodId}`);
  await railItem("quota").click();
  await until("the 额度 page", 10_000, async () => (await page.locator("[data-quota-landing]").count()) === 1);
  // beta's own account first — read here, no beat to wait for.
  await until("the dead gateway wearing 连不上", 30_000, async () =>
    (await page.locator('[data-quota-row][data-trouble="unreachable"]').count()) === 1,
  );
  // alpha's two, carried over on the report beat.
  //
  // **The reading is on the row; the machine that took it is not.** The
  // row wore 「alpha」 when this was written (d84bbe9) and stopped on
  // 2026-09-06 (3e9b4d9): its third segment is 来源, which for a
  // registered account is 「khor 登记」 — the machine holding the key is
  // a property of the account and lives in its detail. So this asks the
  // row only for the figure, and the detail below asks for the name.
  // **Both halves are still asserted**, because together they are the
  // one claim this whole item makes: the key never left alpha, and beta
  // is showing what alpha read.
  await until("alpha's good token as a 45% ring", 90_000, async () => {
    const row = page.locator(`[data-quota-row="${goodId}"]`);
    if ((await row.count()) !== 1) return false;
    return (await row.innerText()).includes("45%");
  });
  // Two rows now wear it: the 401 and the 200-that-says-no. **Both, in
  // one assertion, because they are one claim** — 「refused」 is what a
  // refusal reads as, whichever way the gateway spelled it. And the
  // count is exact rather than `>= 1`: the 200 arm going back to
  // 「读不懂」 would leave the other one standing and a loose count
  // would not notice.
  await until("alpha's two refused keys, the 401 and the 200-that-says-no", 90_000, async () =>
    (await page.locator('[data-quota-row][data-trouble="key-refused"]').count()) === 2,
  );
  // …and neither of them landed on 读不懂它的回答, which is the answer
  // next door and the one this fix is about.
  if ((await page.locator('[data-quota-row][data-trouble="unreadable"]').count()) !== 0) {
    throw new Error("a refusal is being reported as 读不懂它的回答 — the 200 body's `success: false` is not being read");
  }
  // The bearer the gateway saw is alpha's registered key, verbatim —
  // the reading came from alpha reading its own key, not from beta.
  const bearers = readFileSync(join(gwDir, "bearers.log"), "utf8");
  if (!bearers.includes("Bearer sk-good-token") || bearers.includes("sk-x")) {
    throw new Error(`the gateway saw the wrong keys: ${JSON.stringify(bearers)}`);
  }
  // The detail: the good ring's figures, and the CLI's own board says
  // the same number.
  await page.locator(`[data-quota-row="${goodId}"]`).click();
  //   ⚠️ **The unit moved, the machine did not** — and the first fix
  //   here got that backwards. Reading the card out loud (2026-09-11):
  //
  //       gw-good
  //       alpha 读的,3 秒 前
  //       45%
  //       已用
  //       0.90 / 2.00$
  //
  //   So 「alpha as the reader」 is still on the card and was never in
  //   danger; what changed is that money wears a symbol now rather than
  //   a currency code (`0.90 / 2.00 USD` → `0.90 / 2.00$`), from the
  //   ruling that put the ruler in the unit's place. **The first repair
  //   dropped the machine half on a guess about a different ruling**,
  //   which would have quietly thrown away the one thing here that says
  //   the reading came from alpha's own key rather than beta's — the
  //   whole point of the bearer check a few lines up. Restored, and
  //   both halves are asserted.
  await until("the good account's figures, and alpha as the reader", 10_000, async () => {
    const d = page.locator(`[data-quota-detail="${goodId}"]`);
    if ((await d.count()) !== 1) return false;
    const text = await d.innerText();
    return text.includes("0.90 / 2.00$") && text.includes("alpha");
  });
  const board = cli(envB, "quota");
  if (!board.includes("gw-good") || !board.includes("45%")) {
    throw new Error(`khor quota on beta does not carry alpha's reading: ${board}`);
  }
  // 25f) 置顶 crosses, and it is **the one claim about the 额度 list
  //      that only a real run can make.** `check-face` routes
  //      `quota_board` from a hand-written fixture, so over there the
  //      order is the fixture's — the gate can prove the flag is sent
  //      and nothing about what it does to the list. The Rust test
  //      (`khor-sync`, `a_pinned_account_floats…`) proves the sort;
  //      what neither can reach is **the whole path**: a press on one
  //      machine's face, through the replicated doc, out of another
  //      machine's CLI.
  //
  //      **The control arm is in the fixture's names.** The three
  //      accounts are `gw-bad`, `gw-dead`, `gw-good`, so by name
  //      `gw-good` is **last** — an implementation that ignores the
  //      flag leaves it last, and one that pins it puts it first. Had
  //      the pinned row been first alphabetically, 「pinned」 and
  //      「did nothing」 would give the same answer and this would be a
  //      check that cannot fail.
  //      **Only the registered accounts, because the subscription row
  //      is nailed to the top** and pinning cannot move anything above
  //      it (`QuotaLanding`: 「订阅在前,因为它是你没主动要的那份读数」).
  //      Written against the whole list first, and it could never pass:
  //      `order[0]` was the subscription every time, so the red said
  //      「pinning does not work」 about a product that was fine — the
  //      gate had a model of the list that the list never had. Caught
  //      2026-09-07 09:48, the first time it was ever run.
  const rowOrder = async () =>
    await page.locator('[data-quota-row][data-quota-kind="bill"]').evaluateAll((els) =>
      els.map((e) => e.getAttribute("data-quota-row")),
    );
  const beforePin = await rowOrder();
  if (beforePin[beforePin.length - 1] !== goodId) {
    throw new Error(
      `the pin's control arm is gone: gw-good is not last by name (${JSON.stringify(beforePin)}), so pinning it proves nothing`,
    );
  }
  await page.locator(`[data-quota-row="${goodId}"]`).click({ button: "right" });
  await until("the account's menu", 5_000, async () =>
    (await page.locator('[data-row-menu] [data-menu-item="pin"]').count()) === 1,
  );
  await page.locator('[data-menu-item="pin"]').click();
  await until("the pinned account floating to the top of beta's list", 20_000, async () => {
    const order = await rowOrder();
    return order[0] === goodId;
  });
  // …and it is not a fact about beta's screen: the flag rode the
  // replicated doc, so **alpha's own CLI** lists it first too. The
  // first line, not "contains" — the order is the whole claim.
  await until("alpha's khor quota listing the pinned account first", 90_000, () => {
    const said = cli(envA, "quota").trim().split("\n");
    return said.length > 1 && said[0].startsWith("gw-good");
  });
  // 30) a machine leaves the network from its row's menu — `khor forget`
  //     (2026-09-06:「机器左侧 list…右键,置顶或删除」). Two presses, and
  //     the first is the control arm: it arms the sentence and moves
  //     nothing, on both faces — a menu that forgot on one press would
  //     empty the row here and the assertions after it would still
  //     pass. Then the second, and gamma is gone from beta's list **and
  //     from beta's own CLI**: the tombstone travels
  //     (`DeviceDoc::set_gone`), so this is one fact read off both
  //     faces. This machine's own row offers no such item at all —
  //     `Node::forget_device` refuses it by name, and an item whose one
  //     answer is no is not offered.
  //
  //     **Before 26, not after it.** This was written last and read
  //     last, and 26 below **ends the bridge** — so every call made here
  //     went out over a door that was already shut, and the row's menu
  //     wore 「Failed to fetch」, which reads exactly like the node
  //     refusing to forget the machine (2026-09-06, and it took a run
  //     with the menu's own text printed to tell the two apart). What
  //     「last」 was protecting is that nothing *after this* needs gamma,
  //     and 26 does not.
  await openLanding("devices");
  await until("gamma on the device list", 10_000, async () =>
    (await page.locator('[data-device="gamma"]').count()) === 1,
  );
  const listsGamma = () => cli(envB, "devices").split("\n").some((l) => l.startsWith("gamma\t"));
  if (!listsGamma()) throw new Error("probe dead: beta's CLI does not list gamma before anything was forgotten");
  const ownRow = await page.locator('[data-device="beta"]').getAttribute("data-row");
  await openRowMenu(ownRow);
  if ((await page.locator('[data-menu-item="forget"]').count()) !== 0) {
    throw new Error("this machine's own row offers to forget itself — the one press the node always refuses");
  }
  await closeRowMenu();
  const goneRow = await page.locator('[data-device="gamma"]').getAttribute("data-row");
  await openRowMenu(goneRow);
  if ((await page.locator('[data-menu-item="forget"]').count()) !== 1) {
    throw new Error("gamma's row offers no way to forget it");
  }
  await page.locator('[data-menu-item="forget"]').click();
  await until("the sentence between the two presses", 5_000, async () =>
    (await page.locator('[data-menu-confirm="forget"]').count()) === 1,
  );
  await settle();
  if ((await page.locator('[data-device="gamma"]').count()) !== 1 || !listsGamma()) {
    throw new Error("one press forgot the machine — nothing stood between the press and the act");
  }
  await page.locator('[data-menu-item="forget"]').click();
  await until("gamma to leave beta's machine list", 20_000, async () =>
    (await page.locator('[data-device="gamma"]').count()) === 0,
  );
  await until("beta's CLI to stop listing gamma", 20_000, () => !listsGamma(), 1_000);

  // 31) the workspace landing, on the real mesh (工作区批⑥).
  //
  //      **The whole of 工作区 was fixture-only until now.** Five
  //      batches built it and `check-face` drove every one of them
  //      against routed JSON; this file — three real homes, real UDP,
  //      a real bridge — never opened the landing at all. So a
  //      workspace that syncs, a directory listed off another machine,
  //      a file deleted on it and a session filed under it were all
  //      unmeasured on the one road that has a far machine in it.
  //
  //      **One box, two homes, and what that can and cannot prove.**
  //      alpha and beta are two khor homes on one filesystem, so an
  //      absolute path resolves the same for both and 「it listed
  //      alpha's disk rather than beta's」 is not a claim any content
  //      can settle here. What it *can* settle is everything that goes
  //      through the far machine's serve: the row saying `alpha:`, an
  //      mtime alpha answered, a file alpha really unlinked, and the
  //      declaration itself arriving in alpha's own table.
  //
  //      **Before 26, like 30 above.** Written at the end of the file
  //      and run there once: 26 kills the bridge, and every ask after
  //      it answers nothing — the picker still drew (its dialog is this
  //      app's own), the machine row did not, and the failure read as
  //      「the picker has no cross-machine row」 rather than 「there is
  //      no backend」. A section that needs a node goes above the one
  //      that takes the node away.
  //      **The one path in this file that is resolved before it is
  //      used, and 31e is why.** `SCRATCH` is under `target/`, which on
  //      this checkout is a symlink to another volume — so the path
  //      typed into the picker and the path `khor open` records for a
  //      session started there are two spellings of one directory.
  //      Belonging is a prefix comparison over the recorded cwd
  //      (`WorkspacePane`'s `mine`), and two spellings never share a
  //      prefix: the column came up empty with both rows sitting in the
  //      node under `/Volumes/...`. **This is a fixture that walks
  //      around the difference, not a fix for it** — a workspace really
  //      declared through a symlink still collects nothing, and that is
  //      reported rather than patched here (批⑥: 产品不动).
  const wsDir = join(realpathSync(SCRATCH), "alpha-workspace");
  const wsFile = join(wsDir, "notes.md");
  const wsOutside = join(realpathSync(SCRATCH), "alpha-elsewhere");
  const wsLong = join(wsDir, "long.txt");
  const wsKeep = join(wsDir, "keep.txt");
  mkdirSync(join(wsDir, "sub"), { recursive: true });
  mkdirSync(wsOutside, { recursive: true });
  writeFileSync(wsFile, "# a note\n");
  //      A line no column can hold, in a real file on a real disk
  //      (工作区批⑨). **No extension khor has a grammar for**, on
  //      purpose: the painted road and the plain one have to fold the
  //      same way, and only the plain one is reachable without the
  //      Shiki chunk landing first.
  writeFileSync(wsLong, `short\n${"x".repeat(400)}\nafter\n`);
  //      Something for git to have an opinion about, once 31g gives it
  //      one. It is here rather than made later because the tree lists
  //      a directory when it opens it, and a file that arrives after
  //      that is a file with no row to wear a letter.
  writeFileSync(wsKeep, "one\n");
  //      **Deliberately not a checkout**: the git corner has to come out
  //      empty, and a directory with a `.git` in it would fill it.
  //      Dated old so 31b can tell 「alpha answered a fresh mtime」 from
  //      「the column drew something」.
  const OLD = new Date("2020-01-01T00:00:00Z");
  utimesSync(wsFile, OLD, OLD);
  utimesSync(join(wsDir, "sub"), OLD, OLD);

  // 31a) declared from beta's face, on alpha's disk, through the picker.
  await openLanding("workspaces");
  await page.locator("[data-ws-add]").click();
  await until("the directory picker", 10_000, async () =>
    (await page.locator("[data-dir-picker]").count()) === 1,
  );
  // **This machine first, then alpha.** `""` is what the picker sends
  // for 本机, and `ls_with` refused it as 「这个机器名进不了路径」 until
  // 2026-09-15 — the row on the desk drew nothing while the far ones
  // worked, which no fixture could see.
  await page.locator('[data-dir-picker-machine="beta"]').click();
  await until("the picker listing this machine's own home", 20_000, async () =>
    (await page.locator("[data-dir-picker-row]").count()) > 0 &&
    (await page.locator("[data-dir-picker-cannot]").count()) === 0,
  );
  await page.locator('[data-dir-picker-machine="alpha"]').click();
  //      Typed rather than walked from the home the picker opens on:
  //      that home is this developer's own, and the path below is under
  //      the scratch directory. The box is the road for exactly this
  //      (`components/DirPicker` — the tree is the shortcut, the box is
  //      the floor), and Enter walks it, so what comes back is alpha's
  //      listing of that directory and not this face's idea of one.
  await page.locator("[data-dir-picker-path]").fill(wsDir);
  await page.locator("[data-dir-picker-path]").press("Enter");
  await until("alpha listing the directory it was asked about", 20_000, async () =>
    (await page.locator('[data-dir-picker-row="sub"]').count()) === 1,
  );
  await page.locator("[data-dir-picker-take]").click();
  await until("the picker answered and closed", 20_000, async () =>
    (await page.locator("[data-dir-picker]").count()) === 0,
  );
  const wsName = wsDir.split("/").pop();
  await until("the row on beta's list", 20_000, async () =>
    (await page.locator("[data-ws-row] [data-ws-name]").allInnerTexts()).includes(wsName),
  );
  const wsRow = page.locator("[data-ws-row]").filter({ hasText: wsName }).first();
  const where = await wsRow.locator("[data-ws-where]").innerText();
  if (where !== `alpha:${wsDir}`) {
    throw new Error(`the row says ${where} — a workspace is (machine, directory) and this one is alpha's`);
  }
  //      **The decision replicated**, which is the whole reason it is a
  //      document: nobody told alpha, and alpha lists it.
  await until("alpha's own table carrying the declaration", 40_000, () =>
    cli(envA, "ws", "ls").includes(`alpha:${wsDir}`),
  );
  if (!cli(envA, "ws", "ls").split("\n").some((l) => l.split("\t")[1] === wsName)) {
    throw new Error(`khor ws ls on alpha does not name the row ${wsName}: ${cli(envA, "ws", "ls")}`);
  }
  //      Not a repository, so the git corner is empty — never the word
  //      for「no git」, and never「clean」.
  const gitCorner = await wsRow.locator("[data-ws-git]").innerText();
  if (gitCorner.trim() !== "") {
    throw new Error(`a directory that is no checkout drew 「${gitCorner}」 in its git corner`);
  }

  // 31b) the mtime is alpha's answer, re-asked on the way back to the
  //      window (the same kick `WorkspacePane` uses for the marks).
  const setHiddenWs = (on) =>
    page.evaluate((hide) => {
      Object.defineProperty(document, "hidden", { value: hide, configurable: true });
      Object.defineProperty(document, "visibilityState", {
        value: hide ? "hidden" : "visible",
        configurable: true,
      });
      document.dispatchEvent(new Event("visibilitychange"));
    }, on);
  const wasTouched = await wsRow.locator("[data-ws-touched]").innerText();
  if (wasTouched.trim() === "") {
    throw new Error("alpha answered no mtime at all for a directory with files in it");
  }
  writeFileSync(wsFile, "# a note, edited\n");
  await setHiddenWs(true);
  await setHiddenWs(false);
  await until("the row carrying alpha's fresh mtime", 30_000, async () => {
    const now = await wsRow.locator("[data-ws-touched]").innerText().catch(() => "");
    return now.trim() !== "" && now !== wasTouched;
  });

  // 31c) opened: the tree is that directory's.
  await wsRow.click();
  await until("alpha's directory in the tree", 30_000, async () =>
    (await page.locator('[data-file="notes.md"]').count()) === 1 &&
    (await page.locator('[data-dir="sub"]').count()) === 1,
  );

  // 31d) a file deleted across the wire is really gone from alpha's
  //      disk — and the sheet says so before anybody presses, because a
  //      far machine has no recycle bin to put it in.
  await page.locator('[data-file="notes.md"] > button').first().click({ button: "right" });
  await until("the file's menu", 10_000, async () =>
    (await page.locator('[data-menu-item="remove"]').count()) === 1,
  );
  await page.locator('[data-menu-item="remove"]').click();
  await until("the confirmation sheet", 10_000, async () =>
    (await page.locator("[data-remove-dialog]").count()) === 1,
  );
  if ((await page.locator("[data-remove-never-ask]").count()) !== 0) {
    throw new Error("a file on another machine offered 「不再问」 — there is no bin over there to take it back out of");
  }
  const saysGone = await page.locator("[data-remove-says]").innerText();
  if (!saysGone.includes("notes.md")) {
    throw new Error(`the sheet does not name the file: ${saysGone}`);
  }
  await page.locator("[data-remove-go]").click();
  await until("the file gone from alpha's disk", 20_000, () => !existsSync(wsFile));
  await until("and gone from the tree", 20_000, async () =>
    (await page.locator('[data-file="notes.md"]').count()) === 0,
  );

  // 31g) a long line folds into the column, on a real file (工作区批⑨).
  //
  //      「文件显示,要随屏幕宽度自动换行。」「不做换行开关,我们显示行号
  //      的作用就是这个啊」— the user, 2026-09-15. `check-face` measures
  //      this against routed bytes and a 1400 window; here it is alpha's
  //      own file, read over the wire, in the four-column shape a real
  //      desktop actually shows.
  await page.locator('[data-file="long.txt"] > button').first().click();
  await until("alpha's long file on the sheet", 30_000, async () =>
    (await page.locator('[data-sheet] [data-line="3"]').count()) === 1,
  );
  const folded = await page.evaluate(() => {
    const round = (n) => Math.round(n * 10) / 10;
    const pre = document.querySelector("[data-sheet]");
    const box = document.querySelector("[data-file-scroll]");
    const tall = (n) => {
      const el = document.querySelector(`[data-sheet] [data-line="${n}"]`);
      return el === null ? null : round(el.getBoundingClientRect().height);
    };
    const top = (sel) => {
      const el = document.querySelector(sel);
      return el === null ? null : round(el.getBoundingClientRect().top);
    };
    return {
      row: tall(1),
      long: tall(2),
      sheet: round(pre.getBoundingClientRect().width),
      column: round(pre.parentElement.getBoundingClientRect().width),
      box: box === null ? null : [box.scrollWidth, box.clientWidth],
      page: [document.scrollingElement.scrollWidth, document.scrollingElement.clientWidth],
      line3: top('[data-sheet] [data-line="3"]'),
      num3: top('[data-num="3"]'),
    };
  });
  console.log(`workspaces · wrap: ${JSON.stringify(folded)}`);
  if (folded.row === null || folded.long === null || folded.long < folded.row * 1.5) {
    throw new Error(
      `a 400-character line came out ${folded.long} tall against a ${folded.row} row — it did not ` +
        "wrap, and there is no switch to turn wrapping on with",
    );
  }
  if (folded.page[0] > folded.page[1] || (folded.box && folded.box[0] > folded.box[1])) {
    throw new Error(
      `the file scrolls sideways (page ${folded.page}, its own box ${folded.box}) — the ruling is ` +
        "that it folds into the column instead",
    );
  }
  if (folded.sheet > folded.column + 1) {
    throw new Error(`the sheet is ${folded.sheet} wide inside a ${folded.column} column`);
  }
  if (folded.num3 === null || Math.abs(folded.num3 - folded.line3) > 0.5) {
    throw new Error(
      `line 3's number sits ${folded.num3} against its first character at ${folded.line3} — the ` +
        "numbers are what the ruling spent the wrap switch on",
    );
  }

  // 31e) the sessions column: what is running under this directory on
  //      this machine, and nothing else.
  //
  //      Started with a cwd rather than through the wizard, because the
  //      cwd is the whole subject: `khor open` records where it was run
  //      (`live::learn_place`), and `execFileSync` can say where that
  //      is. Two of them, one inside and one beside, so the rule being
  //      measured is「under this directory」and not「on this machine」.
  const wsInside = execFileSync(
    KHOR,
    ["open", "-d", "--title", "ws-inside", "--", "sh", "-c", "sleep 600"],
    { env: envA, cwd: wsDir, encoding: "utf8", timeout: 30_000 },
  ).trim();
  const wsBeside = execFileSync(
    KHOR,
    ["open", "-d", "--title", "ws-beside", "--", "sh", "-c", "sleep 600"],
    { env: envA, cwd: wsOutside, encoding: "utf8", timeout: 30_000 },
  ).trim();
  try {
    //      **The column's default is closed here** — a wide face with
    //      the workspace list open already has four columns, and this
    //      is the fifth (`WorkspacePane` `listOpen`). So the switch is
    //      pressed, and that the column then exists is its own
    //      assertion: without it 「no row in the column」 and 「no
    //      column」 are the same red, and only one of them is about
    //      belonging.
    if ((await page.locator("[data-ws-sessions]").count()) === 0) {
      await page.locator("[data-ws-sessions-switch]").click();
    }
    await until("the workspace's sessions column", 10_000, async () =>
      (await page.locator("[data-ws-sessions]").count()) === 1,
    );
    //      What the node itself holds for these two rows, printed into
    //      the failure rather than left to a second run: the column can
    //      come up empty because the row never arrived, because it
    //      arrived under another machine, or because its birthplace was
    //      not recorded — three different bugs, one empty column.
    const wsRowsSay = async () => {
      const all = await bridgeOp("sessions", { by: "recent" }).catch((e) => String(e));
      if (!Array.isArray(all)) return `the node would not say: ${all}`;
      return all
        .filter((r) => r.title === "ws-inside" || r.title === "ws-beside")
        .map((r) => `${r.title} home=${r.home} place=${r.place}`)
        .join("; ") || "the node lists neither of them";
    };
    await until("the session started inside the workspace", 40_000, async () =>
      (await page.locator('[data-ws-sessions] [data-title="ws-inside"]').count()) === 1,
    ).catch(async (e) => {
      throw new Error(`${e.message} — the node says: ${await wsRowsSay()}`);
    });
    if ((await page.locator('[data-ws-sessions] [data-title="ws-beside"]').count()) !== 0) {
      throw new Error("a session started beside the workspace is listed in it — the rule is 「under this directory」");
    }
    //      **`showVia={false}` is not asserted here, and the control is
    //      why.** The column passes it so a row does not repeat the
    //      machine the header above it just named — but `data-via` is
    //      the 借网 mark (`SessionRow::via`, docs/NET.md), and that
    //      field has no producer in the product yet. A run that
    //      measured it found zero marks in the column *and* zero on the
    //      sessions landing, which is this file's own 假绿 family: an
    //      assertion that cannot fail is not one. It comes back when
    //      something writes a via.
    //
    // 31h) the column's own「+」opens the wizard already answered
    //      (工作区批⑩: 「这就是最早说的『加入我的工作区』」). Both
    //      halves, because either one alone starts the session
    //      somewhere nobody asked for: alpha's name and alpha's
    //      directory.
    await until("the column's own bar", 10_000, async () =>
      (await page.locator("[data-ws-sessions] [data-ws-new-session]").count()) === 1,
    );
    await page.locator("[data-ws-sessions] [data-ws-new-session]").click();
    await until("the wizard", 10_000, async () =>
      (await page.locator("[data-new-session-dialog]").count()) === 1,
    );
    //      **And its machine row, which arrives a round trip later.**
    //      The wizard reads the device table on every opening (it syncs,
    //      so a cached list is a stale answer to the one question it
    //      cannot guess), and it draws no machines at all until that
    //      lands. Read at the moment the dialog appears, 「which machine
    //      is picked」 came back as 「none of them」 — a red about this
    //      script's timing wearing the words of a red about the product.
    await until("the wizard's machine row", 20_000, async () =>
      (await page.locator("[data-new-session-machine]").count()) > 1,
    );
    const prefilled = await page.evaluate(() => ({
      dir: document.querySelector("[data-new-session-dir]")?.value ?? null,
      on: [...document.querySelectorAll("[data-new-session-machine]")]
        .filter((e) => e.getAttribute("data-on") === "true")
        .map((e) => e.getAttribute("data-new-session-machine")),
    }));
    console.log(`workspaces · +: ${JSON.stringify(prefilled)}`);
    if (prefilled.dir !== wsDir) {
      throw new Error(`the wizard opened on ${prefilled.dir} rather than ${wsDir}`);
    }
    if (JSON.stringify(prefilled.on) !== JSON.stringify(["alpha"])) {
      throw new Error(
        `the wizard opened with ${JSON.stringify(prefilled.on)} picked — this workspace is alpha's, ` +
          "and a directory on alpha's disk under beta's name is a session that starts elsewhere",
      );
    }
    await page.keyboard.press("Escape");
    await until("the wizard closed", 10_000, async () =>
      (await page.locator("[data-new-session-dialog]").count()) === 0,
    );

    // 31i) an agent in this project going quiet re-asks git (工作区批⑩:
    //      「khor 里最常改文件的是 agent」). **No timer and no press**:
    //      the only thing that happens between the two readings below is
    //      a session finishing.
    //
    //      The directory becomes a checkout only now — 31a asserted the
    //      opposite corner, and a repository from the start would have
    //      filled it.
    const git = (...args) =>
      execFileSync("git", ["-C", wsDir, ...args], { encoding: "utf8", timeout: 30_000 });
    git("init", "-q");
    git("-c", "user.email=smoke@khor.invalid", "-c", "user.name=smoke", "commit", "-q", "--allow-empty", "-m", "start");
    git("add", "keep.txt");
    git("-c", "user.email=smoke@khor.invalid", "-c", "user.name=smoke", "commit", "-q", "-m", "keep");
    //      One kick, to get a reading khor can be wrong about later.
    //      **What it waits for is a letter, not the absence of one**:
    //      `long.txt` was never added, so a readable checkout puts a U
    //      on it — and waiting for 「no letters」 instead would have been
    //      true at the first instant, before the ask even left, which is
    //      an arm that passes on a khor that cannot read git at all.
    await setHiddenWs(true);
    await setHiddenWs(false);
    await until("the marks back once the directory is a checkout", 30_000, async () =>
      (await page.locator('[data-file="long.txt"] [data-git-mark]').count()) === 1,
    );
    //      …and `keep.txt` is committed and untouched, so it wears
    //      nothing. That is the fact the session below is about to
    //      change.
    if ((await page.locator('[data-file="keep.txt"] [data-git-mark]').count()) !== 0) {
      throw new Error("keep.txt wears a letter before anything touched it — it was just committed clean");
    }
    //      **A session that really goes 忙碌, driven through the op the
    //      hooks drive** (`khor state <word> --session`). The first
    //      spelling here started `sh -c "sleep 3; …"` and waited for it
    //      to finish; the column saw it go `["idle","done"]` and the
    //      letter never came. That is not a bug: **a shell khor opens is
    //      空闲 from birth** (`live::register_observed` writes Idle), so
    //      there is no 忙碌 for it to leave, and the rule is about the
    //      thing that does — 「khor 里最常改文件的是 agent」. An agent
    //      says its words through the hook, and this says them the same
    //      way.
    const wsQuiet = execFileSync(
      KHOR,
      ["open", "-d", "--title", "ws-writes", "--", "sh", "-c", "sleep 600"],
      { env: envA, cwd: wsDir, encoding: "utf8", timeout: 30_000 },
    ).trim();
    //      What the column said about it as it went, so a red names the
    //      link that broke rather than the whole chain.
    const wentBy = [];
    const watch = async () => {
      const word = await page
        .locator(`[data-ws-sessions] [data-row="${wsQuiet}"]`)
        .getAttribute("data-word")
        .catch(() => null);
      if (wentBy[wentBy.length - 1] !== word) wentBy.push(word);
    };
    try {
      //      **The row first, then the word.** A detached host is still
      //      standing up when `khor open` returns, and it seats itself
      //      on the way (`live::reseat` — 「a prompt nobody has typed
      //      into yet」 is 空闲), so a word reported into that gap is
      //      written and then written over. An agent's first hook comes
      //      after it is up; this waits for the same moment.
      await until("the column listing it at all", 60_000, async () => {
        await watch();
        return wentBy[wentBy.length - 1] !== null;
      });
      cli(envA, "state", "busy", "--session", wsQuiet);
      //      **A minute, because this face is beta's.** The row lives on
      //      alpha and reaches this screen over the mesh, on the mesh's
      //      own beat rather than the 2s one this page polls itself on.
      await until("the column showing it busy", 60_000, async () => {
        await watch();
        return wentBy[wentBy.length - 1] === "busy";
      }).catch((e) => {
        throw new Error(
          `${e.message} — the column saw it go ${JSON.stringify(wentBy)}, while alpha's own list says ` +
            JSON.stringify(cli(envA, "sessions")),
        );
      });
      //      Changed **while it was busy**, which is the situation: a
      //      file on that machine moved under a khor that is not
      //      watching the disk.
      writeFileSync(wsKeep, "one\ntwo\n");
      //      Nothing has asked git since, so nothing on screen knows.
      if ((await page.locator('[data-file="keep.txt"] [data-git-mark]').count()) !== 0) {
        throw new Error("keep.txt wore a letter before anything re-asked git — then this proves nothing");
      }
      cli(envA, "state", "idle", "--session", wsQuiet);
      await until("the letter appearing with nobody touching the window", 90_000, async () => {
        await watch();
        return (await page.locator('[data-file="keep.txt"] [data-git-mark]').count()) === 1;
      }).catch(async (e) => {
        const all = await bridgeOp("sessions", { by: "recent" }).catch((x) => String(x));
        const said = Array.isArray(all)
          ? all
              .filter((r) => r.title === "ws-writes")
              .map((r) => `ws-writes word=${r.word} home=${r.home} place=${r.place}`)
              .join("; ") || "the node does not list ws-writes"
          : `the node would not say: ${all}`;
        throw new Error(
          `${e.message} — ${said}; the column saw it go ${JSON.stringify(wentBy)}; ` +
            `git itself says ${JSON.stringify(cli(envA, "git", "status", "", wsDir))}`,
        );
      });
    } finally {
      try {
        cli(envA, "close", wsQuiet);
      } catch {
        // It may already be over; closing a row that has ended is not a
        // failure of this claim.
      }
    }
    const letter = await page
      .locator('[data-file="keep.txt"] [data-git-mark]')
      .getAttribute("data-git-weight");
    console.log(
      `workspaces · an agent stopped: the column saw ${JSON.stringify(wentBy)}, keep.txt now wears ${letter}`,
    );
    if (letter !== "M") {
      throw new Error(`keep.txt wears ${letter} after a file changed under a busy session — git calls that M`);
    }


    // 31j) the change bars wear git's own inks, and the overview ruler
    //      carries the same places — on a real checkout on alpha's disk,
    //      read over the mesh (2026-09-16).
    //
    //      「文件里,有改动或 git 差异的那个竖线,是要有颜色的,和现在的
    //      git 状态颜色一致。」「右侧滑动条后方,就是全文件带颜色的 git
    //      差异的那个竖线。」— the user.
    //
    //      `check-face` measures the three shapes against routed bytes.
    //      What is only true here is that the ink comes out right on a
    //      diff **git itself computed**, between HEAD and a file on
    //      another machine's disk — the node's own `gix` hunks rather
    //      than a fixture's array, which is the one thing a routed
    //      `git_diff` can never be wrong about.
    //
    //      **The change is made to line 1**, so the hunk is a
    //      modification rather than an insertion: `postprocess_lines`
    //      slides an append down to 「a new line 2」 (`git.rs` hunks),
    //      and 修改's ink is the one the ruling names out loud
    //      (「黄色是已修改」, 工作区批⑦).
    writeFileSync(wsKeep, "ONE\ntwo\n");
    await setHiddenWs(true);
    await setHiddenWs(false);
    await until("keep.txt still marked after a second edit", 30_000, async () =>
      (await page.locator('[data-file="keep.txt"] [data-git-mark]').count()) === 1,
    );
    await page.locator('[data-file="keep.txt"] > button').first().click();
    await until("alpha's keep.txt on the sheet with a bar in its gutter", 30_000, async () =>
      (await page.locator('[data-sheet] [data-line="2"]').count()) === 1 &&
      (await page.locator("[data-gutter] [data-hunk]").count()) > 0,
    );
    const painted = await page.evaluate(() => {
      // The four tokens, resolved to what a browser would paint. Read
      // this way rather than compared as hex: a bar's own colour comes
      // back as `rgb(...)` and a token is written as a hex literal, so
      // the only honest comparison is two computed values.
      const inkOf = (name) => {
        const probe = document.createElement("div");
        probe.style.color = `var(${name})`;
        document.body.appendChild(probe);
        const c = getComputedStyle(probe).color;
        probe.remove();
        return c;
      };
      const tokens = {
        modified: inkOf("--git-modified"),
        added: inkOf("--git-added"),
        deleted: inkOf("--git-deleted"),
        conflict: inkOf("--git-conflict"),
        mark: inkOf("--git-mark"),
      };
      const letter = document.querySelector('[data-file="keep.txt"] [data-git-mark]');
      const ruler = document.querySelector("[data-overview]");
      const box = document.querySelector("[data-file-scroll]");
      return {
        tokens,
        weight: letter?.getAttribute("data-git-weight") ?? null,
        letterInk: letter === null ? null : getComputedStyle(letter).color,
        bars: [...document.querySelectorAll("[data-gutter] [data-hunk]")].map((el) => ({
          at: el.getAttribute("data-hunk"),
          ink: getComputedStyle(el).backgroundColor,
        })),
        marks:
          ruler === null
            ? null
            : [...ruler.querySelectorAll("[data-overview-mark]")].map((el) => ({
                at: el.getAttribute("data-overview-mark"),
                ink: getComputedStyle(el).backgroundColor,
                h: Math.round(el.getBoundingClientRect().height * 10) / 10,
              })),
        rulerPastBox:
          ruler === null || box === null
            ? null
            : Math.round((ruler.getBoundingClientRect().left - box.getBoundingClientRect().right) * 10) / 10,
      };
    });
    console.log(`workspaces · git inks: ${JSON.stringify(painted)}`);
    if (painted.bars.length === 0) {
      throw new Error("alpha's modified file drew no change bar, so there is no ink to read");
    }
    if (painted.letterInk === null || painted.weight !== "M") {
      throw new Error(
        `the tree says keep.txt is ${painted.weight} — this claim pairs the bar against 修改's ink and ` +
          "needs git to be calling it a modification",
      );
    }
    for (const bar of painted.bars) {
      if (bar.ink === painted.tokens.mark) {
        throw new Error(
          `the bar at line ${bar.at} is still ${bar.ink}, the foreground at half strength — 「要有颜色的」`,
        );
      }
      if (bar.ink !== painted.letterInk) {
        throw new Error(
          `the bar at line ${bar.at} is ${bar.ink} while the tree draws keep.txt's M in ` +
            `${painted.letterInk} — 「和现在的 git 状态颜色一致」`,
        );
      }
      if (bar.ink !== painted.tokens.modified) {
        throw new Error(
          `…and neither of them is --git-modified (${painted.tokens.modified}), so the pair agrees on ` +
            "a colour that is not git's",
        );
      }
    }
    if (painted.marks === null) {
      throw new Error(
        "there is no overview ruler beside the file — 「右侧滑动条后方,就是全文件带颜色的 git 差异的" +
          "那个竖线」",
      );
    }
    if (painted.marks.length !== painted.bars.length) {
      throw new Error(
        `the ruler carries ${painted.marks.length} mark(s) for ${painted.bars.length} bar(s) — it is the ` +
          "same places, seen whole instead of one at a time",
      );
    }
    for (const mark of painted.marks) {
      if (mark.h <= 0) {
        throw new Error(`the ruler's mark for line ${mark.at} has no height, so nothing is on the map`);
      }
      if (mark.ink !== painted.letterInk) {
        throw new Error(
          `the ruler's mark for line ${mark.at} is ${mark.ink} where the bar and the letter are ` +
            `${painted.letterInk} — one file, one colour`,
        );
      }
    }
    // Over the scrollbar's strip, not beside it (2026-09-16 ruling; the
    // `check-face` probe of the same name says why).
    if (painted.rulerPastBox !== null && painted.rulerPastBox > 0) {
      throw new Error(
        `the ruler is a column of its own, ${painted.rulerPastBox}px past the file's scroller — ` +
          "it lies over the scrollbar's strip,「在滑动条的下方…透过滑动条颜色浮上来」",
      );
    }

    // 31k) 新建 / 改名 / 打开, on alpha's real disk (工作区第五轮 第 35、
    //      36 条). **CLI only, and deliberately** — what the face gate
    //      cannot be wrong about is the button; what only this run can
    //      answer is whether the verb reaches a real filesystem on
    //      another machine and refuses in the one place it promises to.
    //
    //      Asked from beta's home over the mesh (`-m alpha`) and checked
    //      against the disk from here: `existsSync` is the independent
    //      witness, the way it is for 删掉 above. **Every refusal is
    //      asserted twice** — a non-zero exit *and* the bytes still
    //      being what they were — because 「没有 -f」 means the refusal
    //      is the feature, and a verb that refused out loud while
    //      writing anyway would pass the first half alone.
    {
      const born = join(wsDir, "born.txt");
      const moved = join(wsDir, "moved.txt");
      const nested = join(wsDir, "made-dir");
      cli(envB, "touch", "-m", "alpha", born);
      if (!existsSync(born)) {
        throw new Error(`khor touch -m alpha ${born} answered but nothing is on the disk`);
      }
      if (readFileSync(born, "utf8") !== "") {
        throw new Error("khor touch made a file with something in it — a new name is empty");
      }
      cli(envB, "mkdir", "-m", "alpha", nested);
      if (!statSync(nested).isDirectory()) {
        throw new Error(`khor mkdir -m alpha ${nested} did not make a directory`);
      }
      writeFileSync(born, "mine\n");
      const clash = spawnSync(KHOR, ["touch", "-m", "alpha", born], {
        env: envB,
        encoding: "utf8",
        timeout: 30_000,
      });
      if (clash.status === 0) {
        throw new Error("khor touch over an existing name succeeded — 已有就拒, there is no -f");
      }
      if (readFileSync(born, "utf8") !== "mine\n") {
        throw new Error("a refused touch emptied the file anyway — the refusal wrote");
      }
      const twice = spawnSync(KHOR, ["mkdir", "-m", "alpha", nested], {
        env: envB,
        encoding: "utf8",
        timeout: 30_000,
      });
      if (twice.status === 0) {
        throw new Error("khor mkdir over an existing directory succeeded — 已有就拒");
      }
      // …and a move really moves, across the wire.
      cli(envB, "mv", "-m", "alpha", born, moved);
      if (existsSync(born) || readFileSync(moved, "utf8") !== "mine\n") {
        throw new Error("khor mv -m alpha moved nothing, or moved the wrong bytes");
      }
      // A target that is taken is refused and the other file survives —
      // `std::fs::rename` would have eaten it without a word.
      cli(envB, "touch", "-m", "alpha", born);
      writeFileSync(born, "second\n");
      const over = spawnSync(KHOR, ["mv", "-m", "alpha", moved, born], {
        env: envB,
        encoding: "utf8",
        timeout: 30_000,
      });
      if (over.status === 0) {
        throw new Error("khor mv over an existing name succeeded — 目标已有就拒, 不覆盖");
      }
      if (readFileSync(born, "utf8") !== "second\n" || !existsSync(moved)) {
        throw new Error("a refused mv moved something anyway");
      }
      // 第 35 条, the opener. alpha's is `/usr/bin/true` for the life of
      // this run (`envA`), so 「it was handed over」 is measured without
      // a window appearing anywhere.
      cli(envB, "launch", "-m", "alpha", wsFile);
      cli(envB, "launch", "-m", "alpha", "https://example.test/x");
      // **The two refusals, on gamma.** gamma runs no serve, so the
      // verb takes the direct road and reads the environment of the
      // process being started here — which is the only way to ask
      // 「what does a machine with no opener say」 without restarting
      // one of the two that do.
      const noOpener = spawnSync(KHOR, ["launch", wsFile], {
        env: { ...envG, KHOR_LAUNCHER: "" },
        encoding: "utf8",
        timeout: 30_000,
      });
      if (noOpener.status === 0) {
        throw new Error("khor launch on a machine with no opener answered 开了");
      }
      if (!`${noOpener.stderr}`.includes("KHOR_LAUNCHER")) {
        throw new Error(
          `a machine with no opener said ${JSON.stringify(noOpener.stderr)} — the refusal names the ` +
            "variable, which is the one thing a person can do about it",
        );
      }
      const gone = spawnSync(KHOR, ["launch", wsFile], {
        env: { ...envG, KHOR_LAUNCHER: join(SCRATCH, "no-such-opener") },
        encoding: "utf8",
        timeout: 30_000,
      });
      if (gone.status === 0) {
        throw new Error("khor launch with an opener that is not on the disk answered 开了");
      }
      // What is **not** openable is refused before the opener is even
      // looked up — the whitelist is the point (第 35 条).
      for (const bad of ["notes.md", "mailto:someone@example.test", "-a"]) {
        const refused = spawnSync(KHOR, ["launch", "-m", "alpha", bad], {
          env: envB,
          encoding: "utf8",
          timeout: 30_000,
        });
        if (refused.status === 0) {
          throw new Error(
            `khor launch ${bad} was taken — only an absolute path or an http(s) address is`,
          );
        }
      }
    }

    //      Back to the landing, so 31f starts from the list rather than
    //      from wherever the last press left this pane.
    await openLanding("workspaces");
  } finally {
    //      Detached hosts outlive this script unless they are told
    //      (the orphan discipline).
    for (const id of [wsInside, wsBeside]) cli(envA, "close", id);
  }

  // 31f) moved out: the decision replicates back, and the directory is
  //      not touched.
  await until("the list back with the row on it", 20_000, async () =>
    (await page.locator("[data-ws-row] [data-ws-name]").allInnerTexts()).includes(wsName),
  );
  const backRow = page.locator("[data-ws-row]").filter({ hasText: wsName }).first();
  await backRow.click({ button: "right" });
  await until("the row's menu", 10_000, async () =>
    (await page.locator('[data-menu-item="remove"]').count()) === 1,
  );
  await page.locator('[data-menu-item="remove"]').click();
  await until("the row gone from beta's list", 20_000, async () =>
    !(await page.locator("[data-ws-row] [data-ws-name]").allInnerTexts()).includes(wsName),
  );
  await until("and gone from alpha's table", 40_000, () => !cli(envA, "ws", "ls").includes(wsDir));
  if (!existsSync(wsDir) || !existsSync(join(wsDir, "sub"))) {
    throw new Error("移出 took the directory with it — it takes the row and nothing else");
  }
  console.log(`workspaces: declared ${wsName} on alpha from beta's face, listed it, deleted a file on it, and moved it out`);

  // Back to the sessions landing: what follows reads rows off it.
  await openLanding("sessions");

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
  //     **Where the report lands moved with the control.** The action
  //     is a menu item now, so the refusal is worn there — and the menu
  //     **stays open** to wear it. That is the design: a menu that
  //     closed on failure would take the only surface the sentence has
  //     with it, which is the same "started something and nothing
  //     happened" this file refuses everywhere else.
  const failTarget = (await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row)))[0];
  // **The colour is read after the item's own transitions finish**, not
  // the instant the attribute flips. A reading taken at t=0 of a
  // transition is bit-for-bit the colour it is *leaving* — which is what
  // one observed failure here reported, with the failure attribute
  // already set. Waiting on the element's own animations is the "wait
  // for the thing itself" rule; a fixed sleep would measure how busy the
  // machine is.
  const pinItemState = () =>
    page.locator('[data-menu-item="pin"]').evaluate(async (el) => {
      await Promise.all(el.getAnimations().map((a) => a.finished.catch(() => {})));
      return { word: el.innerText.trim(), color: getComputedStyle(el).color };
    });

  //     The control comes first: the same item, pressed while the
  //     backend is there, must come back *not* wearing the face — and a
  //     pin that takes closes the menu, which is itself the success
  //     signal. A mark that is always on is not a failure report.
  await openRowMenu(failTarget);
  const okItem = await pinItemState();
  if (!okItem.word) throw new Error("probe dead: the pin item prints nothing");
  await page.locator('[data-menu-item="pin"]').click();
  await until("the pin to take while the backend is there", 10_000, async () =>
    (await page.locator(`[data-row="${failTarget}"][data-pinned=true]`).count()) === 1,
  );
  await until("the menu to close itself on success", 5_000, async () =>
    (await page.locator("[data-row-menu]").count()) === 0,
  );

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

  await openRowMenu(failTarget);
  //     **The control is read here, not before the successful pin.**
  //     Written the other way first, and it was a false gate: the good
  //     pin legitimately turns 置顶 into 取消置顶, so "the word differs
  //     from the one read at the start" is already true before anything
  //     fails — the wait fell straight through and the colour was read
  //     off an item in its ordinary state. A control has to be taken
  //     from the moment the change is being measured against.
  const beforeFail = await pinItemState();
  if (beforeFail.color === failedColor) {
    throw new Error(`the item already wears the failure colour: ${JSON.stringify(beforeFail)}`);
  }
  await page.locator('[data-menu-item="pin"]').click();
  //     Three things, and the first is what makes the other two
  //     readable: the menu is still there to be read.
  await until("the menu to stay open on failure", 10_000, async () =>
    (await page.locator("[data-row-menu]").count()) === 1,
  );
  await until("the failure word on the item that was pressed", 10_000, async () =>
    (await pinItemState()).word !== beforeFail.word,
  );
  const badItem = await pinItemState();
  // The word changes with it, so the report is not colour alone —
  // compared against the word this same item wore a moment ago rather
  // than spelled here, because the words belong to the catalog.
  if (!badItem.word || badItem.word === beforeFail.word) {
    throw new Error(`the failed item kept its old word: ${JSON.stringify(badItem)}`);
  }
  if (badItem.color !== failedColor) {
    throw new Error(
      `the failed item is not wearing the failure colour: ${badItem.color} vs ${failedColor}`,
    );
  }
  // …and it is that row's report, not a mood the menu is in. Escape out,
  // open another row's menu, and its item must be resting — which is a
  // stronger claim than the old "only one button went red": this says
  // the memory is kept per row rather than per screen.
  await closeRowMenu();
  const otherRow = (await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row)))
    .find((id) => id !== failTarget);
  if (!otherRow) throw new Error("probe dead: only one row to compare against");
  await openRowMenu(otherRow);
  const neighbour = await pinItemState();
  if (neighbour.color === failedColor) {
    throw new Error(`another row's menu inherited the failure: ${JSON.stringify(neighbour)}`);
  }
  await closeRowMenu();

  // 26b) 关会话失败也就地说 (批② + 2026-09-02 关会话移入行菜单). The same
  //      backend-away window, because this is the only place a close is
  //      *certain* to fail: whether a given row can be closed is
  //      `Node::close`'s judgment and the pane deliberately does not
  //      pre-empt it, so with the backend up there is no row this script
  //      can be sure will refuse.
  //
  //      The close lives in the row's menu now, beside 置顶 (标题瘦身:
  //      「标签本来就自带关闭」took it off the header). A rejection leaves
  //      the menu open with the refusal on the item — RowMenu's rule,
  //      the same one the pin failure above rides — which is the
  //      in-place report. The control is the note's own absence
  //      beforehand: a report already on screen would prove nothing
  //      about the press.
  await openRowMenu(failTarget);
  if ((await page.locator('[data-menu-note="close"]').count()) !== 0) {
    throw new Error("a close was reported failed before one was asked for");
  }
  await page.locator('[data-menu-item="close"]').click();
  await until("the confirm", 10_000, async () =>
    (await page.locator('[data-menu-confirm="close"]').count()) === 1,
  );
  await page.locator('[data-menu-item="close"]').click();
  await until("the failure said on the item that asked", 15_000, async () =>
    (await page.locator('[data-menu-note="close"]').count()) === 1,
  );
  if (!(await page.locator('[data-menu-note="close"]').innerText()).trim()) {
    throw new Error("the failure must be a sentence, not an empty note");
  }
  // The confirm goes when it fails — leaving it armed would read as the
  // close still being on offer, on a backend that cannot take it.
  if ((await page.locator('[data-menu-confirm="close"]').count()) !== 0) {
    throw new Error("a failed close must not leave its confirm standing");
  }
  await closeRowMenu();


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
