/**
 * Ask the binary what it answers to.
 *
 * The agent's gate runs outside the khor process (this package is that
 * outside), so khor prints its verb table as JSON on the hidden
 * `_verbs` verb — that exporter exists precisely for this call.
 * Loading the table at session start and never caching it across
 * sessions is what makes the tool surface follow the implementation: a
 * verb added in Rust is correctly gated on the agent's next start with
 * zero edits here.  (The keeper swaps binaries between sessions; a
 * cached table would describe the previous generation.)
 */
import { execFile } from "node:child_process";
import type { Reach, VerbTable } from "./gate.ts";

export interface KhorRun {
  stdout: string;
  stderr: string;
  code: number;
}

/**
 * Runs one khor command as an argv array.  There is deliberately no
 * shell anywhere on this path: nothing to quote, nothing to inject.
 */
export type RunKhor = (args: readonly string[], signal?: AbortSignal) => Promise<KhorRun>;

// Long enough for a slow verb over a slow link; a hung one must still
// come back — an agent has no Ctrl-C.
const TIMEOUT_MS = 120_000;

export function khorRunner(bin: string): RunKhor {
  return (args, signal) =>
    new Promise((resolve, reject) => {
      execFile(
        bin,
        args as string[],
        { signal, timeout: TIMEOUT_MS, maxBuffer: 8 * 1024 * 1024, encoding: "utf8" },
        (error, stdout, stderr) => {
          if (!error) {
            resolve({ stdout, stderr, code: 0 });
            return;
          }
          const failure = error as NodeJS.ErrnoException & { killed?: boolean };
          // A numeric code is the process answering "no"; a string
          // code (ENOENT …) is the process never having existed.
          if (typeof failure.code === "number") {
            resolve({ stdout, stderr, code: failure.code });
            return;
          }
          if (failure.code === "ENOENT") {
            reject(new Error(`khor binary not found at \`${bin}\``));
            return;
          }
          if (failure.killed) {
            reject(new Error(`\`khor ${args[0] ?? ""}\` ran past ${TIMEOUT_MS / 1000}s and was stopped`));
            return;
          }
          reject(failure);
        },
      );
    });
}

const REACHES: ReadonlySet<string> = new Set(["read", "act", "inner"]);

/**
 * A malformed table fails the whole load, loudly, at session start.
 * Repairing it row by row would silently reclassify the bad row's verb
 * as unknown — safe in direction, but a hole nobody is told about; a
 * tier that cannot be looked up must not be manufactured by the loader
 * either.
 */
export async function loadVerbTable(run: RunKhor): Promise<VerbTable> {
  const r = await run(["_verbs"]);
  if (r.code !== 0) {
    throw new Error(`\`khor _verbs\` exited ${r.code}: ${r.stderr.trim()}`);
  }
  let rows: unknown;
  try {
    rows = JSON.parse(r.stdout);
  } catch {
    throw new Error("`khor _verbs` printed something that is not JSON");
  }
  if (!Array.isArray(rows) || rows.length === 0) {
    throw new Error("`khor _verbs` printed no verbs");
  }
  const table = new Map<string, Reach>();
  for (const row of rows) {
    if (typeof row !== "object" || row === null) {
      throw new Error("`khor _verbs` printed a row that is not an object");
    }
    const { word, reach } = row as { word?: unknown; reach?: unknown };
    if (typeof word !== "string" || word.length === 0) {
      throw new Error("`khor _verbs` printed a row without a word");
    }
    if (typeof reach !== "string" || !REACHES.has(reach)) {
      throw new Error(`\`khor _verbs\` gives \`${word}\` the reach \`${String(reach)}\`, which this gate does not know`);
    }
    if (table.has(word)) {
      throw new Error(`\`khor _verbs\` names \`${word}\` twice`);
    }
    table.set(word, reach as Reach);
  }
  return table;
}
