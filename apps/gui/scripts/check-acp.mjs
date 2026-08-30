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
//   8. the handshake's facts reach the face, and only while there is
//      something for them to explain. `ChatFrame::Agent` had carried
//      `replays` since the day it was added, with `gui_host`'s own note
//      saying a face must act on it — **and no face read the frame at
//      all**. Both agents khor wrote replay, so nothing ever showed.
//   9. a `plan` reaches the screen in order, and a finished step reads
//      differently from the one being worked. **claude sends no plan
//      either** — measured 2026-08-30, its tool list on this machine
//      has 128 entries and nothing todo-shaped; an earlier note in this
//      repo said the plan would come from a `TodoWrite` tool, and there
//      is no such tool.
//  10. an image pastes into the composer **only when the agent said it
//      takes images**. Both directions, because "no attach affordance"
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
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

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
    kind: "history",
    update: {
      update: {
        sessionUpdate: "user_message_chunk",
        content: { type: "text", text: "跑一下测试，然后读一下那个文件" },
      },
    },
  },
  // An edit, which carries its own diff — the payload shape khor's
  // 垫片 builds out of claude's `old_string`/`new_string`.
  { kind: "history", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_ok", title: "Edit src/lib.rs", kind: "edit", status: "in_progress",
    rawInput: { file_path: "/w/khor/src/lib.rs", old_string: "let x = 1;", new_string: "let x = 2;" },
    locations: [{ path: "/w/khor/src/lib.rs" }],
    content: [{ type: "diff", path: "/w/khor/src/lib.rs", oldText: "let x = 1;", newText: "let x = 2;" }] } } },
  { kind: "history", update: { update: { sessionUpdate: "tool_call_update", toolCallId: "toolu_ok", status: "completed" } } },
  // A failed shell command that said why.
  { kind: "history", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_bad", title: "跑一遍测试", kind: "execute", status: "in_progress",
    rawInput: { command: "cargo test -p khor-node", description: "跑一遍测试" } } } },
  { kind: "history", update: { update: { sessionUpdate: "tool_call_update", toolCallId: "toolu_bad", status: "failed",
    content: [{ type: "content", content: { type: "text", text: "error[E0425]: cannot find value `x`" } }] } } },
  // A call still running, with nothing behind it — the one line that
  // must **not** grow a control, because a control onto nothing is
  // worse than no control.
  { kind: "history", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_run", title: "Grep kind_of", kind: "search", status: "in_progress" } } },
  // Batch 2. `used`/`size` are the shape claude actually puts on the
  // wire (captured 2026-08-30: 32126 tokens against a 1 000 000 window),
  // so the share the face prints is a real one and not a round number
  // chosen to make the arithmetic obvious.
  { kind: "history", update: { update: { sessionUpdate: "usage_update", used: 320000, size: 1000000, cost: { amount: 0.38777300000000003, currency: "USD" } } } },
  // A title, which claude never sends — see `ChatView`'s note. In the
  // fixture because the *face's* ability to read it is what the next
  // batch depends on, and an ability nothing exercises is one nobody
  // finds out is broken.
  { kind: "history", update: { update: { sessionUpdate: "session_info_update", title: "把工具调用的状态接上" } } },
  // A plan, which claude also never sends — its 128-tool list here has
  // nothing todo-shaped. Three entries so all three statuses are
  // present at once, which is what claim 9 measures them against.
  { kind: "history", update: { update: { sessionUpdate: "plan", entries: [
    { content: "读那个文件", status: "completed", priority: "medium" },
    { content: "改那个文件", status: "in_progress", priority: "high" },
    { content: "跑测试", status: "pending", priority: "low" },
  ] } } },
  {
    kind: "history",
    update: {
      update: {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "一条跑完了，一条失败了，还有一条在跑。" },
      },
    },
  },
  { kind: "history_end" },
];

const row = (id, kind, title) => ({
  id, kind, title, word: "idle", at_ms: Date.now(), unread: 0, source: null,
  pinned: false, stale: false, category: "claude", group: "", face: null,
  hosted: true, attachable: false, last: null, via: null, via_face: null, home: "gate",
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
  const u = document.querySelector("[data-chat-usage]");
  const usage = u && {
    used: Number(u.getAttribute("data-usage-used")),
    size: Number(u.getAttribute("data-usage-size")),
    share: (u.querySelector("[data-usage-share]")?.textContent ?? "").trim(),
    cost: (u.querySelector("[data-usage-cost]")?.textContent ?? "").trim(),
  };
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
  const modeEl = document.querySelector("[data-chat-mode]");
  const shots = document.querySelectorAll("[data-chat-shot]").length;
  const hasInput = !!document.querySelector("[data-chat-input]");
  return {
    lines, failedToken, paragraphs, usage,
    facts: factsLine ? (factsLine.textContent ?? "").trim() : null,
    plan: planRows, shots, hasInput,
    mode: modeEl ? (modeEl.textContent ?? "").trim() : null,
    title: title ? (title.textContent ?? "").trim() : null,
    column: !!document.querySelector("[data-chat-column]"),
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

  const look = async ({ scheme, motion, frames: override, paste = false, openTools = false }) => {
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
    await page.route("**/sessions", json(ROWS));
    // The row's own machine. `home: "gate"` on the row points here, and
    // without it the list paints nothing — which arrives as a click
    // timeout rather than as an empty list, so it reads like the page
    // hung.
    await page.route("**/devices", json([{
      id: "e".repeat(64), name: "gate", me: true, face: null, pinned: false,
      vitals: null, hop: null, sponsor: null, seat: { left: 50, top: 50 },
    }]));
    await page.route("**/hooks_state", json({ installed: false }));
    await page.route("**/quota", json({ quota: null, trouble: null }));
    await page.route("**/dir_pins", json([]));
    await page.route("**/web_pins", json([]));
    await page.route("**/recents", json([]));
    await page.route("**/chat_open", json(true));
    let served = false;
    await page.route("**/chat_poll", (r) => {
      const body = served ? { frames: [], next: 4, gone: false, lost: false }
                          : { frames: feed, next: 4, gone: false, lost: false };
      served = true;
      r.fulfill({ contentType: "application/json", body: JSON.stringify(body) });
    });
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
        const dt = new DataTransfer();
        dt.items.add(file);
        const box = document.querySelector("[data-chat-input]");
        box?.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true }));
      });
      await wait(600);
    }
    const out = await page.evaluate(probe);
    await ctx.close();
    return { ...out, errs, scheme, motion };
  };

  const WANT = ["completed", "failed", "in_progress"];

  // ── batch 3: the handshake's facts reach the face ────────────────
  //
  // A separate fixture rather than more frames on the one above,
  // because the claim is about an **empty** conversation: the line only
  // exists while there is nothing to explain it away, and adding it to
  // a painted conversation would assert the opposite of the rule.
  for (const [label, agentFrame, want] of [
    ["cannot replay", { kind: "agent", name: "reg", replays: false, images: false, embeds: false, lists: false, needs_login: false }, "不回放"],
    ["needs a login", { kind: "agent", name: "reg", replays: true, images: false, embeds: false, lists: false, needs_login: true }, "要先登录"],
    ["ordinary agent", { kind: "agent", name: "reg", replays: true, images: false, embeds: false, lists: false, needs_login: false }, null],
  ]) {
    const m = await look({ scheme: "dark", motion: "no-preference", frames: [agentFrame, { kind: "history_end" }] });
    if (m.errs.length) fail(`facts/${label}: the page threw — ${m.errs[0]}`);
    // The positive control: this fixture paints no conversation, so the
    // *column* is what proves the face got that far. Without it a
    // never-rendered chat would satisfy the "ordinary agent" row by
    // being absent, which is the negative-assertion trap.
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
  // **The load-bearing row is `default → nothing`.** A marker that is
  // always there is not a marker, and a negative claim like that is
  // satisfied for free by a page that never painted — so the rows
  // around it have to find the very element it must not.
  for (const [id, want] of [
    ["plan", "只看不改"],
    ["acceptEdits", "改文件不问"],
    ["bypassPermissions", "全部不问"],
    // A mode this face has never heard of is still not the default.
    ["someFutureMode", "someFutureMode"],
    ["default", null],
  ]) {
    const m = await look({
      scheme: "dark",
      motion: "no-preference",
      frames: [
        { kind: "history", update: { update: { sessionUpdate: "current_mode_update", currentModeId: id } } },
        { kind: "history_end" },
      ],
    });
    if (m.errs.length) fail(`mode/${id}: the page threw — ${m.errs[0]}`);
    if (!m.column) {
      fail(`mode/${id}: probe dead — the conversation face never painted`);
      continue;
    }
    if (want === null) {
      if (m.mode !== null) {
        fail(`mode/${id}: the ordinary mode drew ${JSON.stringify(m.mode)} — a marker that is always there is not one`);
      } else {
        console.log(`mode · ${id}: silent, as it must be`);
      }
    } else if (m.mode !== want) {
      fail(`mode/${id}: drew ${JSON.stringify(m.mode)}, wanted ${JSON.stringify(want)}`);
    } else {
      console.log(`mode · ${id}: ${m.mode}`);
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
  for (const [label, agentFrame, wantPaste] of [
    ["takes images", { kind: "agent", name: "reg", replays: true, images: true, embeds: false, lists: false, needs_login: false }, true],
    ["takes none", { kind: "agent", name: "reg", replays: true, images: false, embeds: false, lists: false, needs_login: false }, false],
    ["not said yet", null, false],
  ]) {
    const m = await look({
      scheme: "dark",
      motion: "no-preference",
      frames: agentFrame ? [agentFrame, { kind: "history_end" }] : [{ kind: "history_end" }],
      paste: true,
    });
    if (m.errs.length) fail(`images/${label}: the page threw — ${m.errs[0]}`);
    if (!m.hasInput) {
      fail(`images/${label}: probe dead — the composer never drew, so nothing below means anything`);
      continue;
    }
    const attached = m.shots > 0;
    if (attached !== wantPaste) {
      fail(
        `images/${label}: pasting an image attached ${m.shots} — this agent must ` +
          `${wantPaste ? "take" : "refuse"} it`,
      );
    } else {
      console.log(`images · ${label}: pasted → ${m.shots} attached`);
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
        if (m.usage.used !== 320_000 || m.usage.size !== 1_000_000) {
          fail(`${say}: the reading arrived as ${m.usage.used}/${m.usage.size}, not 320000/1000000`);
        }
        // 32% — asserted as the *printed* string, because the bug this
        // is about is arithmetic reaching a person, not a number
        // reaching a variable. A face that divided the wrong way would
        // hold the right two numbers in the DOM and print 313%.
        if (!m.usage.share.includes("32")) {
          fail(`${say}: 320000 of 1000000 printed as "${m.usage.share}" — 32 is nowhere in it`);
        }
        if (!m.usage.cost.includes("0.39")) {
          fail(`${say}: the cost printed as "${m.usage.cost}", wanted 0.39 in it`);
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
