// The math chunk: loaded the first time a text holds a formula
// (`lib/md/extras`), never before — KaTeX is the largest part of the
// renderer and most turns never ask for it.
//
// The style sheet is KaTeX's own; `vite.config.ts` cuts its fonts down to
// the woff2 files, which every engine this app runs in reads.
import "katex/dist/katex.min.css";

import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";

export { rehypeKatex, remarkMath };
