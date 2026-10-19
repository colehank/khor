// **The notebook preview behind a lazy boundary** (lane nb-view), the way
// the editor is (`CodeEditor.lazy`): nothing of it is in the entry chunk,
// and an `.ipynb` starts the fetch when it opens as source, so the first
// press of 预览 draws at once.
import { lazy, Suspense, type ComponentProps } from "react";

const load = () => import("./Notebook");
const NotebookImpl = lazy(load);

/** Fetch the chunk now, without waiting for it to be drawn. */
export const preloadNotebook = (): void => void load();

export function Notebook(props: ComponentProps<typeof NotebookImpl>) {
  return (
    <Suspense fallback={null}>
      <NotebookImpl {...props} />
    </Suspense>
  );
}
