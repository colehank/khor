// Lab only (lane md-render): the one Markdown renderer with nothing
// around it, for the gate (`scripts/check-md.mjs`) and for eyes. Nothing
// here talks to a node; a file's pictures come from whatever answers
// `/api/preview` (the gate routes it). `?dark` / `?light` the theme.
//
//   window.mdShow({ text, file? })   paint once, resolve when settled
//   window.mdStream({ text, step })  drip `text` in `step`-sized chunks
//                                    the way a turn arrives, and answer
//                                    what each chunk cost
//   window.mdLong()                  the long reply the measure streams
//
//   #chat / #readme  a turn and a README, painted, for a screenshot
import { useState } from "react";
import { flushSync } from "react-dom";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import * as md from "@/components/Markdown";

import { longReply } from "./md-long";

const { Markdown } = md;
// Before this lane the module had no counter; the measure still runs.
const measured = (md as { measured?: { blocks: number } }).measured ?? { blocks: -1 };

type Shown = { text: string; file?: { machine: string; path: string } };

const settle = () =>
  new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(r, 30))));

let show: (s: Shown) => void = () => {};

function Lab() {
  const [shown, setShown] = useState<Shown>(() =>
    location.hash === "#readme"
      ? { text: README, file: { machine: "m", path: "/repo/README.md" } }
      : location.hash === "#chat"
        ? { text: CHAT }
        : { text: "" },
  );
  show = (s) => flushSync(() => setShown(s));
  // The props this lane added are passed through loosely so the same
  // page measures the renderer before and after them.
  const Md = Markdown as unknown as (p: Record<string, unknown>) => React.ReactNode;
  return (
    <div data-lab-md className="mx-auto max-w-[44em] p-6">
      {shown.file ? (
        <div data-file-preview="md">
          <Md text={shown.text} className="prose-khor px-6 py-4" lines machine={shown.file.machine} path={shown.file.path} />
        </div>
      ) : (
        <Md text={shown.text} className="text-read text-read-ink" />
      )}
    </div>
  );
}

const w = window as unknown as Record<string, unknown>;
w.mdLong = longReply;
w.mdMeasured = () => ({ ...(measured as object) });
w.mdShow = async (s: Shown) => {
  show(s);
  await settle();
};
w.mdStream = async ({ text, step }: { text: string; step: number }) => {
  show({ text: "" });
  await settle();
  const commits: number[] = [];
  let gapMax = 0;
  let last = performance.now();
  let on = true;
  const tick = () => {
    const t = performance.now();
    gapMax = Math.max(gapMax, t - last);
    last = t;
    if (on) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  const t0 = performance.now();
  for (let at = step; at < text.length + step; at += step) {
    const s = performance.now();
    show({ text: text.slice(0, at) });
    commits.push(performance.now() - s);
    // A frame between chunks, the way a poll lands between paints.
    await new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
  }
  await settle();
  await new Promise((r) => setTimeout(r, 300));
  on = false;
  const wall = performance.now() - t0;
  const sorted = [...commits].sort((a, b) => a - b);
  return {
    chunks: commits.length,
    commitMax: sorted[sorted.length - 1],
    commitP95: sorted[Math.floor(sorted.length * 0.95)],
    commitTotal: commits.reduce((a, b) => a + b, 0),
    gapMax,
    wall,
    top: commits
      .map((ms, i) => [i, Math.round(ms)])
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6),
  };
};

const README = `# khor

<p align="center"><img src="docs/logo.png" alt="logo" width="64"></p>

> [!NOTE]
> 一张点对点的设备网。

> [!WARNING]
> 这一版还没有旧版兼容。

## 安装

按 <kbd>⌘</kbd> <kbd>K</kbd> 打开搜索。质能方程 $E = mc^2$，以及：

$$
\\int_0^1 x^2 \\, dx = \\frac{1}{3}
$$

- [x] 已完成
- [ ] 没完成

| 左 | 中 | 右 |
|:--|:-:|--:|
| a | b | c |

<details><summary>展开</summary>

里面的字。

</details>

脚注在这里[^1]，跳到 [安装](#安装)。:rocket:

\`\`\`mermaid
graph TD; A-->B; B-->C;
\`\`\`

[^1]: 脚注的内容。
`;

const CHAT = `我改了三处，**先量再修**。价格从 $5 涨到 $10，范围 3~5 天。

$$
a^2 + b^2 = c^2
$$

行内 \\(x + y\\) 也行。

> [!TIP]
> 会话里也认提示块。

\`\`\`ts
const x: number = 1;
\`\`\`
`;

ReactDOM.createRoot(document.getElementById("root")!).render(<Lab />);
