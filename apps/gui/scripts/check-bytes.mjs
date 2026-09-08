// The bytes gate: **no literal control character in a tracked file.**
//
// Not a style rule. It exists because one of them happened, on
// 2026-09-08, and what it broke was not a feature — it was **searching**:
//
//   `views/UsageHalf.tsx` carried four literal NUL bytes (separators in
//   two composite map keys and one key prefix, written as bytes where
//   they should have been the escape `\u0000`). The program ran
//   correctly: a NUL in a string is a NUL either way, `tsc` was clean
//   and every browser gate was green.
//
//   **`file` called the source `data`.** And this machine's `grep` runs
//   with `-I`, which skips binary files — so every search of that file
//   came back **empty, with no error and no warning**. Three conclusions
//   that evening rested on 「grep found nothing」 about that very file,
//   and each of them was a guess wearing the clothes of a measurement.
//   The symptom had been visible twice (`grep -c ""` answering nothing
//   for a 1076-line file) and was shrugged off both times, because
//   **「I found nothing」 and 「I did not look」 are one output**.
//
//   What caught it was two tools disagreeing: `git grep` read the blob
//   and found a line; `grep` had already decided the file was not worth
//   reading.
//
// # The same character, two tools, and only one of them said so
//
// Writing that commit's message, the character was typed into the
// message itself — and `git commit`'s own input validation **refused
// it by name**. Same byte, same afternoon: one tool blocked it and
// explained, the other returned an empty result and said nothing.
// That contrast is the whole argument for this file: **a rule cannot
// rest on 「a tool would have told me」 when a tool has already chosen
// not to.**
//
// # ⚠️ What this cannot block
//
// **Only what is written literally into a tracked file.** A control
// character built at runtime, read out of data, or written as an escape
// (`"\u0000"`, `'\x1b'`) passes — and it should: `lib/model-band`'s
// separator is exactly that character, spelled as an escape, and it is
// correct. **This gate is about whether the source can be read by the
// tools people search it with**, not about which characters a program
// may hold.
//
// It also says nothing about a file it skipped. The skips are declared
// by extension in `BINARY` below and both counts are printed, because a
// scan whose scope is invisible is a scan whose zero means nothing.
//
// Run: `npm run bytes` (static, no browser, no cargo lock).
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../../..", import.meta.url));

let bad = 0;
const fail = (m) => {
  console.error(m);
  bad += 1;
};

/** Tab, newline, carriage return — the three a text file is made of. */
const ALLOWED = new Set([9, 10, 13]);

/**
 * Where a file carries a control character it should have spelled as an
 * escape, as `line:column` and the code point.
 *
 * Reads **bytes**, not text: the whole failure this gate is named after
 * is a decoder deciding the file is not text and answering nothing.
 */
function offenders(buf) {
  const out = [];
  let line = 1;
  let col = 1;
  for (const b of buf) {
    if (b === 10) {
      line += 1;
      col = 1;
      continue;
    }
    if (b === 127 || (b < 32 && !ALLOWED.has(b))) {
      out.push({ line, col, code: `0x${b.toString(16).padStart(2, "0")}` });
    }
    col += 1;
  }
  return out;
}

// **The detector's own two arms, run before it is trusted.** A scan that
// reports zero is indistinguishable from a scan that cannot find
// anything — this file's entire subject — so the instrument is checked
// against a sample it must flag and a sample it must not, on every run
// rather than only when somebody thinks to tear it down.
{
  const dirty = offenders(Buffer.from("const a = 1;\nconst sep = `x\0y`;\n", "binary"));
  if (dirty.length !== 1 || dirty[0].line !== 2 || dirty[0].code !== "0x00") {
    fail(`probe dead: the detector answered ${JSON.stringify(dirty)} for a sample with one NUL on line 2 — nothing below it means anything`);
  }
  const clean = offenders(Buffer.from('const s = "\\u0000";\n\tconst t = 2;\r\n', "utf8"));
  if (clean.length !== 0) {
    fail(
      `probe void: the detector flagged ${JSON.stringify(clean)} in a file that has only an escape, a tab and a CRLF — ` +
        `a gate that fires on ordinary source is one somebody turns off`,
    );
  }
}

/**
 * Files not read, by extension, with what that costs.
 *
 * Declared rather than sniffed **because sniffing is the bug**: 「this
 * looks binary, skip it」 is the rule that made `grep` lie. Named here,
 * a new one has to be added on purpose, and the count of skips is
 * printed beside the count of reads.
 */
const BINARY = [".png", ".jpg", ".jpeg", ".gif", ".ico", ".icns", ".pdf", ".woff", ".woff2", ".ttf", ".otf", ".zip", ".car", ".webp", ".mp4", ".wasm"];

const tracked = execFileSync("git", ["ls-files", "-z"], { cwd: repo, maxBuffer: 64 << 20 })
  .toString("utf8")
  .split("\0")
  .filter(Boolean);
// The probe before the negative claim: an empty list would let every
// assertion below pass by having looked at nothing.
if (tracked.length < 50) {
  fail(`probe dead: git lists ${tracked.length} tracked files from ${repo} — this gate is pointed at the wrong tree`);
}

const skipped = [];
let read = 0;
for (const rel of tracked) {
  if (BINARY.some((e) => rel.toLowerCase().endsWith(e))) {
    skipped.push(rel);
    continue;
  }
  let buf;
  try {
    buf = readFileSync(join(repo, rel));
  } catch {
    // A tracked path that is not readable (a submodule, a broken link)
    // is not this gate's subject, and is counted rather than hidden.
    skipped.push(`${rel} (unreadable)`);
    continue;
  }
  read += 1;
  const hits = offenders(buf);
  if (hits.length) {
    fail(
      `${rel} carries ${hits.length} literal control character${hits.length > 1 ? "s" : ""} ` +
        `(${hits.slice(0, 4).map((h) => `${h.code} at ${h.line}:${h.col}`).join(", ")}${hits.length > 4 ? ", …" : ""}) — ` +
        `**write it as an escape** (\`\\u0000\`, \`\\x1b\`): the program cannot tell the difference and every tool that ` +
        `reads the file can. \`file\` calls a source with one of these \`data\`, and a \`grep -I\` skips it silently, ` +
        `so every later search of this file answers 「nothing」 whether or not there is anything.`,
    );
  }
}

console.log(`bytes: ${read} tracked files read, ${skipped.length} skipped by extension (${skipped.join(", ") || "none"})`);
if (!bad) console.log("bytes gate: no literal control character in any tracked source");
process.exitCode = bad ? 1 : 0;
