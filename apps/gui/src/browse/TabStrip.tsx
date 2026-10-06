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
import { useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { EditableName, useSinglePress } from "@/components/EditableName";
import { MachineAvatar } from "@/components/Avatar";
import { fontOf, measurer } from "@/components/PathCut";
import type { EditItem } from "@/hooks/use-read-first";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { SessionFace } from "@/components/SessionFace";
import { FaceMark, type KindMark } from "@/components/FaceMark";
import { IconBack, IconCheck, IconClose, IconPlus, IconReload } from "@/components/icons";
import { GlassCircle } from "@/components/Glass";
import { PaneBar } from "@/components/PaneBar";
import { RegionLead } from "@/components/Region";
import { NarrowHead } from "@/components/SideHead";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { SideList, type SideRow } from "@/components/SideList";
import { sessionKind } from "@/components/SessionFace";
import { gui } from "@/gen/catalog";
import { stillMotion, TRIP_MS, useTravel } from "@/lib/travel";
import { cn } from "@/lib/utils";

/** One tab as it is shown: what it calls itself, whose machine it is
    on, and where. Derived by the shell (it holds the device table and
    the kind); this file paints and decides nothing. */
export type TabFace = {
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
      tab nobody has listed yet. */
  act?: { kind: "reload" | "stop" | "save"; label: string; onPress: () => void };
  /** **The pill reloads**: a 2-pt line runs along its own bottom edge
      while a join, a load or a read is under way (UX-33 §2; UX-31's slot). */
  loading?: boolean;
};

/** The face a tab wears: the session assembly when it is a session,
    the bare machine face when it is a place. One decision, both the
    strip and the switcher read it — **at the size of the seat**: the
    strip's tab is 28 tall and takes the assembly at a tab's size
    (`SessionFace` `tab`, halo and corner inside the tab's box); the
    switcher's rows are list rows and keep a row's. */
export function TabAvatar({ f, bare, seat }: { f: TabFace; bare: string; seat: "tab" | "row" }) {
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
function MidName({ text }: { text: string }) {
  const KEEP = 5;
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
      const room = sizer.clientWidth;
      if (room <= 0) return;
      const wide = measurer(fontOf(sizer));
      if (wide(text) <= room - 1) {
        setShown(text);
        return;
      }
      const chars = [...text];
      const tail = chars.slice(-KEEP).join("");
      let lo = 0;
      let hi = chars.length - KEEP;
      while (lo < hi) {
        const mid = Math.ceil((lo + hi) / 2);
        if (wide(`${chars.slice(0, mid).join("")}…${tail}`) <= room - 1) lo = mid;
        else hi = mid - 1;
      }
      setShown(`${chars.slice(0, lo).join("")}…${tail}`);
    };
    fit();
    const watch = new ResizeObserver(fit);
    watch.observe(sizer);
    return () => watch.disconnect();
  }, [sizer, text]);
  if (text.length <= KEEP * 2) return <span className="min-w-0 truncate">{text}</span>;
  return (
    <span data-mid-name className="relative flex min-w-0">
      <span ref={setSizer} aria-hidden className="invisible min-w-0 overflow-hidden whitespace-nowrap">
        {text}
      </span>
      <span aria-hidden className="absolute inset-0 overflow-hidden whitespace-nowrap">
        {shown}
      </span>
      <span className="sr-only">{text}</span>
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
function useRebalance(host: () => HTMLElement | null, active: string) {
  const was = useRef(new Map<string, number>());
  const last = useRef(active);
  const note = () => {
    const h = host();
    if (!h) return;
    was.current = new Map([...h.querySelectorAll<HTMLElement>("[data-tab]")].map((t) => [t.getAttribute("data-tab") ?? "", t.getBoundingClientRect().width]));
  };
  useLayoutEffect(() => {
    const h = host();
    if (!h) return;
    const before = was.current;
    const changed = last.current !== active;
    last.current = active;
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

export function TabStrip({
  faces,
  active,
  onFocus,
  // Closing left the pill with UX-33 (the sheet, ⌘W and the menu close);
  // the strip still takes the handler so every caller stays one shape.
  onClose,
  onNew,
  onSwitch,
  switching = false,
  head,
  tail,
  at = "top",
  glass = false,
  frontAct = false,
}: {
  /** **The front tab's act at the capsule's end, and none on the pills**
      (UX-39, the owner 2026-10-06 22:35: a ↻ on every pill and one more
      at the strip's end was 「有点奇怪」). On the wide face a tab is a
      tab; the one act stands at the end, Safari's arrangement. The
      session strip leaves this off (the terminal's own ↻ already stands
      in the verbs seat) and so does the browser (the address field has
      its ↻); the workspace strip takes it for the file's ↻ / ✓. ⚠️ 墓碑
      (UX-33 §1): the third seat on every wide pill. */
  frontAct?: boolean;
  /** **The glass look** (the redesign, 2026-10-02): the tabs are pills in
      one glass capsule — the one in front a `fill-strong` pill with its
      ×, the rest ink — and the count sits at the capsule's end after a
      hairline. `head` and `tail` stand outside the capsule, as glass
      circles of their own. The session tabs wear it, at the foot of a
      session on the narrow face and at the top of the content on the
      wide one; a workspace's file tabs keep the strip they had. */
  glass?: boolean;
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  /** Add a tab. Absent on a strip whose tabs are not made here — the
      session strip's tabs come from the list, so it draws no "+". */
  onNew?: () => void;
  /** Open the switcher. Drawn **only once the strip has stopped showing
      every tab** — see the file head; a landing that has no switcher to
      open leaves this out and gets the strip alone. */
  onSwitch?: () => void;
  /** Whether the switcher the count opens is up — the count stays drawn
      while it is, so the press that opened it can close it. */
  switching?: boolean;
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
  /**
   * Which side of its content the strip stands on (2026-10-02, iOS 第三步
   * A). `"top"` is every strip there was: R0, the tabs standing on the
   * board and joined to it by their foot. `"bottom"` is the narrow
   * session page's bar (`components/SessionBar`): the content is above,
   * so the open tab **hangs** from it — its foot is at the top, its
   * rounded corners at the bottom, the label still on the row's
   * centreline — and the paper it is joined to is the session over it.
   * Same tabs, same widths, same scroll; only the joint moves.
   */
  at?: "top" | "bottom";
}) {
  const below = at === "bottom";
  /** The list is folded and its strip's block leads this row
      (`RegionLead`): the capsule follows the block, a gap after it. */
  const led = useContext(RegionLead) !== null;
  const strip = useRef<HTMLDivElement>(null);
  // The front tab's piece slides to the tab brought forward, in the glass
  // capsule (F7.2, `lib/travel`).
  useTravel(below ? "tabs-below" : "tabs", () => (glass ? strip.current : null), active);
  const widths = useRebalance(() => (glass ? strip.current : null), active);
  // Whether the strip has given up on its own promise. Measured off the
  // element, not counted from `faces.length` and a width: how many tabs
  // fit depends on the region, which the rail, the window and the
  // landing all have a say in, and a number derived here would be a
  // second opinion about a layout the browser has already decided.
  const [crowded, setCrowded] = useState(false);
  /** Which ends have a tab cut off behind them, for the fade on that
      side (docs/SHELL.md 第三轮 第 2 条). No fade where nothing is cut:
      a fade over a whole tab would be a mark that says 「more this
      way」 about nothing. */
  const [cut, setCut] = useState({ left: false, right: false });
  /**
   * **The tab in front is scrolled into view** whenever it changes — a
   * click, a shortcut, a pick from the switcher, a new one (第三轮 第 2
   * 条) — and when the strip itself changes width, which is how a tab
   * that was in view ends up past the edge without anybody touching it.
   * Measured against the strip rather than with `scrollIntoView`, which
   * also scrolls every ancestor that can scroll, and the shell's own
   * boxes are not this strip's to move.
   */
  const bringFront = useRef(() => {});
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
      setCrowded(over > 1);
      setCut({ left: el.scrollLeft > 1, right: over - el.scrollLeft > 1 });
      widths.note();
    };
    measure();
    // The observer catches the region changing; the dependency catches
    // tabs opening and closing. Neither alone sees both.
    const ro = new ResizeObserver(() => {
      bringFront.current();
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

  useLayoutEffect(() => bringFront.current(), [active, faces.length]);

  /** Whether the front tab's hint is up (`TabFace.again`). One for the
      strip: only the tab in front ever has one. */
  const [hint, setHint] = useState(false);
  useEffect(() => setHint(false), [active]);
  /** A press on the front tab, resolved against a double-click on its
      name — which renames, and must not show and hide the day on its
      way there (`useSinglePress`). */
  const pressAgain = useSinglePress(() => faces.find((f) => f.id === active)?.again?.toggle?.());

  return (
    <div
      data-tab-bar
      // **The band is the app's ground and the open tab is the board's
      // colour** (docs/SHELL.md 第十三轮 第 5 条: the band wears nothing
      // of its own since the board under it floats on `--background`,
      // and the open tab is `--card` with its top corners rounded and
      // no bottom edge, joined to the board), which is the whole of how
      // a tab says it is in front: it reads as continuous with what is
      // under it, and the ones that are not open sit transparent on the
      // ground behind it. That is Chrome's arrangement, and it is not a
      // stylistic borrowing — it is the only one where "in front" is
      // said by the same surface that the content is on.
      //
      // The first spelling had it the other way round (a light tab on
      // the page ground, a rule under the strip) and the open tab read
      // as a floating pill with a line separating it from its own
      // content — measured on a screenshot, which is the only way that
      // kind of mistake shows.
      //
      // No `border-b`: the change of ground *is* the boundary, and a
      // rule as well would cut the open tab off from the page again.
      // A tab reaching the band's floor and a top-only radius do the
      // rest.
      //
      // The strip scrolls rather than shrinking its tabs (tokens.css
      // `--tab-w` says why they are fixed), and `flex-none` keeps it
      // from being the thing that gives when the window is short.
      //
      // **The band is as tall as the floor under a finger**, which is
      // the one place in this file where Chrome's proportions had to
      // give (tokens.css `--touch`). `overflow-x-auto` makes this a
      // scroll container on *both* axes — a child taller than the band
      // would be clipped or made scrollable, not allowed to hang over
      // it — so a hit area that reaches the floor while the tab stays
      // short is not available here: either the band grows or the
      // target stays under the floor. It grows.
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
      // row's middle — the count, the landing's pins at either end, 「+」
      // and every tab's label — the line macOS's three buttons, the
      // search box and a page's title stand on. Only the tabs reach the
      // floor, by their foot (`--tab-lip`, the tab below). It was
      // `items-end` with the tabs standing on the floor, twenty under
      // the window's edge on the 48 row: the blank the owner asked
      // about.
      //
      // **The band and the scrolling row are two elements now, and the
      // reason is a measurement**: with everything in one scroll
      // container the "+" scrolled away with the tabs — at 20 tabs on a
      // 1400 window it sat at x=1912 while the strip's visible edge was
      // 1400, **512 past the end of the world**, so the way to open a
      // tab was to scroll right until you found it. The band keeps the
      // ground and the padding; the row inside it is what scrolls.
      //
      // ⚠️ **The new-tab button moved back inside the scroller on
      // 2026-09-09, and this comment used to argue it out.** It said
      // Chrome pins it and that pinning means being outside the scroll
      // container — the second half is true and the first was wrong:
      // Chrome puts「+」immediately after the last tab and shrinks tabs
      // rather than scrolling them. Pinned at the far right it read as a
      // control belonging to the row, not to the tabs. **What that costs
      // is real and is the reason the old note existed**: once the strip
      // scrolls, 「+」 scrolls away with the last tab. The way to a new
      // tab when it is crowded is the switcher, which has its own — and
      // the switcher is offered exactly when the strip has stopped
      // showing every tab.
      // **A glass piece aligns by its glass edge** (the owner,
      // 2026-10-03: 「浏览器这没有左对齐」): over the wide content the
      // capsule's left edge stands on the page's left line (`px-page`,
      // the machine page's avatar and the browser's steps capsule
      // under it). ⚠️ 墓碑: `px-row`, 12 short of the line. At a
      // session's foot the row keeps its own inset (`SessionBar`).
      className={cn(
        "flex h-pane-head min-w-0 flex-1 items-center",
        // Folded, the strip's block leads the row and the capsule
        // follows it as before, a gap after it.
        glass ? (below || led ? "gap-in px-row" : "gap-in px-page") : "gap-1 px-in",
      )}
    >
      {head}
      {/* The scroller and the two fades over its ends. **No scrollbar**
          (第三轮 第 2 条:「标签会被移动条顶出窗口. 且此条无法拖动」):
          `app.css` hides it on `[data-tab-strip]`; the wheel, the fades,
          the count and the switcher are the ways past the edge. */}
      <div
        data-tab-capsule={glass || undefined}
        className={cn(
          "relative flex min-w-0",
          // At a session's foot the capsule takes the row between the way
          // back and the keyboard; over the wide face's content it is as
          // long as its tabs (the canvas).
          // **The capsule spans the row** (UX-39, the owner 2026-10-06
          // 22:55: 「全挤在左边了」): Safari's tab bar — the tabs share the
          // whole width between the row's start and the act circle, the
          // front one as wide as its name (up to half), the rest equal.
          // ⚠️ 墓碑 (UX-25): the capsule as long as its tabs, at the left.
          glass ? "glass h-glass-tabs flex-1 items-center self-center rounded-full px-glass-pad" : "flex-1 self-stretch",
        )}
      >
      {!glass && cut.left && (
        <div
          data-tab-fade="left"
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 left-0 z-10 w-6 bg-linear-to-r from-background to-transparent"
        />
      )}
      {!glass && cut.right && (
        <div
          data-tab-fade="right"
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 right-0 z-10 w-6 bg-linear-to-l from-background to-transparent"
        />
      )}
      <div
        ref={strip}
        data-tab-strip
        // `items-center`: 「+」 on the row's centreline with the tabs'
        // labels; the tabs themselves stretch to the floor (their foot).
        className={cn("flex min-w-0 items-center overflow-x-auto", glass ? "flex-1 gap-0.5 self-stretch" : "flex-1 gap-1")}
        // **In the glass capsule a scrolled edge fades** (the audit fix plan
        // F3.3; W09, N19): a mask over the 12 at each end that has a tab
        // cut off behind it, instead of a glyph cut in half.
        style={
          glass && (cut.left || cut.right)
            ? {
                maskImage: `linear-gradient(to right, ${cut.left ? "transparent, black var(--fade-tabs)" : "black, black"}, ${cut.right ? "black calc(100% - var(--fade-tabs)), transparent" : "black, black"})`,
              }
            : undefined
        }
      >
        {faces.map((f) => {
          const on = f.id === active;
          const again = on ? f.again : undefined;
          return (
            <div
              key={f.id}
              data-tab={f.id}
              data-on={on}
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
                  : [f.title, [f.machine, f.path].filter(Boolean).join(": "), f.note]
                      .filter(Boolean)
                      .join(" · ") || undefined
              }
              className={cn(
                // **A `--hit` tall label on the row's centreline and a
                // foot under it** (2026-09-29): `mt-tab-lip` over the
                // label, `pb-tab-lip` under it, and the tab stretched to
                // the band's floor — so the label (and the ×, and the
                // eye) sit on the line the window's buttons and the search
                // box stand on, while the tab's colour runs on down into
                // the board it is joined to. The label keeps the search
                // box's height (2026-09-20 第 5 条); the foot is the
                // joint. `--hit`, not `--ctl-md`: the tab is the target,
                // and it is also what paints "this one is in front" —
                // a tab whose colour stops short of where it can be
                // pressed is a tab that lies about where it is, which is
                // why the button below reaches into the foot.
                glass
                  ? "group relative flex h-glass-tab items-center gap-1 self-center rounded-full pr-0.5 pl-1.5"
                  : below
                    ? "group relative flex mb-tab-lip items-center gap-1 self-stretch rounded-b-md pr-0.5 pt-tab-lip pl-2"
                    : "group relative flex mt-tab-lip items-center gap-1 self-stretch rounded-t-md pr-0.5 pb-tab-lip pl-2",
                // **Two widths, and each tab wears exactly one.** The one
                // in front takes its own name's width — never less than a
                // stub, so it does not read as the smallest tab on the
                // strip, and up to the ceiling; every other is the stub.
                // Spelled as two branches rather than a base plus an
                // override because `w-tab` is a custom spacing class and
                // `cn()` cannot lift one of those off another (button.tsx
                // measured that) — both would survive and the
                // stylesheet's order would pick.
                //
                // **…and the stubs give together before the strip scrolls**
                // (docs/SHELL.md 第三轮 第 1 条:「先压，再溢出」). A stub is
                // `--tab-w` at rest and shrinks, every one by the same
                // amount, down to `--tab-w-min` — so they are still all
                // one width, which is what 2026-09-02 asked for; only
                // once every stub is at the floor does the strip overflow.
                // The tab in front never gives.
                // **In the glass capsule, Safari's widths** (UX-25 addendum,
                // the owner 2026-10-05): every pill not in front is one
                // width — an equal share of the capsule, `--tab-w` at most,
                // `--tab-w-min` at least — and the one in front takes its
                // whole name up to `--tab-w-on`, the others giving way down
                // to their floor before the strip scrolls. The strip
                // re-balances on each choice over the chosen piece's trip
                // (`useRebalance`). ⚠️ 墓碑 (V07): one ceiling for every
                // tab, the front one too, so a choice moved no width.
                glass
                  ? on
                    ? // The front pill starts from its whole name and grows
                      // with the rest, so it is never the narrowest and a
                      // long name is read whole (up to half the capsule
                      // when it has neighbours).
                      cn("min-w-tab-min shrink-0 grow basis-auto glass-sel text-foreground", faces.length > 1 && "max-w-[50%]")
                    : "min-w-tab-min shrink grow basis-0 text-muted-foreground hover:bg-fill"
                  : on
                    ? "min-w-tab max-w-tab-on flex-none bg-card text-foreground"
                    : "min-w-tab-min shrink basis-tab text-muted-foreground hover:bg-muted",
              )}
            >
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
                  if (again) pressAgain(e);
                }}
                aria-expanded={again?.expanded}
                aria-controls={again?.expanded ? again.controls : undefined}
                // `self-stretch`: the whole tab opens it, not the line of
                // text in the middle of it. Left at content height this
                // was 18 tall inside a tab it filled the width of — the
                // band around the label looked pressable and was not.
                // **…and down through the foot** (2026-09-29): the tab's
                // `pb-tab-lip` would otherwise be a strip of painted tab
                // that moves the window instead of opening it (R0 is a
                // drag region, and only a control keeps its press), so
                // the button hangs `-mb-tab-lip` into it and pads its
                // label back up onto the centreline.
                // `gap-tab-gap`, not the `gap-1` this had: 「标签页上的
                // 标题距离头像稍远一些, 现在有些拥挤」 (2026-09-20 第 3
                // 条). Four pixels put the name against the face while
                // the tab's own `gap-1` put the same four between the
                // name and the × — so the face read as part of the
                // word. Its own token (tokens.css `--tab-gap`) rather
                // than a step up in `--gap-in`, which would move every
                // clump in the app for one complaint about tabs.
                className={cn(
                  "flex min-w-0 flex-1 items-center gap-tab-gap self-stretch text-left",
                  glass && "text-tab",
                  // Into the foot, wherever the foot is.
                  !glass && (below ? "-mt-tab-lip pt-tab-lip" : "-mb-tab-lip pb-tab-lip"),
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
                <TabAvatar f={f} bare="size-4 flex-none" seat="tab" />
                {/* `nested`: the name sits inside the button that opens
                    the tab, so it is not a control here — the same
                    arrangement the list rows keep, and for the same
                    reason (`EditableName`'s head). */}
                {f.rename ? (
                  <span className="flex min-w-0 flex-1 flex-col justify-center">
                    <EditableName
                      kind={f.rename.kind}
                      id={f.rename.id}
                      title={f.label}
                      className={cn("min-w-0 flex-1", glass ? "text-tab" : "text-body")}
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
                      {f.preview && <PreviewMark />}
                      <MidName text={f.label} />
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
                    className={cn("flex min-w-0 flex-1 items-center gap-1", glass ? "text-tab" : "text-body")}
                  >
                    {f.preview && <PreviewMark />}
                    <MidName text={f.label} />
                  </span>
                )}
              </button>
              </TooltipTrigger>
              </TabOpen>
              {again && (
                <TooltipContent data-tab-hint side={below ? "top" : "bottom"}>
                  {again.hint}
                </TooltipContent>
              )}
              </Tooltip>
              {/* **× on every wide pill, always** (UX-39 §7, the owner
                  2026-10-06 22:45–22:50: 「桌面版的看不见 ×」, 「不悬浮就显
                  示 ×」): in the seat the act had; ⌘W, the menu and the
                  sheet still close. The phone's band keeps no ×.
                  ⚠️ 墓碑 (UX-33 §1 → UX-39 §7): the third seat on every
                  wide pill; the wide face's one act is the verbs seat's
                  (or `frontAct`'s, after the capsule). */}
              {!below && (
                <button
                  type="button"
                  data-tab-close
                  // Under a finger the × is a 44 hit box (app.css F1.5).
                  data-glass-hit
                  aria-label={gui.close}
                  className={cn(
                    "flex h-full w-tab-close flex-none items-center justify-center hover:bg-muted hover:text-foreground",
                    on ? "text-muted-foreground" : "text-tertiary",
                    glass ? "rounded-full" : "rounded-sm",
                  )}
                  onClick={(e) => {
                    e.stopPropagation();
                    onClose(f.id);
                  }}
                >
                  <IconClose className="size-affordance" />
                </button>
              )}
              {/* **The third seat, on the phone's band** (UX-33): what
                  aligns this tab with its source — a session's ↻, a
                  page's ↻ / ×, a file's ↻ / ✓; the save face wears the
                  brand ink. */}
              {f.act && below && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      data-tab-act={f.act.kind}
                      aria-label={f.act.label}
                      className={cn(
                        "flex h-full w-tab-close flex-none items-center justify-center focus-visible:-outline-offset-2",
                        glass ? "rounded-full" : "rounded-sm",
                        f.act.kind === "save" ? "text-brand-strong" : on ? "text-muted-foreground" : "text-tertiary group-hover:text-muted-foreground",
                        "hover:bg-muted hover:text-foreground",
                      )}
                      onClick={(e) => {
                        e.stopPropagation();
                        f.act?.onPress();
                      }}
                    >
                      {f.act.kind === "save" ? (
                        <IconCheck className="size-affordance" />
                      ) : f.act.kind === "stop" ? (
                        <IconClose className="size-affordance" />
                      ) : (
                        <IconReload className={cn("size-affordance", f.loading && "animate-spin motion-reduce:animate-none")} />
                      )}
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side={below ? "top" : "bottom"}>{f.act.label}</TooltipContent>
                </Tooltip>
              )}
              {/* **Progress under the pill itself** (UX-33 §2): a 2-pt line
                  along this pill's bottom edge while it reloads, Safari's
                  manner; nothing when done. */}
              {f.loading && (
                <span data-tab-progress aria-hidden="true" className="pointer-events-none absolute inset-x-3 bottom-0.5 h-0.5 overflow-hidden rounded-full bg-fill">
                  <span className="block h-full w-[55%] animate-tab-load rounded-full bg-brand-strong" />
                </span>
              )}
            </div>
          );
        })}
        {onNew && (
          <Button
            size="icon"
            variant="ghost"
            data-tab-new
            aria-label={gui.new_tab}
            // The `mb-1` it used to wear lifted a 30 button off the band's
            // floor so it read as level with the 30 tabs. Both are `--hit`
            // now and both centred on the row (2026-09-29: the tabs' labels
            // are, their feet reach the floor) — a margin would put it off
            // the line.
            className="min-h-touch min-w-touch flex-none"
            onClick={onNew}
          >
            <IconPlus />
          </Button>
        )}
      </div>
      {glass && onSwitch && (crowded || switching) && (
        <>
          <span aria-hidden="true" className="h-4.5 w-px flex-none bg-border" />
          <TabCount n={faces.length} onPress={onSwitch} open={switching} bare />
        </>
      )}
      </div>
      {/* The front tab's one act, after the capsule (`frontAct`). */}
      {frontAct && !below && (() => {
        const f = faces.find((x) => x.id === active);
        if (!f?.act) return null;
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
      {/* Pinned against the scrolling row, in the order a hand reaches
          them: the way to see the ones that scrolled off, then the way
          to add another.

          **The condition can stop holding while somebody is inside the
          switcher** — closing tabs from in there is one of the three
          things they went in to do — and then this button leaves while
          the switcher is still up. Measured (14 tabs, closing from
          inside): it goes at 11, and the switcher stays up with 11 rows,
          every one of them a way out, picking one puts it away. So the
          way *in* disappearing is not a way *out* disappearing, and
          holding the button drawn on `switching` would be drawing "go
          see your tabs" over the screen that is showing them. */}
      {!glass && onSwitch && (crowded || switching) && <TabCount n={faces.length} onPress={onSwitch} open={switching} />}
      {tail}
    </div>
  );
}

/**
 * The way to the switcher: the count *is* the control, which is what
 * makes it able to say how many without a second mark.
 *
 * **Both faces use it, for one reason spelled two ways.** On narrow it
 * is always there, because a phone shows one screen and the strip
 * cannot be that screen. On wide it appears only once the strip has
 * stopped showing every tab (`TabStrip`) — the same question, "how do I
 * get to the ones I cannot see", asked by a window that ran out of room
 * instead of by one that never had any. It is not a narrow control
 * borrowed onto the wide face; the narrow face is just the width where
 * the condition is permanently true.
 */
export function TabCount({
  n,
  onPress,
  open = false,
  bare = false,
}: {
  n: number;
  onPress: () => void;
  open?: boolean;
  /** Inside the glass capsule: the number alone, no box round it. */
  bare?: boolean;
}) {
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
      <span className={cn("text-caption tabular-nums", bare ? "text-muted-foreground" : "rounded-sm border px-1")}>{n}</span>
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
