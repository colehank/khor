// A home that is not the operator's, for every process a gate starts.
//
// **测试与 smoke 绝不碰真身份** (docs/handoff/001.md). Until 2026-09-22 the
// gates gave their children a scratch `KHOR_HOME` and the operator's own
// `HOME` — and Claude's login lives under `HOME` (`~/.claude`) and in the
// login keychain, so the dev bridge a smoke run started read the real
// login and spent it against Anthropic's usage endpoint (s25-investigator's
// reading: quota 1835 ms, quota_board 5477 ms).
//
// So every child gets:
//   · a `HOME` of its own under the run's scratch directory, holding a
//     **fake** Claude login (`.claude/.credentials.json`, a token nobody
//     issued) and the two dotfiles a login shell and git need to behave
//     like the operator's without reading theirs: `.zprofile` carries the
//     PATH this script was started with, `.gitconfig` an author;
//   · `KHOR_CLAUDE_USAGE_URL` pointing at a fake usage endpoint this
//     script runs, which answers a reading and **writes down the token
//     every request carried** — so the gate can say not only that the
//     subscription was read, but that it was read with the fake login and
//     with nothing else (`usage.strangers`).
//
// The keychain is the one store `HOME` does not move; the node now asks it
// only when `HOME` is the account's own home (`khor_node::quota`), which a
// scratch one never is.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** The token the fake login carries. Recognisably not a real one. */
export const FAKE_TOKEN = "khor-gate-fake-oauth-token";

/** Lays a home out under `dir` and answers the variables that point a
    child at it. `usageUrl` is the fake endpoint's address. */
export function fakeHome(dir, usageUrl) {
  mkdirSync(join(dir, ".claude"), { recursive: true });
  writeFileSync(
    join(dir, ".claude", ".credentials.json"),
    JSON.stringify({
      claudeAiOauth: {
        accessToken: FAKE_TOKEN,
        // A day ahead, in milliseconds — the shape Claude Code writes.
        expiresAt: Date.now() + 86_400_000,
        rateLimitTier: "default_claude_max_5x",
        subscriptionType: "max",
      },
    }),
  );
  // What a login shell would have found in the operator's own
  // `.zprofile`: the PATH this script was started with. Written out
  // rather than read, so their file is never opened.
  writeFileSync(join(dir, ".zprofile"), `export PATH=${JSON.stringify(process.env.PATH ?? "")}\n`);
  writeFileSync(join(dir, ".gitconfig"), "[user]\n\tname = khor gate\n\temail = gate@khor.invalid\n");
  // The fake endpoint is on this machine's loopback, and this machine
  // sets `http_proxy` / `all_proxy`: loopback is let past them, so the
  // request cannot wander off to a proxy that has no way to reach it.
  const loopback = [process.env.NO_PROXY, "127.0.0.1", "localhost"].filter(Boolean).join(",");
  return { HOME: dir, KHOR_CLAUDE_USAGE_URL: usageUrl, NO_PROXY: loopback, no_proxy: loopback };
}

/**
 * The fake usage endpoint. `seen.asked` counts every request;
 * `seen.strangers` the ones whose bearer was not `FAKE_TOKEN` — **their
 * tokens are never kept or printed**, only counted: the one thing a gate
 * must not do with a real credential that reached it is write it into a
 * log.
 *
 * **A process of its own**, not a server inside the gate's: the gates
 * call `khor` through `execFileSync`, which blocks this event loop for
 * the whole call — a server living here could not answer a request made
 * *during* that call, and the reading would come back 问不出来 for a
 * reason that has nothing to do with the code (measured, this file's
 * first version). The counts come back through a file.
 */
export async function fakeUsage() {
  const dir = mkdtempSync(join(tmpdir(), "khor-fake-usage-"));
  const counts = join(dir, "counts.json");
  const portFile = join(dir, "port");
  const child = spawn(
    process.execPath,
    [
      "-e",
      `
      const { createServer } = require("node:http");
      const { writeFileSync } = require("node:fs");
      const [counts, portFile, token] = process.argv.slice(1);
      const seen = { asked: 0, strangers: 0 };
      writeFileSync(counts, JSON.stringify(seen));
      createServer((req, res) => {
        seen.asked += 1;
        if ((req.headers.authorization ?? "") !== "Bearer " + token) seen.strangers += 1;
        writeFileSync(counts, JSON.stringify(seen));
        res.writeHead(200, { "content-type": "application/json" });
        res.end(JSON.stringify({ five_hour: { utilization: 37, resets_at: null }, seven_day: { utilization: 58, resets_at: null } }));
      }).listen(0, "127.0.0.1", function () {
        writeFileSync(portFile, String(this.address().port));
      });
      `,
      counts,
      portFile,
      FAKE_TOKEN,
    ],
    { stdio: "ignore", detached: false },
  );
  const end = Date.now() + 10_000;
  while (!existsSync(portFile)) {
    if (Date.now() > end) throw new Error("the fake usage endpoint never came up");
    await new Promise((r) => setTimeout(r, 50));
  }
  const port = readFileSync(portFile, "utf8").trim();
  return {
    url: `http://127.0.0.1:${port}/api/oauth/usage`,
    get seen() {
      try {
        return JSON.parse(readFileSync(counts, "utf8"));
      } catch {
        return { asked: 0, strangers: 0 };
      }
    },
    close: async () => {
      child.kill("SIGTERM");
      rmSync(dir, { recursive: true, force: true });
    },
  };
}
