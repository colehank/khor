// The indent unit a file gets (`src/lib/code/indent.ts`, cm-d item 2):
// read from its own lines, the language's convention where they say
// nothing or tie.
import { indentOf } from "../src/lib/code/indent.ts";

let bad = 0;
const want = (what, got, unit, tabSize) => {
  const ok = got.unit === unit && got.tabSize === tabSize;
  if (!ok) bad += 1;
  console.log(`${ok ? "  ok " : "  BAD"} ${what}: ${JSON.stringify(got)}${ok ? "" : ` — want ${JSON.stringify({ unit, tabSize })}`}`);
};

const four = `fn main() {
    let x = 1;
    if x > 0 {
        println!("{x}");
    }
}
`;
const tab = "package main\n\nfunc main() {\n\tx := 1\n\tif x > 0 {\n\t\tprintln(x)\n\t}\n}\n";
const two = `export function f(a) {
  // a doc line
  if (a) {
    return [
      a,
    ];
  }
}
`;
// JSDoc's ` * ` lines move by one and say nothing.
const jsdoc = `/**
 * One.
 * Two.
 */
function g() {
    return 1;
}
`;
want("a 4-space .rs", indentOf(four, "Rust"), "    ", 4);
want("the same lines in a .ts (the file wins over the convention)", indentOf(four, "TypeScript"), "    ", 4);
want("a tab .go", indentOf(tab, "Go"), "\t", 4);
want("tabs in a .ts", indentOf(tab, "TypeScript"), "\t", 4);
want("a 2-space .ts", indentOf(two, "TypeScript"), "  ", 2);
want("a 2-space .rs", indentOf(two, "Rust"), "  ", 2);
want("an empty .rs takes Rust's", indentOf("", "Rust"), "    ", 4);
want("an empty .ts takes TypeScript's", indentOf("", "TypeScript"), "  ", 2);
want("an empty .go takes gofmt's tab", indentOf("", "Go"), "\t", 4);
want("nothing indented, no language", indentOf("a\nb\nc\n", null), "  ", 2);
want("a doc comment's one-space lines", indentOf(jsdoc, "JavaScript"), "    ", 4);
want("an 8-space file", indentOf("a:\n        b\n                c\n        d\n", null), "        ", 8);
want("CRLF lines", indentOf("a {\r\n    b\r\n}\r\n\r\n", "CSS"), "    ", 4);
want("a .py with 4 and 8 deep", indentOf("def f():\n    if x:\n        y\n", "Python"), "    ", 4);
// Steps 2 and 4 once each: the tie goes to the language.
want("a tie in a .rs", indentOf("a\n  b\n      c\n", "Rust"), "    ", 4);
want("the same tie in a .ts", indentOf("a\n  b\n      c\n", "TypeScript"), "  ", 2);

if (bad) {
  console.error(`indent gate: ${bad} wrong`);
  process.exit(1);
}
console.log("indent gate: a file's own indent (tabs, 2, 4, 8), the language's where it says nothing or ties");
