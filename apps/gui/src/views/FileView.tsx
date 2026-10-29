import {useDraftCount} from "@/hooks/use-draft-count";
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
// **It saves on its own since save-a (2026-10-08)**: a second after the
// typing stops and on leaving the file, through `lib/save-road`, which
// also holds what a write that did not land means (a clash stops it, a
// failure is tried again). ✓ and ⌘S are the extra confirmation.
//
// **The editor is CodeMirror 6 since cm-a (2026-10-10, CM-BRIEF)**
// (`components/CodeEditor`, `lib/code`). It was a transparent
// `<textarea>` over a painted `<pre>` for a year, chosen so the git gutter
// would not fight an editor's; the owner chose CodeMirror because the two
// layers' caret never lined up with the paint (OWNER-NOTES-10-09 已知问题
// 1), and the gutter is an extension of the editor now. What it is not
// is an IDE: no completion, no auto-closing, no second caret
// (「不提供任何自动补全什么的」).
//
// **The three things it refuses to paint, each with its own sentence.**
// Not text; cut short by one slice; nothing there at all. They are
// separate because the next move differs: take it and open it
// elsewhere, scroll what is here and take the rest if you need it, or
// nothing. Folded into one「打不开」they would all read as khor being
// broken, which is the failure `check-acp` 第二批 names.
import { useFileHandoff, type FileHandoffControl } from "@/hooks/use-file-handoff";
import { FileHandoffAction, FileHandoffStatus } from "@/components/FileHandoff";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { fileRefresh } from "@/lib/file-refresh";

import {
  fetchPreview,
  type Preview,
  fetchFileStat,
  fetchFileText,
  fetchGitBlame,
  fetchGitDiff,
  mergeFileText,
  pullFile,
  said,
  type FileStat,
  type FileText,
  type GitBlame,
  type GitDiff,
} from "@/api";
import { CodeEditor, preloadEditor } from "@/components/CodeEditor.lazy";
import { editorIn, focusAtTop } from "@/lib/code/place";
import type { EditorHistorySlot } from "@/lib/code/history";
import { useKeptScrollRef } from "@/hooks/use-kept-scroll";
import { HtmlView } from "@/components/HtmlView";
import { Notebook, preloadNotebook } from "@/components/Notebook.lazy";
import { Markdown } from "@/components/Markdown";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { parseDelimited } from "@/lib/csv";
import { blameOfLines, useBlameShown } from "@/lib/blame";
import { groundAttr, usePreviewGround } from "@/lib/preview-ground";
import { useHereHosts } from "@/lib/here";
import { levelOf, treeChanged } from "@/lib/tree-changed";
import {
  confirmSaved,
  forget,
  holdFlush,
  newLine,
  onAnyLanded,
  onLanded,
  PAUSE_MS,
  readForOverwrite,
  reopened,
  useRoad,
  write,
} from "@/lib/save-road";
import { usePinch } from "@/hooks/use-pinch";
import { needsOf } from "@/lib/md/blocks";
import { prepare } from "@/lib/md/extras";
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
/**
 * **Files read lately, by machine and path** (round NET-4): a file
 * opened again is asked what it is now (`fetchFileStat`, one small
 * answer) and shown from here when nothing changed, instead of crossing
 * whole again. A handful, oldest dropped first — a cache of what a
 * person has just been reading, not of the disk.
 */
const HELD_MAX = 24;
const held = new Map<string, FileText>();
const heldKey = (machine: string, path: string) => `${machine}\u0000${path}`;
function remember(machine: string, path: string, f: FileText) {
  const k = heldKey(machine, path);
  held.delete(k);
  held.set(k, f);
  while (held.size > HELD_MAX) {
    const oldest = held.keys().next().value;
    if (oldest === undefined) break;
    held.delete(oldest);
  }
}
/** Whether a file answering `now` is still the one held as `f`: by its
    bytes when the view holds them whole (a digest), by size and mtime
    when it does not. */
const unchanged = (f: FileText, now: FileStat) =>
  f.digest !== "" ? now.digest === f.digest : now.total === f.total && now.at_ms === f.at_ms;
/** Bytes a text takes on disk, for a file whose text was just saved. */
const bytesOf = (text: string) => new TextEncoder().encode(text).length;

// **Every write that lands is what the disk holds now**, whichever view
// asked and whether or not it is still mounted (`lib/save-road`): the
// cache of files read lately moves with it, so coming back to the file
// is a stat that matches rather than a read; and the trees and git's
// letters of that level ask again (UX-43 §1).
onAnyLanded((l) => {
  const had = held.get(heldKey(l.machine, l.path));
  remember(l.machine, l.path, {
    ...(had ?? { cut: false, binary: false }),
    text: l.text,
    digest: l.digest,
    at_ms: l.at_ms,
    total: bytesOf(l.text),
  });
  treeChanged({ machine: l.machine, dir: levelOf(l.path) });
});

/** An unsaved working copy: the text, the digest the next save is
    pinned to, and which line of editing it is (`lib/save-road`'s `by`:
    one per buffer read from the disk). **One piece of state, not two** —
    a save replaces both at once, and holding them apart is how a second
    save gets pinned to a digest that belongs to different bytes. */
export type Draft = { text: string; was: string; by: string };

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
export function rendererOf(path: string): "md" | "table" | "html" | "ipynb" | null {
  const name = path.split("/").pop() ?? "";
  const dot = name.lastIndexOf(".");
  const e = dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
  if (e === "md" || e === "markdown") return "md";
  // A page, drawn in a sandbox (CM-BRIEF step 3, `HtmlView`).
  if (e === "html" || e === "htm") return "html";
  // A notebook, read: cells, code and what it printed, nothing run
  // (lane nb-view, `Notebook`).
  if (e === "ipynb") return "ipynb";
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
  path,
  machine,
  total,
  handoff,
  preview = false,
}: {
  preview?:boolean;
  path:string;
  machine:string;
  total?:number;
  handoff:FileHandoffControl;
  word: string;
  onTake: () => void;
  took: string | null;
}) {
  // Not on a phone (docs/IOS.md 第二步): it takes no files (`useTake`).
  const takes = useHereHosts();
  const [size,setSize]=useState(total);
  useEffect(()=>{
    setSize(total);if(total!==undefined)return;
    let alive=true;void fetchFileStat(machine,path).then(s=>{if(alive)setSize(s.total);}).catch(()=>{});
    return()=>{alive=false;};
  },[machine,path,total]);
  const name=path.split("/").pop()??path;
  const type=name.includes(".")?name.slice(name.lastIndexOf(".")+1).toUpperCase():null;
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
      <span data-file-fallback-name className="max-w-full break-words text-body text-foreground">{name}</span>
      <span className="text-callout text-muted-foreground">{[type,size===undefined?null:sizeWord(size)].filter(Boolean).join(" · ")}</span>
      {word && <span className="text-callout text-muted-foreground">{word}</span>}
      <FileHandoffAction handoff={handoff} preview={preview}/>
      {takes && !handoff.supported && (
        <Button size="sm" data-file-take onClick={onTake} className="mt-1 min-h-touch min-w-touch">
          {gui.file_take}
        </Button>
      )}
      {took !== null && <span className="text-callout text-muted-foreground">{took}</span>}
    </div>
  );
}

/** The face a file opens on. **Source, except a notebook** (the owner,
    2026-10-20). Opening a file never opts into its rendered face
    (2026-10-13): a Markdown, an HTML page or a table is a text a person
    opens to work on, and the preview is one press away. A notebook is
    the one file read far more than it is edited by hand — its JSON is
    the kernel's record, not prose — and its source is where the editor
    is slow: a saved plot is one base64 line (`lib/code/wrap`). So it
    opens as its preview, and source is one press away instead.

    **Asked per file, never carried**: every entry that opens a file —
    the file page, a workspace tab, a git revision, the attachment
    viewer, the bare view — asks this again for each new path, so a
    notebook's preview does not follow into the Markdown opened next,
    and source chosen on one notebook does not outlive it. */
export function defaultFaceOf(path: string): "source" | "preview" {
  return rendererOf(path) === "ipynb" ? "preview" : "source";
}

export function FileView(props: Omit<Parameters<typeof FileBody>[0],"handoff"> & {handoff?:FileHandoffControl}) {
  const own=useFileHandoff(props.machine,props.path,!props.handoff);
  const handoff=props.handoff??own;
  return <div ref={own.owner} data-file-body className="relative flex min-h-0 flex-1 flex-col">
    <FileBody {...props} handoff={handoff}/>
    {!props.handoff&&<FileHandoffStatus handoff={handoff}/>}
  </div>;
}

function FileBody({
  handoff,
  machine,
  path,
  face,
  onSaved,
  draft,
  onDraft,
  saveAt = 0,
  readAt = 0,
  goLine = null,
  onNote,
  git = true,
  reading = false,
  onCanEdit,
  editorHistory,
}: {
  handoff: FileHandoffControl;
  /** An open-tab owner may retain this across the file view's remount. */
  editorHistory?: EditorHistorySlot;
  /** **Read first, under a finger** (2026-10-03, the owner's ruling on
      P1: 「像备忘录」). True while a phone shows this file to be read: no
      editor over the text, so a tap selects, scrolls or does nothing and
      never raises the keyboard. Whoever holds the file's head offers
      「编辑」, which clears it (`beginEditing`), and 「保存」, which saves
      and sets it again. A mouse never reads first: a click edits. */
  reading?: boolean;
  /** Whether this file can be edited at all (the whole of it is here),
      for the head's 「编辑」: not drawn for a file that cannot be. */
  onCanEdit?: (can: boolean) => void;
  /** Whether to ask git for the gutter. A machine's disk (`MachineFiles`)
      is not a checkout, and asking there is a refusal per file. */
  git?: boolean;
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
   * **Not remembered** (「打开先是源码」, 工作区批⑤) — the pane resets it
   * to `defaultFaceOf` when the front tab changes, which is the same rule
   * spelled one level higher. Left out, it is that default. */
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
  const localHistory = useRef<EditorHistorySlot>({});
  const history = editorHistory ?? localHistory.current;
  const previewScroll = useKeptScrollRef(JSON.stringify([machine, path]), history.preview ??= {});
  const nativeOffice = kindOf(path) === "doc" && handoff.previewSupported;
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
  // **The editor's chunk, fetched while the file is read** (CM-BRIEF §2):
  // the strip's 「编辑」 focuses the editor inside its own press
  // (`beginEditing`), so the module has to be resident before the press
  // and not after it (`CodeEditor.lazy`).
  useEffect(() => {
    preloadEditor();
  }, []);
  // A retained draft supplies the first frame after remount. Once this
  // view accepts a buffer, its own copy also owns load/save/merge updates.
  const edit = own ?? draft ?? null;
  const refresh = useRef(fileRefresh());
  useLayoutEffect(() => () => refresh.current.invalidate(), [machine, path]);
  const setEdit = useCallback(
    (next: Draft | null | ((was: Draft | null) => Draft | null)) => {
      refresh.current.invalidate();
      setOwn((was) => (typeof next === "function" ? next(was ?? draft ?? null) : next));
    },
    [draft],
  );
  /** A way out of a clash is under way (a read, a merge, an overwrite). */
  const [working, setWorking] = useState(false);
  const overwriteEpoch = useRef(0);
  useEffect(() => {
    setWorking(false);
    return () => { overwriteEpoch.current += 1; };
  }, [machine, path]);
  /** Bumped to read this file again without changing which file it is
      — the way out of a refusal that keeps the person's own change
      (`theirs`). A counter for `diffAt`'s reason: two reads of one
      path are two questions, and a boolean cannot ask twice. */
  const [reread, setReread] = useState(0);
  /** **What the file's save road says** (`lib/save-road`, save-a): a
      clash or a failure, held there rather than here because it outlives
      this mount — the last write goes as the person leaves the file.
      ⚠️ 墓碑: `saved`, the last save's sentence or `true`, this mount's own. */
  const road = useRoad(machine, path);
  const trouble = road.trouble;
  /** A way out that could not even ask the machine (its read failed). */
  const [oops, setOops] = useState<string | null>(null);
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
  /** A picture zooms under two fingers (`usePinch`). */
  const pinch = usePinch();
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
    if (kindOf(path) === "text" || (kindOf(path) === "doc" && handoff.previewSupported !== false)) return;
    fetchPreview(machine, path)
      .then((p) => live && setLook(p))
      .catch((e: unknown) => live && setLookError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
  }, [machine, path, reread, nativeOffice, handoff.previewSupported]);

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
    setOops(null);
    setError(null);
    setTook(null);
    const take = (f: FileText) => {
      if (!live) return;
      remember(machine, path, f);
      setFile(f);
      // **Only when nobody is holding one.** A draft means somebody
      // typed here and walked away; overwriting it with what is on
      // disk is the loss this whole arrangement exists to stop.
      if (draft === undefined) setEdit({ text: f.text, was: f.digest, by: newLine() });
    };
    const read = () =>
      fetchFileText(machine, path)
        .then(take)
        .catch((e: unknown) => live && setError(said(e)));
    // Read lately: asked what it is now, and shown from memory when
    // nothing changed; read again when something did or the asking
    // failed (an older khor over there answers the read as before).
    const had = held.get(heldKey(machine, path));
    if (had) {
      fetchFileStat(machine, path)
        .then((now) => (unchanged(had, now) ? take(had) : read()))
        .catch(() => read());
    } else {
      void read();
    }
    // Cancelled by ignoring the answer, not by aborting the dial: the
    // slice is already crossing and dropping it on the floor costs the
    // far machine nothing it has not already spent.
    return () => {
      live = false;
    };
  }, [machine, path, reread]);

  // Asked beside the text rather than after it: the two answers are
  // about the same file at the same moment, and waiting for the first
  // would make the gutter arrive late enough to be seen arriving.
  useEffect(() => {
    let live = true;
    setDiff(null);
    if (!git) return;
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
  }, [machine, path, diffAt, git]);

  // **Blame, when it is asked for** (round UX-4; UX-23): asked of the
  // machine holding the file once per file and again after a save, and
  // mapped onto the lines as they are now. Drawn on the caret's line
  // only (`InlineBlame`), on every face.
  const blameShown = useBlameShown();
  const ground = usePreviewGround();
  const [blame, setBlame] = useState<GitBlame | null>(null);
  useEffect(() => {
    let live = true;
    setBlame(null);
    if (!git || !blameShown.on) return;
    fetchGitBlame(machine, path)
      .then((b) => live && setBlame(b))
      // The gutter's rule: what khor could not read is not drawn.
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [machine, path, diffAt, git, blameShown.on]);

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
  // A Markdown file's preview chunks (HTML, anchors, formulas) start
  // loading while the source shows, so the first switch draws at once
  // and lands on the line it was at (`lib/md/extras`).
  const markdown = rendererOf(path) === "md";
  const notebook = rendererOf(path) === "ipynb";
  const hasText = file !== null;
  useEffect(() => {
    if (notebook && hasText) preloadNotebook();
    if (markdown && hasText) prepare(needsOf(body, true), true);
    // Once per file: the body only decides which chunks, and a formula
    // typed later loads when the preview asks for it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [markdown, notebook, hasText, path]);
  /** Editable exactly when the far machine handed over a digest, which
      it does exactly when the text is the whole file (`files::shape`).
      One fact, one place. */
  const canEdit = (edit?.was ?? "") !== "";
  const dirty = file !== null && edit !== null && edit.text !== file.text;
  useDraftCount(onDraft === undefined && dirty ? 1 : 0);
  useEffect(() => onCanEdit?.(canEdit), [canEdit, onCanEdit]);

  const publishDraft = useCallback((next: Draft | null) => {
    // A read in flight cannot clear the retained owner's existing draft.
    if (onDraft === undefined || file === null || next === null) return;
    onDraft(next.text === file.text ? null : next);
  }, [file, onDraft]);
  useEffect(() => publishDraft(edit), [edit, publishDraft]);

  /** How many lines the text has, counted the way `\n` does — the spine
      blame is laid along. */
  const lines = useMemo(() => (body.length === 0 ? 0 : body.split("\n").length), [body]);
  /** Who wrote each line, in this buffer's lines; nothing while the
      buffer differs from the disk, for the change bars' reason (`hunks` below). */
  const blamed = useMemo(
    () => (blame === null || diff === null || dirty ? null : blameOfLines(lines, blame, diff.hunks)),
    [blame, diff, dirty, lines],
  );

  const askedAt = useRef(saveAt);
  useEffect(() => {
    if (saveAt === askedAt.current) return;
    askedAt.current = saveAt;
    save(true);
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

  /** Refresh a clean file without replacing intervening edits or marked text.
      The original disk version must also remain the draft's merge base. */
  const again = () => {
    if (kindOf(path) !== "text" || file === null || dirty) return;
    void refresh.current.read(
      () => fetchFileStat(machine, path)
        .then((now) => (unchanged(file, now) ? null : fetchFileText(machine, path)))
        .catch(() => fetchFileText(machine, path)),
      (f) => {
        if (f.digest === file.digest) return;
        remember(machine, path, f);
        setFile(f);
        setEdit({ text: f.text, was: f.digest, by: newLine() });
        setDiffAt((n) => n + 1);
      },
    ).catch(() => {});
  };

  /** **A write landed**, from this mount or from one that left the file
      a moment ago (`lib/save-road`). The disk is the text written, so
      that is khor's reading of the file now; the buffer is re-pinned to
      the new digest when it is the same line of editing, and keeps
      whatever was typed during the round trip on top. A buffer read
      separately keeps its pin and will be refused, which is right: it
      does not hold this write's text. */
  const savedRef = useRef(onSaved);
  savedRef.current = onSaved;
  useEffect(
    () =>
      onLanded(machine, path, (l) => {
        setFile((f) =>
          f === null ? f : { ...f, text: l.text, digest: l.digest, at_ms: l.at_ms, total: bytesOf(l.text) },
        );
        setEdit((e) => (e === null || e.by !== l.by ? e : { ...e, was: l.digest }));
        setOops(null);
        // A merge saved is a merge answered (✓ over undecided places).
        setClashes(null);
        // The gutter is about HEAD and the file just moved, so it is
        // wrong until it is asked again. Same for the tree's marks,
        // which this view does not hold (`onSaved`, and `treeChanged`
        // from the road for every holder, UX-43 §1).
        setDiffAt((n) => n + 1);
        savedRef.current?.();
      }),
    [machine, path],
  );

  /** **Write the buffer through the save road** (save-a). `asked` is a
      person pressing for it (✓, ⌘S, 保存并关闭): it writes even after a
      clash — the other side may have gone back — and the button says
      已保存 when it lands, or at once when there is nothing to write. */
  const save = (asked = false) => {
    if (edit === null || !canEdit) return;
    // Before the read lands the buffer can only be the pane's draft —
    // something typed and not written — so it is written as it stands.
    if (file !== null && !dirty) {
      if (asked && !road.busy) confirmSaved(machine, path);
      return;
    }
    // The answer is heard by `onLanded` above and the trouble by
    // `useRoad`; the promise has nothing left to say.
    write(machine, path, edit, { asked }).catch(() => {});
  };
  const saveRef = useRef(save);
  saveRef.current = save;

  /** **Autosave**: a second after the typing stops (`PAUSE_MS`). Not
      while a clash is unanswered — nothing is written until the person
      picks a way out — and not over a merge that has not been looked at:
      its text holds both sides and maybe the markers between them, and
      「是否保存仍由你决定」 (`file-merged-clean`) stays true until every
      place is decided (保留 / 放弃, cm-a) or ✓. ⚠️ 墓碑: 「until a
      keystroke」 — the first letter typed ended the merge's review. A failure does not stop it: the next pause tries
      again with the newer text, and the road retries on its own between. */
  const autosave = canEdit && dirty && clashes === null && trouble?.kind !== "clash";
  useEffect(() => {
    if (!autosave) return;
    const id = setTimeout(() => saveRef.current(false), PAUSE_MS);
    return () => clearTimeout(id);
  }, [autosave, edit?.text]);

  /** **Leaving the file writes it** — another tab, the tab closed, the
      page left — and so does the app being hidden or quit (`holdFlush`,
      which `lib/save-road` calls). Read through a ref: the unmount sees
      the last render, not the first. The pane hears the landing through
      `orphan` when no view of this file is mounted by then. */
  const leaving = useRef({ edit, autosave, onDraft, onSaved });
  leaving.current = { edit, autosave, onDraft, onSaved };
  useEffect(() => {
    reopened(machine, path);
    const flush = () => {
      const s = leaving.current;
      if (!s.autosave || s.edit === null) return null;
      return write(machine, path, s.edit, {
        orphan: () => {
          s.onDraft?.(null);
          s.onSaved?.();
        },
      }).catch(() => {});
    };
    const stop = holdFlush(machine, path, flush);
    return () => {
      stop();
      void flush();
    };
  }, [machine, path]);

  /** Throw away what is in the box and read the file again. */
  const theirs = () => {
    overwriteEpoch.current += 1;
    setWorking(false);
    forget(machine, path);
    setOops(null);
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
   * person who presses it, or types on (autosave waits for one of the
   * two, `autosave` above).
   *
   * Both changes are in that text, and where they touched the same
   * lines both are there between markers. khor does not pick; the
   * count says how many times it did not.
   */
  const merge = () => {
    if (edit === null || file === null || working) return;
    const current = refresh.current.checkpoint();
    if (current === null) return;
    const action = ++overwriteEpoch.current;
    const accepts = () => action === overwriteEpoch.current && current();
    setWorking(true);
    setOops(null);
    const base = file.text;
    const mine_ = edit.text;
    fetchFileText(machine, path)
      .then((now) => {
        if (!accepts()) return;
        return mergeFileText(base, mine_, now.text).then((m) => {
          if (!accepts()) return;
          // The buffer and its merge base move together, only while this
          // action still owns the unchanged line of editing.
          setFile((f) => (f === null ? f : { ...f, text: now.text, digest: now.digest, at_ms: now.at_ms }));
          setEdit((e) => (e === null ? e : { ...e, text: m.text, was: now.digest }));
          setClashes(m.clashes);
          forget(machine, path);
        });
      })
      .catch((e: unknown) => { if (accepts()) setOops(said(e)); })
      .finally(() => { if (action === overwriteEpoch.current) setWorking(false); });
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
    if (edit === null || working) return;
    const action = ++overwriteEpoch.current;
    const by = edit.by;
    setWorking(true);
    setOops(null);
    readForOverwrite(machine, path)
      .then((read) => {
        if (action !== overwriteEpoch.current) return;
        const latest = leaving.current.edit;
        if (latest === null || latest.by !== by) return;
        const pin = read.pin(latest.text);
        if (pin === null) return;
        // New typing belongs to the fresh base too. The write and the next
        // keystroke must share this new line, rather than the old line's head.
        remember(machine, path, read.file);
        setFile(read.file);
        setEdit(pin);
        leaving.current = { ...leaving.current, edit: pin };
        return write(machine, path, pin, { asked: true });
      })
      .catch((e: unknown) => {
        // A refusal of the write is the road's trouble and on screen
        // already; what is left is the read that did not happen.
        if (action === overwriteEpoch.current && (e instanceof Error || typeof e === "string")) setOops(said(e));
      })
      .finally(() => {
        if (action === overwriteEpoch.current) setWorking(false);
      });
  };

  const take = () => {
    setTook(null);
    pullFile(machine, path)
      .then((dest) => setTook(gui.took_file(dest)))
      .catch((e: unknown) => setTook(said(e)));
  };

  // ── the three screens that are not a code view ──────────────────
  if (nativeOffice) return <Away machine={machine} path={path} handoff={handoff} preview word="" onTake={take} took={took}/>;
  if (kindOf(path) !== "text") {
    if (lookError !== null) {
      return <Away machine={machine} path={path} handoff={handoff} word={lookError} onTake={take} took={took}/>;
    }
    if (look === null) return <div className="flex-1" />;
    if (look.cut) {
      return (
        <Away machine={machine} path={path} handoff={handoff} total={look.total} word={gui.file_preview_too_big} onTake={take} took={took} />
      );
    }
    if (look.kind === "image" || look.kind === "svg") {
      // **An `<img>`, including for SVG, and that is the security
      // decision.** SVG is markup and can carry scripts; put in the
      // document it would run them, and khor would owe itself a
      // sanitiser. An image element renders the same drawing and runs
      // nothing — the browser's own rule, not khor's.
      return (
        <div className="flex min-h-0 flex-1 items-center justify-center overflow-hidden p-4">
          <img
            ref={pinch}
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
            <div className="flex flex-1 items-center justify-center p-6 text-body text-foreground">
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
            className="flex-none border-t px-3 py-1.5 text-callout text-muted-foreground"
          >
            {look.kind === "pdf" ? gui.pdf_as_words : gui.doc_plain}
          </div>
        </div>
      );
    }
    return <Away machine={machine} path={path} handoff={handoff} word={gui.file_not_text} onTake={take} took={took} />;
  }

  if (error !== null) {
    return <div className="p-4 text-callout text-muted-foreground">{error}</div>;
  }
  if (file === null) return <div className="flex-1" />;

  if (file.binary) {
    return <Away machine={machine} path={path} handoff={handoff} total={file.total} word={gui.file_not_text} onTake={take} took={took}/>;
  }

  // **Nothing is drawn about HEAD while the buffer differs from disk.**
  // The bars name line numbers and one typed line moves every line
  // under it, so a bar left where it was would point at code it is not
  // about — wrong, not merely old. The alternative is a round trip per
  // keystroke onto somebody else's machine, which khor will not do.
  const hunks = dirty ? [] : (diff?.hunks ?? []);
  // ⚠️ 墓碑 (save-a, 2026-10-08): `stale`, 「保存后才能显示改动位置」 in
  // the notice while the box differed from the disk and the gutter had
  // gone quiet. The file saves itself a second after the typing stops
  // now, so the sentence would flash on every pause; a save that does
  // not land says so in its own words below.
  const renders = rendererOf(path);

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
        <div className="flex items-center gap-3 border-b px-3 py-1.5 text-callout text-muted-foreground">
          <span className="tabular-nums">
            {gui.file_cut(sizeWord(file.text.length), sizeWord(file.total))}
          </span>
          {/* Beside the cut line rather than as a strip of its own: it
              is the same fact twice, and「只读了开头」has already told
              the half that explains this one. */}
          <span data-read-only>{gui.file_read_only}</span>
          {handoff.supported ? <FileHandoffAction handoff={handoff}/> : <Button size="sm" onClick={take} className="ml-auto">{gui.file_take}</Button>}
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
      {/* **The editor leaves the screen in 预览 and keeps everything.**
          The buffer, the digest the next save is pinned to, the dirty
          mark: all of them are this component's state and none of them
          lives inside the editor. Switching back finds the text
          exactly as it was — and typing into a preview is not possible,
          because there is nothing there to focus. */}
      {(face ?? defaultFaceOf(path)) === "preview" && renders !== null ? (
        <div
          ref={previewScroll}
          data-file-preview={renders}
          {...groundAttr(ground.ground)}
          className={cn("min-h-0 flex-1 overflow-auto", renders === "html" && "flex flex-col")}
        >
          {renders === "md" ? (
            <Markdown text={body} className="prose-khor px-6 py-4" lines machine={machine} path={path} />
          ) : renders === "html" ? (
            <HtmlView machine={machine} path={path} text={body} ground={ground.ground} />
          ) : renders === "ipynb" ? (
            // A file cut by the cap is half a JSON document: the cut row
            // above already says so, and nothing is drawn under it.
            !file.cut && <Notebook text={body} machine={machine} path={path} />
          ) : (
            <Table text={body} sep={path.toLowerCase().endsWith(".tsv") ? "\t" : ","} />
          )}
        </div>
      ) : (
        // **The file in CodeMirror** (CM-BRIEF, cm-a): numbers, folding,
        // the change gutter and its peek, the overview ruler over the
        // scrollbar, blame at the caret's line's end and ⌘F are the
        // editor's own extensions (`lib/code`). Writable exactly when the
        // whole file is here and nobody is reading it first.
        <CodeEditor
          editorHistory={history}
          historyKey={JSON.stringify([machine, path])}
          path={path}
          text={body}
          editable={canEdit && !reading}
          hunks={hunks}
          blamed={blamed}
          goLine={goLine}
          onComposition={(active) => refresh.current.setComposing(active)}
          onText={(t) => {
            // **The leaving write sees the key at once.** CodeMirror's
            // input is not a React event, so the render that refreshes
            // `leaving` comes a task later; a tab closed in that gap
            // wrote the text without its last key (face save i, cm-b).
            const s = leaving.current;
            if (s.edit !== null) {
              const next = { ...s.edit, text: t };
              leaving.current = { ...s, edit: next };
              setEdit(next);
              publishDraft(next);
            } else {
              setEdit((e) => (e === null ? e : { ...e, text: t }));
            }
          }}
          onSave={() => save(true)}
          // A merge waiting on the person: the disk's text (the merge set
          // `file` to it) against the box, a 保留 / 放弃 per place. Every
          // place decided is the merge answered, and autosave goes on.
          against={clashes !== null && file !== null ? file.text : null}
          onChunks={(left) => left === 0 && setClashes(null)}
        />
      )}
      {/* **The one notice left, and it floats** (2026-09-16:「类似要在
          文件上方新增一列的东西都不要」— the user).

          What can land in it is a save that did not simply happen: the
          file changed somewhere else and autosave stopped (save-a: one
          sentence and the ways out), a write that failed and is being
          tried again (and why), what a merge left behind, and a way out
          that could not reach the machine. The clash is the only place in
          this column where somebody **has** to act; 「没有就不显示」
          covers the rest of the day — a save that lands says nothing
          here (the dot goes out; ✓ and ⌘S say 已保存 on the button).

          **`absolute`, at the foot, over the text.** Anything in the
          flow above the file moves the line a person is typing on the
          moment it appears; this covers the last row or two instead,
          and the file underneath is still where it was. It is at the
          foot rather than the head because the head is where the caret
          usually is — a notice there would cover the thing it is about.

          **It goes when the reason goes**: the trouble when a write lands
          or the person picks a way out, `clashes` when every place of the
          merge is decided and on the save. */}
      {(trouble !== null || clashes !== null || oops !== null) && (
        <div
          data-file-notice
          // `pointer-events-none` on the frame and back on inside it:
          // the strip spans the column, and the part of it that is not
          // the card must not eat presses meant for the code under it.
          className="pointer-events-none absolute inset-x-0 bottom-0 z-10 flex justify-center p-2"
        >
          {/* Floating over the file, so the one floating language
              (20-states 浮层, 2026-09-28): it was the board's `card`
              under a shadow of Tailwind's scale. */}
          <div className="pointer-events-auto flex max-w-full flex-wrap items-center gap-3 rounded-lg border bg-popover px-3 py-1.5 text-callout text-muted-foreground shadow-(--shadow-float)">
            {/* **Which trouble, from the disk and not from the words**
                (`lib/save-road`): the far machine's refusal is in its
                own language; what decides is what the file holds now. */}
            {trouble?.kind === "clash" && clashes === null && (
              <span data-save-state="refused" title={trouble.said}>
                {gui.file_clash}
              </span>
            )}
            {trouble?.kind === "failed" && (
              <span data-save-state="failed">{gui.file_unsaved_why(trouble.said)}</span>
            )}
            {clashes !== null && (
              <span data-merged={clashes}>
                {clashes === 0 ? gui.file_merged_clean : gui.file_merged_clashes(clashes)}
              </span>
            )}
            {oops !== null && <span data-save-oops>{oops}</span>}
            {/* **Three ways out of a clash, where there used to be
                none** (2026-09-13). A save khor would not make left the
                sentence 「这个文件在你编辑期间变了」 standing and nothing
                to press — the person's only move was to copy their text
                somewhere else by hand. Two of these throw one side away
                and say so on the button; the third keeps both and is the
                one a person usually wants, which is why it sits between
                them rather than at the end. */}
            {trouble?.kind === "clash" && clashes === null && (
              <span className="flex shrink-0 gap-2">
                <Button
                  size="sm"
                  data-take-theirs
                  disabled={working}
                  aria-busy={working || undefined}
                  onClick={theirs}
                  className="min-h-touch min-w-touch"
                >
                  {gui.file_take_theirs}
                </Button>
                <Button
                  size="sm"
                  data-merge
                  disabled={working}
                  aria-busy={working || undefined}
                  onClick={merge}
                  className="min-h-touch min-w-touch"
                >
                  {gui.file_merge}
                </Button>
                <Button
                  size="sm"
                  data-keep-mine
                  disabled={working}
                  aria-busy={working || undefined}
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
    <table data-table className="w-full border-collapse text-mono">
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

/**
 * **Into the text, where the reader was** (P1, 2026-10-03): focuses the
 * editor inside `box` with the caret at the start of the first line the
 * scroller shows, without moving it. Called from the press on 「编辑」,
 * in the same gesture — WebKit raises the soft keyboard only for a focus
 * a touch asked for — so the caller renders the editor writable first
 * (`flushSync`) and then calls this.
 */
export function beginEditing(box: HTMLElement | null): void {
  const view = editorIn(box);
  if (view) focusAtTop(view);
}
