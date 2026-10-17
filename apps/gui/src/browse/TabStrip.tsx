import { useTabReorder } from "@/browse/use-tab-reorder";
import { usePillSignal } from "@/lib/pill-signal";
// A browsing landing's tabs, in the two shapes a browser has.
//
// **The shell's, not one landing's** (docs/DESIGN.md 浏览型外壳): every
// tab here is a face and a word, and neither the strip nor the switcher
// knows what kind of place is behind one.
//
// **Wide is the strip**: every tab on screen at once, because the whole
// point of a tab is that you can see the other ones. **Narrow is a
// count and a switcher** — a phone shows one screen at a time
// (docs/UX.md 姿势), so the strip cannot be there and Chrome's own
// answer is the one people already know: the number of tabs is the
// button, and pressing it is where switching, closing and opening
// happen.
//
// That is not two designs. It is one model — a list of places, one of
// them open — drawn at the two widths, and neither face has a tab the
// other does not.
//
// **…and the wide half of that had a condition nobody checked.** "Every
// tab on screen at once" is not a property of the strip, it is a
// property of the strip *while the tabs still fit*. Tabs are two fixed
// widths now (tokens.css `--tab-w` / `--tab-w-on`: a stub when not in
// front, the whole name when it is — 2026-09-02:「未选中固定长度,选中
// 变宽」), so the strip scrolls the moment the stubs and the open tab
// outgrow the room, and the tabs past that edge are literally not on
// screen. At that point the strip answers none of the
// three things somebody with fifteen tabs is doing: getting back to
// the one they were just on, seeing what is even open, or closing a
// batch.
//
// So the count and the switcher are **not the narrow face's** — they
// are the answer to "too many to find one in", which is a question
// about the number of tabs and not about the width of the window. The
// wide face gets the same control, on the condition its own premise
// fails on: the strip is actually scrolling. That is falsifiable in
// both directions — if the tabs fit and every label is readable, the
// button must not be there, because then the strip really is showing
// you all of them and a second way to a thing that is not hidden is
// clutter (this file's original judgment, kept, with its condition now
// written down and measured rather than assumed).
//
// **The strip has left that arrangement** (UX-45, Chrome's): every
// tab is drawn and all are one width, front or not (`--tab-w-even`),
// shrinking together down to `--tab-w-floor` — the face and the × —
// shedding as they go (a tab not in front keeps its × for the pointer
// only, then its name goes), and past the floor the strip scrolls, the
// front tab kept in view; after the last tab a hairline and ＋. No count
// and no switcher there. ⚠️ 墓碑 (UX-25 → UX-45): the count after the
// strip once it scrolled, and the switcher it opened (`Tabs`). ⚠️ 墓碑
// (ux-q): the plain strip and the strip under a session's content —
// every strip is the glass one at the top; the phone's band is
// `SessionBar`'s, and the narrow shell's count (`TabCount`, below) is
// `DetailPane`'s head's.
import { useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { EditableName, useSinglePress } from "@/components/EditableName";
import { MachineAvatar } from "@/components/Avatar";
import { fontOf, measurer } from "@/components/PathCut";
import type { EditItem } from "@/hooks/use-read-first";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { SessionFace } from "@/components/SessionFace";
import { FaceMark, type KindMark } from "@/components/FaceMark";
import { IconBack, IconCheck, IconClose, IconPlus, IconReload, IconUpload } from "@/components/icons";
import { GlassCircle } from "@/components/Glass";
import { NoticeLook, NoticePanel, NoticeScrim, useGrownPill } from "@/components/PillNotice";
import type { SessionInteraction } from "@/lib/session-interaction";
import { PaneBar } from "@/components/PaneBar";
import { RegionLead } from "@/components/Region";
import { NarrowHead } from "@/components/SideHead";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { SideList, type SideRow } from "@/components/SideList";
import { BellMark } from "@/components/BellMark";
import { sessionKind } from "@/components/SessionFace";
import { gui } from "@/gen/catalog";
import { stillMotion, TRIP_MS, useTravel } from "@/lib/travel";
import { cn } from "@/lib/utils";

/** One tab as it is shown: what it calls itself, whose machine it is
    on, and where. Derived by the shell (it holds the device table and
    the kind); this file paints and decides nothing. */
export type TabFace = {
  /** Optional content owned by this tab, inside the same group capsule. */
  group?: ReactNode;
  id: string;
  label: string;
  /** The tab's long name, where `label` is the short one: a page's own
      title while its pill writes the host (UX-35 §4). The tabs sheet's
      and the switcher's rows write it, and it leads the hover title.
      Absent, `label` is the whole name. */
  title?: string;
  face: Avatar | null;
  /** The whole address, for the switcher's second line and the wide
      tab's hover title. Empty on a fresh tab. */
  path: string;
  /** Whose machine this tab is on, for the hover title. The avatar has
      always carried this and could never spell it. */
  machine?: string | null;
  /** Present on a session tab: the row's identity facts, drawn exactly
      as the list row draws them (`SessionFace` — the halo is the state,
      the corner is the kind; 2026-09-02:「标签页的标签头像应该和
      session列表一致」). A browsing tab has none and keeps the bare
      machine face. */
  session?: { word: string; kind: string; category: string | null };
  /** A browsing page's kind mark on its machine's face (the redesign):
      the face says which machine, the mark that it is a page. */
  mark?: KindMark;
  /** What this tab's name renames, or absent when the name is not the
      user's to change (2026-09-11:「session 标签页不可双击重命名,我需要
      和机器一样」).

      **Opt-in per tab, because the strip carries two kinds.** A session
      tab wears its row's title, which is a name somebody gave it; a
      browsing tab wears a page's own name, which is the page's and not
      anybody's to override. Handing the strip a blanket 「tabs are
      renamable」 would put an editor on the second kind, where Enter
      would write into a table that has no row for it. */
  rename?: { kind: "session" | "machine"; id: string };
  /** A terminal's progress line under its name (roadmap 4.1). */
  progress?: ReactNode;
  /** A second line under the name, on the **strip** (工作区批⑧:
      「标签上显示文件名,下方显示其路径」). `path` is the whole address
      and belongs to the hover title and the switcher; this is the short
      form a tab has room for — for a file tab, where it sits relative
      to the workspace root. Absent on a tab whose name is already the
      whole of what there is to say. */
  sub?: string;
  /** One more sentence for the hover title, from whatever this tab's
      view has to add — a diff that stopped short of the whole file
      (第十四轮 第 2 条). **Not drawn anywhere on the strip**: it is the
      long form, and the long form is what a `title` is for. */
  note?: string | null;
  /** Something is typed here and not saved. The close mark becomes a
      filled dot, VS Code's arrangement, and goes back to a × the moment
      a save lands. */
  dirty?: boolean;
  /** The session's bell rang while it was off screen (item 67,
      `lib/bell-marks`): the badge's dot before the name. */
  rang?: boolean;
  /** This tab is the **preview**: one single click opened it and the
      next single click will replace it (工作区批⑧). Italic, the one
      place this app uses the slope, because what it marks is exactly
      「this is not settled」. */
  preview?: boolean;
  /* ⚠️ 墓碑 (UX-34, 2026-10-06): `glyph` — what a file tab wore instead
     of a face — went with UX-32's WorkspacePane: a file tab's seat is the
     assembly (the machine's face, the file's kind as the mark) like every
     other tab's. */
  /** The tab's own right-click menu. **Hung on the button that opens
      the tab**, never on the tab box around it: `RowMenu`'s trigger
      carries an `onPointerDown`, and a press handler on a bare element
      is a target only a pointer can find. */
  menu?: RowAction[];
  /** 源码 / 预览, for a tab whose subject has two faces — a document, a
      picture, a page (2026-09-17 ruling 47:「预览按钮放在标签页的标签
      上」— the user).
   *
   * **On the tab rather than at the end of the strip**, which is where
   * it sat until this ruling. What it switches is one tab's own face,
   * and a control at the end of a strip reads as being about the strip;
   * a person with four tabs open could not tell which one the eye
   * belonged to except by knowing it was always the front one.
   *
   * Absent on a tab with nothing to preview, and **drawn only on the
   * tab in front**: every other tab is a stub with room for a name and
   * a close, and a second mark on each of them would be a row of
   * controls aimed at files nobody is looking at. */
  look?: { on: boolean; label: string; onToggle: () => void };
  /** 「编辑」 / 「保存」 on a file's front tab under a finger (P1,
      `hooks/use-read-first`): the file reads first, and this is the way
      into typing and back out. Absent under a mouse and for a file that
      cannot be edited. */
  edit?: EditItem | null;
  /**
   * What pressing this tab **again** does — while it is already in front
   * — and what hovering it then says (2026-09-29, the owner, of the
   * session's day: 「点击标签后显示吧。悬停的时候加一个提示，点击查看近24
   * 小时状态」).
   *
   * Read only on the tab in front: a press on any other tab switches to
   * it, as it always has, and it gets no hint. `expanded` and `controls`
   * make the tab a disclosure button — the strip's tabs are plain
   * buttons, not an ARIA tablist, so `aria-expanded` is the honest role
   * for 「this press shows or hides that row」 (a `tab` would have wanted
   * `aria-selected` and meant something else). Absent `toggle`, the tab
   * says `hint` and a press does nothing: there is nothing to open.
   *
   * The strip does not know what is being opened; the session's day is
   * the one caller (`App`), and the words are the caller's.
   */
  again?: { hint: string; expanded?: boolean; controls?: string; toggle?: () => void };
  /** **The tab's act, 「和来源对齐」** (UX-33, the owner 2026-10-06): a
      session's ↻ re-attach, a page's ↻ reload (× while it loads), a
      file's ↻ re-read when clean and ✓ save when changed. On the phone's
      band it is the pill's third seat, on every pill; on the wide strip
      no pill wears it (UX-39 §7) — the front tab's stands after the
      capsule where the strip asks for it (`frontAct`). Absent only on a
      tab nobody has listed yet. `saved` is not an act but the file's ✓
      saying 已保存 for a moment after a save somebody asked for (save-a):
      a state, drawn in the seat and pressing nothing. */
  act?: { kind: "reload" | "stop" | "save" | "saved"; label: string; onPress: () => void };
  /** **The pill reloads**: a 2-pt line runs along its own bottom edge
      while a join, a load or a read is under way (UX-33 §2; UX-31's slot). */
  loading?: boolean;
  /** **How far a drop onto this tab has landed** (item 60), 0 to 1: the
      same line, filled this far rather than running. `loading` wins. */
  share?: number;
  /** Operator ↔ session activity, independent of `session.word`.
      Avatar halo reads `session`; pill perimeter reads `interaction`. */
  interaction?: SessionInteraction;
};

/** The face a tab wears: the session assembly when it is a session,
    the bare machine face when it is a place. One decision, both the
    strip and the switcher read it — **at the size of the seat**: the
    strip's tab is 28 tall and takes the assembly at a tab's size
    (`SessionFace` `tab`, halo and corner inside the tab's box); the
    switcher's rows are list rows and keep a row's. */
export function TabAvatar({ f, bare, seat }: { f: TabFace; bare: string; seat: "tab" | "row" }) {
  // **A file going in** (item 71): the upload mark in the face's seat,
  // the seat's whole size as the face was; in full ink whatever the
  // tab's, as a face is never dimmed.
  if (f.interaction?.drop)
    return (
      <span data-tab-dropping className={cn(bare, "grid flex-none place-items-center text-foreground")}>
        <IconUpload className="size-full" />
      </span>
    );
  if (f.session) return <SessionFace face={f.face} {...f.session} size={seat} />;
  // A browsing page: the machine it goes through, wearing its kind's
  // mark the way a session's tab does (the redesign, 2026-10-02).
  if (f.mark) return <FaceMark face={{ machine: f.face }} mark={f.mark} size={seat} />;
  return <MachineAvatar face={f.face} className={bare} />;
}

/** A preview tab's mark: a hollow ring at caption size, before the name. */
function PreviewMark() {
  return <span data-tab-preview aria-hidden className="size-1.5 flex-none rounded-full border border-current opacity-70" />;
}

/**
 * A name cut **in the middle** when it has to be (docs/SHELL.md 第三轮 第
 * 1 条:「名字从中间省略」): the head truncates, the tail keeps its last
 * few characters — the part that tells `notes-draft.md` from
 * `notes-final.md`, and a file's extension. Short names are one span, so
 * nothing about them changes.
 */
export function MidName({ text, after = "", even = false }: { text: string; after?: string; even?: boolean }) {
  const KEEP = 5;
  /** The fewest characters of the head a middle cut may leave. */
  const HEAD = 3;
  const [sizer, setSizer] = useState<HTMLSpanElement | null>(null);
  const [shown, setShown] = useState(text);
  // **Cut by measure** (2026-10-03): the head used to be a CSS `truncate`
  // span beside the tail, and `text-overflow` drops whole glyphs and
  // keeps their box, so up to a glyph of blank stood between 「…」 and
  // the tail (「loc… .html」). The full name in an invisible layer sets
  // the label's width (the tab's own cap still applies); the visible
  // layer is the head cut to what fits, 「…」, then the last few
  // characters, all one run.
  useLayoutEffect(() => {
    if (sizer === null || text.length <= KEEP * 2) {
      setShown(text);
      return;
    }
    const fit = () => {
      const room = sizer.getBoundingClientRect().width;
      if (room <= 0) return;
      // **A name that fits is never cut** (UX-45, the owner 2026-10-07:
      // 「frontier_diagno…cs.py」 in a tab it fitted). Asked of the layout
      // itself — the full name overflowing its box or not — rather than
      // of a measure against a rounded width with a pixel taken off,
      // which cut names that fitted to the fraction.
      if (sizer.scrollWidth <= sizer.clientWidth) {
        setShown(text);
        return;
      }
      const wide = measurer(fontOf(sizer));
      const fits = (s: string) => wide(s + after) <= room;
      const chars = [...text];
      /** The plain end cut, 「fron…」: when not even a few characters of
          the head stand before the tail. **Never a tail with no head**
          (the owner: 「…ue.p」 said nothing). */
      const endCut = () => {
        let n = chars.length - 1;
        while (n > 1 && !fits(`${chars.slice(0, n).join("")}…`)) n -= 1;
        setShown(`${chars.slice(0, n).join("")}…`);
      };
      // **`even`: as much of the end as of the start**, Finder's middle
      // cut — for a name with no extension to keep (a file going in,
      // the owner 2026-10-07 13:15), where a fixed tail of wide
      // characters left no start at all.
      if (even) {
        let n = chars.length - 1;
        const cut = (k: number) => `${chars.slice(0, Math.ceil(k / 2)).join("")}…${chars.slice(chars.length - Math.floor(k / 2)).join("")}`;
        while (n > 0 && !fits(cut(n))) n -= 1;
        if (Math.ceil(n / 2) < HEAD) endCut();
        else setShown(cut(n));
        return;
      }
      // The middle cut, the tail shortened first (down to two) before the
      // head is let go below `HEAD` characters.
      for (let keep = KEEP; keep >= 2; keep -= 1) {
        const tail = chars.slice(-keep).join("");
        let lo = 0;
        let hi = chars.length - keep;
        while (lo < hi) {
          const mid = Math.ceil((lo + hi) / 2);
          if (fits(`${chars.slice(0, mid).join("")}…${tail}`)) lo = mid;
          else hi = mid - 1;
        }
        if (lo >= HEAD) {
          setShown(`${chars.slice(0, lo).join("")}…${tail}`);
          return;
        }
      }
      endCut();
    };
    fit();
    const watch = new ResizeObserver(fit);
    watch.observe(sizer);
    return () => watch.disconnect();
  }, [sizer, text, after, even]);
  if (text.length <= KEEP * 2) return <span className="min-w-0 truncate">{text + after}</span>;
  return (
    <span data-mid-name className="relative flex min-w-0">
      <span ref={setSizer} aria-hidden className="invisible min-w-0 overflow-hidden whitespace-nowrap">
        {text + after}
      </span>
      <span aria-hidden className="absolute inset-0 overflow-hidden whitespace-nowrap">
        {shown + after}
      </span>
      <span className="sr-only">{text + after}</span>
    </span>
  );
}

/** The tab's opener, inside its menu when it has one. **A component
    rather than a ternary around the button**, so the button is written
    once: two copies of it is how the hover title and the anchor drift
    apart, and the anchor is what every gate finds a tab by. */
function TabOpen({ menu, children }: { menu?: RowAction[]; children: ReactNode }) {
  return menu && menu.length > 0 ? <RowMenu items={menu}>{children}</RowMenu> : <>{children}</>;
}

/**
 * **The strip re-balances with the usual transition** (UX-25 addendum).
 * The pills' widths are flex's (the pill in front its name, the rest an
 * equal share), so a choice re-lays the whole row at once; this runs each
 * pill from the width it had to the width it has, over the chosen piece's
 * trip (`lib/travel`, the same beat), so the eye sees the front pill grow
 * and its neighbours give way rather than a snap. Widths are noted at
 * every layout and on the strip's resize, so the run starts from where
 * the pills really were. Reduced motion: no run.
 */
function useRebalance(host: () => HTMLElement | null, active: string, kick = 0) {
  const was = useRef(new Map<string, number>());
  const last = useRef(active);
  // **…and when a held row lets go** (UX-45): `kick` counts the releases,
  // so the tabs grow back into the room over the same beat, not a snap.
  const kicked = useRef(kick);
  const note = () => {
    const h = host();
    if (!h) return;
    was.current = new Map([...h.querySelectorAll<HTMLElement>("[data-tab]")].map((t) => [t.getAttribute("data-tab") ?? "", t.getBoundingClientRect().width]));
  };
  useLayoutEffect(() => {
    const h = host();
    if (!h) return;
    const before = was.current;
    const changed = last.current !== active || kicked.current !== kick;
    last.current = active;
    kicked.current = kick;
    note();
    if (!changed || before.size === 0 || stillMotion()) return;
    const ease = getComputedStyle(document.documentElement).getPropertyValue("--ease-move").trim() || "ease";
    for (const t of h.querySelectorAll<HTMLElement>("[data-tab]")) {
      const id = t.getAttribute("data-tab") ?? "";
      const from = before.get(id);
      const to = was.current.get(id) ?? 0;
      if (from === undefined || Math.abs(from - to) < 1) continue;
      t.animate(
        [
          { flexBasis: `${from}px`, flexGrow: 0, flexShrink: 0, minWidth: 0, maxWidth: "none" },
          { flexBasis: `${to}px`, flexGrow: 0, flexShrink: 0, minWidth: 0, maxWidth: "none" },
        ],
        { duration: TRIP_MS, easing: ease },
      );
    }
  });
  return { note };
}

/** Shared short divider for outer and nested tab controls. */
export function TabDivider() {
  return <span aria-hidden="true" data-tab-divider className="mx-in h-4.5 w-px flex-none bg-border" />;
}

export function TabStrip({
  faces,
  active,
  onFocus,
  // Closing left the pill with UX-33 (the sheet, ⌘W and the menu close);
  // the strip still takes the handler so every caller stays one shape.
  onClose,
  onNew,
  head,
  tail,
  frontAct = false,
  seek = 0,
  grouped = false,
  onReorder,
}: {
  /** **A count of picks, the tab in front included** (fix-a): each new
      value brings the front tab into view, like a change of `active`.
      A person who wheeled the strip away from the tab in front and then
      picks that same tab from the list asked to see it; `active` alone
      cannot say so, as it does not change. Absent, only `active` does. */
  seek?: number;
  /** Group heads retain identity while their supplied content scrolls. */
  grouped?: boolean;
  onReorder?: (ids:string[]) => void | Promise<unknown>;
  /** **The front tab's act at the capsule's end, and none on the pills**
      (UX-39, the owner 2026-10-06 22:35: a ↻ on every pill and one more
      at the strip's end was 「有点奇怪」). On the wide face a tab is a
      tab; the one act stands at the end, Safari's arrangement. The
      session strip leaves this off (the terminal's own ↻ already stands
      in the verbs seat) and so does the browser (the address field has
      its ↻); the workspace strip takes it for the file's ↻ / ✓. ⚠️ 墓碑
      (UX-33 §1): the third seat on every wide pill. */
  frontAct?: boolean;
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  /** Add a tab. Absent on a strip whose tabs are not made here — the
      session strip's tabs come from the list, so it draws no "+". */
  onNew?: () => void;
  /** Whatever the landing pins against the **left** end of the strip.
      One caller so far: the workspace's narrow face, where the file is
      one screen of three and this is the way back to the tree
      (2026-09-16 ruling 33). **Before the scroller**, for `tail`'s
      reason — it is not about the tabs, so it must not scroll away with
      them — and at the head because a way back belongs where a hand
      looks for one. */
  head?: ReactNode;
  /** Whatever the landing pins against the right end of the strip
      (工作区批⑧: the workspace puts its three switches there —
      「标签页的最右侧,显示现有的文件、session 和 vscode 切换」).
      **Outside the scroller**, unlike 「+」: these are not about the
      tabs, so they must not scroll away with them. */
  tail?: ReactNode;
}) {
  /** The list is folded and its strip's block leads this row
      (`RegionLead`): the capsule follows the block, a gap after it. */
  const led = useContext(RegionLead) !== null;
  const strip = useRef<HTMLDivElement>(null);
  const spread = grouped && faces.some(f => f.id === active && f.again?.expanded);
  const heldWidths = useRef(new Map<string, number>());
  const wasSpread = useRef(spread);
  useLayoutEffect(() => {
    if (wasSpread.current === spread) return;
    wasSpread.current = spread;
    const row = strip.current;
    if (!row || stillMotion()) return;
    const duration = parseFloat(getComputedStyle(row).getPropertyValue("--dur-120"));
    row.animate([{opacity:0},{opacity:1}], {duration,easing:"ease-out"});
  }, [spread]);
  // The front tab's piece slides to the tab brought forward, in the glass
  // capsule (F7.2, `lib/travel`).
  useTravel("tabs", () => strip.current, active);
  /** How many times a held row has let go (`held`): the re-balance runs
      on each. */
  const [released, setReleased] = useState(0);
  const widths = useRebalance(() => spread ? null : strip.current, active, released);
  const reorder=useTabReorder({host:()=>strip.current,selector:"[data-tab]",ids:faces.map(f=>f.id),onReorder:spread?undefined:onReorder,exclude:"[data-content-tabs]",onStart:()=>releaseHeld()});
  const orderedFaces=reorder.order.map(id=>faces.find(f=>f.id===id)).filter((f):f is TabFace=>!!f);
  /** Which ends have a tab cut off behind them, for the fade on that
      side (docs/SHELL.md 第三轮 第 2 条). No fade where nothing is cut:
      a fade over a whole tab would be a mark that says 「more this
      way」 about nothing. */
  const [cut, setCut] = useState({ left: false, right: false });
  /**
   * **The tab in front is scrolled into view** whenever it changes — a
   * click, a shortcut, a pick from a menu, a new one (第三轮 第 2
   * 条) — whenever it is picked again (`seek`), and when the strip
   * itself changes width, which is how a tab
   * that was in view ends up past the edge without anybody touching it.
   * Measured against the strip rather than with `scrollIntoView`, which
   * also scrolls every ancestor that can scroll, and the shell's own
   * boxes are not this strip's to move.
   */
  const bringFront = useRef(() => {});
  /** Whether the row is held (`held`, below): a held row keeps its
      scroll when a tab goes (UX-45), so the next × stays under the
      pointer — neither the count changing nor the strip resizing brings
      the front tab back; a new front tab still is. */
  const holding = useRef(false);
  bringFront.current = () => {
    const el = strip.current;
    const tab = el?.querySelector<HTMLElement>(`[data-tab="${CSS.escape(active)}"]`);
    if (!el || !tab) return;
    const box = el.getBoundingClientRect();
    const t = tab.getBoundingClientRect();
    if (t.left >= box.left - 0.5 && t.right <= box.right + 0.5) return;
    // **The front tab whole, and the strip's left edge on a tab's start**
    // (V07): of the tabs up to the front one, the earliest that lets the
    // front one still fit is the one the strip starts at — so no orphan
    // × or half a name is left at the left end; what is cut is at the
    // right, under the fade, the start of what comes next.
    const tabs = [...el.querySelectorAll<HTMLElement>("[data-tab]")];
    const at = tabs.indexOf(tab);
    const start = (e: HTMLElement) => e.getBoundingClientRect().left - box.left + el.scrollLeft;
    const frontRight = t.right - box.left + el.scrollLeft;
    let to = start(tab);
    for (let j = 0; j <= at; j++) {
      if (frontRight - start(tabs[j]) <= el.clientWidth) {
        to = start(tabs[j]);
        break;
      }
    }
    el.scrollLeft = to;
  };
  useEffect(() => {
    const el = strip.current;
    if (!el) return;
    // A pixel of tolerance: sub-pixel flex widths make an exact
    // comparison flap between renders on a strip that fits.
    const measure = () => {
      const over = el.scrollWidth - el.clientWidth;
      setCut({ left: el.scrollLeft > 1, right: over - el.scrollLeft > 1 });
      widths.note();
    };
    measure();
    // The observer catches the region changing; the dependency catches
    // tabs opening and closing. Neither alone sees both.
    const ro = new ResizeObserver(() => {
      if (!holding.current) bringFront.current();
      measure();
    });
    ro.observe(el);
    el.addEventListener("scroll", measure, { passive: true });
    /**
     * **A vertical wheel over the strip moves it sideways** (第三轮 第
     * 2 条): the scrollbar is gone, and a mouse has no other way to say
     * 「further along」. A trackpad's own sideways swipe is left alone.
     *
     * Not passive, because it has to `preventDefault` — and it does so
     * **even at the end**: a wheel that ran off the strip must not be
     * handed to whatever is behind it (第 1 条:「滚到头不带动整页」).
     * A strip that fits takes nothing: there is nowhere to go, and the
     * wheel belongs to whoever else wants it.
     */
    const wheel = (e: WheelEvent) => {
      if (el.scrollWidth - el.clientWidth <= 1) return;
      if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
      e.preventDefault();
      el.scrollLeft += e.deltaY;
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => {
      ro.disconnect();
      el.removeEventListener("scroll", measure);
      el.removeEventListener("wheel", wheel);
    };
  }, [faces.length]);

  /** Where a held strip stood when its × was pressed: its scroll and its
      width. */
  const kept = useRef<{ left: number; width: number } | null>(null);
  /** **The room a held close leaves at the end** (ux-q): a strip scrolled
      to its right end would lose its scroll as the row got shorter (the
      browser clamps it), and the row would slide right under the pointer.
      An empty seat after the last tab keeps the row as long as the scroll
      needs, so the next tab's × lands under the pointer at the end too. */
  const room = useRef<HTMLDivElement>(null);
  const frontWas = useRef(active);
  const seekWas = useRef(seek);
  useLayoutEffect(() => {
    const moved = frontWas.current !== active || seekWas.current !== seek;
    frontWas.current = active;
    seekWas.current = seek;
    const el = strip.current;
    if (holding.current && el && room.current && kept.current) {
      room.current.style.width = "0";
      const { left, width } = kept.current;
      room.current.style.width = `${left > 0 ? Math.max(0, left + width - el.scrollWidth) : 0}px`;
      el.scrollLeft = left;
    }
    if (holding.current && !moved) return;
    bringFront.current();
  }, [active, faces.length, seek]);

  /** Whether the front tab's hint is up (`TabFace.again`). One for the
      strip: only the tab in front ever has one. */
  const [hint, setHint] = useState(false);
  /**
   * **Closing with the pointer keeps the row still** (UX-45, Chrome's rule;
   * the owner 2026-10-07: closing many tabs, the pointer stays on one
   * tab's ×). A × pressed on the wide glass strip holds every pill at the
   * width it had: the tabs after the closed one slide left by exactly one
   * tab, so the next one's × lands under the pointer, and the room is
   * left empty at the end. The pointer leaving the capsule lets go and
   * the tabs share the room again. ⌘W and the menu never hold.
   */
  const [held, setHeld] = useState<number | null>(null);
  holding.current = held !== null;
  useEffect(() => {
    if (faces.length === 0) setHeld(null);
  }, [faces.length]);
  const releaseHeld = () => { kept.current = null; setHeld(null); setReleased(n => n + 1); };
  const priorIds = useRef(new Set(faces.map(f => f.id)));
  useEffect(() => {
    if (faces.some(f => !priorIds.current.has(f.id))) releaseHeld();
    priorIds.current = new Set(faces.map(f => f.id));
  }, [faces.map(f => f.id).join("\n")]);
  useEffect(() => {
    const el = bar.current;
    if (!el) return;
    let width = el.getBoundingClientRect().width;
    const observer = new ResizeObserver(() => {
      const next = el.getBoundingClientRect().width;
      if (Math.abs(next - width) > .5) releaseHeld();
      width = next;
    });
    observer.observe(el);return () => observer.disconnect();
  }, []);
  useEffect(() => setHint(false), [active]);
  /** A press on the front tab, resolved against a double-click on its
      name — which renames, and must not show and hide the day on its
      way there (`useSinglePress`). */
  const pressAgain = useSinglePress(() => faces.find((f) => f.id === active)?.again?.toggle?.());
  /** **The front tab grows like the phone's pill** (NOTICE-BRIEF): a
      press on it while something stops its session opens the panel below
      it, across the capsule's width; the tab is its own self again while
      the panel says the notice. */
  const stops = faces.find((f) => f.id === active)?.interaction?.notices ?? [];
  const signal = usePillSignal(active, faces.find((f) => f.id === active)?.interaction);
  const grown = useGrownPill(stops.length);
  const capsule = useRef<HTMLDivElement>(null);
  const [front, setFront] = useState<HTMLElement | null>(null);
  useLayoutEffect(() => {
    setFront(grown.growth.mounted ? (strip.current?.querySelector<HTMLElement>(`[data-tab="${CSS.escape(active)}"]`) ?? null) : null);
  }, [grown.growth.mounted, active]);
  const foldGrown = grown.fold;
  useEffect(() => foldGrown(), [active, foldGrown]);

  /**
   * **The row folds, never squeezes** (sb-a, 2026-10-09, the owner with
   * the 附栏 open: 「缩小窗口不能把整个顶部导航给略缩畸形了」 — the
   * capsule had shrunk to nothing and the front act's ↻ stood past the
   * column, over the 附栏's head). The names go first: the tabs narrow to
   * their faces (`--tab-w-floor`, the container queries). Then the
   * secondary buttons — ＋, the front tab's act, the verbs at the end
   * (`tail`) — step out, while the capsule would be narrower than one
   * tab; they come back once there is room for them again (their width
   * measured as they left, so the row does not flap at the edge).
   */
  const bar = useRef<HTMLDivElement>(null);
  const extras = useRef<HTMLSpanElement>(null);
  const plus = useRef<HTMLSpanElement>(null);
  const [folded, setFolded] = useState(false);
  const spare = useRef(0);
  /** The capsule's width when they left. */
  const leftAt = useRef(0);
  useLayoutEffect(() => {
    const row = bar.current;
    const cap = capsule.current;
    if (!row || !cap) return;
    const check = () => {
      const floor = Number.parseFloat(getComputedStyle(cap).getPropertyValue("--tab-w-floor")) || 64;
      const room = cap.getBoundingClientRect().width;
      const over = row.scrollWidth > row.clientWidth + 0.5;
      if (!folded && (over || room < floor)) {
        // What leaves, gaps included, is what must be there to come back.
        const gap = Number.parseFloat(getComputedStyle(row).columnGap) || 0;
        const w = (el: HTMLElement | null) => (el ? el.getBoundingClientRect().width : 0);
        spare.current = w(extras.current) + w(plus.current) + 2 * gap;
        leftAt.current = room;
        setFolded(true);
      } else if (folded && room - spare.current > Math.max(floor, leftAt.current) + 1) setFolded(false);
    };
    const ro = new ResizeObserver(check);
    ro.observe(row);
    check();
    return () => ro.disconnect();
  }, [folded]);

  return (
    <div
      ref={bar}
      data-tab-bar
      data-tab-bar-folded={folded || undefined}
      // ⚠️ 墓碑 (ux-q, 2026-10-07): the plain strip — the band on the
      // app's ground with the open tab the board's colour, joined to it
      // by its foot (第十三轮 第 5 条) — and the strip hung under a
      // session's content (`at="bottom"`, iOS 第三步 A). Every strip is
      // the glass capsule now, at the top (`Tabs`); the phone's band is
      // `SessionBar`'s own.
      //
      // **…and it is the same height as a pane's top row, from the same
      // token** (2026-09-20 第 5 条:「标签页的高度, 要和侧边栏搜索框的
      // 高度一致, 保持和谐. 现在, 标签页普遍比侧边栏更高一些」). They
      // were 48 and 44 — two numbers arrived at separately, agreeing
      // about nothing — and the gate now subtracts them and asks for
      // zero.
      //
      // **One centreline** (2026-09-29,「整行收紧到 38」): the band is
      // the window's first row, and everything in it is centred on the
      // row's middle — the landing's pins at either end, the capsule
      // and its ＋ — the line macOS's three buttons, the search box and a
      // page's title stand on. It was `items-end` with the tabs standing
      // on the floor, twenty under the window's edge on the 48 row: the
      // blank the owner asked about.
      //
      // **The band and the scrolling row are two elements now, and the
      // reason is a measurement**: with everything in one scroll
      // container the "+" scrolled away with the tabs — at 20 tabs on a
      // 1400 window it sat at x=1912 while the strip's visible edge was
      // 1400, **512 past the end of the world**, so the way to open a
      // tab was to scroll right until you found it. The band keeps the
      // ground and the padding; the row inside it is what scrolls.
      //
      // **A glass piece aligns by its glass edge** (the owner,
      // 2026-10-03: 「浏览器这没有左对齐」): over the wide content the
      // capsule's left edge stands on the page's left line (`px-page`,
      // the machine page's avatar and the browser's steps capsule
      // under it). ⚠️ 墓碑: `px-row`, 12 short of the line.
      className={cn(
        "flex h-pane-head min-w-0 flex-1 items-center gap-in",
        // Folded, the strip's block leads the row and the capsule
        // follows it as before, a gap after it.
        led ? "px-row" : "px-page",
      )}
    >
      {head}
      {/* The capsule and the scroller in it. **No scrollbar** (第三轮 第 2
          条:「标签会被移动条顶出窗口. 且此条无法拖动」): `app.css` hides it
          on `[data-tab-strip]`; the wheel and the faded ends are the ways
          past the edge. */}
      <div
        ref={capsule}
        data-tab-capsule
        data-tabs-held={held !== null || undefined}
        onPointerLeave={
          held !== null
            ? () => {
                kept.current = null;
                setHeld(null);
                setReleased((n) => n + 1);
              }
            : undefined
        }
        // **The capsule spans the row** (UX-39, the owner 2026-10-06
        // 22:55: 「全挤在左边了」): the tabs from the row's start, ＋ after
        // the last one, the act circle at the end. ⚠️ 墓碑 (UX-25): the
        // capsule as long as its tabs, at the left.
        className="glass relative flex h-glass-tabs min-w-0 flex-1 items-center self-center rounded-full px-glass-pad"
      >
      <div
        ref={strip}
        data-tab-strip
        {...reorder.handlers}
        // The row is as long as its tabs (they stop at `--tab-w-even`),
        // so ＋ follows the last one (UX-45).
        className="flex min-w-0 flex-initial items-center gap-0.5 self-stretch overflow-x-auto"
        // **A scrolled edge fades** (the audit fix plan F3.3; W09, N19): a
        // mask over the 12 at each end that has a tab cut off behind it,
        // instead of a glyph cut in half.
        style={
          cut.left || cut.right
            ? {
                maskImage: `linear-gradient(to right, ${cut.left ? "transparent, black var(--fade-tabs)" : "black, black"}, ${cut.right ? "black calc(100% - var(--fade-tabs)), transparent" : "black, black"})`,
              }
            : undefined
        }
      >
        {orderedFaces.map((f) => {
          const on = f.id === active;
          const again = on ? f.again : undefined;
          // The front tab's notice, unless its panel is out (it says it).
          const notices = f.interaction?.notices?.length && !(on && grown.growth.mounted) ? f.interaction?.notices : undefined;
          // **Every pill sheds as it shrinks, the front one too** (UX-45,
          // Chrome's strip; the owner 「等宽先看看」): its own width
          // decides (`@container/tab`, tokens.css `--container-tab-*`) —
          // the ×, then the name, down to the face alone.
          return (
            <div
              key={f.id}
              data-tab={f.id}
              data-reorder-id={f.id}
              data-tab-group={grouped ? "" : undefined}
              data-group-open={grouped && on && again?.expanded || undefined}
              data-on={on}
              data-tab-notice={notices?.[0].kind}
              data-pill-signal={on ? signal : undefined}
              style={{
                ...(held !== null ? { width: grouped ? heldWidths.current.get(f.id) ?? held : held, ...(grouped ? { flexBasis: heldWidths.current.get(f.id) ?? held } : {}), flexShrink: 0 } : undefined),
                // Grown, its lower corners give way to the panel below.
                ...(on && grown.growth.mounted ? { borderBottomLeftRadius: 0, borderBottomRightRadius: 0 } : undefined),
              }}
              // Machine first, then where on it — the same order the
              // address bar commits them in, and the same order the
              // sentence reads: this tab is *on* that machine.
              // **And whatever this tab's view has left to whisper**
              // (第十四轮 第 2 条): the sentences that used to stand in
              // a row under the strip now ride here, after the address,
              // on the one tab they are about.
              //
              // **Not on a tab carrying `again`'s hint**: that one says
              // what a press does, at once, and the system's title a
              // beat later would be a second float over the same tab
              // saying something else. The address is the stub's hover,
              // and the open session's row is lit in the list beside.
              title={
                again
                  ? undefined
                  : // The whole name first (UX-45): a pill that sheds its
                    // name keeps it here.
                    [f.title ?? f.label, [f.machine, f.path].filter(Boolean).join(": "), f.note]
                      .filter(Boolean)
                      .join(" · ") || undefined
              }
              className={cn(
                grouped ? "group group/session-tab relative flex h-glass-tab items-center self-center rounded-full" : "@container/tab group relative flex h-glass-tab items-center self-center rounded-full",
                // **One width, front or not** (UX-45, the owner 2026-10-07
                // 「等宽先看看」, Chrome's): every pill is `--tab-w-even`
                // and they shrink together, down to `--tab-w-floor` — the
                // face and the × (the owner: a crowded strip still owes the
                // front tab its ×) — and past that the strip scrolls; the
                // front one is told by its lit glass only. ⚠️ 墓碑 (UX-39
                // §7 → UX-45): the front pill grown to its whole name, up
                // to half the capsule, the rest an equal share. ⚠️ 墓碑
                // (UX-25): `--tab-w-min` as the floor and the count after
                // it. A width, not a basis: the pill is a size container,
                // so only a stated width gives the row its length.
                !grouped && "w-tab-even min-w-tab-floor shrink grow-0",
                on ? (grouped ? "text-foreground" : "glass-sel text-foreground") : "text-muted-foreground hover:bg-fill",
              )}
            >
              <div data-tab-head={grouped ? "" : undefined} className={grouped ? cn("@container/tab", on && "glass-sel") : "contents"}>
              {/* **Every tab sits in a Tooltip, and only the front one's
                  can open** (`again`): kept on all of them so that a tab
                  coming to the front does not remount its button — and
                  the name editor inside it, halfway through the
                  double-click that brought it there. Below the tab, the
                  side a toolbar's hint hangs on (components/Tooltip), and
                  never a place to go. */}
              <Tooltip
                open={!!again && hint}
                onOpenChange={(o) => {
                  if (again) setHint(o);
                }}
                disableHoverableContent
              >
              <TabOpen menu={f.menu}>
              <TooltipTrigger asChild>
              <button
                type="button"
                data-tab-open={f.id}
                // A press on the tab in front is `again`'s — after it has
                // been what every press on a tab is (`onFocus`, a no-op on
                // the tab already in front).
                onClick={(e) => {
                  onFocus(f.id);
                  // The front tab whose session is stopped grows instead.
                  if (on && f.interaction?.notices?.length) {
                    if (grown.open) grown.fold();
                    else grown.grow();
                    return;
                  }
                  if (again) pressAgain(e);
                }}
                onKeyDown={grouped ? e => {
                  // Space belongs to this button. Consume it before the
                  // session's type-to-terminal routing can take focus.
                  if (e.key !== " " || e.metaKey || e.ctrlKey || e.altKey) return;
                  e.preventDefault();
                  if (!e.repeat) e.currentTarget.click();
                } : undefined}
                aria-expanded={again?.expanded}
                aria-controls={again?.expanded ? again.controls : undefined}
                // `self-stretch`: the whole tab opens it, not the line of
                // text in the middle of it. Left at content height this
                // was 18 tall inside a tab it filled the width of — the
                // band around the label looked pressable and was not.
                // `gap-tab-gap`, not the `gap-1` this had: 「标签页上的
                // 标题距离头像稍远一些, 现在有些拥挤」 (2026-09-20 第 3
                // 条). Four pixels put the name against the face while
                // the tab's own `gap-1` put the same four between the
                // name and the × — so the face read as part of the
                // word. Its own token (tokens.css `--tab-gap`) rather
                // than a step up in `--gap-in`, which would move every
                // clump in the app for one complaint about tabs.
                className={cn(
                  "flex min-w-0 flex-1 items-center gap-tab-gap self-stretch text-left text-tab",
                  // The pill's own insets, here so the face can centre
                  // once the name is gone; right only while no × stands.
                  // **The face never moves under the pointer** (ux-q):
                  // once the name is gone, every pill — the front one
                  // too — centres its face in the room left of the ×'s
                  // seat, and a pill not in front keeps that seat empty
                  // until the pointer comes (the × below). ⚠️ 墓碑
                  // (UX-45): the face centred on the whole pill, stepping
                  // to the start when the × came in under the pointer.
                  grouped ? (on ? "pl-1.5" : "pl-1.5 pr-1.5 session-tab-floor:justify-center session-tab-floor:px-0") : on
                    ? "pl-1.5 @max-tab-face/tab:justify-center @max-tab-face/tab:px-0"
                    : "pl-1.5 pr-1.5 group-hover:pr-0 @min-tab-x/tab:pr-0 @max-tab-face/tab:justify-center @max-tab-face/tab:px-0",
                )}
              >
                {/* **A two-line file tab stands its glyph beside both
                    lines** (2026-09-16:「标签页上, 文件类型的icon, 过小
                    了. 而且上面是文件名,下面是路径. icon只和上面对齐,
                    还对的不是很齐. 可以放大一点, 和这两行对齐」— the
                    user).
                    
                    It sat on the name's line for one batch, which was
                    the answer to 「centred on the pair it hangs between
                    them」 — true of a 16 glyph beside 12 and 10 point
                    text, and the wrong fix: what hung was a small mark
                    with nothing to align *to*. Grown to the height of
                    both lines it has the pair itself as its measure,
                    which is where a session tab's avatar already
                    stands. */}
                {notices ? (
                  <span data-tab-notice-look className="flex min-w-0 flex-1 items-center gap-tab-gap text-tab">
                    <NoticeLook list={notices} seat="size-4" />
                  </span>
                ) : (
                <span data-tab-avatar className={cn(grouped ? "flex flex-none" : "contents", grouped && !on && "session-tab-floor:group-hover/session-tab:opacity-0 session-tab-floor:group-focus-within/session-tab:opacity-0")}><TabAvatar f={f} bare="size-4 flex-none" seat="tab" /></span>
                )}
                {/* `nested`: the name sits inside the button that opens
                    the tab, so it is not a control here — the same
                    arrangement the list rows keep, and for the same
                    reason (`EditableName`'s head). */}
                {notices ? null : f.rename ? (
                  <span className={cn("flex min-w-0 flex-1 flex-col justify-center", grouped ? "session-tab-floor:hidden" : "@max-tab-face/tab:hidden")}>
                    <EditableName
                      kind={f.rename.kind}
                      id={f.rename.id}
                      title={f.label}
                      className="min-w-0 flex-1 text-tab"
                      inputClassName="w-full text-callout"
                      nested
                      // The hook the width gate measures, kept on the box
                      // that clips rather than on a wrapper around it
                      // (`boxData` says why that distinction is not a
                      // detail).
                      boxData={{ "data-tab-label": "" }}
                    />
                    {/* ⚠️ 墓碑 (UX-20): a terminal's directory stood under
                        its name here (roadmap 2.5). A tab is one line. */}
                    {f.progress}
                  </span>
                ) : f.sub ? (
                  // **Two lines, and the second one is the way to tell
                  // two tabs apart** (工作区批⑧). Four files called
                  // `mod.rs` make four tabs that read the same; where
                  // they are is the only thing that differs, so it is on
                  // the tab rather than only in a hover title. The pair
                  // fits inside the band because the band is the
                  // finger's floor (44) and two lines of 12/10 are 32.
                  <span data-tab-label className="flex min-w-0 flex-1 flex-col justify-center">
                    <span data-tab-name className="flex min-w-0 items-center gap-1 text-body leading-tight">
                      {/* **The preview's mark** (the audit fix plan F4.7;
                          W17): a small ring before the name, in place
                          of the slope — the one italic in the app. */}
                      {f.rang && <BellMark />}
                      {f.preview && <PreviewMark />}
                      {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : <MidName text={f.label} />}
                    </span>
                    {/* **Both lines start on one edge**, which is the
                        glyph's right side: the icon is beside the pair
                        now rather than inside the first line, so the
                        `pl-5` that used to push the path under the
                        name's text would push it past the name. */}
                    <span
                      data-tab-sub
                      className="truncate text-caption leading-tight text-muted-foreground"
                    >
                      {f.sub}
                    </span>
                  </span>
                ) : (
                  <span
                    data-tab-label
                    data-tab-name
                    className={cn("flex min-w-0 flex-1 items-center gap-1 text-tab", grouped ? "session-tab-floor:hidden" : "@max-tab-face/tab:hidden")}
                  >
                    {f.rang && <BellMark />}
                    {f.preview && <PreviewMark />}
                    {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : <MidName text={f.label} />}
                  </span>
                )}
              </button>
              </TooltipTrigger>
              </TabOpen>
              {again && (
                <TooltipContent data-tab-hint side="bottom">
                  {/* While files go in, the long form (「正在传入 … （M/N
                      已完成）」, the owner 2026-10-07 13:15) is what the
                      front tab's float says: the drop is on the tab in
                      front, and its hint is back once they are in. */}
                  {f.interaction?.drop && f.title ? (
                    f.title
                  ) : (
                    // The name first (UX-45): the front pill may be the
                    // face alone, and this float is its hover.
                    <>
                      <span className="block">{f.title ?? f.label}</span>
                      <span className="block text-muted-foreground">{again.hint}</span>
                    </>
                  )}
                </TooltipContent>
              )}
              </Tooltip>
              {/* **× at rest on every wide pill that has room** (UX-45
                  narrows UX-39 §7): the front pill at every width, down to
                  the floor (the owner 2026-10-07: crowded, the tab pressed
                  still owes its ×); any other at rest while it is at least
                  `--container-tab-x` wide, and under the pointer below
                  that, down to the floor (Chrome's). UX-39 §7's 「不悬浮就
                  显示 ×」 holds while there is room for it. In the seat the
                  act had; ⌘W, the menu and the sheet still close.
                  ⚠️ 墓碑 (UX-33 §1 → UX-39 §7): the third seat on every
                  wide pill; the wide face's one act is the verbs seat's
                  (or `frontAct`'s, after the capsule). ⚠️ 墓碑 (ux-q):
                  that seat on the strip hung under a session (`at`), the
                  phone's band before `SessionBar` drew its own. */}
              <button
                  type="button"
                  data-tab-close
                  // Under a finger the × is a 44 hit box (app.css F1.5).
                  data-glass-hit
                  aria-label={gui.close}
                  className={cn(
                    "flex h-full w-tab-close flex-none items-center justify-center hover:bg-muted hover:text-foreground",
                    on ? "text-muted-foreground" : "text-tertiary",
                    "rounded-full me-0.5",
                    // The front pill's × at every width; another's at rest
                    // while it has `--container-tab-x`, under the pointer
                    // down to the floor (UX-45, the owner 2026-10-07).
                    // Face alone, its seat is kept (invisible) under a
                    // mouse so the face does not move when it shows
                    // (ux-q); a finger has no hover and no seat.
                    grouped && !on && "session-tab-floor:absolute session-tab-floor:right-0 session-tab-floor:top-0 session-tab-floor:opacity-0 session-tab-floor:pointer-events-none session-tab-floor:group-hover/session-tab:opacity-100 session-tab-floor:group-hover/session-tab:pointer-events-auto session-tab-floor:group-focus-within/session-tab:opacity-100 session-tab-floor:group-focus-within/session-tab:pointer-events-auto",
                    grouped && !on && "@max-tab-x/tab:opacity-0 @max-tab-x/tab:group-hover/session-tab:opacity-100 @max-tab-x/tab:group-focus-within/session-tab:opacity-100",
                    !grouped && !on &&
                      "@min-tab-face/tab:@max-tab-x/tab:hidden @min-tab-face/tab:@max-tab-x/tab:group-hover:flex pointer-coarse:@max-tab-face/tab:hidden @max-tab-face/tab:invisible @max-tab-face/tab:group-hover:visible",
                  )}
                  onKeyDown={grouped ? e => {
                    if(e.key!==" "||e.metaKey||e.ctrlKey||e.altKey)return;
                    e.preventDefault();if(!e.repeat)e.currentTarget.click();
                  } : undefined}
                  onClick={(e) => {
                    e.stopPropagation();
                    // A pointer's press holds the row (`held`); a press
                    // from the keyboard (detail 0) re-lays at once.
                    if (e.detail > 0) {
                      const pill = e.currentTarget.closest<HTMLElement>("[data-tab]");
                      if (pill) setHeld(pill.getBoundingClientRect().width);
                      const el = strip.current;
                      if (grouped && el) heldWidths.current = new Map([...el.querySelectorAll<HTMLElement>("[data-tab]")].map(t => [t.dataset.tab || "", t.getBoundingClientRect().width]));
                      kept.current = el && { left: el.scrollLeft, width: el.clientWidth };
                    }
                    onClose(f.id);
                  }}
                >
                  <IconClose className="size-affordance" />
                </button>
              {/* **Progress under the pill itself** (UX-33 §2): a 2-pt line
                  along this pill's bottom edge while it reloads, Safari's
                  manner; nothing when done. */}
              {f.loading ? (
                <span data-tab-progress aria-hidden="true" className="pointer-events-none absolute inset-x-3 bottom-0.5 h-0.5 overflow-hidden rounded-full bg-fill">
                  <span className="block h-full w-[55%] animate-tab-load rounded-full bg-brand-strong" />
                </span>
              ) : (
                f.share !== undefined && (
                  <span data-tab-progress={f.share} aria-hidden="true" className="pointer-events-none absolute inset-x-3 bottom-0.5 h-0.5 overflow-hidden rounded-full bg-fill">
                    <span className="block h-full rounded-full bg-brand-strong" style={{ width: `${Math.round(f.share * 100)}%` }} />
                  </span>
                )
              )}
              </div>
              {grouped && f.group}
            </div>
          );
        })}
        {/* The room a held close leaves (`room`), while the row is held. */}
        {held !== null && <div ref={room} data-tab-hold-room aria-hidden="true" className="-ms-0.5 flex-none self-stretch" />}
      </div>
      {/* **After the last tab, a hairline and ＋** (UX-45, Chrome's): in
          the capsule, outside the scroller, so it stays when the strip
          scrolls. ＋ makes a new tab whose page is where one chooses.
          ⚠️ 墓碑 (UX-25 → UX-45): the count here once the strip was
          crowded, and the switcher it opened — every tab is drawn now,
          down to its face, and the strip scrolls past that. */}
      {onNew && (
        <span ref={plus} data-tab-new-seat className={cn("flex flex-none items-center", folded && "hidden")}>
          <TabDivider />
          <Button
            size="icon"
            variant="ghost"
            data-tab-new
            aria-label={gui.new_tab}
            className="min-h-touch min-w-touch flex-none rounded-full"
            onClick={() => { releaseHeld(); onNew(); }}
          >
            <IconPlus />
          </Button>
        </span>
      )}
      </div>
      {/* The secondary buttons, one seat so they step out together (the
          fold above); kept mounted, the verbs' seat is a portal's. */}
      <span ref={extras} data-tab-extras className={cn("flex flex-none items-center gap-in empty:hidden", folded && "hidden")}>
      {/* The front tab's one act, after the capsule (`frontAct`). */}
      {frontAct && (() => {
        const f = faces.find((x) => x.id === active);
        if (!f?.act) return null;
        // **已保存, said once at the button** (save-a, 「保存按钮只是作为一个
        // 额外的确定」): the ✓ turns to ink and its name stands under it for
        // the moment, without a hover — a confirmation nobody has to look
        // for. Its own instance, so the open name is not the hover one's.
        if (f.act.kind === "saved") {
          return (
            <Tooltip key="saved" open>
              <TooltipTrigger asChild>
                <GlassCircle
                  data-tab-act="saved"
                  data-tab-front-act
                  aria-label={f.act.label}
                  aria-disabled
                  className="pointer-events-none"
                >
                  <IconCheck />
                </GlassCircle>
              </TooltipTrigger>
              <TooltipContent side="bottom" data-saved-said>
                {f.act.label}
              </TooltipContent>
            </Tooltip>
          );
        }
        return (
          <Tooltip>
            <TooltipTrigger asChild>
              <GlassCircle
                data-tab-act={f.act.kind}
                data-tab-front-act
                aria-label={f.act.label}
                className={f.act.kind === "save" ? "text-brand-strong" : undefined}
                onClick={() => f.act?.onPress()}
              >
                {f.act.kind === "save" ? <IconCheck /> : f.act.kind === "stop" ? <IconClose /> : <IconReload className={cn(f.loading && "animate-spin motion-reduce:animate-none")} />}
              </GlassCircle>
            </TooltipTrigger>
            <TooltipContent side="bottom">{f.act.label}</TooltipContent>
          </Tooltip>
        );
      })()}
      {tail}
      </span>
      {grown.growth.mounted && (
        <>
          {/* Over the board only: the strip is the window's frame. */}
          <NoticeScrim
            on={grown.growth.big}
            openedAt={grown.openedAt}
            onFold={grown.fold}
            fixed
            style={{ top: capsule.current?.getBoundingClientRect().bottom }}
          />
          <NoticePanel list={stops} tab={front} board={capsule.current} growth={grown.growth} />
        </>
      )}
    </div>
  );
}

/**
 * The way to the switcher on the narrow face: the count *is* the
 * control, which is what makes it able to say how many without a second
 * mark. A phone shows one screen and the strip cannot be that screen.
 * ⚠️ 墓碑 (UX-45): the wide strip's count once it stopped showing every
 * tab — it draws every tab now, down to its face.
 */
export function TabCount({ n, onPress, open = false }: { n: number; onPress: () => void; open?: boolean }) {
  return (
    <Button
      variant="ghost"
      size="icon"
      data-tab-count
      // **A switch** (docs/SHELL.md 第四轮 第 10 条): pressed again, the
      // switcher it opened goes; which way it is, is said out loud.
      aria-expanded={open}
      data-open={open}
      aria-label={gui.switch_tabs(n)}
      onClick={onPress}
      // The count keeps its size; the box around it takes the floor.
      // This is the only way to every other tab, and on narrow it sits
      // at the end of the address bar where a thumb reaches — the one
      // control on this landing where being under the floor would be
      // felt every time.
      className="min-h-touch min-w-touch flex-none"
    >
      <span className="rounded-sm border px-1 text-caption tabular-nums">{n}</span>
    </Button>
  );
}

/**
 * The switcher: every tab as a row, with the way to close one and the
 * way to open another.
 *
 * A screen rather than a sheet, because it **is** the place you are —
 * the tab it replaces comes back the moment one is picked, and a sheet
 * over a screen you cannot see is a sheet over nothing.
 *
 * **It takes the tab's region, not the whole landing, and on wide the
 * strip stays above it.** That is deliberate rather than left over: the
 * switcher replaces what the *tab* was showing, and the strip is still
 * how a person knows which tab they will come back to. What it costs is
 * that the open tab appears twice for as long as the switcher is up —
 * accepted, because the alternative is hiding the one mark that says
 * where you are while you decide where to go.
 *
 * Rows run the full width of the region on both faces. Nothing caps
 * them: a wider window spends the extra on the path, which is the half
 * of a row that distinguishes two tabs with the same leaf, and a cap
 * would be a measure invented here for a list that has no other one.
 */
export function TabSwitcher({
  faces,
  active,
  onFocus,
  onClose,
  search = true,
  seat,
  onBack,
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  /** The narrow shell's switcher is a screen of its own, and every such
      screen has its way back: 「‹」 in the window's first row, beside the
      switcher's box (`NarrowHead`, 2026-09-28). It had none — picking a
      tab or pressing the rail were the only ways off it. */
  onBack?: () => void;
  /** Whether the switcher wears its own search box (docs/SHELL.md 第三轮
      第 3 条:「切换器头上加搜索框」). */
  search?: boolean;
  /** `"side"`: the box sits in the window's first row (`PaneBar`) — the
      narrow shell's switcher is a screen of its own, and a second box
      under L0's would be two search fields on one screen (2026-09-28). */
  seat?: "side";
}) {
  const [query, setQuery] = useState("");
  // By what a row shows — the name and where it is — and nothing else.
  const q = query.trim().toLowerCase();
  const shown = q === "" ? faces : faces.filter((f) => `${f.title ?? f.label} ${f.path}`.toLowerCase().includes(q));
  const box = search && (
    <PaneBar searchLabel={gui.search_tabs} query={query} onQuery={setQuery} seat={onBack ? "head" : seat} />
  );
  return (
    // In the content pane (no way back drawn), the switcher keeps to the
    // pane's two lines like its head (W-V5): the list's own 8 inside,
    // the rest here.
    <div data-tab-switcher className={cn("flex h-full min-h-0 flex-col", !onBack && "px-[calc(var(--page-inset)-var(--gap-in))]")}>
      {onBack ? (
        <NarrowHead>
          {(seated) => {
            const back = (
              <GlassCircle aria-label={gui.back} data-back onClick={onBack}>
                <IconBack />
              </GlassCircle>
            );
            return seated ? (
              <>
                {back}
                {box}
              </>
            ) : (
              <div className="flex min-h-pane-head flex-none items-center gap-in border-b px-2">
                {back}
                {box}
              </div>
            );
          }}
        </NarrowHead>
      ) : (
        box
      )}
      {/* **The rows are the list's rows** (the audit fix plan F2.6;
          N13): the same anatomy every list wears — the face in its slot,
          the name over the path, the hairline, the picked block — so a
          session looks the same here as in its list. Closing is the
          row's menu's, as before. ⚠️ 墓碑: rows of their own, 44 high
          with a 40 face and a grey block. */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        <SideList
          rows={shown.map(
            (f): SideRow => ({
              id: f.id,
              group: "",
              pinned: false,
              face: { machine: f.face },
              mark: f.session ? sessionKind(f.session.kind, f.session.category) : f.mark,
              halo: f.session?.word,
              title: (
                <span data-tab-label data-tab-name className="block truncate">
                  {f.title ?? f.label}
                </span>
              ),
              name: f.title ?? f.label,
              // One line (UX-20): no path under a tab's name.
              find: `${f.title ?? f.label} ${f.path}`,
              selected: f.id === active,
              menu: [...(f.menu ?? []), { key: "close-tab", label: gui.close, run: () => onClose(f.id) }],
              onOpen: () => onFocus(f.id),
              rowData: { "data-switch-tab": f.id, "data-on": f.id === active },
              openData: { "data-switch-open": f.id },
            }),
          )}
          label={() => ""}
          none={gui.no_matches}
        />
      </div>
    </div>
  );
}
