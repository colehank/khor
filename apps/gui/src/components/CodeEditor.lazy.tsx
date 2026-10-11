// **The editor behind a lazy boundary** (CM-BRIEF §2, the bundle split).
// `CodeEditor` draws from the whole of CodeMirror's runtime plus the
// language-package table `@codemirror/language-data` carries, and all of
// that was in the entry chunk: measured with the split undone, CodeMirror
// alone is 1,472.77 kB raw / 495.67 kB gzip of the 2,238.10 kB entry
// bundle. None of it is needed to draw the rail, a list or a terminal, so
// it rides a `React.lazy` fetch instead.
//
// # The host's helpers are not here
//
// `editorIn`, `topLineOf`, `placeLine` and `focusAtTop` used to be
// exported from `CodeEditor` itself, and each reached `EditorView`
// through `EditorView.findFromDOM` — so one import of any helper dragged
// the runtime in with it, which is what kept the split from working at
// all. They live in `lib/code/place`, reaching the view through `__view`
// on the box; `EditorView` is only a *type* there, erased before the
// bundle is drawn. **A host imports from `lib/code/place` and from here,
// never from `./CodeEditor`.**
//
// # Why `preloadEditor` exists
//
// A phone's 「编辑」 is one act in one gesture (`use-read-first`): it
// renders the editor writable and focuses it in the same press, because
// WebKit raises the soft keyboard only for a focus a touch asked for
// (`FileView`'s `beginEditing`). A chunk that arrived *after* that press
// would have no view to focus, and the keyboard would not come up. So a
// file pane starts the fetch as soon as it mounts — the two presses it
// takes to reach 「编辑」 are enough for the chunk to land, and the same
// call covers the diff tab's own. `DetailPane` warms the terminal's
// chunk the same way.
import { lazy, Suspense, type ComponentProps } from "react";

const load = () => import("./CodeEditor");
const CodeEditorImpl = lazy(() => load().then((m) => ({ default: m.CodeEditor })));

/** Fetch the editor's chunk now, without waiting for it to be drawn. */
export const preloadEditor = (): void => void load();

export function CodeEditor(props: ComponentProps<typeof CodeEditorImpl>) {
  return (
    <Suspense
      // The same box the editor fills, so the fetch does not move the
      // text under it: the mount is a `min-h-0 flex-1` child of a
      // `min-h-0 flex-1 flex-col`, and a fallback of no height would
      // collapse the column and spring the deck up for a frame.
      fallback={<div data-loading="editor" className="min-h-0 flex-1" />}
    >
      <CodeEditorImpl {...props} />
    </Suspense>
  );
}
