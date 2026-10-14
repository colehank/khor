// Two texts against each other on `@codemirror/merge` (CM-BRIEF step 2):
// what the diff tab, the history's 「与当前比较」 and a refused save's
// merge draw. Side by side on a wide window, one column on a narrow one;
// unchanged runs folded behind a 「展开」 bar; per-chunk 保留 / 放弃 only
// where a choice is being made (the merge after a refusal).
//
// **Git's inks, not the package's.** What was there is `git-deleted`,
// what is there now is `git-added` — the tree's letters and the gutter's
// bars say the same two things in the same two colours — a tenth for a
// line's wash and a quarter for the words that changed inside it.
import { EditorState, type Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

import { gui } from "@/gen/catalog";

/** HEAD's text (or a revision's), rebuilt from the file as it is now and
    the hunks that lead from that to this — all `git_diff` and `git_show`
    hand over. **It needs no editor**, so it lives in `lib/code/text`:
    `DiffView` calls it before the merge view has been fetched (CM-BRIEF
    §2, the bundle split). Re-exported here, where it was first. */
export { textBefore } from "@/lib/code/text";

/** The words `@codemirror/merge` asks for, from the catalog. */
export const diffPhrases = (): Extension => EditorState.phrases.of({
  "$ unchanged lines": gui.diff_unchanged("$"),
  Accept: gui.merge_keep,
  Reject: gui.merge_drop,
});

/** Unchanged runs longer than this fold, keeping this much context on
    either side of every change. */
export const COLLAPSE = { margin: 3, minSize: 6 };

export const diffTheme = EditorView.theme({
  "&.cm-merge-a .cm-changedLine, .cm-deletedChunk": {
    backgroundColor: "color-mix(in srgb, var(--git-deleted) 10%, transparent)",
  },
  "&.cm-merge-b .cm-changedLine, .cm-inlineChangedLine, .cm-insertedLine": {
    backgroundColor: "color-mix(in srgb, var(--git-added) 10%, transparent)",
  },
  "&.cm-merge-a .cm-changedText, .cm-deletedChunk .cm-deletedText": {
    background: "color-mix(in srgb, var(--git-deleted) 25%, transparent)",
  },
  "&.cm-merge-b .cm-changedText, .cm-insertedLine .cm-changedText": {
    background: "color-mix(in srgb, var(--git-added) 25%, transparent)",
  },
  ".cm-deletedChunk": {
    paddingLeft: "var(--gap-row)",
    color: "var(--muted-foreground)",
  },
  ".cm-deletedChunk .cm-chunkButtons": {
    insetInlineEnd: "var(--gap-in)",
    display: "flex",
    gap: "var(--gap-xxs)",
    fontFamily: "var(--font-sans)",
  },
  ".cm-chunkButtons .cm-merge-btn": {
    border: "none",
    margin: "0",
    padding: "0 var(--gap-in)",
    borderRadius: "var(--radius-xs)",
    font: "inherit",
    fontSize: "var(--text-caption)",
    lineHeight: "var(--text-mono--line-height)",
    color: "var(--foreground)",
    // **A face, not a wash** (2026-10-14). The buttons float at the
    // end of a deleted chunk, over the very lines they are about, and
    // `--fill-strong` is a *translucent* grey (0.1 light, 0.15 dark):
    // the code underneath showed through and mixed with 保留 / 放弃,
    // which is the one thing a button's own label can never survive.
    // The measure that caught it is the pixel probe in
    // `scripts/check-merge-buttons.mjs`: with the deleted lines hidden
    // and shown again, 71% of the buttons' pixels moved by up to 153 —
    // the face was a window. `--popover` is opaque and is the app's one
    // floating face (the file column's own notice is painted with it),
    // so the buttons read as what they are: a control above the text.
    backgroundColor: "var(--popover)",
    cursor: "default",
  },
  // The bar a folded run is: the neutral fill, the muted ink, pressed to
  // open (the package's own behaviour; its words come from the catalog).
  ".cm-collapsedLines": {
    padding: "var(--gap-xxs) var(--gap-row)",
    cursor: "default",
    color: "var(--muted-foreground)",
    background: "var(--fill)",
    fontFamily: "var(--font-sans)",
    fontSize: "var(--text-caption)",
  },
  ".cm-collapsedLines:before, .cm-collapsedLines:after": { content: "none" },
  ".cm-changeGutter": { width: "calc(var(--gap-xxs) * 1.5)", paddingLeft: "0" },
  "&.cm-merge-a .cm-changedLineGutter, .cm-deletedLineGutter": { background: "var(--git-deleted)" },
  "&.cm-merge-b .cm-changedLineGutter, .cm-inlineChangedLineGutter": { background: "var(--git-added)" },
});
