// **One file of a commit behind a lazy boundary** (cm-d item 1, the
// bundle split of CM-BRIEF §2): `CommitDiff` carries `@codemirror/merge`
// and the editor core, which the entry chunk does not; the commit page
// fetches the chunk as it opens, while the node is still answering.
//
// **Not `React.lazy`.** A lazy component suspends on its first render even
// when its module is already here, and React 19 holds a suspended
// boundary's reveal for up to 300 ms — the commit page drew its files at
// 420 ms where the answer and the chunk were both in by 26 (cm-d §1). The
// module is kept once it has come, and a file drawn after that draws at
// once.
import { useEffect, useState, type ComponentProps } from "react";

type Impl = typeof import("./CommitDiff").CommitDiff;

let impl: Impl | null = null;
const load = () => import("./CommitDiff").then((m) => (impl = m.CommitDiff));

/** Fetch the chunk now, before the commit's files arrive. */
export const preloadCommitDiff = (): void => void load();

export function CommitDiff(props: ComponentProps<Impl>) {
  const [Diff, setDiff] = useState<Impl | null>(() => impl);
  useEffect(() => {
    if (Diff === null) void load().then((d) => setDiff(() => d));
  }, [Diff]);
  return Diff === null ? <div data-loading="commit-diff" /> : <Diff {...props} />;
}
