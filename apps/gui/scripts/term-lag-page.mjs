// The page half of the typing-lag probe (audit/TERM-LAG-BRIEF.md step 1):
// one function, handed to whatever can evaluate script in the page — a
// playwright `page.evaluate` on the web face, the scratch app's eval hub
// in the Mac app. It is a hand tool, never a gate.
//
// **What it measures** is key → echo, the person's own reading of lag:
// the time from handing xterm a character (`term.input`, the same road a
// committed keystroke takes, so `onData` → `send` → `termKey` all run)
// to the moment that character comes back in a `term.write` — the host's
// echo, painted. Into `cat` or a shell prompt; the session must echo.
//
// Each key is a letter from a 26-letter wheel, so a key is matched by
// its letter; a stall longer than 26 keys' worth would make two keys
// share a letter, and the probe then counts the later one as lost rather
// than guessing.
//
// Predictive echo (`lib/term-predict`) writes a typed letter at once and
// would read as a 0 ms echo: run with it off (it is off on a session
// with no wire, which is the case the owner reported).
//
// Alongside: every `longtask` the page reports (the WebContent main
// thread busy > 50 ms), with its start and length, so a stall can be put
// next to what the page was doing.
export async function lagProbe({ keys = 200, every = 50, settle = 2000 } = {}) {
  const box = document.querySelector("[data-terminal]");
  const term = box?.__xterm;
  if (!term) return { error: "no terminal on the page" };
  const decoder = new TextDecoder();
  const sent = new Map();
  const echoes = [];
  const write = term.write;
  term.write = function (data, done) {
    const at = performance.now();
    const text = typeof data === "string" ? data : decoder.decode(data);
    for (const ch of text) {
      const t0 = sent.get(ch);
      if (t0 === undefined) continue;
      sent.delete(ch);
      echoes.push({ ch, sent: t0, back: at, ms: at - t0 });
    }
    return write.call(this, data, done);
  };
  const long = [];
  let observer = null;
  try {
    observer = new PerformanceObserver((list) => {
      for (const e of list.getEntries()) long.push({ start: e.startTime, ms: e.duration });
    });
    observer.observe({ type: "longtask", buffered: false });
  } catch {
    // WebKit has no `longtask` entry type; the gaps between frames below
    // stand in for it.
  }
  // The page's own frame clock: a gap between two animation frames over
  // 50 ms is the main thread busy (WebKit's stand-in for `longtask`).
  const gaps = [];
  let last = performance.now();
  let ticking = true;
  const frame = (t) => {
    if (t - last > 50) gaps.push({ start: last, ms: t - last });
    last = t;
    if (ticking) requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
  const lost = [];
  const begin = performance.now();
  const beganAt = Date.now();
  for (let i = 0; i < keys; i += 1) {
    const ch = String.fromCharCode(97 + (i % 26));
    if (sent.has(ch)) lost.push({ ch, sent: sent.get(ch) });
    sent.set(ch, performance.now());
    term.input(ch, true);
    await new Promise((r) => setTimeout(r, every));
  }
  await new Promise((r) => setTimeout(r, settle));
  for (const [ch, t0] of sent) lost.push({ ch, sent: t0 });
  ticking = false;
  observer?.disconnect();
  term.write = write;
  // A newline so the next run starts on a clean line of `cat`.
  term.input("\r", true);
  const ms = echoes.map((e) => e.ms).sort((a, b) => a - b);
  const q = (p) => (ms.length ? ms[Math.min(ms.length - 1, Math.floor(p * ms.length))] : null);
  const rel = (t) => Math.round(t - begin);
  return {
    keys,
    every,
    beganAt,
    echoed: ms.length,
    lost: lost.length,
    p50: q(0.5),
    p95: q(0.95),
    max: ms.length ? ms[ms.length - 1] : null,
    stalls: echoes.filter((e) => e.ms > 150).map((e) => ({ at: rel(e.sent), ms: Math.round(e.ms) })),
    longtasks: long.map((e) => ({ at: rel(e.start), ms: Math.round(e.ms) })),
    frameGaps: gaps.filter((g) => g.start >= begin).map((g) => ({ at: rel(g.start), ms: Math.round(g.ms) })),
  };
}
