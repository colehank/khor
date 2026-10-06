// What one tab is showing: one machine's directory, painted in the
// node's order (it sorts, this pane never re-sorts — docs/UX.md).
// Navigation is the tree's own: down by pressing a directory, up by the
// 上一级 row, anywhere by the address bar above.
//
// **Where the tab is belongs to the tab, not to this pane** (标签页一期).
// It arrives as `path` and every move is reported back through
// `onPath`, which is what lets the strip title a tab, the address bar
// draw a breadcrumb, and a reload land where the person left off —
// three faces of one fact, and one place that owns it. The pane keeping
// its own idea of where it was is how those three drift apart.
//
// **There is no "back" on either face any more.** There used to be one
// on narrow, back to the machine list; that list is gone, so the
// button had nothing left to return to (docs/UX.md 返回 only exists
// where the list is really off screen). Going elsewhere is the
// switcher; going up is 上一级.
//
// Taking a file is the row's one action. The landing (the downloads
// directory) was chosen silently, so the row says where the file went;
// a failure wears its words on the same row — the pin's rule, because
// this app has nowhere that collects messages.
import { useEffect, useState } from "react";

import { EmptyState } from "@/components/RegionState";
import { fetchLs, type DirListing } from "@/api";
import { RemoveDialog } from "@/components/RemoveDialog";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconBack, IconChevronRight, IconFiles } from "@/components/icons";
import { useNarrow } from "@/hooks/use-narrow";
import { RowMenu } from "@/components/RowMenu";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { Mark, markInk, useRemoveFlow, useTake } from "@/views/fileRow";

/** Bytes as a person reads them. **Exported because the file view
    prints the same ladder** (工作区批2): a row saying 1.2 MB beside a
    banner saying 1.19 MB about one file is a difference a reader has to
    stop and resolve, and two copies of a rounding rule drift on the
    first change to either. */
export function sizeWord(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let u = 0;
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024;
    u += 1;
  }
  return `${v >= 100 ? Math.round(v) : Math.round(v * 10) / 10} ${units[u]}`;
}

export function FilesPane({
  machine,
  path,
  onPath,
  onLanded,
  onFile,
  open,
  here,
  marks,
  onDiff,
  onTop,
  topLabel,
}: {
  /** ‹ at the root goes on here, under `topLabel`'s name (UX-37 §1:
      the 附栏's 工作区 climbs to the root, then to every workspace).
      Absent, the root has no way up, as before. */
  onTop?: () => void;
  topLabel?: string;
  /** The machine's name — what the node resolves. */
  machine: string;
  /** Where to open; "" is that machine's home. */
  path: string;
  /** Where the tab should now be. Called both for a step the person
      took and for the home a `""` resolved to — the tab holds the
      answer, this pane only ever asks. */
  onPath: (path: string) => void;
  /** A directory really opened. Separate from `onPath` because the two
      are different facts: `onPath` fires for an *intention* that may
      still be refused, this one only after a listing came back. */
  onLanded: (path: string) => void;
  /** A file was picked, or the open one went away (`null`, 工作区批④
      — deleting the file somebody is reading has to take the reading
      with it). The tree says which; what happens to it belongs to
      whoever holds this pane (工作区批2). */
  onFile: (path: string | null) => void;
  /** Which file is open, so the row can say so. Absolute, or null. */
  open: string | null;
  /** Whether the machine this tree is reading is the one khor is
      running on.

      **拉过来 means「from that machine to this one」, so on this one it
      has nothing to mean.** It was drawn on every row of every tree
      until 2026-09-13, which on a local workspace offered to copy a
      file into the downloads directory it was already reachable from
      — an action whose whole word is a direction that does not exist
      here. Nothing about it is specific to workspaces: browsing your
      own machine had the same button, for the same reason.

      Passed rather than worked out here: which machine is this one is
      the shell's fact (`DeviceRow::me`), and a pane that fetched the
      device table to answer it would be a second reading of something
      already on screen. */
  here: boolean;
  /** Absolute path → git's mark for it, folded so that a directory
      carries one when anything under it does (`WorkspacePane`).
      **`null` is「nobody asked」or「nobody could answer」, which is not
      the same as an empty map** — an empty map says every file here
      matches HEAD, and a tree khor could not read must not claim that.
      Absent entirely when this pane is not inside a workspace. */
  marks?: Map<string, string> | null;
  /** git's letter beside a file was pressed: open that file against
      HEAD (工作区第五轮 第 38 条). Absent where there is nowhere to open
      one — browsing a machine's whole disk, which is not a checkout —
      and the letter stays a letter then. */
  onDiff?: (path: string) => void;
}) {
  const [listing, setListing] = useState<DirListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { took, take, takes } = useTake(machine);
  /** Bumped when this pane changed the directory under itself, which is
      the one change its own effect cannot notice: the path did not
      move, so nothing in the deps would. */
  const [again, setAgain] = useState(0);

  // The pane opens where the tab points — "" asks for home, and the
  // answer carries the real path, which is handed straight back so the
  // tab stops saying "home" and starts saying where that is.
  //
  // **The listing it already has is not asked for again**, which is
  // what keeps that hand-back from being a second round trip: the
  // resolved path arrives as a new `path`, and this sees it is the one
  // already on screen.
  useEffect(() => {
    let stopped = false;
    // Already here. Clearing the refusal is part of that: coming back
    // to a directory that is on screen must not leave the sentence
    // about the one that could not be reached standing over it.
    //
    // **`again` gets past this on purpose**: after a removal the path
    // is the same path and the listing on screen is the stale one, so
    // the pane drops it (`reread`) and this sees nothing to keep.
    if (listing && listing.path === path) {
      setError(null);
      return;
    }
    fetchLs(machine, path)
      .then((l) => {
        if (stopped) return;
        setListing(l);
        setError(null);
        if (l.path !== path) onPath(l.path);
        // **After a real listing, with the machine's own spelling of the
        // path rather than whatever was typed.** What the shell does
        // with that — records a visit, re-reads its places — is the
        // shell's business (`BrowseLanding`); this only says that
        // something really opened.
        onLanded(l.path);
      })
      // The message is the node's, in the catalog's words.
      .catch((e) => {
        if (!stopped) setError(String(e instanceof Error ? e.message : e));
      });
    return () => {
      stopped = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, path, again]);


  const narrow = useNarrow();
  const up = () => {
    if (!listing) return;
    onPath(listing.path.slice(0, listing.path.lastIndexOf("/")) || "/");
  };

  /** Read this directory again. The listing is dropped first so the
      effect above has nothing to short-circuit on. */
  const reread = () => {
    setListing(null);
    setAgain((n) => n + 1);
  };

  // 删掉, shared with the workspace's tree (`views/fileRow`). What this
  // pane adds is what happens afterwards: the directory is re-read, and
  // a file somebody was reading stops being open — a view onto a file
  // that is no longer there is the one state neither tree may leave
  // behind.
  const { actions, dialog } = useRemoveFlow({
    machine,
    here,
    onGone: (full) => {
      if (open === full) onFile(null);
      reread();
    },
  });

  // No header: which machine and where on it are the address bar's, and
  // a second copy of them here would be two spellings of one fact one
  // poll apart. What is left is the listing.
  return (
    <section className="flex h-full min-w-0 flex-col">
      <div data-files-list className="min-h-0 flex-1 overflow-y-auto">
        {error ? (
          // **The one thing this must never be is blank.** Enter in the
          // address bar says where to go; arriving is a different fact,
          // and when it does not happen the refusal is the whole of what
          // this pane has to say — in the node's own sentence, at the
          // size of something meant to be read, not as a footnote under
          // an empty directory that was never opened.
          <div data-cannot-reach className="p-4 text-body">
            {error}
          </div>
        ) : !listing ? null : (
          <>
            {(listing.path !== "/" || onTop) && (
              <button
                type="button"
                data-up
                data-up-top={listing.path === "/" ? "" : undefined}
                onClick={listing.path === "/" ? onTop : up}
                // The floor under a finger (tokens.css `--touch`), which
                // this row was under by a third on its height alone. It
                // is a whole row wide, so only the short way needed
                // raising, and the wash it lights up with is painted on
                // the row itself — grown, the lit box and the pressable
                // box stay the same box.
                // **Where this is, in 上一级's place** (the audit fix plan
                // F4.2; N08, W16): the way up is a ‹ before the current
                // directory's name, so the listing says which directory
                // it is; 上一级 is the button's name.
                aria-label={listing.path === "/" ? topLabel : gui.up_dir}
                title={listing.path}
                // In the entries' anatomy (W-V16): the ‹ in the kind's 20
                // box, so the name starts where theirs do, and the
                // listing's hairline from the names (`app.css`).
                data-files-here-row
                className="flex min-h-touch w-full items-center gap-2 px-4 text-left text-callout text-muted-foreground hover:bg-muted/50"
              >
                <span className="flex w-5 flex-none justify-center">
                  <IconBack className="size-affordance" />
                </span>
                <span data-files-here className="min-w-0 truncate font-label text-foreground">
                  {listing.path.split("/").filter(Boolean).pop() ?? listing.path}
                </span>
              </button>
            )}
            {listing.entries.length === 0 && (
              <EmptyState data-empty>{gui.empty_dir}</EmptyState>
            )}
            {listing.entries.map((e) => {
              const full = `${listing.path === "/" ? "" : listing.path}/${e.name}`;
              const faded = e.name.startsWith(".");
              return e.dir ? (
                <div
                  key={e.name}
                  // **One gutter for the whole listing, and 16 is the
                  // number the rest of this pane already uses** — the
                  // 上一级 row, the refusal, the empty word and the
                  // truncation note are all `p-4`, and every row's name
                  // starts at 16 from the left. This row was the only
                  // thing at 8, so a directory's pin sat half a step
                  // closer to the edge than a file's 拿过来 right under
                  // it, and the right edge of the list wobbled at
                  // exactly the row where the listing changes from
                  // directories to files. Measured 8 against 16.
                  //
                  // **The 8 goes to the name, not to a spacer.** The
                  // other way to line these up is to give the file row a
                  // hole the size of a pin, and that is worse than the
                  // wobble: it is blank space defended by a rule.
                  // One row height and one hairline with the files under
                  // it (F4.2; W16).
                  className={cn(
                    "group flex min-h-touch items-center pr-4 hover:bg-muted/50",
                    faded && "text-muted-foreground",
                  )}
                >
                  {/* **The menu hangs on the button, never on the row
                      around it.** `RowMenu`'s trigger carries an
                      `onPointerDown` — that is the long press — and a
                      press handler on a bare `<div>` is a target only a
                      pointer can find, which `check-face`'s section 4
                      catches by name (it caught this one). On the
                      button the handler sits on something focus reaches
                      and a keyboard's Menu key can fire `contextmenu`
                      on — the same shape `SessionsList` uses. */}
                  <RowMenu items={actions(full, true)}>
                  <button
                    type="button"
                    data-dir={e.name}
                    onClick={() => onPath(full)}
                    // Raised to the floor here rather than on the row
                    // around it: **this button is the target** — the row
                    // is a wrapper that also holds the pin — and a row
                    // grown from outside would have left the pressable
                    // half its old height inside a taller stripe that
                    // lights up. The hover wash is the row's, and the
                    // row is exactly as tall as this.
                    // Stretched to the row, which wears the floor with its
                    // hairline inside it — the file row's arrangement, so
                    // the two are one height (F4 review; a floor here too
                    // made the folder row its hairline taller).
                    className="flex min-w-0 flex-1 items-center gap-2 self-stretch px-4 text-left text-callout"
                  >
                    {/* **A folder has its glyph** (F4.2; N08, W16), so every
                        name in the listing starts on one x — **in the file
                        kind's own 20 box** (F4 review): the rail glyph
                        keeps its own size whatever class it is handed, so
                        it stood 32 wide, the row 33 tall against a file's
                        28 and the name 12 further right. */}
                    <span aria-hidden="true" className="inline-flex size-5 flex-none text-muted-foreground [&>svg]:size-full">
                      <IconFiles />
                    </span>
                    <span className="min-w-0 flex-1 truncate">{e.name}</span>
                    <Mark mark={marks?.get(full)} dir />
                    {narrow && <IconChevronRight className="size-affordance flex-none text-tertiary" />}
                  </button>
                  </RowMenu>
                  {/* ⚠️ 墓碑: a pin button stood here, writing 「置顶的目录」
                      — retired with that column (docs/SHELL.md 第六轮).
                      A directory kept on purpose is a workspace, and the
                      workspace list's own 置顶 is where it is pinned. */}
                </div>
              ) : (
                <div
                  key={e.name}
                  data-file={e.name}
                  // **The floor is worn, not padded around.** A file row
                  // holds a control that already carries `min-h-touch`,
                  // so `py-2` on the row was 8 + 44 + 8: measured 60
                  // against a directory row's 44, and since a directory
                  // listing draws its directories first, the list's
                  // rhythm broke at the first file and stayed broken.
                  // The row is the floor's height and the button fills
                  // it, which is the same arrangement the directory row
                  // above arrived at from the other side.
                  //
                  // **`min-h-touch` on the row rather than only on the
                  // button**, because the button is not always there:
                  // once a file has been taken the row is a word, and a
                  // row that shrank to the height of its text would put
                  // the break back in the same list — one row further
                  // down and only after a press, which is the harder
                  // version of the same bug to see.
                  className={cn(
                    "group flex min-h-touch items-center gap-3 px-4 text-callout",
                    faded && "text-muted-foreground",
                    // The open file wears the list's chosen block (F4.2;
                    // W16), not bold.
                    open === full && "rounded-md border-transparent bg-fill-strong text-foreground",
                  )}
                >
                  {/* **The name is the way in now** (工作区批2). It
                      used to be a word beside a 取下来 button, because
                      the only thing a browsing tab could do with a file
                      was take it. A workspace can read one, so the row
                      opens it and taking stays where it was — pressing
                      a file and pressing the button next to it are two
                      different asks, and the bigger target is the one
                      people mean. */}
                  <RowMenu items={actions(full, false)}>
                  <button
                    type="button"
                    onClick={() => onFile(full)}
                    // **The name wears the letter's ink** (工作区批⑦):
                    // 「颜色与字符表示」 is one decoration in two
                    // places, and a coloured letter beside a grey name
                    // is the half of VS Code's arrangement that does
                    // not survive being read from across the column.
                    // Last in the list so it wins the open row's
                    // `text-foreground` — a file's git state does not
                    // stop being true because somebody is reading it.
                    className={cn(
                      "flex min-w-0 flex-1 items-center gap-2 self-stretch text-left",
                      open === full && "text-foreground",
                      markInk(marks?.get(full)),
                    )}
                  >
                    {/* The same kind icon the workspace tree draws
                        (2026-09-16): a file is the same file whichever
                        tree it was found in. **20, not the tree's 24**:
                        this pane's rows are the finger's floor rather
                        than 28, and its text is the ordinary `text-callout`
                        — the icon is sized to the line it sits on, and
                        「基本占满这一行」 is the rule both are obeying,
                        not one number carried across. */}
                    <FileKindIcon name={e.name} className="size-5" />
                    <span className="min-w-0 truncate">{e.name}</span>
                  </button>
                  </RowMenu>
                  <Mark
                    mark={marks?.get(full)}
                    onPress={onDiff === undefined ? undefined : () => onDiff(full)}
                  />
                  {took?.path === full ? (
                    <span
                      data-took={took.state}
                      className={cn(
                        "max-w-[50%] truncate text-caption",
                        took.state === "failed" ? "text-state-failed" : "text-muted-foreground",
                      )}
                    >
                      {took.word}
                    </span>
                  ) : (
                    <>
                      {/* **The size and the mtime left this row on
                          2026-09-11** (工作区批3), and they left because
                          the row moved house. They were a browsing tab's
                          facts, on a pane that had the width of a page;
                          this pane is now a project tree in a 224-wide
                          column, and five things had to share it — name,
                          git mark, size, time, 取下来. Measured at 1400
                          with the mark added: **the name button came to
                          26 wide**, under the 44 floor, and the name is
                          the row's whole point.
                          Nothing was hidden to make room: a file's byte
                          count and its mtime answer 「should I take
                          this」, which is the browsing question, and the
                          tree's question is 「which file」. The size is
                          still said where taking is decided — the file
                          view's own head says it for a binary and for a
                          cut one. */}
                      {!here && takes && (
                      <Button
                        size="sm"
                        variant="ghost"
                        data-take={e.name}
                        // Always drawn where it means anything, unlike
                        // the pin above: it is the row's one *remote*
                        // action, a word rather than a repeated glyph.
                        // On this machine it is not drawn at all — see
                        // `here`; the row still has the thing it is for,
                        // which is opening the file.
                        //
                        // Both floors, though only the short one bites
                        // today: the `sm` height is well under it, while
                        // the word makes the button wide enough on its
                        // own. The width floor is here so that a shorter
                        // word in the catalog cannot quietly take the
                        // target back under.
                        className="min-h-touch min-w-touch flex-none"
                        onClick={() => take(full)}
                      >
                        {gui.take_file}
                      </Button>
                      )}
                    </>
                  )}
                </div>
              );
            })}
            {/* One dialog for the whole listing, not one per row: what
                is being asked about is state, and a sheet per row would
                be one mounted component per file in the directory. */}
            <RemoveDialog {...dialog} />
            {listing.truncated && (
              // The cap is however many rows actually came — the number
              // belongs to the node, and a copy here would drift.
              <div className="p-4 text-caption text-muted-foreground">
                {cli.dir_truncated(listing.entries.length)}
              </div>
            )}
          </>
        )}
      </div>
    </section>
  );
}

