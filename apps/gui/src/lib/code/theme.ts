// How the editor looks (CM-BRIEF step 1): every value a token, the font
// the app already sets on the box (`font-mono text-mono leading-mono`,
// inherited rather than restated), and the code in GitHub's colours.
//
// **The colours are not chosen here.** A painted token gets a class
// (`tok-keyword`, `tok-string` …) and app.css gives each class the value
// `github-light-default` / `github-dark-default` give the same TextMate
// scope — what the Shiki sheet painted until this lane, and what
// github.com paints (the user, 2026-09-15:「做到 gh web 预览代码的那种高
// 亮」). The theme switch is app.css's, keyed on `:root[data-theme]` as
// the palette is.
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

/** Lezer's tags to the classes app.css colours, grouped the way GitHub's
    theme groups the TextMate scopes they stand for. Exported for the
    peek's old lines (`git.ts`), painted with the same classes. */
export const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.modifier, t.controlKeyword, t.operatorKeyword, t.definitionKeyword, t.moduleKeyword, t.self], class: "tok-keyword" },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], class: "tok-comment" },
  { tag: [t.string, t.docString, t.character, t.attributeValue, t.special(t.string), t.regexp, t.escape], class: "tok-string" },
  { tag: [t.number, t.integer, t.float, t.bool, t.null, t.atom, t.unit, t.color, t.constant(t.name), t.standard(t.name), t.attributeName], class: "tok-constant" },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.function(t.definition(t.variableName)), t.macroName], class: "tok-function" },
  { tag: [t.typeName, t.className, t.namespace, t.definition(t.variableName), t.definition(t.typeName)], class: "tok-entity" },
  { tag: [t.tagName, t.quote], class: "tok-tag" },
  { tag: [t.heading, t.heading1, t.heading2, t.heading3, t.heading4, t.heading5, t.heading6], class: "tok-heading" },
  { tag: [t.link, t.url], class: "tok-link" },
  { tag: t.emphasis, class: "tok-emphasis" },
  { tag: t.strong, class: "tok-strong" },
  { tag: t.strikethrough, class: "tok-strike" },
  { tag: t.inserted, class: "tok-inserted" },
  { tag: t.deleted, class: "tok-deleted" },
  { tag: t.changed, class: "tok-changed" },
  { tag: t.invalid, class: "tok-invalid" },
]);

export const codeHighlight = syntaxHighlighting(highlight);

/** The editor's own surfaces. **Transparent on the card** — the editor is
    the file on the one ground, not a box on it; the gutters stand on the
    same ground and are parted from the text by a hairline, as the
    numbers column was. */
export const codeTheme = EditorView.theme({
  "&": {
    height: "100%",
    backgroundColor: "transparent",
    color: "var(--foreground)",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "inherit",
    lineHeight: "inherit",
    overflowAnchor: "none",
  },
  ".cm-content": { padding: "var(--gap-in) 0" },
  // **The caret and the selection, drawn** (`drawSelection`, CodeEditor's
  // head comment): the caret in the text's own ink (below);
  // the selection the app's grey, as a selected row is (README 颜色) —
  // `fill-strong` while the editor has the keys, the lighter `fill` once
  // it lost them, as macOS greys a selection then. The package's rules
  // are a chain of five classes under `.cm-focused`; the same chain here
  // is what outranks them. On iOS the main selection stays the platform's
  // and only the handles are drawn, in the caret's ink.
  // **The caret as tall as a Chinese character's ink** (the owner
  // 2026-10-13). CodeMirror sizes `.cm-cursor` to the glyph's box with an
  // inline top and height (16 on 13 in the app's WebKit), which overhangs
  // a CJK character's ink (11–12) by two or three above and below. The
  // element keeps that box and draws nothing; its `::before` is the
  // caret: 0.88 of the type size — the ideograph's ink, a little over a
  // capital's or a digit's — centred in the box, where both inks centre,
  // in the element's own border width so the stroke is unchanged.
  ".cm-cursor": { borderLeftColor: "transparent" },
  ".cm-cursor::before": {
    content: '""',
    position: "absolute",
    right: "100%",
    top: "50%",
    height: "calc(var(--text-mono) * 0.88)",
    transform: "translateY(-50%)",
    borderLeftStyle: "inherit",
    borderLeftWidth: "inherit",
    borderLeftColor: "var(--foreground)",
  },
  ".cm-selectionBackground": { backgroundColor: "var(--fill)" },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground": { backgroundColor: "var(--fill-strong)" },
  ".cm-selectionHandle": { backgroundColor: "var(--foreground)" },
  ".cm-line": { padding: "0 var(--gap-row)" },
  ".cm-gutters": {
    backgroundColor: "var(--card)",
    color: "var(--muted-foreground)",
    border: "none",
  },
  ".cm-lineNumbers": { borderRight: "var(--hair) solid var(--border)" },
  ".cm-lineNumbers .cm-gutterElement": {
    padding: "0 var(--gap-xxs) 0 var(--gap-in)",
    fontVariantNumeric: "tabular-nums",
  },
  // **The caret's line** (cm-d item 3): the lightest neutral fill, the
  // ground a hover lies on — under the text, never read as a shape; the
  // brand's wash is the line a link asked for, and stays the only one.
  // Before `.cm-anchored`, which wins where the two are one line.
  ".cm-activeLine": { backgroundColor: "var(--fill)" },
  ".cm-activeLineGutter": { backgroundColor: "var(--fill)", color: "var(--foreground)" },
  ".cm-lineNumbers .cm-activeLineGutter": { color: "var(--foreground)" },
  // **The line a link or the terminal asked for** (`goLine`): the brand's
  // wash, which is exactly its post — 「垫在被定位到的那一行」(README 颜色
  // 第 3 条) — and never a selection or a hover.
  ".cm-anchored": { backgroundColor: "var(--brand-wash)" },
  // Folding: a chevron of the app's own (`icons.tsx`, cloned), only under
  // the pointer unless the range is folded — a column of arrows on every
  // block is noise to a reader.
  ".cm-foldGutter .cm-gutterElement": {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    width: "var(--icon-sm, 1em)",
    color: "var(--tertiary)",
    cursor: "default",
  },
  // The package's own sheet gives the marker span the pointing hand; a
  // control wears the arrow (30-behavior 选字与手).
  ".cm-foldGutter span": { cursor: "default" },
  ".cm-foldGutter [data-fold='open']": { opacity: "0" },
  ".cm-gutters:hover .cm-foldGutter [data-fold='open']": { opacity: "1" },
  ".cm-foldGutter svg": { width: "0.75em", height: "0.75em" },
  ".cm-foldPlaceholder": {
    backgroundColor: "var(--fill)",
    border: "none",
    color: "var(--muted-foreground)",
    borderRadius: "var(--radius-xs)",
    padding: "0 var(--gap-xxs)",
    margin: "0 var(--gap-xxs)",
  },
  // **The selected word's other places**: the search hit's own grey,
  // since they are what ⌘F would find.
  ".cm-selectionMatch": { backgroundColor: "var(--fill-strong)", borderRadius: "var(--radius-xs)" },
  ".cm-searchMatch .cm-selectionMatch": { backgroundColor: "transparent" },
  // Search matches: the same neutral grey, rounded and over the text
  // where the selection is square and under it.
  ".cm-searchMatch": { backgroundColor: "var(--fill-strong)", borderRadius: "var(--radius-xs)" },
  ".cm-searchMatch-selected": { backgroundColor: "color-mix(in srgb, var(--git-modified) 28%, transparent)" },
  ".cm-panels": { backgroundColor: "var(--card)", color: "var(--foreground)" },
  ".cm-panels.cm-panels-top": { borderBottom: "var(--hair) solid var(--border)" },
  ".cm-panels.cm-panels-bottom": { borderTop: "var(--hair) solid var(--border)" },
  // ---- the git gutter (`git.ts`) ----
  ".cm-git-gutter": { width: "calc(var(--gap-xxs) * 1.5)" },
  ".cm-git-gutter .cm-gutterElement": { padding: "0", position: "relative" },
  ".cm-git-bar": { position: "absolute", inset: "0", cursor: "default" },
  ".cm-git-bar[data-kind='modified']": { backgroundColor: "var(--git-modified)" },
  ".cm-git-bar[data-kind='added']": { backgroundColor: "var(--git-added)" },
  ".cm-git-bar[data-kind='deleted']": {
    backgroundColor: "var(--git-deleted)",
    top: "auto",
    height: "calc(var(--text-mono--line-height) / 4)",
    bottom: "calc(var(--text-mono--line-height) / -8)",
  },
  ".cm-git-bar[data-kind='deleted'][data-edge='top']": {
    top: "calc(var(--text-mono--line-height) / -8)",
    bottom: "auto",
  },
  ".cm-git-bar[data-on]": { boxShadow: "0 0 0 var(--hair) color-mix(in srgb, var(--foreground) 40%, transparent)" },
  // The peek (`git.ts`): HEAD's lines in the change's own ink, a tenth for
  // the wash and the same hue as a hairline for the edges. The wash is
  // the ink over whatever ground the file is on, not mixed with `card`:
  // mixed, its hue is no longer the bar's (face 4p, cm-b).
  ".cm-git-peek": { padding: "0 var(--gap-row)", borderTop: "var(--hair) solid", borderBottom: "var(--hair) solid" },
  ".cm-git-peek[data-kind='modified']": {
    color: "var(--git-modified)",
    borderColor: "color-mix(in srgb, var(--git-modified) 40%, transparent)",
    backgroundColor: "color-mix(in srgb, var(--git-modified) 10%, transparent)",
  },
  ".cm-git-peek[data-kind='deleted']": {
    color: "var(--git-deleted)",
    borderColor: "color-mix(in srgb, var(--git-deleted) 40%, transparent)",
    backgroundColor: "color-mix(in srgb, var(--git-deleted) 10%, transparent)",
  },
  ".cm-git-peek[data-kind='added']": {
    color: "var(--git-added)",
    borderColor: "color-mix(in srgb, var(--git-added) 40%, transparent)",
    backgroundColor: "color-mix(in srgb, var(--git-added) 10%, transparent)",
  },
  // The old lines in the file's own ink (cm-e item 3); the wash and the
  // edges are what say 「old」. The insertion's one sentence is khor's, not
  // code: it keeps the change's ink.
  ".cm-git-was": { minHeight: "var(--text-mono--line-height)", whiteSpace: "pre-wrap", overflowWrap: "anywhere", color: "var(--foreground)" },
  ".cm-git-was[data-said]": { color: "inherit" },
  // The head bar over the old lines (`peek-head.tsx`): the interface's
  // own type and ink, not the file's — it is khor talking, not HEAD. A
  // hairline in the change's hue under it, as the peek's edges are.
  ".cm-git-peek-head": {
    display: "flex",
    alignItems: "center",
    gap: "var(--gap-xxs)",
    margin: "0 calc(var(--gap-row) * -1)",
    padding: "0 var(--gap-xxs) 0 var(--gap-row)",
    borderBottom: "var(--hair) solid",
    borderColor: "inherit",
    fontFamily: "var(--font-sans)",
    fontSize: "var(--text-callout)",
    lineHeight: "var(--text-callout--line-height)",
    color: "var(--foreground)",
  },
  // The overview ruler over the scrollbar's strip (`git.ts`).
  ".cm-git-ruler": {
    position: "absolute",
    top: "0",
    bottom: "0",
    right: "0",
    width: "var(--overview-ruler)",
    zIndex: "10",
  },
  ".cm-git-ruler > span": {
    position: "absolute",
    left: "calc(var(--gap-xxs) / 2)",
    right: "calc(var(--gap-xxs) / 2)",
    minHeight: "var(--overview-mark)",
    borderRadius: "var(--radius-xs)",
    opacity: "0.5",
  },
  // The open peek's mark: full ink and the bar's own ring, so the walk
  // reads in the margin.
  ".cm-git-ruler > span[data-on]": {
    opacity: "1",
    boxShadow: "0 0 0 var(--hair) color-mix(in srgb, var(--foreground) 40%, transparent)",
  },
  ".cm-git-ruler > span[data-kind='modified']": { backgroundColor: "var(--git-modified)" },
  ".cm-git-ruler > span[data-kind='added']": { backgroundColor: "var(--git-added)" },
  ".cm-git-ruler > span[data-kind='deleted']": { backgroundColor: "var(--git-deleted)" },
  // Blame at the end of the caret's line (`blame.ts`): takes no room.
  ".cm-blame": {
    display: "inline-block",
    width: "0",
    height: "var(--text-mono--line-height)",
    verticalAlign: "top",
    position: "relative",
  },
  ".cm-blame > span": {
    position: "absolute",
    top: "0",
    left: "0",
    paddingInlineStart: "var(--gap-group)",
    whiteSpace: "nowrap",
    overflow: "hidden",
    fontSize: "var(--text-caption)",
    lineHeight: "var(--text-mono--line-height)",
    color: "var(--tertiary)",
    userSelect: "none",
  },
});
