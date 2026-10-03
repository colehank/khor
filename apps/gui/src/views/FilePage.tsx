/**
 * **One file, full screen** (round UX-3, the owner's ruling 2026-10-04:
 * every full-screen content page gives the screen to the content; nothing
 * sits top-left; what is not content floats at the bottom).
 *
 * The phone's file — out of a workspace's tree or a machine's files —
 * and a machine's in-place file on every platform. No title rows: the
 * file starts under the status bar, and its name is a small glass pill at
 * the top centre that steps aside while the text moves and comes back
 * when it rests or is tapped (Safari's address bar). At the bottom, one
 * row of glass circles: ‹ at the left — a way out that can be seen, the
 * edge swipe being the other — and at the right 分享, 「…」 and 编辑.
 *
 * **Read first, edit in place** (P1, now on this page's circle). A file
 * opens to be read — Markdown and tables rendered, code as code. 编辑
 * opens the editor **at the line the reader was at** (for Markdown, the
 * line the top block starts on) and raises the keyboard; the circle then
 * reads ✓ 保存, and saving goes back to reading at the same place. Before
 * this the switch landed at the top of the file, which was the bug the
 * owner named. Leaving with something unsaved asks first.
 *
 * Nothing is drawn that cannot be used: no 编辑 on a file that is not
 * whole here (a picture, a cut file), no share where nothing can take a
 * file, no VS Code where the machine has none.
 */
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { flushSync } from "react-dom";

import { launchOn, pullTemp, renamePath, reveal } from "@/api";
import { CloseDraftDialog } from "@/components/CloseDraftDialog";
import type { CodeWindow } from "@/components/CodeWindow";
import { GlassCircle } from "@/components/Glass";
import { IconBack, IconCheck, IconMore, IconPencil, IconShare } from "@/components/icons";
import { RemoveDialog } from "@/components/RemoveDialog";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { useCoarse } from "@/hooks/use-coarse";
import { FILE_STEPS, fileSizeStyle, useFileSize } from "@/lib/file-size";
import { baseName } from "@/lib/paths";
import { cn } from "@/lib/utils";
import { bytesOf, revealHere, shares } from "@/views/chat-files";
import { useRemoveFlow } from "@/views/fileRow";
import { beginEditing, FileView, kindOf, rendererOf, type Draft } from "@/views/FileView";

// ── where the reader is ───────────────────────────────────────────

/** The scroller the file is shown in now: the rendered face's, or the
    source's. */
function scrollerIn(box: HTMLElement | null): HTMLElement | null {
  return box?.querySelector<HTMLElement>("[data-file-preview], [data-file-scroll]") ?? null;
}

/** The source line at the top of what is shown: the first line (or, in
    rendered Markdown, the first block) whose bottom is below the
    scroller's top edge, past its padding. */
export function topLine(box: HTMLElement | null): number {
  const s = scrollerIn(box);
  if (!s) return 1;
  const edge = s.getBoundingClientRect().top + parseFloat(getComputedStyle(s).paddingTop || "0");
  const marks = s.matches("[data-file-preview]")
    ? [...s.querySelectorAll<HTMLElement>("[data-src-line]")]
    : [...s.querySelectorAll<HTMLElement>("[data-line]")];
  const first = marks.find((m) => m.getBoundingClientRect().bottom > edge + 1);
  const n = Number(first?.dataset.srcLine ?? first?.dataset.line ?? "1");
  return Number.isFinite(n) && n > 0 ? n : 1;
}

/** Scrolls what is shown so `line` stands at the top, under the padding:
    the line itself in the source, the block that holds it when rendered. */
export function placeAt(box: HTMLElement | null, line: number): void {
  const s = scrollerIn(box);
  if (!s) return;
  let mark: HTMLElement | undefined;
  if (s.matches("[data-file-preview]")) {
    for (const b of s.querySelectorAll<HTMLElement>("[data-src-line]")) {
      if (Number(b.dataset.srcLine) <= line) mark = b;
      else break;
    }
  } else {
    mark = s.querySelector<HTMLElement>(`[data-line="${line}"]`) ?? undefined;
  }
  if (!mark) return;
  const pad = parseFloat(getComputedStyle(s).paddingTop || "0");
  s.scrollTop += mark.getBoundingClientRect().top - s.getBoundingClientRect().top - pad;
}

/** **The place holds while the keyboard comes up** (round UX-3, measured
    in the simulator): WebKit on iOS reveals the focused field as the
    keyboard rises, and the file's editor is the whole sheet, so it was
    revealed from its top — the source placed at line 30 was at line 1
    185 ms later, intermittently, as the keyboard came in or did not. For
    a moment after 编辑, a jump back to the top and every resize of the
    visible area put the line back. */
function holdWhileKeyboardRises(box: HTMLElement | null, line: number): void {
  const s = box?.querySelector<HTMLElement>("[data-file-scroll]");
  if (!s || line <= 1) return;
  const vv = window.visualViewport;
  const put = () => placeAt(box, line);
  const jumped = () => {
    if (s.scrollTop < 1) put();
  };
  s.addEventListener("scroll", jumped);
  vv?.addEventListener("resize", put);
  window.setTimeout(() => {
    s.removeEventListener("scroll", jumped);
    vv?.removeEventListener("resize", put);
  }, HOLD_MS);
}

/** How long the place is held after 编辑: the keyboard's rise, with room. */
const HOLD_MS = 900;

// ── the verbs, for the page's circles and the wide strip ──────────

/** 分享 and 「…」: what one file offers besides reading it. Drawn as two
    glass circles by the page, and at the right end of the workspace's
    tab strip on a Mac (`FileVerbs`). */
function useFileVerbs({
  machine,
  path,
  here,
  code,
  onGone,
  onMoved,
}: {
  machine: string;
  path: string;
  here: boolean;
  code?: CodeWindow | null;
  onGone: (path: string) => void;
  onMoved: (from: string, to: string) => void;
}) {
  const coarse = useCoarse();
  const [finder, setFinder] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);
  const size = useFileSize();
  useEffect(() => {
    void revealHere().then(setFinder);
  }, []);
  useEffect(() => setSaid(null), [path]);
  const removal = useRemoveFlow({ machine, here, onGone });
  const kind = kindOf(path);
  const sharing = !finder && kind !== "doc" && shares();
  // On a Mac the file is handed to this machine's other apps: a far one
  // is fetched into a temporary folder first (`pullTemp`).
  const local = () => pullTemp(here ? "" : machine, path);
  const fail = (e: unknown) => setSaid(String((e as Error)?.message ?? e));
  const share = () =>
    void bytesOf(machine, path)
      .then((f) => (f ? navigator.share({ files: [f] }) : undefined))
      .catch(() => {
        // The sheet dismissed is a rejection too; nothing to say.
      });
  const previewKind = rendererOf(path) !== null || kind === "image" || kind === "svg" || kind === "pdf";
  const textKind = kind === "text";
  const menu: { key: string; label: string; run: () => void; destructive?: boolean }[] = [
    {
      key: "copy-path",
      label: gui.tab_copy_path,
      run: () => void navigator.clipboard.writeText(path).then(() => setSaid(gui.copied), () => setSaid(gui.copy_failed)),
    },
    { key: "rename", label: gui.rename_ask, run: () => setRenaming(true) },
    ...(code?.offered ? [{ key: "code", label: gui.file_in_code, run: () => code.want() }] : []),
    ...removal.actions(path, false).map((a) => ({ key: a.key, label: a.label, run: () => void a.run(), destructive: true })),
  ];
  const shareCircle: ReactNode = finder ? (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <GlassCircle aria-label={gui.chat_file_share} data-file-share tip={!coarse && gui.chat_file_share}>
          <IconShare />
        </GlassCircle>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" side="top" sideOffset={8} data-file-share-menu>
        <DropdownMenuItem onSelect={() => void local().then((p) => launchOn("", p)).catch(fail)}>
          {gui.file_open_other}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => void local().then((p) => reveal(p)).catch(fail)}>
          {gui.chat_file_reveal}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  ) : sharing ? (
    <GlassCircle aria-label={gui.chat_file_share} data-file-share tip={!coarse && gui.chat_file_share} onClick={share}>
      <IconShare />
    </GlassCircle>
  ) : null;
  const moreCircle: ReactNode = (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <GlassCircle aria-label={gui.rail_more} data-file-more tip={!coarse && gui.rail_more}>
          <IconMore />
        </GlassCircle>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" side="top" sideOffset={8} data-file-more-menu className="min-w-nav-menu">
        {menu
          .filter((m) => !m.destructive)
          .map((m) => (
            <DropdownMenuItem key={m.key} data-file-more-item={m.key} onSelect={m.run}>
              {m.label}
            </DropdownMenuItem>
          ))}
        {/* 字号, for what reads (text, rendered or not); a picture or a
            PDF has no type to size. Two keys on one row, kept open. */}
        {(textKind || previewKind) && kind !== "image" && kind !== "svg" && kind !== "pdf" && (
          <div data-file-size className="flex items-center gap-in px-2 py-1 text-body">
            <span className="flex-1">{gui.type_size}</span>
            <Button
              size="sm"
              aria-label={gui.file_size_down}
              data-file-size-down
              disabled={size.step === 0}
              onClick={() => size.set(size.step - 1)}
            >
              A−
            </Button>
            <Button
              size="sm"
              aria-label={gui.file_size_up}
              data-file-size-up
              disabled={size.step === FILE_STEPS.length - 1}
              onClick={() => size.set(size.step + 1)}
            >
              A+
            </Button>
          </div>
        )}
        <DropdownMenuSeparator />
        {menu
          .filter((m) => m.destructive)
          .map((m) => (
            <DropdownMenuItem
              key={m.key}
              data-file-more-item={m.key}
              data-destructive
              className="text-destructive focus:text-destructive data-[highlighted]:text-destructive"
              onSelect={m.run}
            >
              {m.label}
            </DropdownMenuItem>
          ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
  const dialogs = (
    <>
      <RemoveDialog {...removal.dialog} />
      <RenameDialog
        open={renaming}
        name={baseName(path)}
        onClose={() => setRenaming(false)}
        onRename={async (name) => {
          const dir = path.slice(0, path.lastIndexOf("/")) || "/";
          const to = `${dir === "/" ? "" : dir}/${name}`;
          if (to === path) return;
          await renamePath(machine, path, to);
          onMoved(path, to);
        }}
      />
    </>
  );
  return { shareCircle, moreCircle, dialogs, said, size };
}

/** 重命名: the name only, in its directory (`FileTree`'s rule: a typed
    `/` would move the file somewhere nothing is showing). */
function RenameDialog({
  open,
  name,
  onClose,
  onRename,
}: {
  open: boolean;
  name: string;
  onClose: () => void;
  onRename: (name: string) => Promise<void>;
}) {
  const [value, setValue] = useState(name);
  const [refused, setRefused] = useState<string | null>(null);
  useEffect(() => {
    if (open) {
      setValue(name);
      setRefused(null);
    }
  }, [open, name]);
  const go = () => {
    const v = value.trim();
    if (!v || v.includes("/")) return;
    onRename(v).then(onClose, (e: unknown) => setRefused(String((e as Error)?.message ?? e)));
  };
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent form data-file-rename>
        <DialogHeader>
          <DialogTitle>{gui.rename}</DialogTitle>
        </DialogHeader>
        <Input
          autoFocus
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && go()}
        />
        {refused && <span className="text-callout text-state-failed">{refused}</span>}
        <DialogFooter>
          <Button data-dialog-cancel onClick={onClose}>
            {gui.delete_keep}
          </Button>
          <Button variant="primary" data-file-rename-go onClick={go}>
            {gui.rename}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** The Mac's workspace strip carries the same verbs at its right end
    (round UX-3): 分享 and 「…」, beside the tab they are about. */
export function FileVerbs(props: Parameters<typeof useFileVerbs>[0]) {
  const v = useFileVerbs(props);
  return (
    <div data-file-verbs className="flex flex-none items-center gap-in">
      {v.shareCircle}
      {v.moreCircle}
      {v.dialogs}
    </div>
  );
}

// ── the page ──────────────────────────────────────────────────────

export function FilePage({
  machine,
  path,
  here,
  git = false,
  draft,
  onDraft,
  saveAt = 0,
  readAt = 0,
  goLine = null,
  onSaved,
  onNote,
  onBack,
  onGone,
  onMoved,
  code = null,
}: {
  machine: string;
  /** Absolute, on that machine. */
  path: string;
  /** Whether the file is on the machine khor runs on. */
  here: boolean;
  /** Ask git for the change gutter (a workspace's file; not a disk's). */
  git?: boolean;
  draft?: Draft;
  onDraft?: (d: Draft | null) => void;
  /** A save asked for from outside (the strip's close-over-a-draft). */
  saveAt?: number;
  readAt?: number;
  goLine?: { line: number; at: number } | null;
  onSaved?: () => void;
  onNote?: (note: string | null) => void;
  /** Leave the page (the ‹ circle, the edge swipe's twin). */
  onBack: () => void;
  /** The file was deleted from here. */
  onGone: (path: string) => void;
  /** The file was renamed from here. */
  onMoved: (from: string, to: string) => void;
  /** The workspace's VS Code window, where that machine has one. */
  code?: CodeWindow | null;
}) {
  const coarse = useCoarse();
  const box = useRef<HTMLDivElement>(null);
  const [writing, setWriting] = useState(false);
  const [can, setCan] = useState(false);
  const [saves, setSaves] = useState(0);
  const [asking, setAsking] = useState(false);
  /** Where to put the reader after the face changes (a source line). */
  const restore = useRef<number | null>(null);
  /** A save this page asked for, and what follows it: back to reading, or
      leaving the page (the draft question's 保存). */
  const saving = useRef<"read" | "leave" | null>(null);
  const verbs = useFileVerbs({ machine, path, here, code, onGone, onMoved });
  useEffect(() => {
    setWriting(false);
    saving.current = null;
  }, [path]);

  const previewable = rendererOf(path) !== null;
  const face = writing ? "source" : previewable ? "preview" : "source";
  const dirty = draft !== undefined;

  // The face changed: put the reader where they were, before paint.
  useLayoutEffect(() => {
    const line = restore.current;
    if (line === null) return;
    restore.current = null;
    placeAt(box.current, line);
  }, [writing]);

  const edit = () => {
    const line = topLine(box.current);
    restore.current = line;
    flushSync(() => setWriting(true));
    // Placed already by the layout effect; focus at that line, in the
    // press's own gesture so a phone raises its keyboard.
    beginEditing(box.current);
    holdWhileKeyboardRises(box.current, line);
  };
  const read = () => {
    restore.current = topLine(box.current);
    setWriting(false);
  };
  const save = (then: "read" | "leave") => {
    (document.activeElement as HTMLElement | null)?.blur?.();
    if (!dirty) {
      if (then === "leave") onBack();
      else read();
      return;
    }
    saving.current = then;
    setSaves((n) => n + 1);
  };
  const saved = () => {
    onSaved?.();
    const then = saving.current;
    saving.current = null;
    if (then === "leave") onBack();
    else if (then === "read") read();
  };
  const leave = () => (dirty ? setAsking(true) : onBack());

  // **The name steps aside while the text moves** and comes back when it
  // rests, or when the text is tapped.
  const [rest, setRest] = useState(true);
  const still = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(still.current), []);
  const moved = () => {
    setRest(false);
    clearTimeout(still.current);
    still.current = setTimeout(() => setRest(true), 700);
  };

  return (
    <div
      data-file-page
      data-writing={writing || undefined}
      className="relative flex min-h-0 flex-1 flex-col bg-card"
      style={fileSizeStyle(verbs.size.step)}
    >
      <div
        ref={box}
        className="flex min-h-0 flex-1 flex-col"
        onScrollCapture={moved}
        onPointerUp={() => setRest(true)}
      >
        <FileView
          key={`${machine} ${path}`}
          machine={machine}
          path={path}
          face={face}
          git={git}
          saveAt={saveAt + saves}
          readAt={readAt}
          goLine={goLine}
          onSaved={saved}
          onNote={onNote}
          // Read first under a finger (P1); a mouse edits on a click, as
          // it always has — 编辑 still opens the source where it reads.
          reading={coarse && !writing}
          onCanEdit={setCan}
          draft={draft}
          onDraft={onDraft}
        />
      </div>
      {/* The name, at the top centre, over the text. */}
      <div
        data-file-name-pill
        data-rest={rest || undefined}
        aria-hidden={!rest || undefined}
        className={cn(
          "pointer-events-none absolute inset-x-0 top-[calc(var(--safe-top)+var(--gap-in))] flex justify-center px-[calc(var(--glass-btn)+var(--page-inset))]",
          "transition-opacity duration-[var(--dur-180)] motion-reduce:transition-none",
          rest ? "opacity-100" : "opacity-0",
        )}
      >
        <span className="glass max-w-full truncate rounded-full px-row py-1 text-callout font-title text-foreground">
          {baseName(path)}
        </span>
      </div>
      {/* The bottom row: ‹ at the left; 分享, 「…」 and 编辑 at the right. */}
      <div
        data-file-deck
        className="pointer-events-none absolute inset-x-0 bottom-[calc(var(--safe-bottom)+var(--gap-in))] flex items-end justify-between px-page"
      >
        <GlassCircle aria-label={gui.back} data-file-back tip={!coarse && gui.back} onClick={leave} className="pointer-events-auto">
          <IconBack />
        </GlassCircle>
        <div className="pointer-events-auto flex items-center gap-in">
          {verbs.said && (
            <span data-file-said className="glass rounded-full px-row py-1 text-callout text-muted-foreground">
              {verbs.said}
            </span>
          )}
          {verbs.shareCircle}
          {verbs.moreCircle}
          {can && (
            <GlassCircle
              aria-label={writing ? gui.watch_save : gui.accounts_edit}
              data-file-edit={writing ? "save" : "edit"}
              tip={!coarse && (writing ? gui.watch_save : gui.accounts_edit)}
              onClick={() => (writing ? save("read") : edit())}
              className={cn(writing && "glass-sel text-[var(--rail-on-ink)]")}
            >
              {writing ? <IconCheck /> : <IconPencil />}
            </GlassCircle>
          )}
        </div>
      </div>
      {verbs.dialogs}
      <CloseDraftDialog
        name={asking ? baseName(path) : null}
        onSave={() => {
          setAsking(false);
          save("leave");
        }}
        onDrop={() => {
          setAsking(false);
          onDraft?.(null);
          onBack();
        }}
        onKeep={() => setAsking(false)}
      />
    </div>
  );
}
