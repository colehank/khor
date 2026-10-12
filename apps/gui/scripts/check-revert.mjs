// 「还原」's change (`src/lib/code/revert.ts`, cm-e item 2): each shape of
// hunk, applied to a text, gives back HEAD's text.
import { revertChange } from "../src/lib/code/revert.ts";

let bad = 0;
/** A `Text`'s line table over a plain string. */
const linesOf = (s) => {
  const starts = [0];
  for (let i = 0; i < s.length; i += 1) if (s[i] === "\n") starts.push(i + 1);
  return {
    length: s.length,
    lines: starts.length,
    line: (n) => ({ from: starts[n - 1], to: n < starts.length ? starts[n] - 1 : s.length }),
  };
};
const apply = (s, c) => s.slice(0, c.from) + c.insert + s.slice(c.to);
const want = (what, now, hunk, head) => {
  const c = revertChange(linesOf(now), hunk);
  const got = c === null ? null : apply(now, c);
  const ok = got === head;
  if (!ok) bad += 1;
  console.log(`${ok ? "  ok " : "  BAD"} ${what}: ${JSON.stringify(c)}${ok ? "" : ` — gave ${JSON.stringify(got)}, want ${JSON.stringify(head)}`}`);
};

const head = "a\nb\nc\nd\n";
want("modified, one line", "a\nB\nc\nd\n", { at: 2, lines: 1, was: ["b"] }, head);
want("modified, two lines for one", "a\nX\nY\nd\n", { at: 2, lines: 2, was: ["b", "c"] }, head);
want("modified, the first line", "A\nb\nc\nd\n", { at: 1, lines: 1, was: ["a"] }, head);
want("added, in the middle", "a\nb\nnew\nnewer\nc\nd\n", { at: 3, lines: 2, was: [] }, head);
want("added, at the top", "new\na\nb\nc\nd\n", { at: 1, lines: 1, was: [] }, head);
want("added, at the foot, no final break", "a\nb\nc\nd\nnew", { at: 5, lines: 1, was: [] }, "a\nb\nc\nd\n");
want("added, at the foot", "a\nb\nc\nd\nnew\n", { at: 5, lines: 1, was: [] }, head);
want("deleted, in the middle", "a\nd\n", { at: 1, lines: 0, was: ["b", "c"] }, head);
want("deleted, at the top", "c\nd\n", { at: 0, lines: 0, was: ["a", "b"] }, head);
want("deleted, at the foot", "a\nb\n", { at: 2, lines: 0, was: ["c", "d"] }, head);
const stale = revertChange(linesOf("a\n"), { at: 9, lines: 1, was: ["x"] });
if (stale !== null) bad += 1;
console.log(`${stale === null ? "  ok " : "  BAD"} a hunk past the text: ${JSON.stringify(stale)}`);

if (bad) {
  console.error(`check-revert: ${bad} wrong`);
  process.exit(1);
}
console.log("check-revert: ok");
