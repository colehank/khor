// One file, read (工作区批2). What the middle column of a workspace
// shows when a file is picked out of the tree.
//
// **Editable since 工作区批4, and only when the whole file is here.**
// What a save writes back is what is on screen, so a view holding the
// first 256 KB of a larger file has nothing honest to write — those
// stay read-only and say so. The permission is a single fact travelling
// with the text (`FileText.digest`), not a second judgment made here
// that could disagree with the one that decoded it.
//
// **The editor is a transparent `<textarea>` over the painted `<pre>`.**
// Not CodeMirror: its gutter would fight the git gutter this file
// already draws, and the part of it worth the dependency — completion,
// bracket matching, multiple cursors — is the part this app said no to
// (「不提供任何自动补全什么的」). What that costs is written at `Sheet`
// below, where it is spent.
//
// **The three things it refuses to paint, each with its own sentence.**
// Not text; cut short by one slice; nothing there at all. They are
// separate because the next move differs: take it and open it
// elsewhere, scroll what is here and take the rest if you need it, or
// nothing. Folded into one「打不开」they would all read as khor being
// broken, which is the failure `check-acp` 第二批 names.
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import {
  fetchPreview,
  type Preview,
  fetchFileText,
  fetchGitDiff,
  mergeFileText,
  pullFile,
  saveFileText,
  type FileText,
  type GitDiff,
  type GitHunk,
} from "@/api";
import { Markdown } from "@/components/Markdown";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { parseDelimited } from "@/lib/csv";
import { languageOf, paintFileLines } from "@/lib/highlight";
import { cn } from "@/lib/utils";
import { sizeWord } from "@/views/FilesPane";

/** The same table `khor_node::preview::kind` keeps, for the one
    question this view must answer **before** asking anything: which of
    the two roads to send this file down.

    ⚠️ **A second copy of a table, and it is the kind this repo
    normally refuses.** It exists because the alternative is worse: to
    learn the kind from the node the view must first ask, and asking
    means either spending a slice decoding a picture as text or leaving
    the pane blank for a round trip on every file. What keeps the two
    honest is `check-face`, which drives one file of each kind and would
    go red the day this list and the node's disagree. */
/** An unsaved working copy: the text, and the digest the next save is
    pinned to. **One piece of state, not two** — a save replaces both at
    once, and holding them apart is how a second save gets pinned to a
    digest that belongs to different bytes. */
export type Draft = { text: string; was: string };

const IMAGES = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "avif"];
const DOCS = ["docx", "xlsx", "pptx", "doc", "xls", "ppt"];

export function kindOf(path: string): "image" | "svg" | "pdf" | "doc" | "text" {
  const name = path.split("/").pop() ?? "";
  const dot = name.lastIndexOf(".");
  const e = dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
  if (IMAGES.includes(e)) return "image";
  if (e === "svg") return "svg";
  if (e === "pdf") return "pdf";
  if (DOCS.includes(e)) return "doc";
  return "text";
}

/** Which second face this text file has, or `null` for one with none
    (工作区批⑤).

    **A property of the format, not of the road.** Everything here is a
    text file: it opens as source, it saves, it merges, it wears a
    change gutter. What this answers is whether there is anything to
    *also* show — and where there is not, no switch is drawn rather
    than a greyed one. */
export function rendererOf(path: string): "md" | "table" | null {
  const name = path.split("/").pop() ?? "";
  const dot = name.lastIndexOf(".");
  const e = dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
  if (e === "md" || e === "markdown") return "md";
  if (e === "csv" || e === "tsv") return "table";
  return null;
}

/** A file khor will not paint, and the one thing it can still do about
    it. **One component for the three refusals** (too big, not text,
    nothing khor knows) because they differ only in the sentence — and
    the way out is the same. */
function Away({
  word,
  onTake,
  took,
}: {
  word: string;
  onTake: () => void;
  took: string | null;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
      <span className="text-fact text-foreground">{word}</span>
      <Button variant="outline" size="sm" onClick={onTake} className="mt-1 min-h-touch min-w-touch">
        {gui.file_take}
      </Button>
      {took !== null && <span className="text-aux text-muted-foreground">{took}</span>}
    </div>
  );
}

/** The language a path's extension names, or null to draw it plain.
    **The extension and nothing else**: a file with no suffix is not a
    guess worth making, and `highlightAuto` is the expensive path this
    app already refuses (`lib/highlight`). */
export function languageOfPath(path: string): string | null {
  const name = path.split("/").pop() ?? "";
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return null;
  return languageOf(name.slice(dot + 1));
}

export function FileView({
  machine,
  path,
  face = "source",
  onSaved,
  draft,
  onDraft,
  saveAt = 0,
  readAt = 0,
  goLine = null,
  onNote,
}: {
  machine: string;
  /** Absolute, on that machine. */
  path: string;
  /** 源码 or 预览.
   *
   * **Held by the pane since 2026-09-16**, because the switch is on the
   * tab strip now: 「取消源码/预览的按钮,放在页面的右上角那,如果可预览,
   * 就是一个预览的 icon,再次点击回到源码」(the user). The strip is one
   * component up, and a face this view kept to itself would be a state
   * the button could not reach.
   *
   * **Still 源码 on every file and still not remembered** (「打开先是
   * 源码」, 工作区批⑤) — the pane resets it when the front tab changes,
   * which is the same rule spelled one level higher. */
  face?: "source" | "preview";
  /** A save landed. The workspace's marks and branch are stale the
      moment one does, and this view is not the one holding them. */
  onSaved?: () => void;
  /** The unsaved working copy for **this** file, if there is one, held
      by whoever owns the pane.
      ⚠️ **It lives up there because this view is remounted on every
      file** (`key` is machine + path), and until 2026-09-12 that meant
      clicking another file in the tree threw away everything typed —
      no warning, no dialog, no way back. Lifting the draft is the fix
      that costs a person nothing: walk away, walk back, it is still
      there. */
  draft?: Draft;
  onDraft?: (draft: Draft | null) => void;
  /* ⚠️ **Tombstone: `undo` / `onUndo` went with the strip**
     (2026-09-16). What this file held before the last save was kept by
     the pane and put back into the box by a 「回到存之前」 button — and
     that button lived on the only row this view ever drew over the
     file, which is the row the user had taken off:「修改文件不要显示
     "存好了"和"回到存之前", 类似要在文件上方新增一列的东西都不要」.
     Keeping the state with nothing to press it would be a pane
     remembering a text for a control that is not there. **What is given
     up is said out loud**: the 「存完才发现存错了」 minute has no khor
     answer now; the editor's own undo stack is the whole of it. */
  /** Bumped by whoever holds this pane to ask for a save — the sheet
      that catches a tab being closed over a draft (工作区批⑧).
      **A counter and not a callback handed the other way**, because the
      thing that has to happen is exactly what pressing 存 does, and a
      second road into the save is a second place the digest contract
      can be got wrong (`save`). */
  saveAt?: number;
  /** Bumped by whoever holds this pane when it is worth asking the far
      machine what this file says **now** — 「打开的文件在盘上变了,没
      草稿就直接换新内容」(the user, 2026-09-15, 工作区批⑩).
   *
   * **The same beat the marks ride, and for the same reason**: coming
   * back to the window is the moment a stale screen starts to mislead,
   * and khor里最常改文件的是 agent. A timer was the other road and it is
   * the worse one — a read of somebody else's disk every few seconds for
   * an answer that is almost always the one already on screen.
   *
   * A counter, because two returns are two questions. */
  readAt?: number;
  /** **What this file's tab should whisper on hover**, or `null` when
      it has nothing to add (第十四轮 第 2 条).

      One sentence today: the diff stopped short of the whole file
      (`git_more`). It used to be a row under the tab strip, and the
      user's ruling took every such row away — 「任何标签页下方都不要有
      副标题」 — so what is left of it goes where a tab already says the
      long form of things, its `title`. Reported upward rather than
      drawn here, because the tab is the pane's (`WorkspacePane`). */
  onNote?: (note: string | null) => void;
  /** A line somebody asked for from outside — a path with a `:12` on it
      pressed in a conversation (工作区第五轮 第 35 条).
   *
   * **Stamped rather than just numbered.** Pressing the same
   * `main.rs:12` twice is two requests, and the second one has to move
   * the view again even though the number did not change; an effect on
   * the number alone would do nothing the second time and read as the
   * link being broken. */
  goLine?: { line: number; at: number } | null;
}) {
  /** What this file turns out to be, when it is not code. `null`
      while khor is still asking, and for the kind that needs no
      asking — `text`, which is the road this view already had. */
  const [look, setLook] = useState<Preview | null>(null);
  const [lookError, setLookError] = useState<string | null>(null);
  const [file, setFile] = useState<FileText | null>(null);
  /** The working copy, and the digest it is pinned to. **One piece of
      state, not two**: a save replaces both at once (the text it wrote
      and the digest the far machine answered), and holding them apart
      is how a second save gets pinned to a digest that belongs to
      different bytes. `null` until the file arrives. */
  const [own, setOwn] = useState<Draft | null>(null);
  /** The buffer: this mount's own once it has one, and the pane's draft
      until then.
      **That order, and it is the second spelling of it** (工作区批⑧).
      It read `draft ?? own` while every `setEdit` also handed the new
      value straight up to the pane in the same tick — so the two could
      not disagree. Reporting the draft from an effect made them able
      to: the pane's copy is now one render behind, and a buffer read
      from behind is a buffer that never moves. A merge landed in `own`
      and the screen went on showing the text from before it, for as
      long as the file stayed open (`check-face` 4b-bis).

      `own` is never staler than `draft` — `draft` is derived from it —
      so preferring it is not a race won by luck. What the fallback is
      still for is the beat after a remount, when this mount has read
      nothing yet and the pane is holding what somebody typed. */
  const edit = own ?? draft ?? null;
  const setEdit = useCallback(
    (next: Draft | null | ((was: Draft | null) => Draft | null)) => {
      setOwn((was) => (typeof next === "function" ? next(was ?? draft ?? null) : next));
    },
    [draft],
  );
  const [saving, setSaving] = useState(false);
  /** Bumped to read this file again without changing which file it is
      — the way out of a refusal that keeps the person's own change
      (`theirs`). A counter for `diffAt`'s reason: two reads of one
      path are two questions, and a boolean cannot ask twice. */
  const [reread, setReread] = useState(0);
  /** What the last save did — a sentence when it refused, `true` for a
      moment when it landed. */
  const [saved, setSaved] = useState<string | true | null>(null);
  /** How many clashes the last merge left, or `null` when the box holds
      nothing merged. **Its own state rather than a word inside
      `saved`**: `saved` is what the *file* did and a merge does nothing
      to the file — reusing it would paint 「refused」 red over a box
      that is merely waiting to be saved. */
  const [clashes, setClashes] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [took, setTook] = useState<string | null>(null);
  const [diff, setDiff] = useState<GitDiff | null>(null);
  // ⚠️ **Tombstone: `diffError`** (第十四轮 第 2 条). It was kept apart
  // from `diff` so the refusal could be printed — no bars and no answer
  // look identical on screen, and the old rule was that khor must not
  // let the second pass for the first. The user's ruling is that the
  // row is worse than the ambiguity:「任何标签页下方都不要有副标题」.
  // So a diff khor could not read is drawn the way a directory that is
  // not a checkout is drawn — **nothing** — and the state it was held
  // in is gone with the row that read it.
  /** Which hunk is expanded, by index. **One at a time and closed by
      default** — 「打开是当前状态,git 状态显示在左侧,点击左侧状态后,
      才展开 diff」(the user, 2026-09-11). A file that opened already
      unfolded would be a diff view wearing a file view's name. */
  const [shown, setShown] = useState<number | null>(null);
  /** The one scroller in this view. Held so a step can bring its hunk
      into view **without `scrollIntoView`**, which walks every
      scrollable ancestor including the window — the same call that
      exposed the whole app scrolling as one page on 2026-09-12. Here
      the offset is arithmetic on the line height, which this view knows
      exactly. */
  const box = useRef<HTMLDivElement>(null);
  /** Bumped to ask the diff again. **A counter rather than a boolean**:
      two saves in a row have to be two questions, and a flag that was
      already true would answer the second one with the first one's
      gutter. */
  const [diffAt, setDiffAt] = useState(0);

  /** The PDF, as something a viewer will open (工作区批⑤).

      **A `blob:` and not a `data:`.** A data URL carries the whole file
      in the address, which has a length ceiling, and Chrome has blocked
      `data:` navigation inside frames before — a PDF is exactly the
      size of file that finds both. Revoked when it is replaced or the
      view goes, or every file read leaks its own bytes for the life of
      the page.

      **Nothing in `tauri.conf.json` had to change**: its `csp` is
      `null`, so no policy is emitted and a `blob:` frame is not being
      allowed by an exception somebody could tighten away by accident.
      The day a policy is set, `frame-src blob:` is what this needs —
      written here because a JSON file cannot carry the note.

      **That the engine really paints it is measured, not assumed**:
      `npm run wk` puts a blob PDF in an iframe in a real `WKWebView`,
      snapshots it and counts dark pixels (WebKit gives the page no
      document to query, so the picture is the only witness). Chrome's
      side is `check-face` 4j. */
  const pdfAt = useMemo(() => {
    if (look === null || look.kind !== "pdf" || look.data === "") return null;
    const raw = atob(look.data);
    const bytes = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i += 1) bytes[i] = raw.charCodeAt(i);
    return URL.createObjectURL(new Blob([bytes], { type: look.media }));
  }, [look]);
  useEffect(() => {
    if (pdfAt === null) return;
    return () => URL.revokeObjectURL(pdfAt);
  }, [pdfAt]);

  // **Asked first and alone: what is this.** The node answers with the
  // content, so a file that turns out to be a picture never has its
  // bytes decoded as text on the way (`khor_gui_core::files::preview`).
  useEffect(() => {
    let live = true;
    setLook(null);
    setLookError(null);
    if (kindOf(path) === "text") return;
    fetchPreview(machine, path)
      .then((p) => live && setLook(p))
      .catch((e: unknown) => live && setLookError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
  }, [machine, path, reread]);

  useEffect(() => {
    let live = true;
    if (kindOf(path) !== "text") {
      // Nothing to decode as text, and asking would spend a slice on a
      // file this view is not going to paint as code.
      setFile(null);
      setEdit(null);
      return;
    }
    setFile(null);
    if (draft === undefined) setEdit(null);
    setSaved(null);
    setError(null);
    setTook(null);
    fetchFileText(machine, path)
      .then((f) => {
        if (!live) return;
        setFile(f);
        // **Only when nobody is holding one.** A draft means somebody
        // typed here and walked away; overwriting it with what is on
        // disk is the loss this whole arrangement exists to stop.
        if (draft === undefined) setEdit({ text: f.text, was: f.digest });
      })
      .catch((e: unknown) => live && setError(String(e)));
    // Cancelled by ignoring the answer, not by aborting the dial: the
    // slice is already crossing and dropping it on the floor costs the
    // far machine nothing it has not already spent.
    return () => {
      live = false;
    };
  }, [machine, path]);

  // Asked beside the text rather than after it: the two answers are
  // about the same file at the same moment, and waiting for the first
  // would make the gutter arrive late enough to be seen arriving.
  useEffect(() => {
    let live = true;
    setDiff(null);
    setShown(null);
    fetchGitDiff(machine, path)
      .then((d) => live && setDiff(d))
      // **Swallowed on purpose** (第十四轮 第 2 条): a diff khor could
      // not read leaves the gutter empty, which is what a file with no
      // changes looks like too. The alternative was a sentence under
      // the tab strip, and that is the thing the user struck out.
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [machine, path, diffAt]);

  // **The one thing the vanished rows still had to say**, handed to
  // the tab that owns this file so it can wear it as a hover title
  // (第十四轮 第 2 条). In an effect rather than during the render,
  // because it is a report out of this component and a parent's state
  // may not be set while a child is rendering. Cleared on the way out,
  // or a tab that stopped being the front one would keep whispering
  // about a diff nobody is looking at.
  useEffect(() => {
    onNote?.(diff?.more === true ? cli.git_more : null);
    return () => onNote?.(null);
  }, [onNote, diff?.more]);

  /** What the view is showing and would write back. Falls back to the
      file's own text so the read-only paths need no second branch.
      **Not named `shown`** — that one is the open hunk's index, and the
      two collided here once already. */
  const body = edit?.text ?? file?.text ?? "";
  /** Editable exactly when the far machine handed over a digest, which
      it does exactly when the text is the whole file (`files::shape`).
      One fact, one place. */
  const canEdit = (edit?.was ?? "") !== "";
  const dirty = file !== null && edit !== null && edit.text !== file.text;

  // **「a draft」means「the buffer differs from the disk」, and it is
  // reported from one place** (工作区批⑧). Until this batch every
  // `setEdit` handed the buffer up, including the one that *loads* the
  // file and the one that re-pins it after a save — so the pane counted
  // a freshly opened file as unsaved, and a save could never take the
  // count back down. Nobody noticed while the only reader was a line in
  // a head saying 「N 个文件未存」; the tab's dot is a mark per file,
  // and a dot that never goes out is the kind of wrong somebody stares
  // at (「保存成功了,圆点消失」).
  //
  // Derived here rather than spelled at each writer because the two
  // halves move on different beats: a keystroke replaces the buffer, a
  // save replaces khor's reading of the file, and a rule written at
  // five call sites is a rule that will disagree with itself.
  //
  // **Silent until the file has arrived.** On a remount the pane is
  // already holding the draft and `file` is null for a beat; reporting
  // then would throw away what somebody typed before the read lands.
  useEffect(() => {
    if (onDraft === undefined || file === null || edit === null) return;
    onDraft(edit.text === file.text ? null : edit);
  }, [edit, file, onDraft]);

  const lang = languageOfPath(path);
  // **Above the three early returns on purpose.** A hook after a
  // conditional `return` is a hook that runs on some renders and not
  // others, which React answers by throwing — and it throws at the
  // root, so the whole app went blank rather than this pane
  // (2026-09-12, found by opening it).
  //
  // **The paint is deferred while typing, and the reason is measured.**
  // On this repo's largest editable file (`crates/node/src/link.rs`,
  // 174 KB, 3773 lines), painting it costs 24 ms and putting the
  // resulting 6383 spans into the document and laying them out costs
  // another 40 — **about two thirds of what a keystroke would cost**,
  // for a decoration. The truth is in the textarea either way, so the
  // decoration is the right thing to make wait.
  //
  // With it deferred, one keystroke measured end to end in the app
  // (dispatch plus forced layout, milliseconds on this machine):
  //
  // | file | bytes | ms |
  // |---|---|---|
  // | `ipc.rs` | 9.7 K | 4–13 |
  // | `files.rs` | 15.9 K | 7–12 |
  // | `proto.rs` | 61.6 K | 19–29 |
  // | `link.rs` | 173.6 K | 31–53 |
  //
  // About a quarter of a millisecond per kilobyte, so the median file
  // in a checkout (6.9 KB, measured in 工作区批2) is under 10 and the
  // largest one that can be edited at all is under 70. **No size
  // ceiling is needed and none is imposed** — which is a conclusion
  // from the table, not a hope.
  //
  // ⚠️ **Those numbers are synchronous on purpose.** The first three
  // attempts at them used `setTimeout` and `requestAnimationFrame` and
  // reported a frozen renderer; a background tab serves no animation
  // frames at all and clamps timers to about a second, so what was
  // being measured was Chrome's throttle. Three files in a row came
  // back at exactly 1000 ms, which is what gave it away.
  //
  // While it is waiting the `<pre>` shows the plain text, never the
  // last painting: a stale painting is the wrong letters under the
  // caret, which is worse than no colour by a long way.
  const [painted, setPainted] = useState<{ text: string; lines: string[] | null } | null>(null);
  useEffect(() => {
    if (lang === null) {
      setPainted(null);
      return;
    }
    // No wait at all on a file that is only being read — a new file
    // must not flash plain before it colours. The wait exists for the
    // burst of keystrokes, so it starts when the burst does.
    // Asynchronous since Shiki (2026-09-15): the first file in a
    // language waits for its grammar. `live` drops a paint whose text is
    // no longer the buffer's, so a slow grammar cannot land old paint
    // under new keystrokes.
    let live = true;
    const id = setTimeout(() => {
      void paintFileLines(body, lang).then((lines) => live && setPainted({ text: body, lines }));
    }, dirty ? PAINT_IDLE : 0);
    return () => {
      live = false;
      clearTimeout(id);
    };
  }, [body, lang, dirty]);


  /** The file, one entry per source line. **The text's own split is the
      spine, not the painter's** (工作区批⑨): the numbers, the gutter and
      the editor all count lines the way `\n` does, and a painting that
      disagreed would move every one of them. `paintFileLines` refuses to
      hand back a different count for exactly that reason. */
  const rows = useMemo(() => (body.length === 0 ? [] : body.split("\n")), [body]);
  /** Where each source line starts, in pixels down from the sheet's own
      top, plus one last entry for the bottom of the last line — so a
      hunk covering lines `at`…`at+n` is `tops[at-1]` to `tops[at-1+n]`
      with no special case at the end.
   *
   * **Measured, because the file wraps now** (工作区批⑨). Until this
   * batch every one of these was arithmetic: line `n` began at
   * `line-height × n`, which is exact for a `<pre>` that never wraps
   * and simply wrong for one that does — 「文件显示,要随屏幕宽度自动
   * 换行」(the user, 2026-09-15). A long line now occupies three rows
   * and everything below it has moved three rows down; no formula this
   * side of laying the text out knows by how much.
   *
   * **Empty until the first layout**, and the numbers and the bars draw
   * nothing rather than guess: a bar at a line it is not about is worse
   * than a bar that arrives a frame late. */
  const [tops, setTops] = useState<number[]>([]);
  /** How tall the whole file is inside its scroller, and how far down
      that scroller the first line begins — the two numbers the overview
      ruler needs to squeeze the file onto one screen (2026-09-16).
   *
   * **Measured beside `tops` rather than computed from it.** The last
   * entry of `tops` is where the text ends, which is not where the
   * scroll ends: the box has padding of its own, and the day it grows a
   * footer the ruler would put every mark slightly too low. `pad` is
   * scroll-invariant on purpose — it is read as「sheet top minus box
   * top plus how far the box is scrolled」, so scrolling does not have
   * to re-measure anything. */
  const [span, setSpan] = useState<{ total: number; pad: number }>({ total: 0, pad: 0 });
  /** How tall the open peek block is, or 0 when none is open.
   *
   * **What it is for changed in 工作区第五轮.** It used to be the hole
   * the numbers column had to leave, because the peek sat in the sheet's
   * flow and pushed every line below it down. Out of the flow nothing
   * moves and the numbers need nothing — what needs this is the block
   * itself, which hangs *upward* off the top edge of its line and
   * therefore has to know its own height to know where to start.
   *
   * **Measured on the block and not on the sheet.** An absolutely
   * positioned child does not change its parent's height, so the
   * observer watching the sheet never fires when this opens: it has to
   * be watched itself. Zero for one frame while it is, which puts it
   * level with the line and then moves it up. */
  const [peekH, setPeekH] = useState(0);
  /** The painted sheet, so its lines can be measured. */
  const sheet = useRef<HTMLPreElement>(null);
  useLayoutEffect(() => {
    const el = sheet.current;
    const pk = el?.querySelector<HTMLElement>("[data-peek]") ?? null;
    if (pk === null) {
      setPeekH((was) => (was === 0 ? was : 0));
      return;
    }
    const read = () => {
      const h = pk.getBoundingClientRect().height;
      setPeekH((was) => (was === h ? was : h));
    };
    read();
    const ro = new ResizeObserver(read);
    ro.observe(pk);
    return () => ro.disconnect();
    // `shown` opens and closes it; the rest are what re-lay the block
    // it is measuring. `peekAt` rides `shown`, so it is not repeated.
  }, [shown, body, painted, face]);

  // **The old text is brought onto the screen when it opens**
  // (2026-09-16:「点击后,窗口应该移动到这个状态条显示内容的地方。现在
  // 如果在一个很长的状态条点击,看不见东西,需要拖动状态条最上方才能看
  // 见」). The peek lies *above* its hunk, so a press low on a tall bar
  // opens something off the top of the scroller. Arithmetic on rects,
  // for `box`'s reason below — never `scrollIntoView`. Only when it is
  // actually out of view: a peek already on screen must not jump.
  useLayoutEffect(() => {
    if (shown === null) return;
    const el = box.current;
    const peek = el?.querySelector<HTMLElement>("[data-peek]");
    if (!el || !peek) return;
    const b = el.getBoundingClientRect();
    const p = peek.getBoundingClientRect();
    // **Its top edge to the scroller's top edge, every time**
    // (2026-09-16:「点击git状态bar的时候, 如果从文件第一行开始就有此bar,
    // 跳转到的是文件的第一行,但仍看不清提示. 还得鼠标拖动. 应该直接跳转
    // 到提示框的最上方」— the user).
    //
    // It used to scroll **only when the block was out of view**, and to
    // a third of the way down. Both halves are what the user hit: a
    // hunk at line 1 lays its old text *above* line 1, so scrolling to
    // the line leaves the block off the top; and a block taller than
    // the viewport is partly in view by any test that asks about its
    // top edge, so 「it is already visible」 was true of a panel whose
    // first line was above the screen. Asking nothing and always
    // landing the same way is also the shorter rule to hold: press a
    // bar, the old text is at the top.
    //
    // `--gap-row` of air above it, so the block does not start flush
    // against the column's edge — the same gap the rows either side of
    // it are spaced by.
    const air = Number.parseFloat(getComputedStyle(el).getPropertyValue("--gap-row")) || 0;
    el.scrollTop += p.top - b.top - air;
    // **`peekH` is in here on purpose** (工作区第五轮): the block hangs
    // upward off its line, so on the frame it opens it has not been
    // measured yet and is sitting level with the line. Landing once on
    // that would put it a block's height too low; this runs again the
    // moment the height is known, and then settles.
  }, [shown, peekH]);
  // **A layout effect and a `ResizeObserver`, both.** The effect catches
  // the text changing; the observer catches the column changing width,
  // which is the whole point of wrapping and which no render here is
  // told about (the seam is dragged one component up, and the window is
  // not a prop at all).
  /** The last measurement, for the anchor below. A ref, not the state:
      the observer's callback closes over the render it was made in and
      would read stale tops. */
  const lastTops = useRef<{ tops: number[]; pad: number } | null>(null);
  useLayoutEffect(() => {
    const el = sheet.current;
    if (el === null) return;
    const read = () => {
      const seen = el.querySelectorAll<HTMLElement>("[data-line]");
      // **Rects, not `offsetTop`.** A line sits at a multiple of 21.6
      // and `offsetTop` is rounded to a whole pixel, so the numbers
      // column — whose blocks are the differences between neighbours —
      // drifted up to half a pixel away from the text it names. The
      // gate measures that difference at 0.5, which is exactly where
      // rounded arithmetic lands.
      const base = el.getBoundingClientRect().top;
      const next: number[] = [];
      for (const row of seen) next.push(row.getBoundingClientRect().top - base);
      const last = seen[seen.length - 1];
      next.push(last === undefined ? 0 : last.getBoundingClientRect().bottom - base);
      // **Nothing is written when nothing moved.** The observer fires on
      // every layout that touches this box, including the one this
      // state change causes — a new array every time is a render loop
      // with a repaint in it.
      setTops((was) =>
        was.length === next.length && was.every((v, i) => v === next[i]) ? was : next,
      );
      const b = box.current;
      if (b !== null) {
        const total = b.scrollHeight;
        const pad = el.getBoundingClientRect().top - b.getBoundingClientRect().top + b.scrollTop;
        setSpan((was) => (was.total === total && was.pad === pad ? was : { total, pad }));
        // **The line at the top stays at the top when the lines re-wrap**
        // (2026-09-16:「调整窗口宽度自动换行的时候,应该保持最上方一行,
        // 稳定视觉一致性」). A narrower column folds every long line
        // above the viewport into more rows, and the same scrollTop
        // then lands somewhere further down the file; the person was
        // reading line 40 and is now looking at 31. So the source line
        // whose top was at the scroller's edge is found in the old
        // measurement and put back at the edge in the new one, offset
        // within the line kept as far as the line still reaches.
        // **Only when the line count held** — a keystroke that adds a
        // line is an edit, and the caret's own scroll owns that.
        const prev = lastTops.current;
        if (prev !== null && prev.tops.length === next.length && next.length > 1) {
          const st = b.scrollTop;
          let i = 0;
          while (i + 2 < prev.tops.length && prev.pad + prev.tops[i + 1] <= st + 0.5) i += 1;
          const within = Math.max(0, st - (prev.pad + prev.tops[i]));
          const room = Math.max(0, next[i + 1] - next[i] - 1);
          const want = pad + next[i] + Math.min(within, room);
          if (Math.abs(want - st) > 0.5) b.scrollTop = want;
        }
        lastTops.current = { tops: next, pad };
      }
      // How tall the open peek came out, so the numbers column can wait
      // out exactly as much of itself as the sheet just spent on it.
    };
    read();
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => ro.disconnect();
  }, [body, painted, face]);

  // **Esc shuts the open hunk** (2026-09-16). A window listener rather
  // than a key handler on the bar: the bar is a 12-wide strip nobody
  // focuses, and what Esc is expected to shut is whatever is open on
  // the screen — pressing the bar again is the other way, and it is the
  // one a pointer finds.
  useEffect(() => {
    if (shown === null) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setShown(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [shown]);

  // The ask from outside, and **nothing on the first render**: the
  // value arrives with the mount, and a mount is not an ask. The ref
  // moves with each real one, so bringing a tab to the front and asking
  // it to save is two beats rather than a save on every remount.
  // A line asked for from outside (ruling 35). Waits for the layout —
  // `tops` is empty until the first measurement, and with wrapping there
  // is no arithmetic to fall back on (`Ruler`'s rule) — and puts that
  // line a third of the way down, which is where `goTo` puts a hunk.
  const wentTo = useRef<number>(0);
  useEffect(() => {
    if (goLine === null || goLine.at === wentTo.current) return;
    const el = box.current;
    const top = tops[goLine.line - 1];
    if (el === null || top === undefined) return;
    wentTo.current = goLine.at;
    el.scrollTop = Math.max(0, span.pad + top - el.clientHeight / 3);
  }, [goLine, tops, span]);

  const askedAt = useRef(saveAt);
  useEffect(() => {
    if (saveAt === askedAt.current) return;
    askedAt.current = saveAt;
    save();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [saveAt]);

  // …and the same for the ask to re-read (工作区批⑩). Nothing on the
  // first render for the same reason: the value arrives with the mount,
  // and the mount already read the file.
  const readAsked = useRef(readAt);
  useEffect(() => {
    if (readAt === readAsked.current) return;
    readAsked.current = readAt;
    again();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [readAt]);

  /** Ask the far machine what this file says now, and take the answer
      only where taking it costs nobody anything (工作区批⑩).
   *
   * **A draft stops it dead, before the round trip is even read.**
   * 「有草稿维持『存的时候报冲突』」— the buffer is somebody's work and
   * the disk is somebody else's; khor does not pick between them, and
   * the place that already asks is the save, which has three ways out
   * (`theirs` / `merge` / `mine`). Touching `file` here would be worse
   * than useless: `file.text` is the **base** the merge is computed
   * against, so moving it under a draft would make that merge
   * three-way against a base nobody ever had.
   *
   * **The same digest means nothing happened**, and saying so would be
   * a repaint of an unchanged file plus a fresh diff request.
   *
   * A failure is silent: a machine that could not be reached on the way
   * back into the window is not news about this file, and the next save
   * says its own piece in its own words. */
  const again = () => {
    if (kindOf(path) !== "text" || file === null || dirty) return;
    fetchFileText(machine, path)
      .then((f) => {
        if (f.digest === file.digest) return;
        setFile(f);
        setEdit({ text: f.text, was: f.digest });
        // The gutter is about HEAD and the file just moved underneath
        // it, so it is wrong until it is asked again.
        setDiffAt((n) => n + 1);
      })
      .catch(() => {});
  };

  const save = () => {
    if (edit === null || !canEdit || saving) return;
    setSaving(true);
    setSaved(null);
    const sent = edit.text;
    saveFileText(machine, path, edit.was, sent)
      .then((w) => {
        // **Pinned to what was sent, not to what is in the box now.**
        // Somebody typing during the round trip must keep their
        // keystrokes, and the digest that came back is the digest of
        // `sent` — so the text baseline moves to `sent` and whatever
        // they typed on top of it is still unsaved.
        setFile((f) => (f === null ? f : { ...f, text: sent, digest: w.digest, at_ms: w.at_ms }));
        setEdit((e) => (e === null ? e : { ...e, was: w.digest }));
        setClashes(null);
        setSaved(true);
        // The gutter is about HEAD and the file just moved, so it is
        // wrong until it is asked again. Same for the tree's marks,
        // which this view does not hold.
        setDiffAt((n) => n + 1);
        onSaved?.();
      })
      .catch((e: unknown) => setSaved(String(e instanceof Error ? e.message : e)))
      .finally(() => setSaving(false));
  };

  /** Throw away what is in the box and read the file again. */
  const theirs = () => {
    setSaved(null);
    setClashes(null);
    setEdit(null);
    onDraft?.(null);
    setReread((n) => n + 1);
  };

  /** Keep both sides: merge the file as it is now into the box.
   *
   * **Three texts, none of them written.** `file.text` is what this
   * view read (the base the digest in `edit.was` belongs to), the box
   * is one side, and the machine is asked for the other. What comes
   * back goes in the box pinned to the digest that came with it, so
   * the save that follows is an ordinary one — and it is still the
   * person who presses it.
   *
   * Both changes are in that text, and where they touched the same
   * lines both are there between markers. khor does not pick; the
   * count says how many times it did not.
   */
  const merge = () => {
    if (edit === null || file === null || saving) return;
    setSaving(true);
    setSaved(null);
    const base = file.text;
    const mine_ = edit.text;
    fetchFileText(machine, path)
      .then((now) =>
        mergeFileText(base, mine_, now.text).then((m) => {
          // The base moves with the merge: a second refusal has to be
          // measured against what this merge started from, not against
          // the text the view first opened.
          setFile((f) => (f === null ? f : { ...f, text: now.text, digest: now.digest, at_ms: now.at_ms }));
          setEdit((e) => (e === null ? e : { text: m.text, was: now.digest }));
          setClashes(m.clashes);
        }),
      )
      .catch((e: unknown) => setSaved(String(e instanceof Error ? e.message : e)))
      .finally(() => setSaving(false));
  };

  /** Save what is in the box over whatever is there now.
   *
   * **Two steps, and the first one is a read.** The refusal means the
   * digest khor is holding is not the file's any more, so a save pinned
   * to it will be refused again; this asks the far machine what the
   * file is *now* and pins to that. The gap between the two is real and
   * it is the gap the person just chose to accept — the button says so.
   */
  const mine = () => {
    if (edit === null || saving) return;
    setSaving(true);
    setSaved(null);
    const sent = edit.text;
    fetchFileText(machine, path)
      .then((now) => saveFileText(machine, path, now.digest, sent))
      .then((w) => {
        setFile((f) => (f === null ? f : { ...f, text: sent, digest: w.digest, at_ms: w.at_ms }));
        setEdit((e) => (e === null ? e : { ...e, text: sent, was: w.digest }));
        setClashes(null);
        setSaved(true);
        setDiffAt((n) => n + 1);
        onSaved?.();
      })
      .catch((e: unknown) => setSaved(String(e instanceof Error ? e.message : e)))
      .finally(() => setSaving(false));
  };

  const take = () => {
    setTook(null);
    pullFile(machine, path)
      .then((dest) => setTook(gui.took_file(dest)))
      .catch((e: unknown) => setTook(String(e)));
  };

  // ── the three screens that are not a code view ──────────────────
  if (kindOf(path) !== "text") {
    if (lookError !== null) {
      return <div className="p-4 text-aux text-muted-foreground">{lookError}</div>;
    }
    if (look === null) return <div className="flex-1" />;
    if (look.cut) {
      return (
        <Away word={gui.look_too_big(sizeWord(look.total))} onTake={take} took={took} />
      );
    }
    if (look.kind === "image" || look.kind === "svg") {
      // **An `<img>`, including for SVG, and that is the security
      // decision.** SVG is markup and can carry scripts; put in the
      // document it would run them, and khor would owe itself a
      // sanitiser. An image element renders the same drawing and runs
      // nothing — the browser's own rule, not khor's.
      return (
        <div className="flex min-h-0 flex-1 items-center justify-center overflow-auto p-4">
          <img
            data-look={look.kind}
            src={`data:${look.media};base64,${look.data}`}
            alt=""
            className="max-h-full max-w-full object-contain"
          />
        </div>
      );
    }
    if (look.kind === "pdf" && pdfAt !== null) {
      // **The file itself, drawn by the browser** (「pdf 你咋转成字符
      // 了,应该直接预览」— the user, 2026-09-14). An `<iframe>` rather
      // than `<object>`: both reach the same viewer, and a frame is the
      // one of the two whose failure mode is a blank rectangle rather
      // than a silent fallback to nothing.
      return (
        <div className="flex min-h-0 flex-1 flex-col">
          <iframe
            data-pdf-frame
            src={pdfAt}
            title={path.split("/").pop() ?? path}
            className="min-h-0 flex-1 border-0"
          />
        </div>
      );
    }
    if (look.kind === "doc" || look.kind === "pdf") {
      return (
        <div className="flex min-h-0 flex-1 flex-col">
          {look.empty ? (
            <div className="flex flex-1 items-center justify-center p-6 text-fact text-foreground">
              {gui.doc_empty}
            </div>
          ) : (
            <div className="min-h-0 flex-1 overflow-auto">
              {/* Safe because the node escaped it: every raw HTML event
                  in the source became text before this string existed
                  (`khor_node::preview::html`, asserted by its own
                  test). Nothing here re-sanitises, because a second
                  guard is a second thing to be wrong. */}
              <article
                data-look="doc"
                className="prose-khor px-6 py-4"
                dangerouslySetInnerHTML={{ __html: look.html }}
              />
            </div>
          )}
          {/* Said once, under the document rather than over it: it is a
              caveat about what you are reading, not a warning.

              **A PDF that got here says something else**: it is not on
              the words road because khor reads words, it is on it
              because this browser draws no PDFs
              (`khor_node::preview::PDF_HERE`), and that is a different
              sentence about a different reason. */}
          <div
            data-doc-note={look.kind}
            className="flex-none border-t px-3 py-1.5 text-aux text-muted-foreground"
          >
            {look.kind === "pdf" ? gui.pdf_as_words : gui.doc_plain}
          </div>
        </div>
      );
    }
    return <Away word={gui.file_not_text} onTake={take} took={took} />;
  }

  if (error !== null) {
    return <div className="p-4 text-aux text-muted-foreground">{error}</div>;
  }
  if (file === null) return <div className="flex-1" />;

  if (file.binary) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
        <span className="text-fact text-foreground">{gui.file_not_text}</span>
        <span className="text-aux text-muted-foreground tabular-nums">
          {sizeWord(file.total)}
        </span>
        <Button variant="outline" size="sm" onClick={take} className="mt-1">
          {gui.file_take}
        </Button>
        {took !== null && <span className="text-aux text-muted-foreground">{took}</span>}
      </div>
    );
  }

  const paint = painted !== null && painted.text === body ? painted.lines : null;

  // **Nothing is drawn about HEAD while the buffer differs from disk.**
  // The bars name line numbers and one typed line moves every line
  // under it, so a bar left where it was would point at code it is not
  // about — wrong, not merely old. The alternative is a round trip per
  // keystroke onto somebody else's machine, which khor will not do.
  const hunks = dirty ? [] : (diff?.hunks ?? []);
  /** Whether the gutter has gone quiet because of an edit, as opposed
      to because the file matches HEAD. Only then is there something to
      say. */
  const stale = dirty && (diff?.hunks.length ?? 0) > 0;
  /** Put a hunk on screen, from the overview ruler.
   *
   * **It scrolls and opens nothing** (2026-09-16). The ruler is the map
   * — 「点击后……滚过去」 is the whole of what a mark on it promises, and
   * opening the old text as well would move the lines under the very
   * place it just scrolled to. Reading what HEAD had is the gutter
   * bar's press, one column over from the line it is about.
   *
   * **Asked of the bar, not computed from the line height.** The bar is
   * already sitting where CSS put it, so its `offsetTop` is the answer
   * by construction — a second arithmetic here would be a copy of
   * `--text-code--line-height` that nothing keeps in step, and the day
   * the token moved one of the two would be wrong. Found **by the line
   * it names**, not by position in the list: a hunk whose lines this
   * buffer no longer has draws no bar at all, and counting would then
   * scroll to somebody else's. */
  /** Where a press on the bare overview strip lands: that fraction of
      the file, **centred in the viewport** rather than put at the top.
      The strip is the file end to end, so the place pressed is the place
      meant, and the half-screen either side of it is the context that
      makes it readable (`Ruler`'s `onGoAt`). */
  const goRatio = (ratio: number) => {
    const el = box.current;
    if (el === null) return;
    const want = ratio * el.scrollHeight - el.clientHeight / 2;
    el.scrollTop = Math.max(0, Math.min(el.scrollHeight - el.clientHeight, want));
  };

  const goTo = (h: GitHunk) => {
    const el = box.current;
    const bar = el?.querySelector<HTMLElement>(`[data-hunk="${h.at}"]`);
    if (el && bar) {
      // A third of the way down rather than at the top: a hunk pinned
      // to the ceiling is a hunk with no lines above it to compare
      // against, and comparing is the whole reason to go there.
      el.scrollTop = bar.offsetTop - el.clientHeight / 3;
    }
  };

  const renders = rendererOf(path);
  /** The hunk whose old text is being read, laid into the file itself.
      `null` closes it, and so does anything that takes the bars away —
      `hunks` empties the moment somebody types, and a peek about a diff
      that is no longer being drawn would be a panel with no bar. */
  const peek = shown !== null && hunks.length > 0 ? (diff?.hunks[shown] ?? null) : null;
  /** Which line the peek block is laid **above**, 1-based, or `null`.
   *
   * A hunk's removed lines belong directly over the lines that replaced
   * them, so an ordinary change puts them above `at`. **A pure deletion
   * is the exception**: nothing replaced those lines, and where they
   * used to begin is the boundary *below* `at` — the same boundary its
   * notch in the gutter stands on. `rows.length + 1` is the end of the
   * file, which is a real answer (a tail somebody deleted); past that is
   * a diff naming lines this buffer no longer has, and then nothing is
   * drawn, exactly as the gutter draws no bar for it. */
  const peekAt = (() => {
    if (peek === null) return null;
    const at = peek.lines === 0 ? peek.at + 1 : peek.at;
    return at >= 1 && at <= rows.length + 1 ? at : null;
  })();

  return (
    // `relative`: the one notice this view still has floats at the foot
    // of the column rather than standing over the file (see it below).
    //
    // **Named so the gate can measure what is directly under the tab
    // strip** (第十四轮 第 2 条 的门): on an ordinary file the first
    // child of this box is the file itself, and any band of words that
    // came back would be exactly the thing the user struck out.
    <div data-file-view className="relative flex min-h-0 flex-1 flex-col">
      {/* **源码 / 预览 has no strip of its own any more** (2026-09-16):
          the switch is one glyph at the far end of the tab strip, drawn
          by `WorkspacePane` beside the three column switches, and this
          view is handed the answer. 「取消源码/预览的按钮,放在页面的右上
          角那」— the user. Two buttons naming both faces cost a whole
          row of the column to say what a pressed-in icon says in the
          strip that was already there.

          **The preview still renders `body`** — the buffer, half-typed
          and unsaved — never the file on disk. That is the whole reason
          this is a face and not a kind: a rendered view lagging the
          text by one save would be showing a document nobody has. */}
      {/* **The strip that counted the hunks is gone** (2026-09-16):
          「取消 git 文件上方的状态栏,比如现在的『1 处改动 第 1 处』」—
          the user. What it was for — reaching a hunk eight hundred lines
          down, which the gutter alone cannot do because you cannot press
          something off the bottom of the screen — is the overview ruler
          now, past the scrollbar, where the whole file is one column
          tall and every place is on it at once. A count and a 第 N 处
          were the words that stood in for a picture nobody had drawn
          yet. */}
      {/* **And so is the row that said 存好了 / 未存 / 回到存之前**
          (2026-09-16:「修改文件不要显示"存好了"和"回到存之前", 类似要在
          文件上方新增一列的东西都不要」— the user). Every one of those
          three had a place that already said it — the tab's own dot,
          ⌘S, the dot going out — and what the row cost was paid on the
          keystroke: it appeared the moment somebody typed, and the line
          they were typing on moved down by the height of a strip.

          What is left is the refusal, and it is the one thing on this
          screen a person **must** act on (a save khor would not make,
          and what a merge left behind). So it stays — at the foot of
          the column, floating over the file rather than pushing it
          (`data-file-notice` below). */}
      {file.cut && (
        <div className="flex items-center gap-3 border-b px-3 py-1.5 text-aux text-muted-foreground">
          <span className="tabular-nums">
            {gui.file_cut(sizeWord(file.text.length), sizeWord(file.total))}
          </span>
          {/* Beside the cut line rather than as a strip of its own: it
              is the same fact twice, and「只读了开头」has already told
              the half that explains this one. */}
          <span data-read-only>{gui.file_read_only}</span>
          <Button variant="outline" size="sm" onClick={take} className="ml-auto">
            {gui.file_take}
          </Button>
          {/* Where it landed, **on the row whose button sent it** — a
              press with no answer is a press nobody can check, and this
              row is already here and already about the same file
              (第十四轮 第 2 条 took a row of its own away, not the
              report). */}
          {took !== null && <span className="flex-none">{took}</span>}
        </div>
      )}
      {/* ⚠️ **Tombstone: the three sentences under the tab strip**
          (第十四轮 第 2 条, 2026-09-24:「工作区打开某个git目录显示了
          "xxx does not appear to be a git repository"…此外, 这一行不要
          显示任何内容. 也就是任何标签页下方都不要有副标题」— the user).

          They were git's refusal, 「the file landed in …」 and 「too many
          changes to draw」: three explanations of why the gutter is not
          showing what it might have. The refusal was the one that gave
          the rule away — it was never git's judgment about the
          repository (第十四轮 第 4 条: a deleted working directory), and
          it printed a lie one row under the tab on a screen whose whole
          job is the file.

          **Unreadable is drawn as no bars**, which is the rule the
          workspace's own head already follows for a directory that is
          not a checkout (工作区批⑦): khor says nothing about HEAD rather
          than saying something wrong about it. What is still worth
          knowing — a diff that stopped short — rides the tab's hover
          title (`onNote` below), where it costs no row. Where the file
          landed is reported by the row whose button did it (the cut row
          above, and the 取走 cards). */}
      {/* Three columns that scroll as one, **and every line is a block
          of its own** (工作区批⑨).

          It used to be two `<pre>`s each holding one string, because a
          painted block's spans cross line ends and splitting the markup
          by「\n」would cut them in half. Shiki hands the lines over
          already separated (`paintFileLines`), so that cost is gone —
          and it had to go: with wrapping on, 「where does line 40
          start」 is a question only the laid-out text can answer, and a
          single box has nothing in it to ask.

          **The gutter is still a column of its own.** It cannot be a
          marker inside either of the other two — one of them has a
          transparent editor over it — so it floats, and its bars are
          placed at the measured tops rather than at a multiple of the
          line height. */}
      {/* **One padding box for all three columns**, rather than `py-2`
          on each. Everything the gutter draws is placed by arithmetic
          from the top of the first line, and a padding of its own would
          put that origin inside its own border box — eight pixels above
          where the text actually starts, on every bar. */}
      {/* **The editor leaves the screen in 预览 and keeps everything.**
          The buffer, the digest the next save is pinned to, the dirty
          mark: all of them are this component's state and none of them
          lives inside the textarea. Switching back finds the text
          exactly as it was — and typing into a preview is not possible,
          because there is nothing there to focus. */}
      {face === "preview" && renders !== null ? (
        <div data-file-preview={renders} className="min-h-0 flex-1 overflow-auto">
          {renders === "md" ? (
            <Markdown text={body} className="prose-khor px-6 py-4" />
          ) : (
            <Table text={body} sep={path.toLowerCase().endsWith(".tsv") ? "\t" : ","} />
          )}
        </div>
      ) : (
      // **The ruler is beside the scroller, not inside it** (2026-09-16,
      //「右侧滑动条后方」). Inside, it would scroll along with the file
      // and stop being a map of the whole of it; outside and to the
      // right, it is past the scrollbar exactly as VS Code's is.
      /* **The ruler lies over the scrollbar's own strip, not beside it**
         (2026-09-16:「文件右侧的 git 状态导航条是在滑动条的下方的,可以
         透过滑动条颜色浮上来」). The first cut put it past the scrollbar
         as a column of its own, 14 wide with a border — a second bar
         next to the bar. VS Code paints its marks *inside* the
         scrollbar track under a translucent slider, so the strip is
         one strip. Here the ruler is laid absolutely over the
         scroller's right edge, the track is already transparent
         (`app.css`), and the thumb over this scroller is thinned to
         a wash so a mark under it still reads (`[data-file-scroll]`
         in app.css). The wrapper is `relative` for that alone. */
      <div className="relative flex min-h-0 flex-1">
      <div
        ref={box}
        // Named so a gate can ask **this** box whether anything scrolls
        // sideways (工作区批⑨). The ruling is that a file wraps, and a
        // page that does not scroll while its inner scroller does is the
        // same file off the right edge with one more thing in the way.
        data-file-scroll
        // **The engine's own scroll anchoring is off here** — this view
        // anchors the top line itself when the sheet re-wraps (`read()`
        // above), and Blink's anchoring on top of that is two hands on
        // one wheel; WebKit, which the desktop app runs in, has none,
        // which is why the ruling was made at all.
        className="min-h-0 flex-1 overflow-auto [overflow-anchor:none]"
      >
        <div className="flex min-w-full items-start py-2">
          {/* **One block per number, as tall as the line it names**
              (工作区批⑨). A joined string was exact while every line was
              one row tall; with wrapping it would put 40 beside line 43.
              Each block takes the measured height of its line, so the
              digits land on that line's **first** visual row and the
              next number waits out however many rows the wrap took —
             「不做换行开关,我们显示行号的作用就是这个啊」(the user,
              2026-09-15). */}
          <pre
            data-numbers
            aria-hidden="true"
            // **The divider is this column's right edge, and the change
            // bar stands outside it, on the text's side** (2026-09-16:
            // 「状态条收进了行号与文字分割线的内侧了,分割线到文字部分空了
            // 一截…要么让文字往左不要空,要么状态条放在分割线外侧」). The
            // bar used to carry the divider on its own right, which left
            // the sheet's padding as a blank gap past the line; now the
            // bar sits in that gap, VS Code's arrangement.
            className="shrink-0 select-none border-r pl-2 pr-1 text-right font-mono text-code leading-code text-muted-foreground tabular-nums"
          >
            {/* No spacer for the peek any more: it floats over the
                text rather than taking a row, so the numbering beside
                it never moves (工作区第五轮, `Peek`). */}
            {rows.map((_, i) => (
              <div
                key={i}
                data-num={i + 1}
                style={tops.length > i + 1 ? { height: tops[i + 1] - tops[i] } : undefined}
              >
                {i + 1}
              </div>
            ))}
          </pre>
          <Gutter
            hunks={hunks}
            shown={shown}
            tops={tops}
            onShow={(i) => setShown(shown === i ? null : i)}
          />
          {/* **`min-w-0`, where it used to be `min-w-max`.** That was
              the column growing to the longest line and the file
              scrolling sideways; the ruling is that it wraps instead,
              so the column takes the width it is given and the text
              folds into it. */}
          <div className="relative min-w-0 flex-1">
            <Sheet
              ref={sheet}
              rows={rows}
              text={body}
              paint={paint}
              peek={peek}
              peekAt={peekAt}
              peekTop={peekAt === null ? 0 : (tops[peekAt - 1] ?? 0)}
              peekNext={peekAt === null ? 0 : (tops[peekAt] ?? tops[peekAt - 1] ?? 0)}
              peekH={peekH}
              // **No editor while a hunk is open, and that is the price
              // of 「就地显示」** (2026-09-16). The old text goes into
              // the sheet's own flow, and the editor over it is a
              // `<textarea>` — one box holding one string, with nowhere
              // to put a block. Anything inserted on one side and not
              // the other is the caret a row off its letter from there
              // down, which is the one failure this arrangement may not
              // have (`check-face` 4c).
              //
              // **It costs almost nothing, and that is measurable rather
              // than hoped**: bars are drawn only when the buffer
              // matches the disk (`hunks`), so a file with a draft in it
              // has no bar to press in the first place. What is given up
              // is starting to type *while* looking at what HEAD had —
              // one Esc away, and the buffer is untouched throughout.
              edit={canEdit}
              onText={(t) => {
                setClashes(null);
                setEdit((e) => (e === null ? e : { ...e, text: t }));
              }}
              onSave={save}
            />
          </div>
        </div>
      </div>
      <Ruler
        hunks={hunks}
        tops={tops}
        span={span}
        shown={shown}
        onGo={goTo}
        onGoAt={goRatio}
      />
      </div>
      )}
      {/* **The one notice left, and it floats** (2026-09-16:「类似要在
          文件上方新增一列的东西都不要」— the user).

          Three things can land in it and all three are about a save
          that did not simply happen: khor refused to write (the far
          copy moved under the buffer), what a merge left behind, and
          the gutter having gone quiet because the box no longer matches
          the disk. The first of those is the only place in this column
          where somebody **has** to act, which is why it is still drawn
          at all — 「没有就不显示」 covers the rest of the day.

          **`absolute`, at the foot, over the text.** Anything in the
          flow above the file moves the line a person is typing on the
          moment it appears; this covers the last row or two instead,
          and the file underneath is still where it was. It is at the
          foot rather than the head because the head is where the caret
          usually is — a notice there would cover the thing it is about.

          **It goes when the reason goes**, which is already how all
          three of its facts behave: `clashes` clears on the next
          keystroke and on the save, `saved` clears when the next save
          starts, and `stale` is false the moment the box matches the
          disk again. */}
      {(typeof saved === "string" || clashes !== null || stale) && (
        <div
          data-file-notice
          // `pointer-events-none` on the frame and back on inside it:
          // the strip spans the column, and the part of it that is not
          // the card must not eat presses meant for the code under it.
          className="pointer-events-none absolute inset-x-0 bottom-0 z-10 flex justify-center p-2"
        >
          <div className="pointer-events-auto flex max-w-full flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-1.5 text-aux text-muted-foreground shadow-md">
            {typeof saved === "string" && <span data-save-state="refused">{saved}</span>}
            {clashes !== null && (
              <span data-merged={clashes}>
                {clashes === 0 ? gui.file_merged_clean : gui.file_merged_clashes(clashes)}
              </span>
            )}
            {/* The gutter going quiet, said here rather than as a row of
                its own: a file with no bars reads as a file that matches
                HEAD, and that is exactly the claim khor cannot make
                while there is something typed in the box. */}
            {stale && typeof saved !== "string" && <span data-git-stale>{gui.git_stale}</span>}
            {/* **Three ways out of a refusal, where there used to be
                none** (2026-09-13). A save khor would not make left the
                sentence 「这个文件在你编辑期间变了」 standing and nothing
                to press — the person's only move was to copy their text
                somewhere else by hand. Two of these throw one side away
                and say so on the button; the third keeps both and is the
                one a person usually wants, which is why it sits between
                them rather than at the end. */}
            {typeof saved === "string" && (
              <span className="flex shrink-0 gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  data-take-theirs
                  disabled={saving}
                  onClick={theirs}
                  className="min-h-touch min-w-touch"
                >
                  {gui.file_take_theirs}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  data-merge
                  disabled={saving}
                  onClick={merge}
                  className="min-h-touch min-w-touch"
                >
                  {gui.file_merge}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  data-keep-mine
                  disabled={saving}
                  onClick={mine}
                  className="min-h-touch min-w-touch"
                >
                  {gui.file_keep_mine}
                </Button>
              </span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

/** Delimited text as a table (工作区批⑤).
 *
 * **No header row**, which is the judgment the node's table already
 * made before it moved here: a csv's first line is a header often and
 * data sometimes, and khor cannot tell. Every row is drawn the same and
 * the file reads the way it is.
 *
 * **No cap either.** The node's table cut at 2000 rows because it was
 * building a string to put on a wire; this is built from the buffer
 * already on screen, which was capped when the file was read
 * (`files::shape`) and says so in its own line. A second cut would be a
 * second sentence about one cut. */
function Table({ text, sep }: { text: string; sep: string }) {
  const rows = useMemo(() => parseDelimited(text, sep), [text, sep]);
  return (
    <table data-table className="w-full border-collapse text-code">
      <tbody>
        {rows.map((row, i) => (
          <tr key={i} className="border-b">
            {row.map((cell, j) => (
              <td key={j} data-cell className="border-r px-2 py-1 align-top font-mono">
                {cell}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** How long a keystroke has to be the last one before the colours come
    back, in milliseconds. Short enough that a pause between words
    repaints, long enough that a burst of typing repaints once.
    **Not a token**: tokens.css holds lengths, and this is a duration
    nothing else in the app shares. */
const PAINT_IDLE = 120;

/** The one rule that makes the caret land on its letter: the painted
    copy and the editor over it wrap at the same places.

    **Set once and spread onto both**, because the pair is the whole
    trick and a class edited on one of them is the drift `check-face`
    goes red about. `anywhere` rather than `break-word`: a 400-character
    path with no space in it has to fold somewhere, and the alternative
    is the sideways scroll the ruling took away. */
const WRAP = "font-mono text-code leading-code whitespace-pre-wrap [overflow-wrap:anywhere]";

/**
 * The file's one surface: the painted text, with a transparent editor
 * laid exactly over it when the file may be written back.
 *
 * **Why a textarea over a `<pre>` and not an editor component.** The
 * two have to agree character for character, and they do because they
 * are the same string in the same font, size, line height, padding and
 * wrapping — a browser laying out one lays out the other identically.
 * That is the whole trick, and it is also the whole of its fragility:
 * **every typographic property here must be set on both (`WRAP`), and
 * `check-face` measures the caret against the ink to catch the day one
 * of them drifts.**
 *
 * **What it does not buy, said where it is spent:** no bracket
 * matching, no multiple cursors, no folding, and no completion — the
 * last of which is not a cost at all (「不提供任何自动补全什么的」). A
 * person who wants those opens the file in VS Code, which is the
 * batch after this one.
 *
 * **One block per line, and the newlines are in the editor only**
 * (工作区批⑨). The `<pre>` draws each line as a `<div>`: a block box
 * breaks where a `\n` would, so the two lay out the same, and the
 * numbers and the gutter get one element per line to measure. An empty
 * line has no content to give it height, which is what the `min-h`
 * is for — without it a blank line collapses and everything below it
 * climbs one row.
 *
 * **The `<pre>` keeps the text even under the editor.** It would be
 * cheaper to leave it empty and let the textarea show through, but then
 * the column would have no height of its own and the line tops, the
 * gutter and the numbers would all be measuring a box with nothing in
 * it.
 */
function Sheet({
  ref,
  rows,
  text,
  paint,
  peek,
  peekAt,
  peekTop,
  peekNext,
  peekH,
  edit,
  onText,
  onSave,
}: {
  ref: React.Ref<HTMLPreElement>;
  /** The file split on 「\n」 — the spine every column counts by. */
  rows: string[];
  /** The same text unsplit, which is what the editor holds. */
  text: string;
  /** The painted markup, one entry per line, or null for a language
      this build does not carry — then the `<pre>` shows plain text. */
  paint: string[] | null;
  /** The hunk being read, or null. */
  peek: GitHunk | null;
  /** Which line it is laid above, 1-based, or null for「nothing open」
      (`FileView`'s `peekAt` says how the number is chosen). */
  peekAt: number | null;
  /** That line's own offset from the top of this `<pre>`, measured —
      the peek hangs off it rather than taking a row (`Peek`). */
  peekTop: number;
  /** The offset of the line after it, for the case where the block will
      not fit above (`Peek`). */
  peekNext: number;
  /** How tall the peek measured last paint, so it can be hung *above*
      the line. Zero on the first paint, which puts it level with the
      line for one frame and then it moves up. */
  peekH: number;
  /** Whether to lay an editor over it at all. **A file that cannot be
      written back grows no textarea**, rather than a disabled one: a
      caret that appears and then refuses every keystroke is worse than
      no caret, and the strip above already says why this one is only
      to look at. */
  edit: boolean;
  onText: (text: string) => void;
  onSave: () => void;
}) {
  return (
    <>
      <pre
        ref={ref}
        // Named so a gate can find *this* `<pre>` and not the line
        // numbers beside it. The first probe for「does the expanded
        // panel cover any code」 fell back to `document.querySelector
        // ("pre")`, which is the number column — and reported no
        // overlap against the old overlay too, so the assertion passed
        // on the behaviour it was written to forbid (2026-09-13).
        data-sheet
        // Hidden from a screen reader when the textarea is there: the
        // two hold the same words, and read twice they are gibberish.
        aria-hidden={edit || undefined}
        // `relative` so the peek can hang off a line's top edge without
        // taking a row of its own (`Peek`).
        className={cn("relative px-3", WRAP)}
      >
        {rows.map((line, i) => (
          <div
            key={i}
            data-line={i + 1}
            className="min-h-[var(--text-code--line-height)]"
          >
            {paint === null ? (
              line
            ) : (
              // Safe for `paint`'s reason: Shiki escapes every
              // character of the input before it wraps any of it, so
              // this is the file's own text plus spans this build
              // generated (`lib/highlight`'s `line`).
              <span dangerouslySetInnerHTML={{ __html: paint[i] }} />
            )}
          </div>
        ))}
        {peek !== null && peekAt !== null && (
          <Peek hunk={peek} at={peekTop} next={peekNext} tall={peekH} />
        )}
      </pre>
      {edit && (
        <textarea
          data-edit
          value={text}
          onChange={(e) => onText(e.target.value)}
          onKeyDown={(e) => {
            // The one shortcut. `metaKey` and `ctrlKey` both, because
            // this face runs on a mac app, a browser on a phone and a
            // Windows desktop from one build.
            if ((e.metaKey || e.ctrlKey) && e.key === "s") {
              e.preventDefault();
              onSave();
            }
          }}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          // **`soft`, which is the attribute half of `WRAP`** (工作区
          // 批⑨). It was `off` until this batch, and the reason given
          // was that a wrapped editor line would sit beside an unwrapped
          // painted one — true then, because only one of the two
          // wrapped. Both do now, and what `off` would cost instead is
          // an editor that scrolls sideways under a sheet that does not.
          // **`soft` and never `hard`**: `hard` writes the wrap points
          // into the value, which would save a file with newlines
          // nobody typed.
          wrap="soft"
          // Transparent ink over the painted copy, and the caret is the
          // one thing drawn — `text-transparent` hides the glyphs but
          // not the selection, which browsers paint from their own
          // highlight colour and is exactly what a selection should
          // look like here.
          className={cn(
            "absolute inset-0 resize-none overflow-hidden border-0 bg-transparent px-3",
            WRAP,
            "text-transparent caret-foreground outline-none",
          )}
        />
      )}
    </>
  );
}

/** Which of git's four inks a change wears, from what the change **is**
    (2026-09-16: 「文件里,有改动或 git 差异的那个竖线,是要有颜色的,和现
    在的 git 状态颜色一致」— the user).
 *
 * The bars were `--git-mark` until then: the foreground at half
 * strength, argued for on the grounds that a bar says only 「different
 * from HEAD」. The tree's letters had already stopped saying only that
 * (工作区批⑦) — they say **which way** different, in these same four
 * inks — and a file whose row wears a yellow M has no business drawing
 * its own changes in grey. So the gutter reads the way the row does.
 *
 * Three of the four are reachable here and the fourth is not: `C` is a
 * conflict, which is a fact about a path and not about a place inside
 * one, so `git diff` never hands one over. */
function hunkInk(h: GitHunk): string {
  // Nothing on screen replaced these lines: they are gone, and gone is
  // 删除's ink. Asked first because such a hunk also has `was`, and the
  // order of these two tests is the whole difference between 删 and 改.
  if (h.lines === 0) return "bg-git-deleted";
  // Lines with nothing behind them in HEAD — 新增.
  if (h.was.length === 0) return "bg-git-added";
  return "bg-git-modified";
}

/** The change gutter: one bar per hunk, each one pressable.

    **Every bar presses, including the ones with nothing removed.** A
    pure insertion has no old lines to show, so the honest options were
    a dead target that looks like a live one or a sentence saying the
    lines are new — and two bars that look alike and behave differently
    is the worse of the two (`gui.git_added`). */
function Gutter({
  hunks,
  shown,
  tops,
  onShow,
}: {
  hunks: GitHunk[];
  shown: number | null;
  /** Where every line begins, measured off the sheet (`FileView`'s
      `tops`). **Empty means nothing has been laid out yet** and the
      gutter draws no bars — a bar at a line it is not about is worse
      than a bar one frame late, and with wrapping there is no formula
      to fall back on. */
  tops: number[];
  onShow: (i: number) => void;
}) {
  return (
    // `self-stretch` rather than a height of its own: the column has to
    // be exactly as tall as the text beside it, and the text already
    // knows how tall it is. It also makes the rule between the numbers
    // and the code run the whole way down instead of stopping where the
    // last hunk does.
    // **4 wide, not 12** (2026-09-16:「左侧的 git 状态条细一点,占用太多
    // 空间了。刚好适合点击就行」). VS Code's dirty-diff decoration is 3px;
    // this is one more so a pointer lands on it without aiming, and
    // nothing else — the column exists for the bar and takes the bar's
    // width. The touch floor is waived for it in `check-face` for the
    // reason written there.
    // **6 wide** (2026-09-16:「左侧的 git 状态条稍微再宽一点,点击性受限
    // 了」, after 4 turned out too thin to land on) — still a bar, not a
    // column; and no divider of its own any more, the numbers carry it.
    <div data-gutter={hunks.length} className="relative w-1.5 shrink-0 self-stretch">
      {hunks.map((h, i) => {
        const gone = h.lines === 0;
        // The line the hunk starts on, and the one after everything it
        // covers. Out of range — a diff that names lines this buffer no
        // longer has — draws nothing at all, which is the same answer
        // as「not measured yet」and the right one: khor has no place to
        // put a bar about a line it cannot find.
        const from = tops[h.at - 1];
        const to = tops[h.at - 1 + h.lines];
        // A deletion stands on the boundary **below** line `at`, which
        // is where the lines that went away used to begin.
        const edge = tops[h.at];
        if (from === undefined || to === undefined || (gone && edge === undefined)) return null;
        return (
          <button
            type="button"
            key={`${h.at} ${h.lines}`}
            data-hunk={h.at}
            data-on={shown === i}
            aria-label={gui.git_hunk}
            onClick={() => onShow(i)}
            // The floor does not apply here and that is deliberate: a
            // gutter bar is not a control a finger hunts for, it is the
            // shape of a fact, and a 44-tall target on a 21.6 line
            // would cover three of its neighbours. What it costs is
            // said where it is spent — on a phone the bars are read,
            // and the file is read, and neither is pressed.
            className={cn(
              // **The ink is the hunk's own now** (2026-09-16), where it
              // used to be `--git-mark` — the foreground at half
              // strength, which is why the resting weight lived in the
              // token rather than in an opacity. A hue cannot be dimmed
              // the same way and stay the colour it is naming, so the
              // bar is drawn at full strength and the one that is open
              // is told apart by the ring instead.
              "absolute left-0 w-full cursor-pointer",
              hunkInk(h),
              shown === i && "ring-1 ring-foreground/40",
            )}
            style={
              gone
                ? {
                    // A deletion has no lines to stand beside, so it is
                    // a notch on the boundary between two rather than a
                    // bar over any of them — sitting on a line it would
                    // claim that line had changed. The transform
                    // centres the notch on the boundary rather than a
                    // second arithmetic subtracting half its height.
                    top: edge,
                    height: "calc(var(--text-code--line-height) / 4)",
                    transform: "translateY(-50%)",
                  }
                : { top: from, height: to - from }
            }
          />
        );
      })}
    </div>
  );
}

/** What HEAD had, **in the file itself, directly over the lines that
    replaced it** (2026-09-16: 「点击后,不要在下面一行显示情况。就地显示。
    跟 vscode 一样」— the user).
 *
 * It was a panel pinned under the whole column until then, and the
 * argument for that was a real constraint: the code was one `<pre>`
 * holding one painted string, so nothing could be put between two of
 * its lines without cutting a span that crossed them. **工作区批⑨ took
 * that constraint away** — the sheet is one block per line now, because
 * wrapping needed something to measure — and what was left was a panel
 * describing line 412 sitting at the bottom of the screen.
 *
 * **It floats rather than sitting in the flow** (工作区第五轮,「原文
 * 开着时不能打字」). In the flow it pushed every line below it down, and
 * a textarea is one box holding one string — it cannot be pushed in the
 * middle — so the two fell out of register and the editor had to step
 * off the glass while this was open. Out of the flow nothing moves, and
 * the caret stays where the person left it. The cost is that it covers
 * the few lines above the change, which is what VS Code's peek does
 * too: a peek is something you are looking through, not a row of the
 * file.
 *
 * **Plain, unpainted text.** What is read here is which lines were
 * there, and highlighting them would make the removed copy look exactly
 * like the live one two rows below it — the tint and the ink are what
 * say which is which.
 */
/** The peek's three inks, **the bar's own** (2026-09-16:「黄色的状态条
    点击后,弹出的状态底色是红色,跟黄色不一致」): the old lines of a
    modification sit on the modification's yellow, a deletion's on red,
    and an insertion's one-line note on green — a tenth for the wash,
    full for the letters, the same hue as a hairline so the block has an
    edge on both themes. Same three tests as `hunkInk`, in the same
    order, so the two can never disagree. */
function peekInk(h: GitHunk): string {
  if (h.lines === 0) return "border-git-deleted/40 bg-git-deleted/10 text-git-deleted";
  if (h.was.length === 0) return "border-git-added/40 bg-git-added/10 text-git-added";
  return "border-git-modified/40 bg-git-modified/10 text-git-modified";
}

function Peek({
  hunk,
  at,
  next,
  tall,
}: {
  hunk: GitHunk;
  /** The top of the line this is about, inside the sheet. */
  at: number;
  /** The top of the line after it — where this goes when it will not
      fit above. */
  next: number;
  tall: number;
}) {
  const gone = hunk.was.length > 0;
  // **Above where there is room, below where there is not, and never
  // over the line itself.** Above is the arrangement (2026-09-16:
  // 「就地显示」 — the old lines directly over the ones that replaced
  // them), and out of the flow it cannot push anything down to make
  // room, so a hunk at line 1 has nowhere above to be: a block clamped
  // to the top of the column would sit *on* the very line it is being
  // compared against, which is the comparison losing its other half.
  // Below, it covers the lines after instead — which are not what is
  // being read at that moment.
  //
  // `tall` is 0 for the frame before it is measured, so the first paint
  // is「above」 and flush with the line; it settles on the next.
  const top = tall <= at ? at - tall : next;
  return (
    <div
      // Two elements, because it needs two grounds: the outer is opaque
      // (it is covering live code, and a tenth-strength wash over text
      // is not a block, it is a smudge) and the inner carries the tint.
      // **`data-peek` is on the inner one** — the tint is what says
      // which of the three kinds this is, and a gate reading the ground
      // off the named element has to find the colour, not the paper.
      // The two boxes are the same rectangle, so every measurement of
      // place and height is the same either way.
      className="absolute inset-x-0 z-10 bg-background shadow-sm"
      style={{ top }}
    >
      <div data-peek={hunk.at} className={cn("border-y px-3", peekInk(hunk))}>
        {gone ? (
          hunk.was.map((line, i) => (
            <div key={i} className="min-h-[var(--text-code--line-height)]">
              {line}
            </div>
          ))
        ) : (
          // **A pure insertion has no old text, and says so rather than
          // opening empty** — the bar is pressable everywhere it is drawn
          // (`Gutter`), and a press that produced a blank block would be
          // the dead target that rule exists to avoid.
          <div className="min-h-[var(--text-code--line-height)]">{gui.git_added}</div>
        )}
      </div>
    </div>
  );
}

/** The overview ruler: the whole file squeezed into one column past the
    scrollbar, with a mark where every hunk is (2026-09-16: 「右侧滑动条
    后方,就是全文件带颜色的 git 差异的那个竖线。跟 vscode 一样」).
 *
 * **This is what the 「1 处改动 第 1 处」 strip was standing in for.**
 * That strip existed because the gutter can only draw a bar beside a
 * line, and a line eight hundred down is not on the screen — so the
 * count and the two steps were words doing a picture's job. Here the
 * file is one column tall whatever its length, so where the changes are
 * is a thing you look at, and going to one is a press on the place
 * itself.
 *
 * **Proportions, and one floor.** A mark's top and height are fractions
 * of the scroll, so they hold while the column is resized without a
 * single re-measurement; `--overview-mark` keeps a one-line hunk in a
 * long file from rounding away to nothing.
 *
 * **Nothing is drawn before the first layout** (`tops` empty), the same
 * rule the gutter holds: a mark at a place it is not about is worse
 * than a mark one frame late, and with wrapping there is no arithmetic
 * to fall back on. */
function Ruler({
  hunks,
  tops,
  span,
  shown,
  onGo,
  onGoAt,
}: {
  hunks: GitHunk[];
  tops: number[];
  /** How tall the scroll is and where the text starts inside it
      (`FileView`'s `span`). */
  span: { total: number; pad: number };
  shown: number | null;
  onGo: (hunk: GitHunk) => void;
  /** A press on the bare strip: the fraction down it that was pressed
      (`FileView`'s `goRatio`). */
  onGoAt: (ratio: number) => void;
}) {
  return (
    // **Always here, even on a clean file.** A column that appeared with
    // the first change would move the text sideways under somebody's
    // caret the moment they saved.
    <div
      data-overview={hunks.length}
      // Over the scroller's right edge, the width of its scrollbar.
      // Nothing of the strip is drawn — no border, no ground — so on a
      // trackpad Mac, where the scrollbar is an overlay that fades,
      // what stays is the marks alone, which is the VS Code picture.
      //
      // **The whole column takes a press** (工作区第五轮 第 40 条): it is
      // the file end to end, so pressing two thirds of the way down it
      // means「take me two thirds of the way down the file」— and until
      // now that press fell through to the scrollbar beneath and did
      // nothing at all. A mark still means「open that change」, which is
      // why it stops the press from reaching here as well.
      //
      // **Below the touch floor on purpose, and on record**: this strip
      // is as wide as the scrollbar it stands on, and a 44px one would
      // cover a tenth of the text. What is reachable here is reachable
      // another way — the gutter's bars are a full row tall, and
      // scrolling is scrolling.
      className="absolute inset-y-0 right-0 z-10 w-[var(--overview-ruler)]"
      onClick={(e) => {
        const box = e.currentTarget.getBoundingClientRect();
        if (box.height <= 0) return;
        onGoAt((e.clientY - box.top) / box.height);
      }}
    >
      {span.total > 0 &&
        hunks.map((h, i) => {
          const from = tops[h.at - 1];
          // A deletion covers no current line, so it is a mark on the
          // boundary rather than a band over anything — the floor gives
          // it its height, exactly as the notch in the gutter has one.
          const to = tops[h.at - 1 + h.lines];
          if (from === undefined || to === undefined) return null;
          return (
            <button
              type="button"
              key={`${h.at} ${h.lines}`}
              data-overview-mark={h.at}
              data-on={shown === i}
              aria-label={gui.git_hunk}
              onClick={(e) => {
                // The strip under it means「go to this place」; a mark
                // means「open this change」, and a press is one or the
                // other.
                e.stopPropagation();
                onGo(h);
              }}
              // The touch floor does not apply, for the gutter bar's
              // reason said one column over: this is the shape of a
              // fact rather than a control a finger hunts for, and a
              // 44-tall mark would cover a fifth of the file.
              className={cn(
                // **Half, so the scrollbar's thumb reads through a mark**
                // (2026-09-16:「文件右侧悬浮的滚动条,现在在 git 状态条上完全
                // 看不见了」, and then again:「文件右侧的滑动按钮还是在状态
                // 条下看不清, 透明度再低一点」). The marks stand above the
                // scroller for their press; an opaque mark hid the
                // overlay thumb completely where the two crossed. VS
                // Code's decorations are translucent for the same
                // reason. **Both sides moved on the second pass**: the
                // mark came down from .6 and the thumb went up to half
                // ink (`--scroll-thumb-strong`) — the thing that has to
                // be readable is the one you drag.
                "absolute inset-x-0.5 min-h-[var(--overview-mark)] cursor-pointer rounded-xs opacity-50",
                hunkInk(h),
              )}
              style={{
                top: `${((span.pad + from) / span.total) * 100}%`,
                height: `${((to - from) / span.total) * 100}%`,
              }}
            />
          );
        })}
    </div>
  );
}
