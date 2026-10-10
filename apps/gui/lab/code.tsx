// Lab only (lane cm-a, CM-BRIEF step 5): the file view on CodeMirror with
// fake data, for the owner's eyes before it is wired for real. Nothing
// here talks to a node. `#1` … picks the screen, `?dark` / `?light` the
// theme, `?w=390` the width.
//
//   #1  a TypeScript file: numbers, folding, the change gutter (改 / 加 /
//       删), the ruler over the scrollbar, blame on the caret's line
//   #2  the same with a peek open (HEAD's lines in the file) and ⌘F's bar
//   #3  a phone's read-first view of the same file
//   #4  a long line wrapping, the line a link asked for (brand wash), a
//       folded block
//   #5  the diff tab, side by side (HEAD | 工作副本), unchanged runs folded
//   #6  the same on a phone: one column, HEAD's lines laid in
//   #7  a refused save merged: the disk's text against the box, 保留 / 放弃
//       on every place
//   #8  an HTML file's 渲染 face: the page in a sandbox, its script not run
import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import { CodeDiff } from "@/components/CodeDiff";
import { CodeEditor, editorIn } from "@/components/CodeEditor";
import { HtmlView } from "@/components/HtmlView";
import { textBefore } from "@/lib/code/diff";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitHunk } from "@/gen/bindings/GitHunk";
import { openFind } from "@/lib/code/find";
import { showHunk } from "@/lib/code/git";
import { foldCode } from "@codemirror/language";
import { EditorSelection } from "@codemirror/state";

const SRC = `// The save road: a write per pause, and what a refusal means.
import { fetchFileText, writeFile } from "@/api";
import { PAUSE_MS } from "@/lib/save-road";

export type Draft = { text: string; was: string; by: string };

/** Write the buffer, pinned to the digest it was read at. */
export async function save(machine: string, path: string, d: Draft): Promise<string> {
  const now = await fetchFileText(machine, path);
  if (now.digest !== d.was) {
    // Somebody else wrote it first: stop, and let the person choose.
    throw new Error("clash");
  }
  const landed = await writeFile(machine, path, d.text, d.was);
  return landed.digest;
}

export function pause(): Promise<void> {
  return new Promise((r) => setTimeout(r, PAUSE_MS));
}

const RETRIES = 3;
for (let i = 0; i < RETRIES; i += 1) {
  console.log("try", i, "of", RETRIES, "— a deliberately long line that has to fold into the width of the column because the file view wraps rather than scrolling sideways");
}
`;

const HUNKS: GitHunk[] = [
  { at: 2, lines: 1, was: ['import { fetchFileText } from "@/api";'] },
  { at: 11, lines: 2, was: [] },
  { at: 16, lines: 0, was: ["  console.log(\"saved\", landed.digest);", "  // TODO: tell the tab"] },
  { at: 23, lines: 1, was: ["const RETRIES = 2;"] },
];

const C1: GitCommit = { id: "7a921aa4c0ffee", author: "colehank", when_ms: Date.now() - 3 * 86_400_000, subject: "save road: pin every write to its digest" };
const C2: GitCommit = { id: "3f00d1e2b7", author: "net-d", when_ms: Date.now() - 5 * 3_600_000, subject: "retry a failed write on the next pause" };
const BLAMED = SRC.split("\n").map((_, i) => (i === 10 || i === 11 ? null : i % 3 === 0 ? C2 : C1));

const LONG = SRC + Array.from({ length: 40 }, (_, i) => `export const step${i} = ${i} * RETRIES; // nothing changed here`).join("\n") + "\nexport const last = true;\n";
const LONG_HUNKS: GitHunk[] = [...HUNKS, { at: LONG.split("\n").length - 1, lines: 1, was: ["export const last = false;"] }];
const THEIRS = SRC.replace("const RETRIES = 3;", "const RETRIES = 5;").replace("// Somebody else", "// Another writer");

const PAGE = `<!doctype html><html><head><title>Report</title>
<style>body{font:15px/1.5 -apple-system,sans-serif;margin:2rem;max-width:40rem}table{border-collapse:collapse}td,th{border:1px solid #ccc;padding:4px 8px}</style>
</head><body>
<h1>Run 42</h1>
<p id="said">Scripts are off: this sentence would read <b>SCRIPT RAN</b> if one had run.</p>
<table><tr><th>step</th><th>loss</th></tr><tr><td>1000</td><td>2.31</td></tr><tr><td>2000</td><td>1.87</td></tr></table>
<p><img src="plot.png" alt="plot.png (relative, read through khor)" width="200"> <img src="https://example.com/pixel.png" alt="remote (refused)" width="120"></p>
<script>document.getElementById("said").textContent = "SCRIPT RAN";</script>
</body></html>`;

function Screen({ at }: { at: string }) {
  if (at === "5" || at === "6") {
    return (
      <CodeDiff path="lib/save.ts" before={textBefore(LONG, LONG_HUNKS)} after={LONG} beforeLabel="HEAD" afterLabel="工作副本" narrow={at === "6"} />
    );
  }
  if (at === "7") return <Merging />;
  if (at === "8") {
    return (
      <div data-file-preview="html" className="flex min-h-0 flex-1 flex-col">
        <HtmlView machine="lab" path="/home/lab/site/index.html" text={PAGE} ground="auto" />
      </div>
    );
  }
  return <Editing at={at} />;
}

function Merging() {
  const [text, setText] = useState(SRC.replace("throw new Error(\"clash\");", "throw new Error(\"clash: \" + path);"));
  const [left, setLeft] = useState<number | null>(null);
  return (
    <>
      <CodeEditor path="lib/save.ts" text={text} editable against={left === 0 ? null : THEIRS} onText={setText} onChunks={setLeft} />
      <div className="flex-none border-t px-3 py-1.5 text-callout text-muted-foreground">已合并对方的修改，是否保存仍由你决定 · {left ?? "…"}</div>
    </>
  );
}

function Editing({ at }: { at: string }) {
  const [text, setText] = useState(SRC);
  const [goLine, setGoLine] = useState<{ line: number; at: number } | null>(null);
  useEffect(() => {
    const t = setTimeout(() => {
      const v = editorIn(document.body);
      if (!v) return;
      if (at === "2") {
        v.dispatch({ effects: showHunk.of(0) });
        openFind(v);
        const input = document.querySelector<HTMLInputElement>("[data-code-find-input]");
        if (input) {
          const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
          set.call(input, "digest");
          input.dispatchEvent(new Event("input", { bubbles: true }));
        }
      }
      if (at === "1" || at === "3") {
        v.dispatch({ selection: EditorSelection.cursor(v.state.doc.line(9).to), userEvent: "select.pointer" });
      }
      if (at === "4") {
        setGoLine({ line: 18, at: 1 });
        v.dispatch({ selection: EditorSelection.cursor(v.state.doc.line(8).to) });
        foldCode(v);
      }
    }, 300);
    return () => clearTimeout(t);
  }, [at]);
  return (
    <CodeEditor
      path="lib/save.ts"
      text={text}
      editable={at !== "3"}
      hunks={HUNKS}
      blamed={at === "4" ? null : BLAMED}
      goLine={goLine}
      onText={setText}
    />
  );
}

function Lab() {
  const [at, setAt] = useState(() => location.hash.slice(1) || "1");
  useEffect(() => {
    const on = () => setAt(location.hash.slice(1) || "1");
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  const w = new URLSearchParams(location.search).get("w");
  return (
    <div className="flex h-dvh justify-center bg-background">
      <div data-lab-frame className="flex min-h-0 flex-col border-x" style={{ width: w ? `${w}px` : "min(100%, 60rem)" }}>
        <div className="flex h-10 flex-none items-center border-b px-3 text-callout text-muted-foreground">save.ts</div>
        <Screen key={at} at={at} />
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(<Lab />);
