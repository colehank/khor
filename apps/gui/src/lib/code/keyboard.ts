// **The caret above the soft keyboard** (KB-CARET, the owner 2026-10-12:
// 「键盘唤醒了，但没有尊重光标位置。打字的时候，能否保持光标在键盘上合适位置。」).
//
// Six rules; three live here and in `CodeEditor.tsx`:
//
// 2. **Tap → reveal, riding the keyboard's animation.** When `.cm-scroller`
//    shrinks (the visual viewport shrank around it), reveal the caret head
//    with `nearest` and one line of bottom margin so it lands above the
//    floor.  One reveal per animation frame (`revealCaretAfterResize`), called
//    from the `ResizeObserver` set up by `observeScroller` in `CodeEditor`.
//
// 3. **While typing, a comfort band.** `EditorView.scrollMargins` adds one
//    line of bottom margin while `footCovered()` is true; desktops see no
//    extra margin.
//
// 4. **Keyboard going down moves nothing.** When the scroller grows, the
//    `ResizeObserver` pins `scrollTop` for one frame so WebKit's layout
//    growth does not snap the view.
//
// Rules 1 (inset already true), 5 (selection end — head), and 6 (goLine
// one-third) are handled at call sites in `CodeEditor.tsx`.
import { type Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

import { footCovered } from "@/lib/viewport";

/** One rAF token: only one reveal per animation frame. */
let revealFrame = 0;

/**
 * **Reveal the caret after the scroller shrank** (rule 2): called from
 * the `ResizeObserver` in `observeScroller`.  The scroller already has its
 * new height; dispatching `scrollIntoView` here brings the caret above the
 * new floor without touching anything that was already visible.
 */
export function revealCaretAfterResize(view: EditorView): void {
  if (revealFrame) return;
  revealFrame = requestAnimationFrame(() => {
    revealFrame = 0;
    if (!view.hasFocus) return;
    const head = view.state.selection.main.head;
    const lineHeight = view.defaultLineHeight;
    const View = view.constructor as typeof EditorView;
    view.dispatch({
      effects: View.scrollIntoView(head, { y: "nearest", yMargin: lineHeight }),
    });
  });
}

/**
 * **The scroll-margins extension** (rule 3): one line of bottom clearance
 * while the foot is covered, nothing on desktop.
 */
export function keyboardCaret(): Extension {
  return EditorView.scrollMargins.of((view) => {
    if (!footCovered()) return null;
    return { bottom: view.defaultLineHeight };
  });
}

/**
 * **Set up the ResizeObserver that drives rules 2 and 4** on the scroller.
 * Returns a teardown for `useLayoutEffect`.
 *
 * Rule 2: when the scroller shrank and the editor has focus, reveal the caret.
 * Rule 4: when the scroller grew (keyboard going down), pin `scrollTop` for
 * one frame so the content does not jump.
 */
export function observeScroller(
  scroller: HTMLElement,
  getView: () => EditorView | null,
): () => void {
  let lastHeight = scroller.clientHeight;

  const ro = new ResizeObserver(() => {
    const h = scroller.clientHeight;
    const grew = h > lastHeight;
    const shrank = h < lastHeight;
    lastHeight = h;

    const view = getView();
    if (!view) return;

    if (shrank) {
      // Rule 2: keyboard rising — reveal the caret above the new floor.
      revealCaretAfterResize(view);
    } else if (grew) {
      // Rule 4: keyboard going down — pin scrollTop so content stays put.
      const saved = scroller.scrollTop;
      requestAnimationFrame(() => {
        if (scroller.scrollTop !== saved) scroller.scrollTop = saved;
      });
    }
  });

  ro.observe(scroller);
  return () => ro.disconnect();
}
