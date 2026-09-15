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
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

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
import { IconChevronDown, IconChevronUp } from "@/components/icons";
import { Markdown } from "@/components/Markdown";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { parseDelimited } from "@/lib/csv";
import { languageOf, paintFile } from "@/lib/highlight";
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
  onSaved,
  undo,
  onUndo,
  draft,
  onDraft,
}: {
  machine: string;
  /** Absolute, on that machine. */
  path: string;
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
  /** What this file held **before the last save this face made**, if
      there was one.

      **Held by the pane for the same reason the draft is** (工作区批4
      收尾): the view is remounted on every file, and a one-step undo
      that evaporates when you glance at the file next door is an undo
      nobody can rely on. Absent when nothing has been saved here.

      **In memory, and the limit is said out loud** (`gui.file_undo`'s
      comment): closing the workspace drops it. What it catches is the
      「存完才发现存错了」 moment, which is a minute wide; a version
      history is a different thing with a disk behind it. */
  undo?: string;
  onUndo?: (text: string | null) => void;
}) {
  /** What this file turns out to be, when it is not code. `null`
      while khor is still asking, and for the kind that needs no
      asking — `text`, which is the road this view already had. */
  const [look, setLook] = useState<Preview | null>(null);
  /** 源码 or 预览. **Always opens on 源码** (「打开先是源码」— the user,
      2026-09-14) and is not remembered: a person who read one file
      rendered has said nothing about the next, and a toggle that
      carried over would mean the source they asked for is not what
      opens. */
  const [face, setFace] = useState<"source" | "preview">("source");
  const [lookError, setLookError] = useState<string | null>(null);
  const [file, setFile] = useState<FileText | null>(null);
  /** The working copy, and the digest it is pinned to. **One piece of
      state, not two**: a save replaces both at once (the text it wrote
      and the digest the far machine answered), and holding them apart
      is how a second save gets pinned to a digest that belongs to
      different bytes. `null` until the file arrives. */
  const [own, setOwn] = useState<Draft | null>(null);
  // The draft the pane is holding wins over anything this mount read:
  // coming back to a file must show what was typed, not what is on
  // disk. `setEdit` writes both so the two never disagree.
  const edit = draft ?? own;
  const setEdit = useCallback(
    (next: Draft | null | ((was: Draft | null) => Draft | null)) => {
      setOwn((was) => {
        const now = typeof next === "function" ? next(draft ?? was) : next;
        onDraft?.(now);
        return now;
      });
    },
    [draft, onDraft],
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
  // Kept apart from `diff` for the same reason the workspace head keeps
  // its refusal apart from its branch: no bars and no answer look
  // identical on screen, and one of them says the file is unchanged.
  const [diffError, setDiffError] = useState<string | null>(null);
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

  useEffect(() => setFace("source"), [path]);

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
    setDiffError(null);
    setShown(null);
    fetchGitDiff(machine, path)
      .then((d) => live && setDiff(d))
      .catch((e: unknown) => live && setDiffError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
  }, [machine, path, diffAt]);

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
  const [painted, setPainted] = useState<{ text: string; html: string | null } | null>(null);
  useEffect(() => {
    if (lang === null) {
      setPainted(null);
      return;
    }
    // No wait at all on a file that is only being read — a new file
    // must not flash plain before it colours. The wait exists for the
    // burst of keystrokes, so it starts when the burst does.
    const id = setTimeout(() => setPainted({ text: body, html: paintFile(body, lang) }), dirty ? PAINT_IDLE : 0);
    return () => clearTimeout(id);
  }, [body, lang, dirty]);


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
        // What was there a moment ago, kept before it is overwritten
        // in state. `file.text` is khor's own last reading of the
        // file, which is exactly the thing this save replaced.
        onUndo?.(file?.text ?? null);
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
        onUndo?.(file?.text ?? null);
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
                className="prose-khor mx-auto max-w-prose px-6 py-4"
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

  const html = painted !== null && painted.text === body ? painted.html : null;
  const lines = body.length === 0 ? 0 : body.split("\n").length;

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
  /** Open the hunk `step` places along and put it on screen.
      **Wraps at both ends**, which is what a person means by「下一处」
      on the last one — the alternative is a control that goes dead
      exactly when the file is nearly read. */
  const step = (by: number) => {
    if (hunks.length === 0) return;
    const next = shown === null ? (by > 0 ? 0 : hunks.length - 1) : (shown + by + hunks.length) % hunks.length;
    setShown(next);
    const el = box.current;
    // **Asked of the bar, not computed from the line height.** The bar
    // is already sitting where CSS put it, so its `offsetTop` is the
    // answer by construction — a second arithmetic here would be a
    // copy of `--text-code--line-height` that nothing keeps in step,
    // and the day the token moved one of the two would be wrong.
    const bar = el?.querySelectorAll<HTMLElement>("[data-hunk]")[next];
    if (el && bar) {
      // A third of the way down rather than at the top: a hunk pinned
      // to the ceiling is a hunk with no lines above it to compare
      // against, and comparing is the whole reason to go there.
      el.scrollTop = bar.offsetTop - el.clientHeight / 3;
    }
  };

  const renders = rendererOf(path);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* **源码 / 预览, and the column is switched from its head**
          (工作区批⑤). Drawn only where there is a second face to switch
          to; a file with none gets no strip at all rather than a switch
          with one position.

          **The preview renders `body`** — the buffer, half-typed and
          unsaved — never the file on disk. That is the whole reason
          this is a face and not a kind: a rendered view lagging the
          text by one save would be showing a document nobody has. */}
      {renders !== null && (
        <div className="flex flex-none items-center gap-1 border-b px-3 py-1">
          {(["source", "preview"] as const).map((f) => (
            <Button
              key={f}
              size="sm"
              variant={face === f ? "secondary" : "ghost"}
              data-file-face={f}
              data-on={face === f}
              className="min-h-touch"
              onClick={() => setFace(f)}
            >
              {f === "source" ? gui.file_face_source : gui.file_face_preview}
            </Button>
          ))}
        </div>
      )}
      {/* **Only while something is unsettled.** A file that is open and
          saved says nothing at all — 「没有就不显示」, and a permanent
          「已保存」is a label that stops being read on the second file.
          A refusal stays until the next save, because it is the one
          thing here a person has to act on. */}
      {(dirty || saved !== null) && (
        <div className="flex items-center gap-3 border-b px-3 py-1.5 text-aux text-muted-foreground">
          {/* **A refusal outranks「未存」.** Both are true after one —
              the file is still unsaved *and* khor knows why — and the
              one worth naming is the one that has to be acted on. */}
          <span
            data-save-state={
              typeof saved === "string" ? "refused" : dirty ? "unsaved" : "saved"
            }
          >
            {typeof saved === "string" ? saved : dirty ? gui.file_unsaved : gui.file_saved}
          </span>
          {/* What a merge left behind, and **only while the box still
              holds it**: it clears on the next keystroke and on the
              save, because a count of clashes that outlived the text it
              counted would be a number about nothing. */}
          {clashes !== null && (
            <span data-merged={clashes} className="text-muted-foreground">
              {clashes === 0 ? gui.file_merged_clean : gui.file_merged_clashes(clashes)}
            </span>
          )}
          {/* **Three ways out of a refusal, where there used to be
              none** (2026-09-13). A save khor would not make left the
              sentence 「这个文件在你编辑期间变了」 standing and nothing
              to press — the person's only move was to copy their text
              somewhere else by hand. Two of these throw one side away
              and say so on the button; the third keeps both and is the
              one a person usually wants, which is why it sits between
              them rather than at the end. */}
          {typeof saved === "string" && (
            <span className="ml-auto flex shrink-0 gap-2">
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
          {/* **Only with nothing unsaved in the way.** Offered beside
              未存 it would be two buttons that both change the box,
              and the one that throws work away must not be the easy
              one to hit. */}
          {typeof saved !== "string" && !dirty && undo !== undefined && (
            <Button
              variant="outline"
              size="sm"
              data-undo
              onClick={() => {
                // **Into the box, never onto the disk.** khor writes
                // when somebody presses 存 and at no other moment, so
                // this puts the old text back where it can be read
                // and changed again before it becomes a file.
                setEdit((e) => (e === null ? e : { ...e, text: undo }));
                onUndo?.(null);
              }}
              className="ml-auto min-h-touch min-w-touch"
            >
              {gui.file_undo}
            </Button>
          )}
          {dirty && typeof saved !== "string" && (
            <Button
              variant="outline"
              size="sm"
              data-save
              disabled={saving}
              onClick={save}
              className="ml-auto min-h-touch min-w-touch"
            >
              {gui.file_save}
            </Button>
          )}
        </div>
      )}
      {/* **The count, and only when there is something to count**
          (「没有就不显示」). A clean file grows no bar; a changed one
          says how many places and offers the two steps, which is the
          half of「点开是那一处」that the gutter alone cannot do — a
          hunk eight hundred lines down is not reachable by pressing
          something you cannot see. */}
      {stale && (
        <div className="border-b px-3 py-1.5 text-aux text-muted-foreground">{gui.git_stale}</div>
      )}
      {hunks.length > 0 && (
        <div className="flex items-center gap-2 border-b px-3 py-1.5 text-aux text-muted-foreground">
          <span data-hunk-count={hunks.length} className="tabular-nums">
            {gui.git_hunks(hunks.length)}
          </span>
          {shown !== null && (
            <span data-hunk-at={shown + 1} className="tabular-nums">
              {gui.git_hunk_at(shown + 1)}
            </span>
          )}
          <div className="ml-auto flex items-center gap-0.5">
            <Button
              variant="ghost"
              size="icon"
              data-hunk-step="-1"
              aria-label={gui.git_prev}
              onClick={() => step(-1)}
              className="min-h-touch min-w-touch"
            >
              <IconChevronUp />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              data-hunk-step="1"
              aria-label={gui.git_next}
              onClick={() => step(1)}
              className="min-h-touch min-w-touch"
            >
              <IconChevronDown />
            </Button>
          </div>
        </div>
      )}
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
        </div>
      )}
      {took !== null && (
        <div className="border-b px-3 py-1.5 text-aux text-muted-foreground">{took}</div>
      )}
      {/* Said out loud rather than left as an empty gutter: a file with
          no bars reads as a file that matches HEAD, and that is exactly
          the claim khor cannot make when it could not read the
          repository. Same rule as the workspace head's. */}
      {diffError !== null && (
        <div className="border-b px-3 py-1.5 text-aux text-muted-foreground">{diffError}</div>
      )}
      {diff?.more === true && (
        <div className="border-b px-3 py-1.5 text-aux text-muted-foreground">{cli.git_more}</div>
      )}
      {/* Three columns that scroll as one. The numbers and the code are
          each their own `<pre>` rather than a marker on every line: a
          painted block's spans cross line ends (a block comment is one
          span over many lines), so splitting the markup by「\n」to hang
          anything off each line would cut those spans in half.
          **The gutter between them is where that rule is paid for** —
          it cannot be a marker inside either column, so it is a column
          of its own, and every bar in it is placed by arithmetic on the
          line height rather than by flowing with the text. */}
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
            <Markdown text={body} className="prose-khor mx-auto max-w-prose px-6 py-4" />
          ) : (
            <Table text={body} sep={path.toLowerCase().endsWith(".tsv") ? "\t" : ","} />
          )}
        </div>
      ) : (
      <div ref={box} className="min-h-0 flex-1 overflow-auto">
        <div className="flex min-w-full items-start py-2">
          <pre
            aria-hidden="true"
            className="shrink-0 select-none pl-2 text-right font-mono text-code leading-code text-muted-foreground tabular-nums"
          >
            {Array.from({ length: lines }, (_, i) => i + 1).join("\n")}
          </pre>
          <Gutter
            hunks={hunks}
            shown={shown}
            onShow={(i) => setShown(shown === i ? null : i)}
          />
          <div className="relative min-w-max flex-1">
            <Sheet
              text={body}
              html={html}
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
      )}
      {/* **Under the code, not over it** (2026-09-13). It was an
          overlay laid at the hunk's own line, which put the removed
          lines exactly where they used to be — and hid however many
          current lines it was tall. That cost was carried as a known
          one because the alternative looked impossible: the code is a
          single `<pre>` with a transparent `<textarea>` over it, and
          that trick is what makes the file editable at all, so nothing
          can be inserted *into* the flow without splitting one of them
          and breaking the alignment of both.

          Out of the flow entirely is the way through. The panel takes
          its own height from the column, the code area gives it up, and
          the bar in the gutter stays lit at the hunk's line — which is
          what carried the「where」 anyway, since the panel was already
          below the lines it described rather than on them. */}
      {shown !== null && diff !== null && diff.hunks[shown] !== undefined && (
        <Was hunk={diff.hunks[shown]} onClose={() => setShown(null)} />
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

/**
 * The file's one surface: the painted text, with a transparent editor
 * laid exactly over it when the file may be written back.
 *
 * **Why a textarea over a `<pre>` and not an editor component.** The
 * two have to agree character for character, and they do because they
 * are the same string in the same font, size, line height and padding,
 * with `white-space: pre` on both — a browser laying out one lays out
 * the other identically. That is the whole trick, and it is also the
 * whole of its fragility: **every typographic property here must be set
 * on both, and `check-face` measures the caret against the ink to catch
 * the day one of them drifts.**
 *
 * **What it does not buy, said where it is spent:** no bracket
 * matching, no multiple cursors, no folding, and no completion — the
 * last of which is not a cost at all (「不提供任何自动补全什么的」). A
 * person who wants those opens the file in VS Code, which is the
 * batch after this one.
 *
 * **The `<pre>` keeps the text even under the editor.** It would be
 * cheaper to leave it empty and let the textarea show through, but then
 * the column would have no width of its own and the horizontal scroll,
 * the gutter's arithmetic and the line numbers would all be measuring
 * a box with nothing in it.
 */
function Sheet({
  text,
  html,
  edit,
  onText,
  onSave,
}: {
  text: string;
  /** The painted markup, or null for a language this build does not
      carry — then the `<pre>` shows the plain text instead. */
  html: string | null;
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
        className="px-3 font-mono text-code leading-code"
      >
        {html === null ? (
          text
        ) : (
          // Safe for `paint`'s reason, which `paintFile` inherits:
          // highlight.js escapes every character of the input before it
          // wraps any of it, so this is the file's own text plus spans
          // this build generated.
          <code dangerouslySetInnerHTML={{ __html: html }} />
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
          // `wrap="off"` is the attribute, not a class: a wrapped line
          // would put the editor's line 40 beside the painting's line
          // 39 and every number, bar and hunk below it would be wrong.
          wrap="off"
          // Transparent ink over the painted copy, and the caret is the
          // one thing drawn — `text-transparent` hides the glyphs but
          // not the selection, which browsers paint from their own
          // highlight colour and is exactly what a selection should
          // look like here.
          className={cn(
            "absolute inset-0 resize-none overflow-hidden border-0 bg-transparent px-3",
            "font-mono text-code leading-code text-transparent caret-foreground outline-none",
          )}
        />
      )}
    </>
  );
}

/** A line count as a distance down the page.

    **The line height is read from the same token the text is set in**
    (`--text-code--line-height`), not measured off the DOM: `<pre>` does
    not wrap, so every line is exactly one line tall and the arithmetic
    is exact. A measurement would be the same number arrived at later
    and with a reflow. */
function down(n: number): string {
  return `calc(var(--text-code--line-height) * ${n})`;
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
  onShow,
}: {
  hunks: GitHunk[];
  shown: number | null;
  onShow: (i: number) => void;
}) {
  return (
    // `self-stretch` rather than a height of its own: the column has to
    // be exactly as tall as the text beside it, and the text already
    // knows how tall it is. It also makes the rule between the numbers
    // and the code run the whole way down instead of stopping where the
    // last hunk does.
    <div data-gutter={hunks.length} className="relative w-3 shrink-0 self-stretch border-r">
      {hunks.map((h, i) => {
        const gone = h.lines === 0;
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
              // **No second dimming on top of the token.** `--git-mark`
              // is already the foreground at half strength; an
              // `opacity-60` over it measured about 30% against the
              // card, which on an 11-wide bar is a smudge. The token
              // carries the resting weight and the pointer takes it to
              // full.
              "absolute left-0 w-full cursor-pointer bg-git-mark hover:opacity-100",
              shown !== i && "opacity-80",
            )}
            style={
              gone
                ? {
                    // A deletion has no lines to stand beside, so it is
                    // a notch on the boundary between two rather than a
                    // bar over any of them — sitting on a line it would
                    // claim that line had changed.
                    top: `calc(${down(h.at)} - var(--text-code--line-height) / 8)`,
                    height: `calc(var(--text-code--line-height) / 4)`,
                  }
                : { top: down(h.at - 1), height: down(h.lines) }
            }
          />
        );
      })}
    </div>
  );
}

/** What HEAD had, laid over the lines below the hunk.

    **An overlay rather than a row inserted into the text.** The code is
    one `<pre>` holding one painted string, for the reason at the top of
    this file — nothing can be put between two of its lines without
    cutting a span that spans them. So this floats instead, and the file
    underneath keeps its own line numbers correct while it is open. */
function Was({ hunk, onClose }: { hunk: GitHunk; onClose: () => void }) {
  return (
    // **A ground of its own, not a tint.** The first spelling was
    // `bg-muted` with a hairline top and bottom, and on the dark theme
    // that is within a few percent of the page — the panel read as the
    // removed lines having leaked into the file rather than as a thing
    // beside it. `bg-card` is the ground this app already uses for
    //「a surface of its own」 (the list pane). The shadow went with the
    // overlay: a panel that sits in the column rather than over it is
    // told apart by its border and its ground, and a shadow on it
    // would be claiming a layer it no longer occupies.
    <div
      data-was={hunk.at}
      // **A third of the pane at most**, and its own scroller past
      // that: a hunk that replaced two hundred lines would otherwise
      // take the whole column and leave the file it is about off
      // screen.
      className="max-h-[33%] shrink-0 overflow-y-auto border-t bg-card px-3 py-2"
    >
      <button
        type="button"
        onClick={onClose}
        aria-label={gui.git_hunk}
        className="mb-1 block text-aux text-muted-foreground"
      >
        {hunk.was.length === 0 ? gui.git_added : gui.git_was} · {hunk.at}
      </button>
      {hunk.was.length > 0 && (
        // Plain, unpainted text: what is being read here is which lines
        // were there, and highlighting them would make the removed copy
        // look exactly like the live one two rows above it.
        <pre className="overflow-x-auto font-mono text-code leading-code text-muted-foreground">
          {hunk.was.join("\n")}
        </pre>
      )}
    </div>
  );
}
