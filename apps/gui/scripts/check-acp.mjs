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
  { kind: "history", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_ok", title: "Read src/views/ChatView.tsx", kind: "read", status: "in_progress" } } },
  { kind: "history", update: { update: { sessionUpdate: "tool_call_update", toolCallId: "toolu_ok", status: "completed" } } },
  { kind: "history", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_bad", title: "Bash cargo test -p khor-node", kind: "execute", status: "in_progress" } } },
  { kind: "history", update: { update: { sessionUpdate: "tool_call_update", toolCallId: "toolu_bad", status: "failed" } } },
  { kind: "history", update: { update: { sessionUpdate: "tool_call", toolCallId: "toolu_run", title: "Grep kind_of", kind: "search", status: "in_progress" } } },
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
  return { lines, failedToken, paragraphs, column: !!document.querySelector("[data-chat-column]") };
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

  const look = async ({ scheme, motion }) => {
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
                          : { frames, next: 4, gone: false, lost: false };
      served = true;
      r.fulfill({ contentType: "application/json", body: JSON.stringify(body) });
    });
    await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });
    await wait(900);
    await page.locator("[data-list] [data-row-open]").nth(0).click();
    await wait(1200);
    const out = await page.evaluate(probe);
    await ctx.close();
    return { ...out, errs, scheme, motion };
  };

  const WANT = ["completed", "failed", "in_progress"];

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
      if (kinds !== "read,execute,search") {
        fail(`${say}: kinds reached the DOM as [${kinds}], wanted [read,execute,search]`);
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
