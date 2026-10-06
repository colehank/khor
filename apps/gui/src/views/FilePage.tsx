/**
 * **One file, full screen** (round UX-3, the owner's ruling 2026-10-04:
 * every full-screen content page gives the screen to the content; nothing
 * sits top-left; what is not content floats at the bottom).
 *
 * The phone's file — out of a workspace's tree or a machine's files —
 * a machine's in-place file on every platform, and a file opened in the
 * session's 附栏 on both faces (`docked`, UX-40 §4). Nothing at the top:
 * the file starts under the status bar. At the bottom the content pages'
 * deck (UX-14, the owner 2026-10-05): ‹ · a pill with the name, the
 * preview eye where the file renders, and ✓ while the buffer differs from
 * the file · 分享 · 「…」. It folds into the title strip while the file
 * is read (UX-13) and stays over the keyboard with ✓ in reach.
 *
 * **No edit mode** (UX-14): a text file is editable as soon as it is open
 * (Notes) — a tap puts the caret where the finger is, a drag scrolls, the
 * keyboard's rise does not move the text. A file that renders opens
 * rendered and the eye turns it to its source at the same place. Saving
 * is the ✓; leaving keeps the draft (the row carries its dot), and
 * nothing asks. A Mac clicks into the text and saves with ⌘S, as before.
 * ⚠️ 墓碑: the name pill at the top centre (it faded while the text
 * moved), the 编辑 circle that turned into ✓ 保存, and the question on
 * leaving with a draft.
 *
 * Nothing is drawn that cannot be used: no ✓ on a file that is not whole
 * here (a picture, a cut file), no share where nothing can take a file,
 * no VS Code where the machine has none.
 */
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";

import { launchOn, pullTemp, renamePath, reveal } from "@/api";
import type { CodeWindow } from "@/components/CodeWindow";
import { GlassCircle, GlassItem } from "@/components/Glass";
import { DeckBand, TabSheet, TabsGlyph } from "@/components/SessionBar";
import type { TabFace } from "@/browse/TabStrip";
import { IconBack, IconBranch, IconCheck, IconEye, IconMore, IconReload, IconShare } from "@/components/icons";
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
import type { GitCommit } from "@/gen/bindings/GitCommit";
import { useCoarse } from "@/hooks/use-coarse";
import { FOLD_AT, foldStep, groundUnder, inkOver, type DeckInk, type FoldMemo } from "@/lib/deck-fold";
import { onKeyboardChange } from "@/lib/soft-keyboard";
import { keyboardShown } from "@/lib/viewport";
import { useBlameShown } from "@/lib/blame";
import { FILE_STEPS, fileSizeStyle, useFileSize } from "@/lib/file-size";
import { groundAttr, GROUNDS, usePreviewGround } from "@/lib/preview-ground";
import { baseName } from "@/lib/paths";
import { cn } from "@/lib/utils";
import { bytesOf, revealHere, shares } from "@/views/chat-files";
import { HistorySheet, RevisionView, shortId } from "@/views/FileHistory";
import { useRemoveFlow } from "@/views/fileRow";
import { FileView, kindOf, rendererOf, type Draft } from "@/views/FileView";

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

/* ⚠️ 墓碑 (UX-14): `holdWhileKeyboardRises` put the source back at the
   line 编辑 opened it on while the keyboard rose (round UX-3: WebKit
   revealed the whole-file editor from its top). There is no 编辑 now; the
   page's `hold` keeps the scroller where the tap left it instead. */

/** How long the place is held after a tap: the keyboard's rise, with room. */
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
  history = false,
  onRevision,
  previewing,
  shareInMore = false,
  moreActs = [],
}: {
  /** The page's own additions at the head of 「…」 (`FilePage` `moreActs`). */
  moreActs?: { key: string; label: string; run: () => void }[];
  /** The file page's face (UX-14): 字号 and 预览背景 are offered only
      while the file is shown rendered. Left out (the Mac's workspace
      strip), they are offered as before. */
  previewing?: boolean;
  /** **分享 inside 「…」** (UX-33 §4, the phone's deck: ‹ · pill · 预览 ·
      「…」): the share circle is not drawn and its acts head the menu. */
  shareInMore?: boolean;
  machine: string;
  path: string;
  here: boolean;
  code?: CodeWindow | null;
  onGone: (path: string) => void;
  onMoved: (from: string, to: string) => void;
  /** The file is in a checkout: git 历史 and blame are offered (round UX-4). */
  history?: boolean;
  /** A revision picked from git 历史, to show where the file is. */
  onRevision?: (c: GitCommit) => void;
}) {
  const coarse = useCoarse();
  const [finder, setFinder] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);
  const size = useFileSize();
  const blame = useBlameShown();
  const ground = usePreviewGround();
  const [historyOpen, setHistoryOpen] = useState(false);
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
    ...moreActs,
    ...(shareInMore && finder
      ? [
          { key: "open-other", label: gui.file_open_other, run: () => void local().then((p) => launchOn("", p)).catch(fail) },
          { key: "reveal", label: gui.chat_file_reveal, run: () => void local().then((p) => reveal(p)).catch(fail) },
        ]
      : shareInMore && sharing
        ? [{ key: "share", label: gui.chat_file_share, run: share }]
        : []),
    {
      key: "copy-path",
      label: gui.tab_copy_path,
      run: () => void navigator.clipboard.writeText(path).then(() => setSaid(gui.copied), () => setSaid(gui.copy_failed)),
    },
    { key: "rename", label: gui.rename_ask, run: () => setRenaming(true) },
    ...(history && onRevision ? [{ key: "history", label: gui.file_history, run: () => setHistoryOpen(true) }] : []),
    // Blame is said at the end of the caret's line (UX-23), which a phone
    // has room for as well.
    ...(history
      ? [{ key: "blame", label: blame.on ? gui.blame_hide : gui.blame_show, run: () => blame.set(!blame.on) }]
      : []),
    ...(code?.offered ? [{ key: "code", label: gui.file_in_code, run: () => code.want() }] : []),
    ...removal.actions(path, false).map((a) => ({ key: a.key, label: a.label, run: () => void a.run(), destructive: true })),
  ];
  const shareCircle: ReactNode = shareInMore ? null : finder ? (
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
        {(textKind || previewKind) && kind !== "image" && kind !== "svg" && kind !== "pdf" && previewing !== false && (
          <div data-file-size className="flex min-h-menu-item items-center gap-in px-row text-body">
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
        {/* 预览背景, for a document that renders (round UX-4): three keys on
            one row like 字号's, the one in use pressed. */}
        {rendererOf(path) !== null && previewing !== false && (
          <div data-preview-ground className="flex min-h-menu-item items-center gap-in px-row text-body">
            <span className="flex-1">{gui.preview_ground}</span>
            {GROUNDS.map((g) => (
              <Button
                key={g}
                size="sm"
                aria-pressed={ground.ground === g}
                data-preview-ground-key={g}
                onClick={() => ground.set(g)}
                className={cn(ground.ground === g && "glass-sel text-[var(--rail-on-ink)]")}
              >
                {g === "auto" ? gui.preview_ground_auto : g === "light" ? gui.preview_ground_light : gui.preview_ground_dark}
              </Button>
            ))}
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
      {history && onRevision && (
        <HistorySheet open={historyOpen} onOpenChange={setHistoryOpen} machine={machine} path={path} onPick={onRevision} />
      )}
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
  return { shareCircle, moreCircle, dialogs, said, size, ground: ground.ground };
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
  history = false,
  tabs,
  moreActs,
  docked = false,
}: {
  /** **Inside the session's 附栏, with the main area's deck** (UX-40 §4,
      the owner 2026-10-07 02:10: 「打开一个文件或地址，是不是得和主区一样，
      下方有一个自己的甲板，而不是在左上角点返回和显示标题」): the same
      deck at the sheet's foot (the Mac column's too), never folded — the
      sheet is small already. The 附栏's one head stays above the file;
      nothing else at its top. ⚠️ 墓碑 (UX-37 §6 → UX-40): `bare`, the
      page with no deck — ‹ · the name · ✓ · the preview switch · 「…」 in
      a head at the page's top (`SeatHead`). */
  docked?: boolean;
  /** Acts the page's host adds at the head of 「…」 (UX-30 §4: the
      session's 附栏 offers 「在主区打开」, the one way out of the column). */
  moreActs?: { key: string; label: string; run: () => void }[];
  /** The file is in a checkout: git 历史 is offered (round UX-4). */
  history?: boolean;
  /** **The workspace's open files, as the session page's tabs** (UX-25,
      one tab language): the deck's tabs circle opens the same sheet the
      session deck has — each file's kind, its name, × — and a sideways
      swipe on the pill steps through them. No ＋: a file is opened from
      the tree, and nothing a bare new tab could open. Absent where the
      page stands alone (a machine's in-place file). */
  tabs?: { faces: TabFace[]; active: string; onFocus: (id: string) => void; onClose: (id: string) => void };
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
  const [can, setCan] = useState(false);
  const [saves, setSaves] = useState(0);
  /** Where to put the reader after the face changes (a source line). */
  const restore = useRef<number | null>(null);
  /** **A revision from git 历史, shown where the file was** (round UX-4):
      read-only, the deck's ‹ going back to the file as it is, and the
      pill's compare switch putting it beside the current file. */
  const [rev, setRev] = useState<GitCommit | null>(null);
  const [compare, setCompare] = useState(false);
  const previewable = rendererOf(path) !== null;
  /** **No edit mode** (UX-14, the owner 2026-10-05: 「像备忘录」): a text
      file is editable as soon as it is open — a tap puts the caret where
      the finger is, a drag scrolls. A file that renders opens rendered;
      the pill's eye is the one switch between the two faces, and it keeps
      the place. */
  const [face, setFace] = useState<"source" | "preview">(previewable ? "preview" : "source");
  const verbs = useFileVerbs({
    machine,
    path,
    here,
    code,
    onGone,
    onMoved,
    history,
    onRevision: (c) => {
      setRev(c);
      setCompare(false);
    },
    // 字号 and 预览背景 only while the file is read rendered (UX-14).
    previewing: previewable ? face === "preview" : false,
    // The phone's deck: 分享 inside 「…」 (UX-33 §4).
    shareInMore: true,
    moreActs,
  });
  /** A re-read asked for from the pill's ↻ (UX-33), added to the one asked
      from outside. */
  const [reads, setReads] = useState(0);

  useEffect(() => {
    setFace(rendererOf(path) !== null ? "preview" : "source");
    setRev(null);
  }, [path]);
  const dirty = draft !== undefined;

  // The face changed: put the reader where they were, before paint.
  useLayoutEffect(() => {
    const line = restore.current;
    if (line === null) return;
    restore.current = null;
    placeAt(box.current, line);
  }, [face]);
  const turn = () => {
    restore.current = topLine(box.current);
    setFace((f) => (f === "preview" ? "source" : "preview"));
  };
  /** **Saving is explicit** (UX-14): the ✓ at the pill's end, drawn while
      the buffer differs from the file. Leaving keeps the draft (its row
      carries the dot); nothing asks. */
  const save = () => setSaves((n) => n + 1);

  // **The deck folds while the file is read** (UX-13, `lib/deck-fold`): a
  // file rests at its top. Nothing folds with the keyboard up.
  const [folded, setFolded] = useState(false);
  const memo = useRef<FoldMemo>({ turned: null });
  const lastTop = useRef(0);
  useEffect(() => {
    setFolded(false);
    memo.current = { turned: null };
    lastTop.current = 0;
  }, [path, face, rev]);
  useEffect(
    () =>
      onKeyboardChange(() => {
        if (keyboardShown()) setFolded(false);
      }),
    [],
  );
  /** **The capsule's ink from what is behind it** (UX-29): the file's
      ground under the capsule's centre — a preview's 预览背景, the code's
      ground — sampled as the fold happens and as the reading goes on. */
  const deck = useRef<HTMLDivElement>(null);
  const [ink, setInk] = useState<DeckInk>("dark");
  const sampledAt = useRef(0);
  const sample = () => {
    const cap = deck.current?.querySelector<HTMLElement>("[data-deck-strip]");
    if (!cap) return;
    const b = cap.getBoundingClientRect();
    setInk(inkOver(groundUnder(b.left + b.width / 2, b.top + b.height / 2, deck.current)));
  };
  useLayoutEffect(() => {
    if (folded) sample();
  }, [folded, face, rev]);
  const scrolled = (e: React.UIEvent) => {
    const t = e.target as HTMLElement;
    if (!t.matches("[data-file-scroll], [data-file-preview]")) return;
    const top = t.scrollTop;
    const leaving = top > lastTop.current;
    lastTop.current = top;
    if (keyboardShown()) return;
    const next = foldStep(memo.current, top, leaving, FOLD_AT);
    if (next !== null) setFolded(next);
    if (folded && e.timeStamp - sampledAt.current > 250) {
      sampledAt.current = e.timeStamp;
      sample();
    }
  };
  /** **The tap's caret stays where the finger is** (UX-14): WebKit reveals
      the focused editor as the keyboard rises, and the editor is the
      whole file, so it scrolled; for a moment after a tap puts the caret
      in, every jump and every resize of the visible area put the scroller
      back where it was. */
  const hold = (e: React.FocusEvent) => {
    if (!coarse || !(e.target as HTMLElement).matches("textarea[data-edit]")) return;
    const s = box.current?.querySelector<HTMLElement>("[data-file-scroll]");
    if (!s) return;
    const at = s.scrollTop;
    const put = () => {
      if (Math.abs(s.scrollTop - at) > 1) s.scrollTop = at;
    };
    const vv = window.visualViewport;
    s.addEventListener("scroll", put);
    vv?.addEventListener("resize", put);
    window.setTimeout(() => {
      s.removeEventListener("scroll", put);
      vv?.removeEventListener("resize", put);
    }, HOLD_MS);
  };
  const name = rev === null ? baseName(path) : `${baseName(path)} · ${shortId(rev)}`;
  const [sheet, setSheet] = useState(false);

  return (
    <div
      data-file-page
      data-folded={folded || undefined}
      // **预览背景 covers the page, not only the text** (round UX-4): the
      // deck floats over the document, so it wears the ground it is read
      // on, or it would be the app's ink on a ground that is not the
      // app's.
      {...(rev === null ? face === "preview" : !compare && rendererOf(path) === "md") ? groundAttr(verbs.ground) : {}}
      className="relative flex min-h-0 flex-1 flex-col bg-card"
      style={fileSizeStyle(verbs.size.step)}
    >
      <div
        ref={box}
        className="flex min-h-0 flex-1 flex-col"
        onScrollCapture={docked ? undefined : scrolled}
        onFocusCapture={hold}
      >
        {rev !== null ? (
          <RevisionView machine={machine} path={path} rev={rev} compare={compare} narrow />
        ) : (
        <FileView
          key={`${machine} ${path}`}
          machine={machine}
          path={path}
          face={face}
          git={git}
          saveAt={saveAt + saves}
          readAt={readAt + reads}
          goLine={goLine}
          onSaved={onSaved}
          onNote={onNote}
          onCanEdit={setCan}
          draft={draft}
          onDraft={onDraft}
        />
        )}
      </div>
      {/* **The content pages' deck** (UX-14): ‹ · the pill (the name; the
          preview eye where the file renders; ✓ while it differs from the
          disk) · 分享 · 「…」. Nothing at the top of the page. Folded, the
          strip with the name; a tap brings the deck back. */}
      <div
        ref={deck}
        data-file-deck
        className="pointer-events-none absolute inset-x-0 bottom-[var(--safe-bottom)] flex flex-col items-stretch"
      >
        {verbs.said && (
          <span data-file-said className="glass pointer-events-auto mx-auto mb-in rounded-full px-row py-1 text-callout text-muted-foreground">
            {verbs.said}
          </span>
        )}
        {folded ? (
          // **Safari's capsule** (UX-29): the name alone, centred, on glass
          // over the file, its ink from the ground behind it. ⚠️ 墓碑
          // (UX-13/14): a 24-pt strip across the width.
          <button
            type="button"
            data-deck-strip
            data-deck-ink={ink}
            onClick={() => setFolded(false)}
            className="glass pointer-events-auto mx-auto mb-in flex h-deck-strip max-w-[calc(100%-2*var(--page-inset))] flex-none animate-in items-center justify-center overflow-hidden rounded-full px-row text-callout fade-in"
          >
            <span className="min-w-0 truncate">{name}</span>
          </button>
        ) : (
          <div className="pointer-events-auto flex h-[calc(var(--glass-btn)+2*var(--gap-in))] flex-none items-center gap-in px-row">
            <GlassCircle
              aria-label={rev === null ? gui.back : gui.file_rev_back}
              data-file-back
              tip={!coarse && (rev === null ? gui.back : gui.file_rev_back)}
              onClick={rev === null ? onBack : () => setRev(null)}
            >
              <IconBack />
            </GlassCircle>
            {/* **The pill: 头像 · 名称 · 对齐位** (UX-33): the band of the
                workspace's open files (one at rest, a drag pulls one
                neighbour in), or the name alone where the page stands
                alone; the third seat ↻ reads the file from the disk again
                when clean and is ✓ save while it differs. A revision keeps
                its compare switch there. ⚠️ 墓碑 (UX-14): name · 预览 eye ·
                ✓ — 预览 is the circle beside the pill now. */}
            <div data-file-pill className="glass flex h-glass-btn min-w-0 flex-1 items-center gap-in rounded-full ps-1 pe-1">
              {tabs && rev === null ? (
                <DeckBand faces={tabs.faces} active={tabs.active} onFocus={tabs.onFocus} />
              ) : (
                <span data-file-name className="min-w-0 flex-1 truncate ps-2 text-body">
                  {name}
                </span>
              )}
              {rev !== null ? (
                <GlassItem
                  aria-label={compare ? gui.file_rev_alone : gui.file_rev_compare}
                  aria-pressed={compare}
                  data-file-rev-compare
                  onClick={() => setCompare(!compare)}
                  className={cn(compare && "glass-sel text-[var(--rail-on-ink)]")}
                >
                  <IconBranch />
                </GlassItem>
              ) : dirty && can ? (
                <GlassItem aria-label={gui.watch_save} data-file-save data-tab-act="save" onClick={save} className="text-brand-strong">
                  <IconCheck />
                </GlassItem>
              ) : (
                <GlassItem aria-label={gui.file_reread} data-file-reread data-tab-act="reload" onClick={() => setReads((n) => n + 1)}>
                  <IconReload />
                </GlassItem>
              )}
            </div>
            {rev === null && previewable && (
              <GlassCircle
                aria-label={gui.file_face_preview}
                aria-pressed={face === "preview"}
                data-file-face-toggle={face}
                tip={!coarse && gui.file_face_preview}
                onClick={turn}
                className={cn(face === "preview" && "glass-sel text-[var(--rail-on-ink)]")}
              >
                <IconEye />
              </GlassCircle>
            )}
            {rev === null && verbs.shareCircle}
            {rev === null && verbs.moreCircle}
            {rev === null && tabs && (
              <GlassCircle
                data-deck-tabs
                data-tab-count
                aria-label={gui.switch_tabs(tabs.faces.length)}
                aria-expanded={sheet}
                tip={!coarse && gui.switch_tabs(tabs.faces.length)}
                onClick={() => setSheet(true)}
              >
                <TabsGlyph n={tabs.faces.length} />
              </GlassCircle>
            )}
          </div>
        )}
      </div>
      {tabs && (
        <TabSheet
          open={sheet}
          onOpenChange={setSheet}
          faces={tabs.faces}
          active={tabs.active}
          onFocus={(id) => {
            setSheet(false);
            tabs.onFocus(id);
          }}
          onClose={tabs.onClose}
        />
      )}
      {verbs.dialogs}
    </div>
  );
}
