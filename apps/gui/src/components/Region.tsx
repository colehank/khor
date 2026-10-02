// **The right side of the window, every page** (docs/SHELL.md 第十五轮
// 第 5、6 条; docs/DESIGN.md 第五节「三种页型」).
//
// Two rows, never more: **R0**, a first row at `--pane-head` — the same
// token the left block's row wears, so the two are one line across the
// window — and under it **R1, a board floating on the app's ground**
// (`--card`, `--gap-in` from the region's other three edges, `--radius-xl`
// at the corners). A page fills them; it does not add a row, take one
// away or change a height.
//
// **One component, one prop saying which of three pages this is**:
//
//   detail  A 详情 — pick one thing, see it. R0 is `PageHead`: the
//           page's name, the page's own
//           buttons at the right end. 机器, 额度, 「…」, 工作区.
//   tabs    B 标签 — open several, switch. R0 is the tab strip; with no
//           tab open it is `PageHead` with the landing's name. 会话.
//   doc     C 文档 — the row *is* a live document. R0 is its address bar.
//           浏览.
//
// The kind is written on the region's root (`data-region-kind`) so a
// gate can find every page by what it claims to be.
//
// ⚠️ 墓碑 (第十四轮 第 3 条, 2026-09-24 → 2026-09-25): 「没有标签的页没有
// 行」. A page with no tabs had no R0 and its board started at the top,
// and Windows got a thin row of its own for its three buttons. D2
// (2026-09-25) put R0 back on every page — with something in it this
// time, a way back and a name, like System Settings — so the three are
// at R0's right end again on every page and the thin row is gone.
//
// **The open tab is the board's colour and joins it** — that is the
// whole of how a tab says it is in front (`TabStrip`): its bottom edge
// has no edge, and the board is what it is continuous with.
import { createContext, useContext, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { windowClose, windowMaximizeToggle, windowMinimize, windowsApp } from "@/api";
import { IconClose } from "@/components/icons";
import { WINDOW_DRAG } from "@/components/shell";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * **Windows' habit, drawn at the right end of R0 because Windows has no
 * buttons of its own over it** (第十轮, moved here in 第十三轮):
 * minimise, maximise, close — close turning red under the pointer, the
 * way every window on that desktop behaves. Only that platform draws
 * them: macOS has the system's at the left block's row and Linux keeps
 * its decorations.
 */
export function WindowControls() {
  if (!windowsApp) return null;
  return (
    <div data-window-controls className="flex flex-none items-center self-stretch">
      {[
        { key: "minimize", label: gui.window_minimize, run: windowMinimize, mark: "—" },
        { key: "maximize", label: gui.window_maximize, run: windowMaximizeToggle, mark: "▢" },
        { key: "close", label: gui.window_close, run: windowClose, mark: null },
      ].map((b) => (
        <button
          key={b.key}
          type="button"
          data-window-control={b.key}
          aria-label={b.label}
          onClick={() => void b.run().catch(() => {})}
          className={cn(
            "flex h-full w-12 items-center justify-center text-caption",
            b.key === "close" ? "hover:bg-destructive hover:text-on-destructive" : "hover:bg-secondary",
          )}
        >
          {b.mark ?? <IconClose />}
        </button>
      ))}
    </div>
  );
}

/**
 * Where a page's own buttons go (D9: page buttons live at R0's right
 * end). `undefined` is 「no R0 around this」 — the narrow face, where a
 * page's head is its own (`NarrowHead`) — and there the buttons stay
 * where they were written.
 */
/**
 * **What stands at the start of the content's first row** (the redesign,
 * 方案 B): the wide face's landings' strip, while the list column is
 * folded away — it stays at the window's top-left. Provided by the shell
 * around the content and taken by the first `Region` it reaches; a
 * region inside that one gets none.
 */
export const RegionLead = createContext<ReactNode>(null);

const ActionsSeat = createContext<HTMLElement | null | undefined>(undefined);

/**
 * **A page's button, drawn in R0 by the page that owns it** (2026-09-28,
 * 安全's 「立即锁定」). A 「…」 page is built inside the board, and the
 * one row that holds page buttons is R0 above it — which the shell
 * draws. This lets the page say what its buttons are and the shell say
 * where they stand, the way `SideSeat` does for a landing's search.
 */
export function PageAction({ children }: { children: ReactNode }) {
  const seat = useContext(ActionsSeat);
  if (seat === undefined) return <>{children}</>;
  if (seat === null) return null;
  return createPortal(children, seat);
}

/** Where a page's second title line goes, under its name in R0. */
const SubtitleSeat = createContext<HTMLElement | null | undefined>(undefined);

/** Where a page's words beside its name go, on the name's own line. */
const AsideSeat = createContext<HTMLElement | null | undefined>(undefined);

/**
 * **A page's second title line, inside R0** (2026-09-28, the open
 * workspace: branch, changed files, `机器:路径`). The macOS title-and-
 * subtitle pair — Xcode puts the branch under the project — and not a
 * row between R0 and R1, which nothing may add (10-layout). **Every A
 * page wears one where it has an honest fact to put there** (2026-09-30,
 * 「所有非标签页的落地页和工作区看齐」); which fact each page writes is
 * the table beside 10-layout's page types, and a page with none draws
 * no second line (components/Toolbar).
 *
 * Drawn by the page that owns the facts, seated by the shell, the way
 * `PageAction` is. **Nothing where there is no R0**: the narrow face's
 * head is the page's own (`NarrowHead`) and says what fits there itself.
 * **Written on `caption`'s leading, 14** (2026-09-29): R0 is 38 and the
 * pair is set a rung tight to sit in it with room (`PageHead`).
 */
export function PageSubtitle({ children }: { children: ReactNode }) {
  const seat = useContext(SubtitleSeat);
  if (!seat) return null;
  return createPortal(children, seat);
}

/**
 * **Words beside a page's name, on the name's own line** (2026-09-30,
 * the open workspace: 「标题旁边是 git branch 加改动简介」). `callout`
 * in `secondary`, on the name's baseline — a qualifier of the name, the
 * way Finder's path bar greys the folder's parent beside it; the second
 * line (`PageSubtitle`) is left for the one long value, the location.
 * It gives way before the name does (`PageHead`). Seated by the shell
 * like `PageSubtitle`; nothing where there is no R0.
 */
export function PageTitleAside({ children }: { children: ReactNode }) {
  const seat = useContext(AsideSeat);
  if (!seat) return null;
  return createPortal(children, seat);
}

/** Where a page's category avatar goes, left of its two title lines. */
const AvatarSeat = createContext<HTMLElement | null | undefined>(undefined);

/**
 * **A page's avatar, at R0's left end** (2026-10-01, the owner:
 * 「机器、额度这俩非标签页的页面，最上面的标题改成最左侧头像，占俩行」
 * 「类别头像机器就是系统的 icon 呀，额度页就是厂商的 icon 呀」), two lines
 * tall: `--avatar` 32 is the title lines' 18 + 14, so it stands 3 off R0's
 * top and bottom on the pair's centreline. **32 and 3 are whole pixels**,
 * so a 1× screen draws it without resampling. What stands here is the
 * page's to say:
 *
 * - **a machine: the very avatar its list row wears** (2026-10-02, the
 *   owner, through the lead): the row's own composition — the machine's
 *   face (`MachineAvatar`) with its system's mark the way the row draws it
 *   (`FaceMark`) — so when the row's drawing changes (the face-and-mark
 *   styles, 「浮现」 and 「角标」), the head changes with it;
 * - **an account: the vendor's mark itself** (`sessionMark`), on nothing —
 *   an account has no face — in ink (`currentColor`; every mark in
 *   `BrandIcons` is one colour, filled with it).
 *
 * **No 1 px ring in R0, whatever stands here.** A face carries a hairline
 * edge (`avatar-edge`) so a pale face does not dissolve into the paper of
 * a list; round a 32 circle on the owner's 2560×1440 1× screen that ring
 * cannot be drawn without stair steps (「有一个我屏幕上看起来带锯齿的边」),
 * so the seat takes every border inside a face away. R0 is the frame's
 * grey (`canvas`), not paper, which is the ground the edge was for.
 *
 * ⚠️ 墓碑 (2026-10-01, one evening): every page's avatar was a category
 * mark at `--kind-mark` 18 on an `--avatar` 32 disc of paper with a face's
 * hairline — the ring above, and an 18 mark inside an empty ring read as
 * a badge, not as the two lines' avatar. Then (the same night) the bare
 * system mark at 32 on the machine page, until the owner asked for the
 * row's own avatar there.
 *
 * Seated by the shell like `PageSubtitle`; nothing where there is no R0
 * — the narrow face's head is one line (`NarrowHead`), and an avatar that
 * stands for two lines has no second line there to stand beside.
 */
export function PageAvatar({ kind, children }: { kind: string; children: ReactNode }) {
  const seat = useContext(AvatarSeat);
  if (!seat) return null;
  return createPortal(
    <span
      data-page-avatar-mark={kind}
      aria-hidden="true"
      className="flex flex-none text-foreground [&_[data-face]_*]:border-0"
    >
      {children}
    </span>,
    seat,
  );
}

/** Which of the three page types a region is (docs/DESIGN.md 第五节). */
export type RegionKind = "detail" | "tabs" | "doc";

/**
 * **R0 on a page with no tabs** (D2): the page's name in `title-sm`,
 * and a slot at the right end for the page's own buttons. **No ‹ ›**
 * (2026-09-26, 「机器等非标签页左上角不用有返回与前进，就是标题就行」):
 * a detail page is reached from its list, which is always beside it,
 * so a history over what was picked walked what the list already shows.
 */
export function PageHead({
  title,
  actions,
  caption,
  seatRef,
  subRef,
  asideRef,
  avatarRef,
}: {
  title: ReactNode;
  actions?: ReactNode;
  /** A stale or offline region's one line (第十七轮 §6, 20-states.md):
      「3 分钟前的读数」 at R0's right end, never a banner in the board. */
  caption?: ReactNode;
  /** Where the page's own buttons are seated (`PageAction`), before
      the shell's. */
  seatRef?: (el: HTMLElement | null) => void;
  /** Where the page's second title line is seated (`PageSubtitle`).
      Empty, it takes no room and the name sits alone on R0's middle. */
  subRef?: (el: HTMLElement | null) => void;
  /** Where the words beside the name are seated (`PageTitleAside`).
      Empty, it takes no room. */
  asideRef?: (el: HTMLElement | null) => void;
  /** Where the page's category avatar is seated (`PageAvatar`), left of
      the two lines and `gap-in` off them. Empty, it takes no room. */
  avatarRef?: (el: HTMLElement | null) => void;
}) {
  return (
    <div data-page-head className="flex min-w-0 flex-1 items-center gap-row self-stretch px-page">
      {/* The mark and the name it stands for are one clump — `gap-in`,
          「inside one thing」 — where the page's buttons stand `gap-row`
          off. Both are centred in R0, so the mark and the pair share the
          row's centreline. */}
      <div className="flex min-w-0 flex-1 items-center gap-in">
      {avatarRef && <span ref={avatarRef} data-page-avatar className="flex flex-none empty:hidden" />}
      {/* The name over its second line, the pair centred in R0 — on the
          centreline the window's buttons and the search box stand on.
          **Set one leading rung tight** (2026-09-29, R0 at 38): the name
          on `body`'s 18 rather than `title-sm`'s own 20, and the page's
          second line on `caption`'s 14 (`PageSubtitle`'s callers):
          18 + 14 = 32 leaves 3 over and under, where 20 + 16 = 36 left
          one and the second line's descenders stood on R0's hairline.
          Sizes unchanged — 15 and 12. A name alone is centred the same
          either way. */}
      <div className="flex min-w-0 flex-1 flex-col justify-center">
        {/* The name's line: the name, then what qualifies it on the same
            baseline. **The words beside give way first** — a large
            shrink factor against the name's one — so a long branch is
            cut before the directory's own name is. On `caption`'s
            leading, so the line stays the name's 18 tall. */}
        <div className="flex min-w-0 items-baseline gap-in">
          <h1 data-page-title className="min-w-0 truncate text-title-sm leading-(--text-body--line-height) font-title">
            {title}
          </h1>
          {asideRef && (
            <span
              ref={asideRef}
              data-page-title-aside
              className="flex min-w-0 shrink-[1000] items-baseline gap-xxs overflow-hidden whitespace-nowrap text-callout leading-(--text-caption--line-height) text-muted-foreground empty:hidden"
            />
          )}
        </div>
        {subRef && <div ref={subRef} data-page-subtitle className="flex min-w-0 empty:hidden" />}
      </div>
      </div>
      {caption && (
        <span data-region-caption className="flex-none truncate text-caption text-tertiary tabular-nums">
          {caption}
        </span>
      )}
      {(actions || seatRef) && (
        <div data-page-actions className="flex flex-none items-center gap-in">
          {seatRef && <span ref={seatRef} data-page-actions-seat className="contents" />}
          {actions}
        </div>
      )}
    </div>
  );
}

export function Region({
  kind,
  head,
  title,
  actions,
  caption,
  busy,
  flush,
  className,
  children,
}: {
  kind: RegionKind;
  /** What R0 holds where it is not a `PageHead` — the tab strip (tabs),
      the address bar (doc). **Absent, R0 is a `PageHead`** with the
      `title` below: every page has the row (D2), a page without tabs
      puts its name in it. */
  head?: ReactNode;
  /** The page's name in R0 — the landing's name when nothing is picked,
      the picked thing's name when something is. Read where `head` is
      absent. */
  title?: ReactNode;
  /** The page's own buttons at R0's right end (第十七轮 §3). */
  actions?: ReactNode;
  /** A stale region's caption at R0's right end — in the `PageHead`, or
      after `head` where R0 is a strip or an address bar. */
  caption?: ReactNode;
  /** Refreshing content already seen: a 2 px brand bar along R0's
      bottom edge, the content left as it was (第十七轮 §6). */
  busy?: boolean;
  /** **The board is the panes' own** (第十七轮 §5, D8): an open
      workspace is B inside R1 — three panes, each with a head at
      `--pane-head`, hairlines between them — so the board wears no
      margin and no corner of its own, and a hairline under R0 is the
      one edge between the two. */
  flush?: boolean;
  /** How the board lays its content out. */
  className?: string;
  children: ReactNode;
}) {
  const lead = useContext(RegionLead);
  const [seat, setSeat] = useState<HTMLElement | null>(null);
  const [sub, setSub] = useState<HTMLElement | null>(null);
  const [aside, setAside] = useState<HTMLElement | null>(null);
  const [avatar, setAvatar] = useState<HTMLElement | null>(null);
  return (
    <div data-region data-region-kind={kind} className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div
        data-region-head
        // R0 is one of the window's three handles, marked by the shell
        // (`shell.ts`): its blank moves the window, and what is pressable
        // inside it keeps its press by tauri's own rule.
        {...WINDOW_DRAG}
        className="relative flex h-pane-head flex-none items-end select-none"
      >
        {lead}
        <div className="flex min-w-0 flex-1 self-stretch">
          {head ?? (
            <PageHead
              title={title}
              actions={actions}
              caption={caption}
              seatRef={setSeat}
              // An overview's head is empty since the redesign — the
              // landing's name and count are the left block's — so it
              // offers no second line or avatar seat either.
              subRef={title ? setSub : undefined}
              asideRef={title ? setAside : undefined}
              avatarRef={title ? setAvatar : undefined}
            />
          )}
        </div>
        {/* **Every R0 can say it is stale**, not only a `PageHead`: on a
            tab strip (B) or an address bar (C) the caption sits at the
            row's right end after them, outside the strip — so the strip
            measures what is left and compresses its tabs into that,
            rather than being pushed past the edge. */}
        {head && caption && (
          <span
            data-region-caption
            className="flex-none self-center truncate px-row text-caption text-tertiary tabular-nums"
          >
            {caption}
          </span>
        )}
        <WindowControls />
        {busy && <div data-region-busy aria-hidden className="absolute inset-x-0 bottom-0 h-0.5 bg-brand" />}
      </div>
      {/* The board. On narrow it is the whole screen and wears no
          margin and no corner (`app.css`, keyed on `data-narrow`).
          **A block, not a flex column**: what it holds is written
          `h-full` (a machine's card, a session's pane), and a flex
          item's `min-height: auto` lets that grow past the board to
          its content — the card's process table went out of reach of
          the face gate the first time this was a column. A block with
          a definite height is what the wrapper this replaces was.
          **Joined to R0 with nothing between** — under a strip the open
          tab and the board are one piece, and under a page's name there
          is no second line (第十四轮 第 1 条). */}
      <div
        data-board
        data-board-flush={flush ? "" : undefined}
        className={cn(
          // **One ground** (2026-10-02): the board is no longer a paper
          // inset in a frame — no margin, no corner, at any width. A
          // workspace's three columns keep the hairline under R0.
          "min-h-0 flex-1 overflow-hidden bg-card",
          flush && "border-t",
          className,
        )}
      >
        <RegionLead.Provider value={null}>
        <ActionsSeat.Provider value={head ? undefined : seat}>
          <SubtitleSeat.Provider value={head ? undefined : sub}>
            <AsideSeat.Provider value={head ? undefined : aside}>
              <AvatarSeat.Provider value={head ? undefined : avatar}>{children}</AvatarSeat.Provider>
            </AsideSeat.Provider>
          </SubtitleSeat.Provider>
        </ActionsSeat.Provider>
        </RegionLead.Provider>
      </div>
    </div>
  );
}
