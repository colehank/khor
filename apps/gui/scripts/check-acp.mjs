import {routeCanonicalChat} from './fixtures/canonical-chat-routes.mjs';
// The ACP-surface gate: what the conversation face draws from the
// *protocol's* frames, as opposed to from its own text.
//
// **The hole this fills.** `check-read` measures the prose — size,
// leading, column, contrast — off one painted conversation, and it
// paints a tool call only to have something whose shape differs from a
// blockquote. Nothing anywhere asserts that a frame carrying a *state*
// reaches the screen as one. That mattered little while khor's own
// claude 垫片 was the only source and it never sent a state; it stops
// being little the moment a registry agent is attached, because then
// the states arrive on the wire and being dropped is a loss of real
// data rather than an absence of it.
//
// Claims:
//   0. probe: tool lines painted at all, and the count is the fixture's
//      — every claim below is negative-shaped ("these two differ"), and
//      a dead selector answers those by returning nothing.
//   1. each fixture status reaches the DOM as itself. This is the
//      `tool_call_update` fold: a call opens `in_progress` and a later
//      frame naming its id settles it, so a face that ignored the
//      second frame would report three running tools.
//   2. the three statuses are drawn in three *visibly* different inks,
//      measured as ΔL* rather than as a contrast ratio — the question
//      is "can a person tell these two lines apart", and a ratio
//      answers "can a person read this line against that ground",
//      which is a different question with a different answer.
//   3. a failed tool wears 失败's own ink. Not a new colour: the six
//      words own the palette, and a tool that failed is that word's
//      subject too.
//   4. the fact survives `prefers-reduced-motion`. The running line
//      breathes (`data-word="busy"`, app.css), and the breath is
//      decoration by that rule's own terms — so with motion off the
//      ink must still tell running from settled.
//   5. nothing threw while being measured. React drops only the
//      subtree that threw, so a half-painted conversation answers
//      geometry questions perfectly well.
//   6. a `usage_update` reaches the screen *as a share*. The DOM keeps
//      both numbers, and the assertion is on the printed string —
//      dividing the wrong way holds the right two numbers and prints
//      313%, which is a bug no assertion on the numbers can see.
//   7. a `session_info_update` title is drawn. **claude never sends
//      one** (measured off a real stream-json run: it writes `ai-title`
//      to its transcript and nothing to the stream), so this claim is
//      about registry agents — and an ability nothing exercises is one
//      nobody finds out is broken.
//   8. handshake facts reach the face: login explains an empty composer,
//      while NoLoad does not deny independently captured canonical content.
//   9. a `plan` reaches the screen in order, and a finished step reads
//      differently from the one being worked. **claude sends no plan
//      either** — measured 2026-08-30, its tool list on this machine
//      has 128 entries and nothing todo-shaped; an earlier note in this
//      repo said the plan would come from a `TodoWrite` tool, and there
//      is no such tool.
//  10. an image and a file paste into the composer **only when the
//      agent said it takes each kind** — asserted against each other in
//      one paste, because separate runs let each capability answer for
//      the other. Both directions, because "no attach affordance"
//      is a claim a page that never rendered satisfies for free — so
//      the positive row has to find the very thing the negative row
//      must not.
//  10b. a tool line says which call it was, and three different calls
//      paint three different lines. The second half is the complaint —
//      「大量 bash,不可展开也没有标题」— and a per-line check alone is
//      satisfied by three correct titles that happen to be identical.
//  12. the agent's mode shows when it is not the ordinary one, and
//      **stays silent when it is** — that second half is the design,
//      and it is a negative claim, so the rows around it have to find
//      the element it must not.
//  11. a tool line opens onto its diff or its output, and a line with
//      nothing behind it grows no control at all. Then it is actually
//      opened and the diff and the output are read back — a title that
//      claims to open is not the same as one that does.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

// **The words are the catalog's, read from the generated face** — a
// sentence copied into this file moves out of step the day the catalog
// is reworded (2026-09-28: 「只看不改」 became 「只读不改」 and three
// literals here went stale at once). `words` rather than `gui`: that
// name is this app's directory below.
import { gui as words } from "../src/gen/catalog.ts";

const gui = fileURLToPath(new URL("..", import.meta.url));
const PORT = Number(process.env.ACP_PORT ?? 1456);
const WIDE = 1482;
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

const fail = (m) => {
  console.error(m);
  process.exitCode = 1;
};

/**
 * How far apart two inks must be to count as "a person can tell these
 * two lines apart", in L* (CIE lightness, 0–100).
 *
 * **3 is not a comfort threshold, it is a detection one**, and that is
 * deliberate: the claim being defended is only that the three states
 * are *distinguishable*, not that each is legible on its own — legibility
 * against the ground is `check-palette`'s subject and is asserted there.
 * A larger number here would start failing on hue-only differences that
 * a reader has no trouble with.
 */
const DELTA_L = 3;

/** The fixture, in the protocol's own shape.
 *
 * Three calls and two closings, on purpose: the third is left open, so
 * "running" is a state the fixture actually contains rather than one
 * inferred from the absence of a closing frame.
 *
 * The ids are claude's shape (`toolu_…`) because the face matches
 * updates to calls by id, and an id that is unique per call is the only
 * reason two calls to the same tool do not fold into one line. */
const frames = [
  {
    kind: "note",
    update: {
      update: {
        sessionUpdate: "user_message_chunk",
        content: { type: "text", text: "跑一下测试，然后读一下那个文件" },
      },
    },
  },
  // An edit, which carries its own diff — the payload shape khor's
  // 垫片 builds out of claude's `old_string`/`new_string`.
  { kind: "note", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_ok", title: "Edit src/lib.rs", kind: "edit", status: "in_progress",
    rawInput: { file_path: "/w/khor/src/lib.rs", old_string: "let x = 1;", new_string: "let x = 2;" },
    locations: [{ path: "/w/khor/src/lib.rs" }],
    content: [{ type: "diff", path: "/w/khor/src/lib.rs", oldText: "let x = 1;", newText: "let x = 2;" }] } } },
  { kind: "note", update: { update: { sessionUpdate: "tool_call_update", toolCallId: "toolu_ok", status: "completed" } } },
  // A failed shell command that said why.
  { kind: "note", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_bad", title: "跑一遍测试", kind: "execute", status: "in_progress",
    rawInput: { command: "cargo test -p khor-node", description: "跑一遍测试" } } } },
  { kind: "note", update: { update: { sessionUpdate: "tool_call_update", toolCallId: "toolu_bad", status: "failed",
    content: [{ type: "content", content: { type: "text", text: "error[E0425]: cannot find value `x`" } }] } } },
  // A call still running, with nothing behind it — the one line that
  // must **not** grow a control, because a control onto nothing is
  // worse than no control.
  { kind: "note", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_run", title: "Grep kind_of", kind: "search", status: "in_progress" } } },
  // Batch 2. `used`/`size` are the shape claude actually puts on the
  // wire (captured 2026-08-30: 32126 tokens against a 1 000 000 window),
  // so the share the face prints is a real one and not a round number
  // chosen to make the arithmetic obvious.
  { kind: "note", update: { update: { sessionUpdate: "usage_update", used: 320000, size: 1000000, cost: { amount: 0.38777300000000003, currency: "USD" } } } },
  // A title, which claude never sends — see `ChatView`'s note. In the
  // fixture because the *face's* ability to read it is what the next
  // batch depends on, and an ability nothing exercises is one nobody
  // finds out is broken.
  { kind: "note", update: { update: { sessionUpdate: "session_info_update", title: "把工具调用的状态接上" } } },
  // A plan, which claude also never sends — its 128-tool list here has
  // nothing todo-shaped. Three entries so all three statuses are
  // present at once, which is what claim 9 measures them against.
  { kind: "note", update: { update: { sessionUpdate: "plan", entries: [
    { content: "读那个文件", status: "completed", priority: "medium" },
    { content: "改那个文件", status: "in_progress", priority: "high" },
    { content: "跑测试", status: "pending", priority: "low" },
  ] } } },
  {
    kind: "note",
    update: {
      update: {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "一条跑完了，一条失败了，还有一条在跑。" },
      },
    },
  },
];

const row = (id, kind, title) => ({
  id, kind, title, word: "idle", at_ms: Date.now(), unread: 0, source: null,
  pinned: false, stale: false, category: "claude", group: "", face: null,
  hosted: true, can_control: kind === "gui", attachable: false, last: null, via: null, via_face: null, home: "gate",
});
const ROWS = [row("gui/acp-gate", "gui", "acp-gate-agent")];

/** Everything read off the painted conversation, in the page.
 *
 * The inks come back as the browser's own `rgb(...)` and are converted
 * out here. A gate that pulled the hex out of a token and parsed digits
 * from it would be reading the stylesheet rather than the screen — and
 * that exact shortcut once turned a red proof green in this repo. */
const probe = () => {
  const rgba = (s) => {
    const m = String(s).match(/rgba?\(([^)]+)\)/);
    if (!m) return null;
    const p = m[1].split(",").map((v) => Number(v.trim()));
    return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
  };
  /** The ground an element is actually painted on: the first ancestor
      with a background that is not fully transparent. */
  const groundOf = (el) => {
    for (let n = el; n; n = n.parentElement) {
      const c = rgba(getComputedStyle(n).backgroundColor);
      if (c && c.a > 0) return c;
    }
    return { r: 255, g: 255, b: 255, a: 1 };
  };
  /** **Composited, because the reading inks carry alpha.**
      `--muted-foreground` is `rgba(…, .6)`, and `getComputedStyle`
      hands that back with its alpha intact. Taking the first three
      numbers and calling it the colour measures a swatch nobody sees —
      it read L*93.9 for a grey that lands near 60 on the dark card, and
      it would let two lines that look identical pass as far apart. */
  const flatten = (fg, bg) => {
    const a = fg.a;
    return `rgb(${Math.round(fg.r * a + bg.r * (1 - a))}, ${Math.round(
      fg.g * a + bg.g * (1 - a),
    )}, ${Math.round(fg.b * a + bg.b * (1 - a))})`;
  };
  const lines = [...document.querySelectorAll("[data-chat-tool]")].map((el) => {
    const text = el.querySelector("[data-word-text]") ?? el;
    const fg = rgba(getComputedStyle(text).color);
    return {
      status: el.getAttribute("data-tool-status"),
      kind: el.getAttribute("data-tool-kind"),
      word: el.getAttribute("data-word"),
      openable: el.getAttribute("data-tool-open"),
      title: (el.querySelector("[data-word-text]")?.textContent ?? "").trim(),
      body: !!el.querySelector("[data-tool-body]"),
      // The mark, as a **still image**: what a reader sees with the
      // breath switched off — its composited colour, and whether it is
      // filled at all. Both, because a mark can be told from another by
      // being a different weight of grey *or* by being a ring instead
      // of a disc, and the design uses each of them somewhere.
      dot: (() => {
        const d = el.querySelector("[data-tool-dot]");
        if (!d) return null;
        const cs = getComputedStyle(d);
        const fill = rgba(cs.backgroundColor);
        const filled = !!fill && fill.a > 0.05;
        // The ring is a `box-shadow`; its colour is the first `rgb(...)`
        // in the resolved value.
        const ringHit = /rgba?\([^)]*\)/.exec(cs.boxShadow || "");
        const ring = ringHit ? rgba(ringHit[0]) : null;
        const ground = groundOf(el);
        const showing = filled ? fill : ring;
        return {
          state: d.getAttribute("data-dot-state"),
          filled,
          ink: showing ? flatten(showing, ground) : null,
          size: `${cs.width}x${cs.height}`,
          animated: cs.animationName !== "none" && cs.animationName !== "",
        };
      })(),
      diff: !!el.querySelector("[data-tool-diff]"),
      out: (el.querySelector("[data-tool-out]")?.textContent ?? "").trim(),
      raw: getComputedStyle(text).color,
      ink: fg ? flatten(fg, groundOf(el)) : null,
      title: (el.textContent ?? "").trim(),
    };
  });
  // **Resolved by the engine, not parsed here.** Reading the custom
  // property gives back whatever the stylesheet literally wrote — a
  // hex, an `oklch(...)`, a `color-mix(...)` — and a gate that pulls
  // digits out of that string is comparing a token to a screen. The
  // first run of this gate did exactly that and reported "probe dead"
  // on a perfectly healthy `#ffb8ad`. So: paint the token onto a real
  // element and read what the browser resolved it to, which is the same
  // `rgb(...)` the line under test comes back as.
  const anchor = document.querySelector("[data-chat-tool]") ?? document.body;
  const swatch = document.createElement("span");
  swatch.style.cssText = "position:absolute;left:-9999px;color:var(--state-failed)";
  anchor.appendChild(swatch);
  const fgT = rgba(getComputedStyle(swatch).color);
  const failedToken = fgT ? flatten(fgT, groundOf(anchor)) : null;
  swatch.remove();
  // The positive control for claim 0: something this fixture certainly
  // paints, found by a different selector than the one under test.
  const paragraphs = document.querySelectorAll("[data-chat-column] p").length;
  const factsLine = document.querySelector("[data-chat-facts]");
  const planEls = [...document.querySelectorAll("[data-plan-entry]")];
  const planRows = planEls.map((el) => {
    const fg = rgba(getComputedStyle(el).color);
    return {
      status: el.getAttribute("data-plan-status"),
      text: (el.textContent ?? "").trim(),
      ink: fg ? flatten(fg, groundOf(el)) : null,
    };
  });
  const title = document.querySelector("[data-chat-title]");
  const modeEl = document.querySelector("[data-lab-execution-badge]");
  const shots = document.querySelectorAll("[data-chat-shot]").length;
  const docs = document.querySelectorAll("[data-chat-doc]").length;
  const hasInput = !!document.querySelector("[data-chat-input]");
  return {
    lines, failedToken, paragraphs,
    facts: factsLine ? (factsLine.textContent ?? "").trim() : null,
    plan: planRows, shots, docs, hasInput,
    mode: modeEl ? (modeEl.textContent ?? "").trim() : null,
    title: title ? (title.textContent ?? "").trim() : null,
    column: !!document.querySelector("[data-chat-column]"),
    input: !!document.querySelector("[data-chat-input]"),
    text: document.body.innerText,
  };
};

/** sRGB → L*. The same conversion `check-palette` uses. */
const lstar = (rgb) => {
  const m = String(rgb).match(/rgba?\(([^)]+)\)/);
  if (!m) return null;
  const [r, g, b] = m[1].split(",").slice(0, 3).map((v) => Number(v.trim()) / 255);
  const lin = (c) => (c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));
  const Y = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  return Y > 216 / 24389 ? 116 * Math.cbrt(Y) - 16 : Y * 24389 / 27;
};

let vite = null;
let browser = null;
try {
  vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], {
    cwd: gui, stdio: ["ignore", "pipe", "pipe"],
  });
  let gone = null;
  vite.on("exit", (code) => (gone = `vite exited with ${code} — is ${PORT} already taken?`));
  let ready = false;
  vite.stdout.on("data", (d) => (ready ||= String(d).includes(`:${PORT}`)));
  for (let i = 0; i < 150 && !ready; i++) {
    if (gone) throw new Error(gone);
    await wait(100);
  }
  if (!ready) throw new Error(`vite never came up on ${PORT}`);

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });

  // Read current production controls through the same actions a person takes.
  // This replaces retired composer metadata; no values come from the fixture feed.
  const inspectControls = async (page) => {
    await page.locator('[data-lab-session-settings]').click();
    const menu = page.locator('[data-lab-preferences]');
    await menu.waitFor();
    const root = await menu.evaluate((element) => {
      const facts = Object.fromEntries([...element.querySelectorAll('.composer-lab-fact')].map(row =>
        [...row.children].map(child => (child.textContent ?? '').trim())));
      const agent = [...element.querySelectorAll('[data-slot="dropdown-menu-label"]')]
        .map(row => (row.textContent ?? '').trim()).find(text => text.endsWith('· 会话设置'));
      return {facts, agentIdentity: agent?.replace(/\s*· 会话设置$/, '') ?? null,
        controls: [...element.querySelectorAll('[data-lab-option]')].map(row => ({
          id: row.getAttribute('data-lab-option'), role: row.getAttribute('role'),
          on: row.getAttribute('aria-checked'), tag: row.tagName.toLowerCase(),
          summary: (row.querySelector('.composer-lab-menu-value')?.textContent ?? '').trim(),
          text: (row.textContent ?? '').trim(),
        })),
      };
    });
    const cost = await page.evaluate(() => document.body.innerText.match(/.{0,12}(\$|USD|0\.39).{0,12}/)?.[0] ?? '');
    const options = [];
    let modeMenu = null;
    for (const control of root.controls) {
      if (control.role === 'menuitemcheckbox') {
        options.push(control);
        continue;
      }
      await menu.locator(`[data-lab-option="${control.id}"]`).click();
      const selected = menu.locator('[role="menuitemradio"][data-state="checked"]');
      await selected.waitFor();
      const value = await selected.evaluate(row => ({
        id: row.getAttribute('data-option-value'),
        name: [...row.querySelectorAll(':scope > span')].map(child => (child.textContent ?? '').trim()).find(Boolean) ?? '',
      }));
      assert.equal(control.summary, value.name, `${control.id}: drawn summary matches the selected choice`);
      const choice = {...control, text: value.name, value: value.id, menuRole: await menu.getAttribute('role')};
      if (control.id === 'execution-permission') modeMenu = {...choice, id: value.id};
      else options.push(choice);
      await menu.locator('[data-session-menu-back]').click();
    }
    await menu.getByRole('menuitem', {name: '模型详情', exact: true}).click();
    const modelReported = await menu.locator('[data-lab-model-id]').textContent();
    await page.keyboard.press('Escape');
    await menu.waitFor({state: 'hidden'});
    return {modeMenu, options, permission: modeMenu?.text ?? root.facts['执行权限'] ?? null,
      agentIdentity: root.agentIdentity, modelReported,
      usage: root.facts['上下文'] === undefined ? null : {share: root.facts['上下文'], cost},
    };
  };

  const look = async ({ scheme, motion, frames: override, paste = false, openTools = false, send = false }) => {
    const feed = override ?? frames;
    const ctx = await browser.newContext({
      viewport: { width: WIDE, height: 900 },
      colorScheme: scheme,
      reducedMotion: motion,
      deviceScaleFactor: 1,
    });
    const page = await ctx.newPage();
    const errs = [];
    page.on("pageerror", (e) => errs.push(String(e)));
    const json = (b) => (r) => r.fulfill({ contentType: "application/json", body: JSON.stringify(b) });
    // The current lock reader fails closed: this isolated scenario explicitly
    // says its private fixture has no PIN/lock, rather than relying on a failed read.
    await page.route("**/lock_state", json({on: false, has_pin: false, timeout_min: 0,
      timeouts: [0, 1, 5, 15, 60], pin_min: 6, system_door: false}));
    await page.route("**/sessions", json(ROWS));
    // The row's own machine. `home: "gate"` on the row points here, and
    // without it the list paints nothing — which arrives as a click
    // timeout rather than as an empty list, so it reads like the page
    // hung.
    await page.route("**/devices", json([{
      id: "e".repeat(64), name: "gate", me: true, face: null, pinned: false,
      vitals: null, hop: null, sponsor: null, watch: [], watched: null, seat: { left: 50, top: 50 }, hosts: true,
    }]));
    await page.route("**/hooks_state", json({ installed: false }));
    await page.route("**/quota", json({ quota: null, trouble: null }));
    await page.route("**/dir_pins", json([]));
    await page.route("**/web_pins", json([]));
    await page.route("**/recents", json([]));
    // What the face actually put on the wire when 发送 was pressed.
    // **Attaching and sending are two claims**: a chip on screen and a
    // frame carrying the file are different facts, and the first is
    // satisfied by a send path that drops the second.
    const sent = [];
    await page.route("**/chat_say", (r) => {
      try {
        sent.push(JSON.parse(r.request().postData() ?? "{}"));
      } catch {
        sent.push(null);
      }
      r.fulfill({ contentType: "application/json", body: "null" });
    });
    let served = false;
    await routeCanonicalChat(page, {ids: ['gui/acp-gate'], takeFrames: () => {
      if (served) return [];served = true;return feed;
    }});
    await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });
    await wait(900);
    await page.locator("[data-list] [data-row-open]").nth(0).click();
    await wait(1200);
    if (openTools) {
      // **Opened for real, not asserted from a flag.** A line that
      // wears `data-tool-open=no` is claiming it can open; only
      // pressing it shows whether anything is behind the claim.
      const toggles = await page.locator("[data-tool-toggle]").all();
      for (const t of toggles) await t.click();
      await wait(400);
    }
    if (paste) {
      // A real paste event carrying a real PNG. Synthesised rather than
      // driven through the OS clipboard because the OS clipboard is not
      // available here — but the event is the browser's own
      // `ClipboardEvent` with a `File` in its items, which is exactly
      // what the handler reads.
      await page.evaluate(() => {
        const png = Uint8Array.from(atob(
          "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
        ), (c) => c.charCodeAt(0));
        const file = new File([png], "shot.png", { type: "image/png" });
        // A source file with **no MIME type**, which is what the OS
        // reports for `.rs` here — the case a type-only test would
        // wrongly pass by never exercising.
        const doc = new File(["fn main() {}\n"], "main.rs", { type: "" });
        const dt = new DataTransfer();
        dt.items.add(file);
        dt.items.add(doc);
        const box = document.querySelector("[data-chat-input]");
        box?.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true }));
      });
      await wait(600);
    }
    const controls = await inspectControls(page);
    const out = {...await page.evaluate(probe), ...controls};
    if (send) {
      await page.locator("[data-chat-input]").fill("看看这个");
      await page.locator("[data-chat-send]").click();
      await wait(500);
    }
    await ctx.close();
    return { ...out, errs, scheme, motion, sent };
  };

  const WANT = ["completed", "failed", "in_progress"];

  // ── batch 3: the handshake's facts reach the face ────────────────
  //
  // A separate fixture rather than more frames on the one above,
  // because the claim is about an **empty** conversation: the line only
  // exists while there is nothing to explain it away, and adding it to
  // a painted conversation would assert the opposite of the rule.
  for (const [label, agentFrame, want] of [
    ["NoLoad canonical source", { kind: "agent", name: "reg", replays: false, images: false, embeds: false, lists: false, needs_login: false }, null],
    ["needs a login", { kind: "agent", name: "reg", replays: true, images: false, embeds: false, lists: false, needs_login: true }, words.chat_needs_login],
    ["ordinary agent", { kind: "agent", name: "reg", replays: true, images: false, embeds: false, lists: false, needs_login: false }, null],
  ]) {
    const m = await look({ scheme: "dark", motion: "no-preference", frames: [agentFrame, ...(agentFrame.replays ? [] : [{kind: "note", update: {update: {sessionUpdate: "agent_message_chunk", content: {type: "text", text: "Captured NoLoad history"}}}}])] });
    if (m.errs.length) fail(`facts/${label}: the page threw — ${m.errs[0]}`);
    if (!agentFrame.replays && !m.text.includes("Captured NoLoad history")) fail("NoLoad: the canonical body is missing");
    // Empty ordinary/login fixtures still require the real column. The
    // NoLoad fixture additionally requires its positive canonical body.
    if (!m.column) {
      fail(`facts/${label}: probe dead — the conversation face never painted`);
      continue;
    }
    if (want === null) {
      if (m.facts !== null) {
        fail(`facts/${label}: an agent with nothing to explain drew ${JSON.stringify(m.facts)}`);
      }
    } else if (!m.facts || !m.facts.includes(want)) {
      fail(`facts/${label}: wanted a line containing ${want}, got ${JSON.stringify(m.facts)}`);
    } else {
      console.log(`facts · ${label}: ${m.facts}`);
    }
  }

  // ── the mode marker ──────────────────────────────────────────────
  //
  // Default reports no nondefault permission. The current header exposes
  // the other words as actual configuration facts (or selected choices);
  // positive neighboring rows keep the default negative from passing for free.
  for (const [id, want] of [
    ["plan", words.chat_mode_plan],
    ["acceptEdits", words.chat_mode_accept_edits],
    ["bypassPermissions", words.chat_mode_bypass],
    // A mode this face has never heard of is still not the default.
    ["someFutureMode", "someFutureMode"],
    ["default", null],
  ]) {
    const m = await look({
      scheme: "dark",
      motion: "no-preference",
      frames: [
        { kind: "note", update: { update: { sessionUpdate: "current_mode_update", currentModeId: id } } },
      ],
    });
    if (m.errs.length) fail(`mode/${id}: the page threw — ${m.errs[0]}`);
    if (!m.column) {
      fail(`mode/${id}: probe dead — the conversation face never painted`);
      continue;
    }
    if (want === null) {
      if (m.permission !== null || m.mode !== null) {
        fail(`mode/${id}: the ordinary mode drew ${JSON.stringify(m.permission ?? m.mode)} — a marker that is always there is not one`);
      } else {
        console.log(`mode · ${id}: silent, as it must be`);
      }
    } else if (m.permission !== want) {
      fail(`mode/${id}: drew ${JSON.stringify(m.permission)}, wanted ${JSON.stringify(want)}`);
    } else {
      console.log(`mode · ${id}: ${m.permission}`);
    }
  }

  // ── batch 3a: what can be set ────────────────────────────────────
  //
  // 11. a `settings` frame reaches the screen as controls: the mode
  //     menu wearing the current mode's word, a select wearing its
  //     current value's name, a switch wearing its position. The frame
  //     is the host's (`GuiNote::Settings`), sent whole, so what is
  //     asserted is that the face paints from it and not from the
  //     `current_mode_update` note beside it — the fixture carries no
  //     such note, which is the control: with the menu drawn from the
  //     settings frame alone, the marker (claim above) must stay off,
  //     because the same word drawn twice is the thing the menu
  //     replaced.
  {
    const m = await look({
      scheme: "dark",
      motion: "no-preference",
      frames: [
        {
          kind: "settings",
          settings: {
            modes: {
              currentModeId: "plan",
              availableModes: [
                { id: "default", name: "default" },
                { id: "plan", name: "plan" },
                { id: "someFutureMode", name: "Future" },
              ],
            },
            options: [
              {
                id: "model", name: "Model", type: "select", currentValue: "b",
                options: [{ value: "a", name: "Small" }, { value: "b", name: "Large" }],
              },
              { id: "fast", name: "Fast", type: "boolean", currentValue: true },
            ],
          },
        },
      ],
    });
    if (m.errs.length) fail(`settings: the page threw — ${m.errs[0]}`);
    if (!m.column) {
      fail("settings: probe dead — the conversation face never painted");
    } else {
      if (!m.modeMenu) {
        fail("settings: no mode menu for an agent that listed modes");
      } else if (m.modeMenu.text !== words.chat_mode_plan || m.modeMenu.id !== "plan") {
        fail(`settings: the mode menu wears ${JSON.stringify(m.modeMenu)}, wanted ${words.chat_mode_plan}/plan`);
      } else {
        console.log(`settings · mode menu: ${m.modeMenu.text}`);
      }
      // Each values control must actually open its choice page in the
      // current single configuration menu, not only print a setting name.
      for (const [what, got] of [["the mode menu", m.modeMenu], ["the model select", m.options.find((o) => o.id === "model")]]) {
        if (got && got.menuRole !== "menu") {
          fail(`settings: ${what} did not open its current choice page in a menu (${JSON.stringify(got.menuRole)})`);
        }
      }
      if (m.mode !== null) {
        fail(`settings: the marker drew ${JSON.stringify(m.mode)} beside a menu wearing the same word`);
      }
      const model = m.options.find((o) => o.id === "model");
      const fast = m.options.find((o) => o.id === "fast");
      if (!model || !model.text.includes("Large") || model.value !== "b") {
        fail(`settings: the select wears ${JSON.stringify(model)}, wanted its current value's name (Large)`);
      } else {
        console.log(`settings · select: ${model.text}`);
      }
      if (!fast || fast.on !== "true") {
        fail(`settings: the switch wears ${JSON.stringify(fast)}, wanted aria-checked true`);
      } else {
        console.log(`settings · switch: ${fast.text} on`);
      }
    }
    // The control arm: a frame that lists nothing draws no row at all.
    const none = await look({
      scheme: "dark",
      motion: "no-preference",
      frames: [{ kind: "settings", settings: { modes: null, options: [] } },],
    });
    if (none.modeMenu || none.options.length) {
      fail("settings: an agent with nothing to set must draw no controls");
    } else {
      console.log("settings · nothing listed: nothing drawn");
    }
  }

  // Current settings may omit a selected model. Preserve body/mode/composer
  // and absence-of-invented-model/usage assertions without an old host codec.
  {
    const m = await look({
      scheme: "dark",
      motion: "no-preference",
      frames: [
        { kind: "agent", name: "claude", replays: true, images: true, embeds: true, lists: false, needs_login: false },
        {
          kind: "settings",
          settings: {
            modes: {
              currentModeId: "default",
              availableModes: [
                { id: "default", name: "Default" },
                { id: "acceptEdits", name: "Accept Edits" },
                { id: "plan", name: "Plan Mode" },
              ],
            },
            options: [],
          },
        },
        { kind: "note", update: { sessionId: "s", update: { sessionUpdate: "user_message_chunk", content: { type: "text", text: "旧宿主上的一句话" } } } },
        { kind: "note", update: { sessionId: "s", update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "旧宿主的回答" } } } },
      ],
    });
    if (m.errs.length) fail(`optional model: the page threw — ${m.errs[0]}`);
    if (!m.column || !m.input) {
      fail(`optional model: the conversation did not draw (column ${m.column}, input ${m.input})`);
    } else {
      for (const said of ["旧宿主上的一句话", "旧宿主的回答"]) {
        if (!m.text.includes(said)) fail(`optional model: ${JSON.stringify(said)} is not on the page`);
      }
      if (!m.modeMenu || m.modeMenu.id !== "default") {
        fail(`optional model: the mode seat wears ${JSON.stringify(m.modeMenu)}, wanted the selected execution permission (default)`);
      }
      if (m.usage) fail(`optional model: a context dial was drawn with no reading from the host (${JSON.stringify(m.usage)})`);
      if (!/claude/i.test(m.agentIdentity ?? "")) fail(`optional model: reported agent identity is missing (${JSON.stringify(m.agentIdentity)})`);
      if (m.modelReported !== "宿主未报告模型") fail(`optional model: an unreported model was invented (${JSON.stringify(m.modelReported)})`);
      if (!m.errs.length) console.log("optional model · current payload draws: words, selected default, reported identity, no usage/model");
    }
  }

  // ── batch 3b: the image gate ─────────────────────────────────────
  //
  // **A negative claim, so it needs a positive control first.** "no
  // attach affordance" is satisfied by a page that never rendered, by a
  // renamed selector, and by a typo — so the same probe is run against
  // an agent that *does* take images, and that row has to find the
  // thing the other row must not.
  //
  // The third row is the one the first two cannot reach: **before any
  // handshake has landed**, the capability is unknown, and the code
  // says an unknown capability is not a yes. Written after a red-proof
  // run showed the difference between `!facts?.images` and
  // `facts && !facts.images` changing nothing — because every fixture
  // handed the face an agent frame first, so "unknown" never happened.
  //
  // **The two capabilities are asserted against each other**, not each
  // on its own: a single paste carries one image and one file, and an
  // agent that takes only pictures has to take the picture *and* leave
  // the file. One combined gate would let each capability answer for
  // the other, and the run would look identical.
  for (const [label, agentFrame, wantPaste, wantDoc] of [
    ["images only", { kind: "agent", name: "reg", replays: true, images: true, embeds: false, lists: false, needs_login: false }, true, false],
    ["files only", { kind: "agent", name: "reg", replays: true, images: false, embeds: true, lists: false, needs_login: false }, false, true],
    ["both", { kind: "agent", name: "reg", replays: true, images: true, embeds: true, lists: false, needs_login: false }, true, true],
    ["neither", { kind: "agent", name: "reg", replays: true, images: false, embeds: false, lists: false, needs_login: false }, false, false],
    ["not said yet", null, false, false],
  ]) {
    const m = await look({
      scheme: "dark",
      motion: "no-preference",
      frames: agentFrame ? [agentFrame,] : [],
      paste: true,
      send: true,
    });
    if (m.errs.length) fail(`images/${label}: the page threw — ${m.errs[0]}`);
    if (!m.hasInput) {
      fail(`images/${label}: probe dead — the composer never drew, so nothing below means anything`);
      continue;
    }
    const okPic = m.shots > 0 === wantPaste;
    const okDoc = m.docs > 0 === wantDoc;
    if (!okPic || !okDoc) {
      fail(
        `attach/${label}: one paste of an image and a file attached ` +
          `${m.shots} image(s) and ${m.docs} file(s); wanted ` +
          `${wantPaste ? 1 : 0} and ${wantDoc ? 1 : 0}`,
      );
    } else {
      // …and what was on screen is what went. A chip that never
      // reaches the frame is the same bug as one that never drew,
      // arriving later and harder to see.
      const body = m.sent[0];
      if (!body) {
        fail(`attach/${label}: 发送 was pressed and no chat_say went out`);
      } else {
        const gotPic = (body.images ?? []).length;
        const gotDoc = (body.files ?? []).length;
        if (gotPic !== (wantPaste ? 1 : 0) || gotDoc !== (wantDoc ? 1 : 0)) {
          fail(
            `attach/${label}: the frame carried ${gotPic} image(s) and ${gotDoc} file(s), ` +
              `while the screen showed ${m.shots} and ${m.docs}`,
          );
        } else {
          console.log(
            `attach · ${label}: ${m.shots} image(s), ${m.docs} file(s) — and the frame carried the same`,
          );
        }
      }
    }
  }

  // ── batch 4: the wizard reaches what only the CLI could ──────────
  //
  // Two things live behind `khor agents` and nowhere else: the shipped
  // registry snapshot, and what a registered agent said about its own
  // past conversations. Somebody who never opens a terminal has no way
  // to either — so these claims are about a door existing, and about
  // the door not lying once it is open.
  //
  // 13. **the four answers stay four sentences.** This is the one that
  //     collapses: an empty list makes 「还没问过」, 「它答不了」 and
  //     「它说它没有」 into one shape, and the person's next move
  //     differs in all three (go ask / stop asking / open a new one).
  //     Asserted per case *and* pairwise-distinct — a face with three
  //     branches and one sentence satisfies every per-case check that
  //     only looks for non-empty text.
  // 14. **nothing asks on its own.** Asking starts an agent process
  //     (1.5s measured warm, 5.64s cold); reading what was recorded is
  //     0.017s. So the
  //     count of `agent_ask` after opening the dialog and sitting
  //     through two poll ticks must be **zero**, and one press must
  //     make it exactly one. The positive control is `agent_seen`
  //     having been called at all — otherwise "no asks" is satisfied
  //     by a face that never spoke to the backend.
  // 15. the snapshot's two halves are drawn as two halves: a startable
  //     entry grows 接进来, one that ships a binary grows a sentence
  //     and **no control**. The negative half needs the positive one in
  //     the same run, because "no button" is free for a panel that
  //     never rendered.
  // 17. **the dialog does not burst.** Every claim above reads
  //     attributes and text, and all of them passed on a build whose
  //     conversation rows painted their cwd and their date *outside*
  //     the dialog — the 问一次 button pushed off-screen with them,
  //     and the 目录 box, which this batch never touched, dragged out
  //     with it. `DialogContent` is a grid and a grid item defaults to
  //     `min-width: auto`, so one long cwd widens the whole box and
  //     `truncate` never fires. **A gate that only asks the DOM what it
  //     contains cannot see this**, which is the same lesson as
  //     measuring computed style rather than reading the token: ask the
  //     engine where things landed.
  // 16. attaching goes out **by id**, and opening one of the listed
  //     conversations goes out as a resume carrying that agent and that
  //     id. A row on screen and a frame on the wire are different
  //     facts, and the first is satisfied by a click that sends
  //     nothing (`attach`'s precedent above).
  //
  // The 40/22/18 counts are **not** asserted here: they are the
  // table's, not the face's, and `khor_gui_core::agents`' own test
  // holds them. A second copy of those numbers in a JS fixture would
  // be a number that drifts from the table it claims to describe.
  const KNOWN_FIXTURE = [
    { id: "gemini", name: "Gemini CLI", about: "Google's official CLI for Gemini",
      argv: "npx -y @google/gemini-cli@0.57.0 --acp" },
    { id: "claude-acp", name: "Claude Agent", about: "ACP wrapper for Anthropic's Claude",
      argv: "npx -y @agentclientprotocol/claude-agent-acp@0.70.0" },
    // The other half of the table: real agents khor has no argv for.
    { id: "goose", name: "goose", about: "A local, extensible, open source AI agent", argv: null },
    { id: "cursor", name: "Cursor", about: "Cursor's coding agent", argv: null },
  ];
  const SEEN_ROWS = [
    { id: "sess-a", cwd: "/w/khor", title: "接上注册表", updated_at: "2026-08-30T10:00:00Z" },
    { id: "sess-b", cwd: "/w/other", title: null, updated_at: null },
  ];

  /** Opens the wizard and reads the two new sections off it.
   *
   * `seen` is the `agent_seen` answer under test; `registered` is what
   * `agents` returns. Every call the face makes is counted, because
   * two of these claims are about a call **not** happening. */
  const wizard = async ({
    seen,
    known = KNOWN_FIXTURE,
    registered = [{ name: "gemini", command: "npx gemini" }],
    pick = "gemini",
    settle = 900,
    press = null,
    attachId = null,
    resumeId = null,
    // **Which door** (docs/SHELL.md 第八轮): the registry and attaching
    // stay in 新建; what an agent said it has moved to 导入 whole.
    door = "import",
  }) => {
    const ctx = await browser.newContext({
      viewport: { width: WIDE, height: 900 },
      colorScheme: "dark",
      reducedMotion: "no-preference",
      deviceScaleFactor: 1,
    });
    const page = await ctx.newPage();
    const errs = [];
    page.on("pageerror", (e) => errs.push(String(e)));
    const json = (b) => (r) => r.fulfill({ contentType: "application/json", body: JSON.stringify(b) });
    await page.route("**/lock_state", json({on: false, has_pin: false, timeout_min: 0,
      timeouts: [0, 1, 5, 15, 60], pin_min: 6, system_door: false}));
    await page.route("**/sessions", json([]));
    const fixtureHome = "a".repeat(64);
    await page.route("**/devices", json([{id: fixtureHome, name: "alpha", me: true,
      face: null, pinned: false, vitals: null, hop: null, sponsor: null, watch: [], watched: null,
      seat: {left: 50, top: 50}, hosts: true}]));
    const savedAnswer = seen.said === "unasked" ? seen : {...seen, home: fixtureHome};
    await page.route("**/hooks_state", json({ installed: false }));
    await page.route("**/quota", json({ quota: null, trouble: null }));
    await page.route("**/dir_pins", json([]));
    await page.route("**/web_pins", json([]));
    await page.route("**/recents", json([]));
    await page.route("**/agents", json(registered));
    await page.route("**/agents_known", json(known));
    await page.route("**/agents_known_taken", json(["2026-08-30", 40, 22]));
    const calls = { seen: 0, ask: 0 };
    const sent = { register: [], resume: [] };
    await page.route("**/agent_seen", (r) => {
      calls.seen += 1;
      r.fulfill({ contentType: "application/json", body: JSON.stringify(savedAnswer) });
    });
    await page.route("**/agent_ask", (r) => {
      calls.ask += 1;
      r.fulfill({ contentType: "application/json", body: JSON.stringify(savedAnswer) });
    });
    await page.route("**/register_known_agent", (r) => {
      try {
        sent.register.push(JSON.parse(r.request().postData() ?? "{}"));
      } catch {
        sent.register.push(null);
      }
      r.fulfill({ contentType: "application/json", body: "null" });
    });
    // What the agent said, as the node offers it among the candidates:
    // one row per conversation, spelled `acp:<agent>:<its id>`.
    const offered = seen.said === "sessions"
      ? seen.rows.map((r) => ({
          id: `acp:${registered[0].name}:${r.id}`,
          form: "gui",
          machine: "alpha",
          device: "a".repeat(64),
          title: r.title ?? "",
          place: r.cwd || null,
          at_ms: r.updated_at ? Date.parse(r.updated_at) : 0,
          category: null,
          agent: registered[0].name,
          command: null,
        }))
      : [];
    await page.route("**/import_candidates", json(offered));
    await page.route("**/import_sessions", (r) => {
      try {
        sent.resume.push(JSON.parse(r.request().postData() ?? "{}"));
      } catch {
        sent.resume.push(null);
      }
      r.fulfill({ contentType: "application/json", body: JSON.stringify(["tui/resumed"]) });
    });
    await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });
    await wait(700);
    // The door is the sessions landing's ＋ menu item of that name — its
    // one home since 2026-09-30 (「一个动作只有一个家」): ＋ holds 新会话…
    // and 导入…, R0 holds neither, and 新会话 no longer carries an 导入 of
    // its own. 新会话… makes a new tab whose page is the wizard (UX-45).
    await page.locator("[data-side-head] [data-pane-new]").click();
    await wait(300);
    await page.locator(`[role="menu"] [data-new-item="${door}"]`).click();
    await wait(400);
    if (attachId !== null) {
      await page.locator("[data-new-session-add-agent]").click();
      await wait(400);
    }
    const panel = attachId !== null
      ? await page.evaluate(() => {
          const rows = [...document.querySelectorAll("[data-registry-row]")].map((el) => ({
            id: el.getAttribute("data-registry-row"),
            startable: el.getAttribute("data-registry-startable") === "true",
            attach: !!el.querySelector("[data-registry-attach]"),
            byhand: !!el.querySelector("[data-registry-byhand]"),
            attached: !!el.querySelector("[data-registry-attached]"),
            argv: (el.querySelector("[data-registry-argv]")?.textContent ?? "").trim(),
          }));
          const t = document.querySelector("[data-registry-taken]");
          return { rows, taken: (t?.textContent ?? "").trim() };
        })
      : null;
    if (attachId !== null && attachId !== "") {
      await page.locator(`[data-registry-attach="${attachId}"]`).click();
      await wait(600);
    }
    if (pick && door === "new") {
      // An agent is a tile among the kinds on the new tab's page (UX-45).
      await page.locator(`[data-new-session-tile="agent:${pick}"]`).click();
      await wait(400);
    }
    // **Two poll ticks (POLL_MS is 2000).** The claim is that nothing
    // on a timer asks, and a window shorter than the timer measures
    // the sampling rather than the system.
    await wait(settle);
    const before = { ...calls };
    if (press) {
      await page.locator("[data-agent-ask]").first().click();
      await wait(600);
    }
    if (resumeId) {
      // A tap imports it and opens it (IMPORT-BRIEF).
      await page.locator(`[data-import-row="acp:${registered[0].name}:${resumeId}"]`).click();
      await wait(900);
    }
    const out = await page.evaluate((door) => {
      const box = document.querySelector("[data-agent-seen]");
      return {
        // Null and "" are different: no section at all versus a section
        // that drew nothing.
        said: box ? box.getAttribute("data-said") : null,
        text: box
          ? (box.querySelector("[data-agent-seen-text]")?.textContent ?? "").trim()
          : null,
        rows: [...document.querySelectorAll('[data-import-row^="acp:"]')].map((el) => ({
          id: el.getAttribute("data-import-row"),
          text: (el.textContent ?? "").trim(),
          title: (el.querySelector("[data-import-title]")?.textContent ?? "").trim(),
          where: (el.querySelector("[data-import-where]")?.textContent ?? "").trim(),
          path: el.querySelector("[data-import-where]")?.getAttribute("title") ?? null,
        })),
        // The positive control for every negative claim below: the
        // dialog is open and painting agents at all.
        buttons: door === "new"
          ? document.querySelectorAll('[data-new-session-tile^="agent:"]').length
          : document.querySelectorAll("[data-agent-seen]").length,
        askButton: !!document.querySelector("[data-agent-ask]"),
        // **Where things actually landed**, asked of the engine. `box`
        // is the dialog (the new tab's page for `new`, UX-45); `worst` is
        // how far past its right edge any descendant reaches. Measured
        // against the painted box rather than against `scrollWidth`,
        // because a child positioned outside an `overflow:visible` parent
        // does not grow it.
        geom: (() => {
          const box = document.querySelector(door === "new" ? "[data-new-session-page]" : "[data-import-dialog]");
          if (!box) return null;
          const r = box.getBoundingClientRect();
          let worst = 0;
          let who = "";
          for (const el of box.querySelectorAll("*")) {
            const b = el.getBoundingClientRect();
            if (b.width === 0 && b.height === 0) continue;
            const over = b.right - r.right;
            if (over > worst) {
              worst = over;
              who = el.getAttribute("data-import-row")
                ?? el.getAttribute("data-registry-row")
                ?? el.tagName.toLowerCase();
            }
          }
          return { width: Math.round(r.width), over: Math.round(worst), who };
        })(),
      };
    }, door);
    await ctx.close();
    return { ...out, errs, calls, before, sent, panel };
  };

  // 13) the four answers, four sentences.
  const answers = [
    ["还没问过", { said: "unasked" }, "unasked"],
    ["它答不了", { said: "cannot" }, "cannot"],
    ["它说它没有", { said: "sessions", rows: [] }, "none"],
    ["它列出来了", { said: "sessions", rows: SEEN_ROWS }, "sessions"],
  ];
  const sentences = new Map();
  for (const [label, seen, wantMark] of answers) {
    const w = await wizard({ seen });
    const say = `wizard/${label}`;
    if (w.errs.length) fail(`${say}: the page threw — ${w.errs[0]}`);
    if (w.buttons === 0) {
      fail(`${say}: probe dead — the wizard never painted an agent button, so nothing below means anything`);
      continue;
    }
    if (w.said !== wantMark) {
      fail(`${say}: the section reads data-said=${JSON.stringify(w.said)}, wanted ${wantMark}`);
      continue;
    }
    if (wantMark === "sessions") {
      // The listing says itself; the rows are the answer.
      if (w.rows.length !== SEEN_ROWS.length) {
        fail(`${say}: ${SEEN_ROWS.length} recorded conversations and ${w.rows.length} drew`);
      } else {
        // Its own name where it has one, and where it lives where it
        // does not — the id alone is not something a person picks by.
        // Since 导入 became a search (IMPORT-BRIEF, the owner's lab) a row
        // says its directory by its last part, 「机器 · 目录 · 时间」, and
        // the whole path is the line's tooltip.
        if (!w.rows[0].title.includes("接上注册表")) {
          fail(`${say}: the named conversation reads ${JSON.stringify(w.rows[0].title)}`);
        }
        if (w.rows[1].title !== "other") {
          fail(`${say}: the unnamed conversation must fall back to where it lives, got ${JSON.stringify(w.rows[1].title)}`);
        }
        // Where it lands is on screen, because a click does not go to
        // the 目录 box above.
        if (!w.rows[0].where.includes("khor") || w.rows[0].path !== "/w/khor") {
          fail(`${say}: a row does not say where opening it lands: ${JSON.stringify(w.rows[0])}`);
        }
      }
      console.log(`${say}: ${w.rows.length} conversation(s) drew`);
      continue;
    }
    if (!w.text) {
      fail(`${say}: data-said=${wantMark} and the section said nothing`);
      continue;
    }
    sentences.set(wantMark, w.text);
    console.log(`${say}: ${JSON.stringify(w.text)}`);
  }
  // **The load-bearing half.** Three correct branches that print one
  // sentence pass every per-case check above.
  if (sentences.size !== 3) {
    fail(`wizard: 3 sentences expected, ${sentences.size} were read — the rest of this claim is vacuous`);
  } else {
    const said = [...sentences.values()];
    if (new Set(said).size !== said.length) {
      fail(
        `wizard: 「还没问过」「它答不了」「它说它没有」 printed ${new Set(said).size} distinct ` +
          `sentence(s) — [${said.join(" | ")}]. This is the bug, not a near miss`,
      );
    }
  }

  // 17) the dialog holds its own edges, on the widest thing it can be
  //     asked to paint: a listing of long directories. **A few pixels
  //     of tolerance and no more** — the failure this is written
  //     against was 503.
  //
  //     **The snapshot panel had a case here and it was removed**: it
  //     could not be made to fail. Its `min-w-0` was pulled off the
  //     row, off the scroll box and off the panel itself, one at a
  //     time, and the overhang stayed 0 every time — the panel's rows
  //     are a column of truncating spans, whose min-content width is
  //     zero, so nothing in there can push. An assertion that holds
  //     for free is this repo's own first kind of false green, and a
  //     second geometry row would have read as two surfaces covered
  //     while one of them was never being watched.
  for (const [label, opts] of [
    ["a listing of long paths", {
      seen: { said: "sessions", rows: [
        ...SEEN_ROWS,
        { id: "sess-long", cwd: "/Users/somebody/Desktop/working/khor/apps/gui/src/views/deeply/nested",
          title: "Use the TodoWrite tool right now to record exactly three todos: read the file, change the file, run the tests",
          updated_at: "2026-08-30T10:00:00Z" },
      ] },
    }],
  ]) {
    const w = await wizard(opts);
    const say = `wizard/geometry · ${label}`;
    if (!w.geom) {
      fail(`${say}: probe dead — the dialog was not on screen to measure`);
    } else if (w.geom.over > 4) {
      fail(
        `${say}: something reaches ${w.geom.over}px past the dialog's right edge ` +
          `(${w.geom.who}), on a box ${w.geom.width}px wide — a grid item's ` +
          `min-width:auto widens the whole dialog and truncate never fires`,
      );
    } else {
      console.log(`${say}: box ${w.geom.width}px, worst overhang ${w.geom.over}px`);
    }
  }

  // 14) nothing asks on its own; one press asks once.
  {
    const idle = await wizard({ seen: { said: "unasked" }, settle: 5000 });
    if (idle.errs.length) fail(`wizard/idle: the page threw — ${idle.errs[0]}`);
    if (idle.calls.seen === 0) {
      fail(
        `wizard/idle: probe dead — the face never read what was recorded either, so ` +
          `「0 次去问」 is satisfied by a dialog that talked to nothing`,
      );
    } else if (idle.calls.ask !== 0) {
      fail(
        `wizard/idle: ${idle.calls.ask} agent_ask in 5s with nobody pressing anything — ` +
          `each one starts an agent process (1.5s warm, 5.64s cold)`,
      );
    } else {
      console.log(`wizard/idle: ${idle.calls.seen} read(s), ${idle.calls.ask} ask(s) over 5s`);
    }
    const pressed = await wizard({ seen: { said: "unasked" }, press: true });
    if (!pressed.askButton) {
      fail(`wizard/press: probe dead — there is no 问一次 to press`);
    } else if (pressed.calls.ask - pressed.before.ask !== 1) {
      fail(
        `wizard/press: one press produced ${pressed.calls.ask - pressed.before.ask} ask(s), wanted 1`,
      );
    } else {
      console.log(`wizard/press: one press → ${pressed.calls.ask} ask`);
    }
  }

  // 15 & 16) the snapshot's two halves, and what goes on the wire.
  {
    const w = await wizard({ seen: { said: "unasked" }, attachId: "claude-acp", pick: null, door: "new" });
    const say = "wizard/registry";
    if (w.errs.length) fail(`${say}: the page threw — ${w.errs[0]}`);
    const rows = w.panel?.rows ?? [];
    if (rows.length !== KNOWN_FIXTURE.length) {
      fail(`${say}: probe dead — the snapshot has ${KNOWN_FIXTURE.length} entries and ${rows.length} drew`);
    } else {
      // The positive half first: without it, "no button on the binary
      // ones" is a claim a blank panel satisfies.
      const startable = rows.filter((r) => r.startable);
      const byhand = rows.filter((r) => !r.startable);
      if (startable.filter((r) => r.attach || r.attached).length !== startable.length) {
        fail(`${say}: a startable entry grew neither 接进来 nor 已接入 — ${JSON.stringify(startable)}`);
      }
      // gemini is already registered in the fixture, so it must read
      // as attached rather than offering to attach twice.
      const gem = rows.find((r) => r.id === "gemini");
      if (!gem?.attached || gem?.attach) {
        fail(`${say}: an already-registered entry must read 已接入 and offer no second attach — ${JSON.stringify(gem)}`);
      }
      // **「khor 起得来」without the command is a claim taken on
      // trust.** Asserted as the painted argv, and as its absence on
      // the ones khor has no command for — where printing one would
      // be khor inventing it.
      for (const r of startable) {
        if (!r.argv.startsWith("npx ")) {
          fail(`${say}: ${r.id} claims khor can start it and shows ${JSON.stringify(r.argv)} as the command`);
        }
      }
      for (const r of byhand) {
        if (r.argv) {
          fail(`${say}: ${r.id} ships a binary and khor printed a command for it: ${JSON.stringify(r.argv)}`);
        }
        if (r.attach) {
          fail(`${say}: ${r.id} ships a binary and khor has no command for it, yet it grew 接进来`);
        }
        if (!r.byhand) {
          fail(`${say}: ${r.id} has no command and said nothing — 「自己装」 and 「接不了」 are different sentences`);
        }
      }
      console.log(`${say}: ${startable.length} startable, ${byhand.length} by hand — ${w.panel.taken}`);
    }
    // A table that ages says when it is from.
    if (!(w.panel?.taken ?? "").includes("2026-08-30")) {
      fail(`${say}: the snapshot does not say when it was taken (${JSON.stringify(w.panel?.taken)})`);
    }
    // 16a) …and the attach went out as an id.
    if (w.sent.register.length !== 1 || w.sent.register[0]?.id !== "claude-acp") {
      fail(`${say}: 接进来 put ${JSON.stringify(w.sent.register)} on the wire, wanted one {id:"claude-acp"}`);
    } else {
      console.log(`${say}: 接进来 → register_known_agent {id:"claude-acp"}`);
    }
    // …and attaching asks once, the way `khor agents add` does. The
    // two faces telling a person different things about the same
    // agent is what this row is against.
    if (w.calls.ask !== 1) {
      fail(`${say}: attaching produced ${w.calls.ask} ask(s); the command line asks exactly once at this point`);
    }
  }
  {
    const w = await wizard({
      seen: { said: "sessions", rows: SEEN_ROWS },
      resumeId: "sess-a",
    });
    const say = "wizard/resume";
    if (w.errs.length) fail(`${say}: the page threw — ${w.errs[0]}`);
    const body = w.sent.resume[0];
    if (!body) {
      fail(`${say}: a recorded conversation was picked and imported, and no import_sessions went out`);
    } else if (JSON.stringify(body.ids) !== JSON.stringify(["acp:gemini:sess-a"])) {
      fail(`${say}: the frame carried ${JSON.stringify(body)}, wanted {ids:["acp:gemini:sess-a"]}`);
    } else {
      console.log(`${say}: pick + 导入 → import_sessions ${JSON.stringify(body)}`);
    }
    // **A resume is not the wizard's four answers.** Sending the 目录
    // box would hand the agent another conversation's address, which
    // `session/load` is entitled to refuse.
    if (body && ("dir" in body || "form" in body)) {
      fail(`${say}: the import carried the wizard's answers too: ${JSON.stringify(body)}`);
    }
  }

  for (const scheme of ["dark", "light"]) {
    for (const motion of ["no-preference", "reduce"]) {
      const m = await look({ scheme, motion });
      const say = `${scheme} · motion:${motion}`;

      // 5) before anything else — a thrown subtree measures fine.
      if (m.errs.length) fail(`${say}: the page threw while being measured — ${m.errs[0]}`);

      // 0) probe.
      if (!m.column || m.paragraphs === 0) {
        fail(`${say}: probe dead — no conversation painted (column ${m.column}, ${m.paragraphs} paragraphs)`);
        continue;
      }
      if (m.lines.length !== 3) {
        fail(
          `${say}: probe dead — the fixture has 3 tool calls and ${m.lines.length} line(s) drew. ` +
            `Everything below asks whether two of them differ, and a missing line answers that by accident.`,
        );
        continue;
      }

      // 1) the fold ran: an update settled the call it named.
      const got = m.lines.map((l) => l.status);
      if (WANT.join() !== got.join()) {
        fail(
          `${say}: statuses reached the DOM as [${got.join(", ")}], wanted [${WANT.join(", ")}] — ` +
            `all-in_progress means tool_call_update was dropped`,
        );
      }
      const kinds = m.lines.map((l) => l.kind).join();
      if (kinds !== "edit,execute,search") {
        fail(`${say}: kinds reached the DOM as [${kinds}], wanted [edit,execute,search]`);
      }

      // 10b) **The line says which call it was.** This is the
      //      complaint itself: a conversation of lines all reading
      //      `Bash`. Asserted as the *painted* text and as
      //      distinctness — the second half is the bug, because three
      //      correct-but-identical titles satisfy any per-line check.
      const titles = m.lines.map((l) => l.title);
      const wantTitles = ["Edit src/lib.rs", "跑一遍测试", "Grep kind_of"];
      for (const [i, want] of wantTitles.entries()) {
        if (titles[i] !== want) {
          fail(`${say}: tool line ${i} reads ${JSON.stringify(titles[i])}, wanted ${JSON.stringify(want)}`);
        }
      }
      if (new Set(titles).size !== titles.length) {
        fail(
          `${say}: three different calls painted ${new Set(titles).size} distinct line(s) — ` +
            `[${titles.join(" | ")}]. This is the bug, not a near miss`,
        );
      }

      // 11) **A tool line opens onto what it did — and only when there
      //     is something there.** The complaint this answers was a
      //     conversation of identical un-openable lines. Both halves
      //     are asserted because the negative one ("no control") is
      //     free for a page that never rendered, and the positive one
      //     is what proves the probe alive.
      const [edit, shell, bare] = m.lines;
      if (edit.openable !== "no") {
        fail(`${say}: the edit carries a diff and its line is data-tool-open=${edit.openable}`);
      }
      if (shell.openable !== "no") {
        fail(`${say}: the failed command said why and its line is data-tool-open=${shell.openable}`);
      }
      if (bare.openable !== null) {
        fail(
          `${say}: a call with nothing behind it grew a control (data-tool-open=${bare.openable}) — ` +
            `a control that opens onto nothing is worse than none`,
        );
      }
      if (m.lines.some((l) => l.body)) {
        fail(`${say}: a tool line was already open; closed is the default and the reason is the noise`);
      }

      // …and opened, which is a different claim from openable.
      const o = await look({ scheme, motion, openTools: true });
      if (o.errs.length) {
        fail(`${say}: the page threw while a tool line was opened — ${o.errs[0]}`);
      } else if (o.lines.length !== 3) {
        fail(`${say}: probe dead — ${o.lines.length} tool line(s) after opening`);
      } else {
        const [oEdit, oShell, oBare] = o.lines;
        if (!oEdit.diff) {
          fail(`${say}: the edit was opened and no diff drew`);
        }
        if (!oShell.out.includes("E0425")) {
          fail(`${say}: the failed command was opened and its message is ${JSON.stringify(oShell.out)}`);
        }
        if (oBare.body) {
          fail(`${say}: the line with nothing behind it opened onto something`);
        }
        console.log(`${say}: opened — diff ${oEdit.diff}, output ${oShell.out.length} chars`);
      }

      // 2) three inks a person can tell apart.
      const L = m.lines.map((l) => ({ status: l.status, ink: l.ink, l: lstar(l.ink) }));
      if (L.some((x) => x.l === null)) {
        fail(`${say}: probe dead — an ink did not parse as rgb: ${L.map((x) => x.ink).join(" / ")}`);
        continue;
      }
      console.log(
        `${say}: ` + L.map((x) => `${x.status} L*${x.l.toFixed(1)}`).join("  ") +
          (motion === "reduce" ? "" : `  (busy wears data-word=${m.lines[2].word})`),
      );
      for (const [a, b] of [[0, 1], [0, 2], [1, 2]]) {
        const d = Math.abs(L[a].l - L[b].l);
        if (d < DELTA_L) {
          fail(
            `${say}: ${L[a].status} and ${L[b].status} are ΔL* ${d.toFixed(2)} apart, under ${DELTA_L} — ` +
              `two states drawn the same way is the state not being drawn`,
          );
        }
      }

      // 2b) the running mark is legible **as a still image**.
      //
      // The obvious design is one filled dot in three hues, and it has
      // a hole this repo has already paid for: 忙碌's indigo sits at
      // grey's exact loudness on purpose (ΔL* 1.03), so with the breath
      // off, running and completed would be the same dot. And the
      // breath is off for anyone who asked for less motion — the reader
      // least able to spare a distinction.
      //
      // **So this claim is made with motion out of the picture, in both
      // passes.** A gate that only checked the animated pass would be
      // testing the half that is not at risk.
      const dots = m.lines.map((l) => l.dot);
      if (dots.some((d) => d === null)) {
        fail(`${say}: a tool line has no status mark — ${dots.filter((d) => !d).length} of ${dots.length}`);
      } else {
        // The live probe that earns the comparison below: the marks are
        // really on screen and really have a box, so "they differ" is
        // about two drawn things rather than two absences.
        for (const d of dots) {
          if (d.size === "0px x 0px" || d.size === "auto x auto") {
            fail(`${say}: a status mark measured ${d.size} — nothing is being drawn`);
          }
        }
        // **Two marks are told apart by weight or by shape**, and the
        // check must say which — a plain "these two strings differ"
        // passes for three hues at one lightness, which is precisely
        // the design this shape exists to avoid. Measured: with a
        // filled dot in 忙碌's indigo against a filled dot in grey, the
        // string check was green and the two dots were ΔL* 1.0 apart.
        const dl = dots.map((d) => lstar(d.ink));
        if (dl.some((x) => x === null)) {
          fail(`${say}: probe dead — a mark's colour did not parse: ${dots.map((d) => d.ink).join(" / ")}`);
        } else {
          for (const [a, b] of [[0, 1], [0, 2], [1, 2]]) {
            const shape = dots[a].filled !== dots[b].filled;
            const weight = Math.abs(dl[a] - dl[b]);
            if (!shape && weight < DELTA_L) {
              fail(
                `${say}: ${dots[a].state} and ${dots[b].state} are both ` +
                  `${dots[a].filled ? "filled" : "hollow"} and ΔL* ${weight.toFixed(2)} apart ` +
                  `(under ${DELTA_L}) — with the breath off they are one state`,
              );
            }
          }
          console.log(
            `${say}: marks ` +
              dots
                .map((d, i) => `${d.state}${d.filled ? "●" : "○"}L*${dl[i].toFixed(0)}${d.animated ? "~" : ""}`)
                .join(" "),
          );
        }
        // And the motion, where it is allowed: the running mark is the
        // one that moves, and it is the *only* one — a settled mark
        // that breathed would say a finished call is still going.
        for (const d of dots) {
          const shouldMove = motion !== "reduce" && d.state === "running";
          if (d.animated !== shouldMove) {
            fail(
              `${say}: the ${d.state} mark ${d.animated ? "breathes" : "is still"} under motion:${motion}`,
            );
          }
        }
      }

      // 3) failure wears the word's own ink.
      const failedLine = m.lines.find((l) => l.status === "failed");
      if (failedLine) {
        const want = lstar(m.failedToken);
        const gotL = lstar(failedLine.ink);
        if (want === null) {
          fail(`${say}: probe dead — --state-failed did not resolve to rgb (${m.failedToken})`);
        } else if (Math.abs(want - gotL) > 0.5) {
          fail(
            `${say}: the failed tool is drawn L*${gotL.toFixed(1)} but 失败 is L*${want.toFixed(1)} — ` +
              `a seventh colour was invented for a word that already exists`,
          );
        }
      }

      // 6) the context reading reached the screen, as a share.
      if (!m.usage) {
        fail(`${say}: usage_update carried a reading and nothing drew it`);
      } else {
        // The current header exposes the actual visible percentage, not the
        // retired composer's hidden raw-count attributes. Keep the original
        // 320000/1000000 SDK input and independently require its visible 32%.
        if (m.usage.share !== "32%") {
          fail(`${say}: 320000 of 1000000 printed as "${m.usage.share}", wanted exactly 32%`);
        }
        // ⚠️ This asserted the cost was printed (0.39) until the redesign;
        // the owner decided the composer writes no dollars (2026-10-02),
        // so it now asserts the cost is nowhere on the page.
        if (m.usage.cost) {
          fail(`${say}: the cost reached the page as "${m.usage.cost}" — the composer writes no dollars`);
        }
      }

      // 9) the plan reaches the screen, in order, with its statuses —
      //    and a finished step reads differently from the one being
      //    worked. Order is asserted on its own: a plan is a sequence,
      //    and a face that painted it as a set would drop the one thing
      //    that makes it readable.
      const wantPlan = [
        ["completed", "读那个文件"],
        ["in_progress", "改那个文件"],
        ["pending", "跑测试"],
      ];
      if (m.plan.length !== 3) {
        fail(`${say}: the plan carried 3 entries and ${m.plan.length} drew`);
      } else {
        for (const [i, [st, text]] of wantPlan.entries()) {
          if (m.plan[i].status !== st || m.plan[i].text !== text) {
            fail(
              `${say}: plan row ${i} is ${JSON.stringify([m.plan[i].status, m.plan[i].text])}, ` +
                `wanted ${JSON.stringify([st, text])}`,
            );
          }
        }
        const pl = m.plan.map((r) => lstar(r.ink));
        if (pl.some((x) => x === null)) {
          fail(`${say}: probe dead — a plan ink did not parse as rgb`);
        } else {
          console.log(
            `${say}: plan ` + m.plan.map((r, i) => `${r.status} L*${pl[i].toFixed(1)}`).join("  "),
          );
          // **Both pairs against the present, not just one.** The
          // first version of this asserted only done-vs-doing, and the
          // run printed `in_progress L*90.8  pending L*90.8` — a plan
          // whose not-yet-started steps were drawn exactly like the one
          // being worked, which is the single thing a plan is read for.
          // The assertion was true and too narrow to see it.
          for (const i of [0, 2]) {
            const d = Math.abs(pl[i] - pl[1]);
            if (d < DELTA_L) {
              fail(
                `${say}: the step being worked and a ${m.plan[i].status} one are ` +
                  `ΔL* ${d.toFixed(2)} apart, under ${DELTA_L} — a plan that does not ` +
                  `say which line is now is not a plan`,
              );
            }
          }
          // And the two that are *not* the present stay together: three
          // weights for three statuses would spend contrast on a
          // distinction the order already carries.
          if (Math.abs(pl[0] - pl[2]) > 0.5) {
            fail(
              `${say}: done and not-yet are ΔL* ${Math.abs(pl[0] - pl[2]).toFixed(2)} apart; ` +
                `neither is the present and they should share a weight`,
            );
          }
        }
      }

      // 7) the agent's own title, which claude never sends and other
      //    agents do.
      if (m.title !== "把工具调用的状态接上") {
        fail(`${say}: session_info_update carried a title and the face drew ${JSON.stringify(m.title)}`);
      }

      // 4) with motion off, the running line is still marked as running.
      if (motion === "reduce") {
        const run = L.find((x) => x.status === "in_progress");
        const done = L.find((x) => x.status === "completed");
        if (run && done && Math.abs(run.l - done.l) < DELTA_L) {
          fail(
            `${say}: with motion off, running and settled are ΔL* ${Math.abs(run.l - done.l).toFixed(2)} apart — ` +
              `the breath was carrying the fact, which app.css says it never may`,
          );
        }
      }
    }
  }
} finally {
  if (browser) await browser.close();
  if (vite) vite.kill("SIGTERM");
}

if (!process.exitCode) {
  console.log("acp gate: the protocol's states reach the screen as states");
}
