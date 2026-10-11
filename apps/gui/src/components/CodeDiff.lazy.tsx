// **The diff view behind a lazy boundary** (CM-BRIEF §2, the bundle
// split). `CodeDiff` carries `@codemirror/merge` and the syntax packages
// `@codemirror/language-data` names; the diff tab is one press away from
// a file row, so the chunk is fetched when that press comes.
//
// The same preload as the editor's: a diff tab that is opened and read
// at once should not paint a hollow first (`CodeEditor.lazy`'s note says
// why the fallback keeps the column's height).
import { lazy, Suspense, type ComponentProps } from "react";

const load = () => import("./CodeDiff");
const CodeDiffImpl = lazy(() => load().then((m) => ({ default: m.CodeDiff })));

/** Fetch the diff's chunk now, without waiting for it to be drawn. */
export const preloadDiff = (): void => void load();

export function CodeDiff(props: ComponentProps<typeof CodeDiffImpl>) {
  return (
    <Suspense
      // The holder the diff fills: full height, no width of its own.
      fallback={<div data-loading="diff" className="min-h-0 flex-1" />}
    >
      <CodeDiffImpl {...props} />
    </Suspense>
  );
}
