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
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { EditableName, useSinglePress } from "@/components/EditableName";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { SessionFace } from "@/components/SessionFace";
import { FaceMark, type KindMark } from "@/components/FaceMark";
import { IconBack, IconClose, IconEye, IconPlus } from "@/components/icons";
import { GlassCircle } from "@/components/Glass";
import { PaneBar } from "@/components/PaneBar";
import { NarrowHead } from "@/components/SideHead";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** One tab as it is shown: what it calls itself, whose machine it is
    on, and where. Derived by the shell (it holds the device table and
    the kind); this file paints and decides nothing. */
export type TabFace = {
  id: string;
  label: string;
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
  /** What this tab wears instead of a face. A file tab's subject is a
      file, and a machine's portrait on it would say whose disk rather
      than which file — the one thing the row already repeats. */
  glyph?: ReactNode;
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
};

/** The face a tab wears: the session assembly when it is a session,
    the bare machine face when it is a place. One decision, both the
    strip and the switcher read it — **at the size of the seat**: the
    strip's tab is 28 tall and takes the assembly at a tab's size
    (`SessionFace` `tab`, halo and corner inside the tab's box); the
    switcher's rows are list rows and keep a row's. */
function TabAvatar({ f, bare, seat }: { f: TabFace; bare: string; seat: "tab" | "row" }) {
  if (f.glyph) return <span className="flex-none">{f.glyph}</span>;
  if (f.session) return <SessionFace face={f.face} {...f.session} size={seat} />;
  // A browsing page: the machine it goes through, wearing its kind's
  // mark the way a session's tab does (the redesign, 2026-10-02).
  if (f.mark) return <FaceMark face={{ machine: f.face }} mark={f.mark} size={seat} />;
  return <MachineAvatar face={f.face} className={bare} />;
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
  if (text.length <= KEEP * 2) return <span className="min-w-0 truncate">{text}</span>;
  return (
    <>
      <span className="min-w-0 truncate">{text.slice(0, -KEEP)}</span>
      <span className="flex-none">{text.slice(-KEEP)}</span>
    </>
  );
}

/** The tab's opener, inside its menu when it has one. **A component
    rather than a ternary around the button**, so the button is written
    once: two copies of it is how the hover title and the anchor drift
    apart, and the anchor is what every gate finds a tab by. */
function TabOpen({ menu, children }: { menu?: RowAction[]; children: ReactNode }) {
  return menu && menu.length > 0 ? <RowMenu items={menu}>{children}</RowMenu> : <>{children}</>;
}

export function TabStrip({
  faces,
  active,
  onFocus,
  onClose,
  onNew,
  onSwitch,
  switching = false,
  head,
  tail,
  at = "top",
  glass = false,
}: {
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
  const strip = useRef<HTMLDivElement>(null);
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
    if (t.left < box.left) el.scrollLeft -= box.left - t.left;
    else if (t.right > box.right) el.scrollLeft += t.right - box.right;
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
      className={cn(
        "flex h-pane-head min-w-0 flex-1 items-center",
        glass ? "gap-in px-row" : "gap-1 px-in",
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
          glass
            ? cn("glass h-glass-tabs items-center self-center rounded-full px-glass-pad", below ? "flex-1" : "flex-initial")
            : "flex-1 self-stretch",
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
        className={cn("flex min-w-0 items-center overflow-x-auto", glass ? "gap-0.5 self-stretch" : "flex-1 gap-1", glass && below && "flex-1")}
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
                  : [[f.machine, f.path].filter(Boolean).join(": "), f.note]
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
                glass
                  ? on
                    ? "max-w-tab-on flex-none glass-sel font-title text-foreground"
                    : "max-w-tab flex-none text-muted-foreground hover:bg-fill"
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
                {!(f.glyph && f.sub) && <TabAvatar f={f} bare="size-4 flex-none" seat="tab" />}
                {f.glyph && f.sub && <span className="flex-none">{f.glyph}</span>}
                {/* `nested`: the name sits inside the button that opens
                    the tab, so it is not a control here — the same
                    arrangement the list rows keep, and for the same
                    reason (`EditableName`'s head). */}
                {f.rename ? (
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
                ) : f.sub ? (
                  // **Two lines, and the second one is the way to tell
                  // two tabs apart** (工作区批⑧). Four files called
                  // `mod.rs` make four tabs that read the same; where
                  // they are is the only thing that differs, so it is on
                  // the tab rather than only in a hover title. The pair
                  // fits inside the band because the band is the
                  // finger's floor (44) and two lines of 12/10 are 32.
                  <span data-tab-label className="flex min-w-0 flex-1 flex-col justify-center">
                    <span
                      data-tab-name
                      className={cn("flex min-w-0 text-body leading-tight", f.preview && "italic")}
                    >
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
                    className={cn("flex min-w-0 flex-1", glass ? "text-tab" : "text-body", f.preview && "italic")}
                  >
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
              {/* 预览, between the name and the ×: the order a person
                  reads the tab in is what it is, then what to do with
                  it. The ×'s own box, `--affordance`: every mark fills
                  one share of its box (icons.tsx `LIVE`), so one box is
                  one painted size — the eye 11.4 of ink, the × 12, under
                  the label. ⚠️ 墓碑: 「deliberately the smaller mark」 at
                  `size-3.5` — in a box under the ×'s it painted 14 to the
                  ×'s 9.5. */}
              {on && f.look && (
                <Button
                  size="icon"
                  variant="ghost"
                  data-file-face-switch
                  data-on={f.look.on}
                  aria-label={f.look.label}
                  onClick={() => f.look?.onToggle()}
                  className={cn(
                    "min-h-touch min-w-touch flex-none",
                    f.look.on
                      ? "text-[var(--rail-on-ink)] hover:text-[var(--rail-on-ink)]"
                      : "text-muted-foreground",
                  )}
                >
                  <IconEye className="size-affordance" />
                </Button>
              )}
              <Tooltip>
              <TooltipTrigger asChild>
              <button
                type="button"
                data-tab-close={f.id}
                aria-label={gui.close_tab}
                // **The ×'s own cell** (docs/SHELL.md 第十三轮 第 6 条, the
                // user's VS Code screenshot:「标签页的 x 按钮不要覆盖在
                // 文字上, 就是标签页的一块」). Every tab is a face, a
                // name and this cell — `--tab-close` wide, the tab's whole
                // height, **always there**: the name is a flex item
                // before it and truncates in front of it, so it can never
                // run under the ×. That is the arrangement the cell
                // replaces: a 44-wide close that, on a stub, sat
                // `absolute` over the name's end on a patch of `--muted`,
                // which was the × drawn on top of the word.
                //
                // **On every tab** (the redesign, 2026-10-02, the owner:
                // 「那些标签页, 可以把关闭的 "x" 带上去, 不然, 一定得打开
                // 标签才能关闭」): the front tab's in secondary ink, the
                // others' in tertiary, taking secondary when the tab is
                // under the pointer. A press closes that tab where it
                // stands — it is not brought forward first, so the tab in
                // front stays in front. ⚠️ 墓碑: drawn only on the open tab
                // and under the pointer (the session rows' rule), which left
                // a phone no way to close a tab without opening it. Its own
                // target, the tab's full height and `--tab-close` wide (24
                // under a mouse, 32 under a finger), so a thumb on the label
                // does not close it; the label gives width before it does.
                //
                // **A tab with a draft keeps its mark drawn**, dot or
                // not: the dot *is* the report that something is waiting
                // here, and a report that only appears under a pointer
                // is one a phone never sees — it stands in the cell.
                //
                // A bare `<button>` rather than the icon `Button`: that
                // variant carries `w-ctl-md`, and `cn()` cannot lift a
                // `ctl-*` width off (`button.tsx` measured that), so a
                // cell that asked for `--ctl-sm` would have got whichever
                // the stylesheet emitted last. Under the floor on purpose,
                // and the face gate names it so (第十三轮): the cell's width
                // is the ruling's, and a finger has the switcher's close
                // at the floor.
                className={cn(
                  "flex h-full w-tab-close flex-none items-center justify-center outline-none",
                  glass ? "rounded-full" : "rounded-sm",
                  on || f.dirty ? "text-muted-foreground" : "text-tertiary group-hover:text-muted-foreground",
                  "hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50",
                )}
                onClick={() => onClose(f.id)}
              >
                {/* 「正在修改的,和 vscode 一致,标签上有一个圆点。保存
                    成功了,圆点消失。」— the user, 2026-09-15. **The dot
                    replaces the ×, it does not sit beside it**: one mark
                    in one place, and pressing it still closes, which is
                    what makes 「关一个有草稿的标签问一句」 the thing
                    that catches the mistake rather than a second
                    control nobody aimed at. */}
                {f.dirty ? (
                  <span
                    data-tab-dot
                    aria-hidden="true"
                    className="size-2 rounded-full bg-current"
                  />
                ) : (
                  <IconClose className="size-affordance" />
                )}
              </button>
              </TooltipTrigger>
              <TooltipContent side={below ? "top" : "bottom"}>{gui.close_tab}</TooltipContent>
              </Tooltip>
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
  const shown = q === "" ? faces : faces.filter((f) => `${f.label} ${f.path}`.toLowerCase().includes(q));
  const box = search && (
    <PaneBar searchLabel={gui.search_tabs} query={query} onQuery={setQuery} seat={onBack ? "head" : seat} />
  );
  return (
    <div data-tab-switcher className="flex h-full min-h-0 flex-col">
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
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {shown.map((f) => (
          <div
            key={f.id}
            data-switch-tab={f.id}
            data-on={f.id === active}
            className={cn(
              // `py-1` where it was `py-2`: the row's own height now
              // comes from the two controls in it, both of which are at
              // the finger's floor, and keeping the old padding on top
              // of that would have made a tab list of 60-tall rows out
              // of one that reads fine at 52.
              "flex items-center gap-2 rounded-md px-2 py-1",
              f.id === active && "bg-muted",
            )}
          >
            {/* The row's menu hangs on the button that opens the tab, the
                strip's arrangement (`TabOpen`) — every sidebar's rows
                answer a right-click, this one included (docs/SHELL.md
                第二轮). */}
            <TabOpen menu={[...(f.menu ?? []), { key: "close-tab", label: gui.close, run: () => onClose(f.id) }]}>
            <button
              type="button"
              data-switch-open={f.id}
              onClick={() => onFocus(f.id)}
              // The name and the path are what a person aims at here;
              // at content height the row's padding looked like part of
              // the target and was the parent's.
              className="flex min-h-touch min-w-0 flex-1 items-center gap-2 text-left"
            >
              <TabAvatar f={f} bare="size-avatar flex-none" seat="row" />
              <span className="flex min-w-0 flex-1 flex-col">
                {/* **The same two names the strip's label wears**, because
                    it is the same fact: what this tab is called, as
                    painted. Since 2026-09-20 the rows are the browsing
                    landing's tabs as well as its switcher (docs/SHELL.md
                    第 2 条), so a probe reaching for 「the tab's title」
                    has to find it in whichever seat the tabs are in —
                    `check-tabs` was looking only at the strip's and read
                    the column as a landing with no titles at all. */}
                <span data-tab-label data-tab-name className="truncate text-callout font-label">
                  {f.label}
                </span>
                {/* The whole path under the name: on a phone the strip
                    is not there to give the tab context, so the row has
                    to carry it. Truncated from the tail — a slashed
                    path read right to left is not a path. */}
                {f.path && (
                  <span className="truncate text-caption text-muted-foreground">{f.path}</span>
                )}
              </span>
            </button>
            </TabOpen>
            {/* ⚠️ 墓碑 (the redesign, 2026-10-02, the owner: one act, one
                home): a × stood at the row's end, always drawn. Closing a
                tab is the row's menu's now — right-click, or a long press
                (`TabOpen` below gets it). */}
          </div>
        ))}
      </div>
    </div>
  );
}
