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
import { EditableName } from "@/components/EditableName";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { SessionFace } from "@/components/SessionFace";
import { IconClose, IconEye, IconPlus } from "@/components/icons";
import { PaneBar } from "@/components/PaneBar";
import { Button } from "@/components/ui/button";
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
};

/** The face a tab wears: the session assembly when it is a session,
    the bare machine face when it is a place. One decision, both the
    strip and the switcher read it — **at the size of the seat**: the
    strip's tab is 28 tall and takes the assembly at a tab's size
    (`SessionFace` `tab`, halo and corner inside the tab's box); the
    switcher's rows are list rows and keep a row's. */
function TabAvatar({ f, bare, seat }: { f: TabFace; bare: string; seat: "tab" | "row" }) {
  if (f.glyph) return <span className="flex-none">{f.glyph}</span>;
  return f.session ? (
    <SessionFace face={f.face} {...f.session} size={seat} />
  ) : (
    <MachineAvatar face={f.face} className={bare} />
  );
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
}: {
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
}) {
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
      // `items-end` and a top-only radius do the rest.
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
      // 高度一致, 保持和谐. 现在, 标签页普遍比侧边栏更高一些」).
      // `--pane-head` is the finger's floor plus `--tab-lip`, the
      // sliver of band a tab stands on; a pane's search row centres its
      // controls in exactly that. They were 48 and 44 — two numbers
      // arrived at separately, agreeing about nothing — and the gate
      // now subtracts them and asks for zero.
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
      className="flex h-pane-head min-w-0 flex-1 items-end gap-1 px-in pt-tab-lip"
    >
      {head}
      {/* The scroller and the two fades over its ends. **No scrollbar**
          (第三轮 第 2 条:「标签会被移动条顶出窗口. 且此条无法拖动」):
          `app.css` hides it on `[data-tab-strip]`; the wheel, the fades,
          the count and the switcher are the ways past the edge. */}
      <div className="relative flex min-w-0 flex-1 self-stretch">
      {cut.left && (
        <div
          data-tab-fade="left"
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 left-0 z-10 w-6 bg-linear-to-r from-background to-transparent"
        />
      )}
      {cut.right && (
        <div
          data-tab-fade="right"
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 right-0 z-10 w-6 bg-linear-to-l from-background to-transparent"
        />
      )}
      <div
        ref={strip}
        data-tab-strip
        className="flex min-w-0 flex-1 items-end gap-1 overflow-x-auto"
      >
        {faces.map((f) => {
          const on = f.id === active;
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
              title={
                [[f.machine, f.path].filter(Boolean).join(": "), f.note]
                  .filter(Boolean)
                  .join(" · ") || undefined
              }
              className={cn(
                // `h-touch`, not `h-ctl-md`: the tab is the target, so the
                // tab is what has to be tall enough to land on — and it is
                // also what paints "this one is in front", which is the
                // half that would have been lost by growing an invisible
                // box inside a short pill. A tab whose colour stops short
                // of where it can be pressed is a tab that lies about
                // where it is.
                "group relative flex h-touch items-center gap-1 rounded-t-md pr-0.5 pl-2",
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
                on
                  ? "min-w-tab max-w-tab-on flex-none bg-card text-foreground"
                  : "min-w-tab-min shrink basis-tab text-muted-foreground hover:bg-muted",
              )}
            >
              <TabOpen menu={f.menu}>
              <button
                type="button"
                data-tab-open={f.id}
                onClick={() => onFocus(f.id)}
                // `h-full`: the whole tab opens it, not the line of text
                // in the middle of it. Left at content height this was 18
                // tall inside a tab it filled the width of — the band
                // around the label looked pressable and was not.
                // `gap-tab-gap`, not the `gap-1` this had: 「标签页上的
                // 标题距离头像稍远一些, 现在有些拥挤」 (2026-09-20 第 3
                // 条). Four pixels put the name against the face while
                // the tab's own `gap-1` put the same four between the
                // name and the × — so the face read as part of the
                // word. Its own token (tokens.css `--tab-gap`) rather
                // than a step up in `--gap-in`, which would move every
                // clump in the app for one complaint about tabs.
                className="flex h-full min-w-0 flex-1 cursor-pointer items-center gap-tab-gap text-left"
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
                    className="min-w-0 flex-1 text-body"
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
                    className={cn("flex min-w-0 flex-1 text-body", f.preview && "italic")}
                  >
                    <MidName text={f.label} />
                  </span>
                )}
              </button>
              </TabOpen>
              {/* 预览, between the name and the ×: the order a person
                  reads the tab in is what it is, then what to do with
                  it. `size-3.5` against the close's own glyph — this
                  one is about the tab's content and the × is about the
                  tab, so it is deliberately the smaller mark. */}
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
                  <IconEye className="size-3.5" />
                </Button>
              )}
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
                // Drawn on the open tab and under the pointer, the session
                // rows' rule: a close on every tab at once is the strip's
                // loudest repetition, and the open one needs it always
                // because that is the tab a person is finished with. Not
                // in front and not hovered, the cell keeps its width and
                // paints nothing — so a stub never changes shape when the
                // pointer arrives.
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
                  "flex h-full w-tab-close flex-none items-center justify-center rounded-sm text-muted-foreground outline-none",
                  "hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50",
                  !on &&
                    !f.dirty &&
                    "opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100",
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
                  <IconClose />
                )}
              </button>
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
            // floor so it read as level with the 30 tabs. Both are the
            // finger's floor now, and `items-end` already levels them —
            // keeping the margin would have made the band four taller than
            // anything in it.
            className="min-h-touch min-w-touch flex-none"
            onClick={onNew}
          >
            <IconPlus />
          </Button>
        )}
      </div>
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
      {onSwitch && (crowded || switching) && <TabCount n={faces.length} onPress={onSwitch} open={switching} />}
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
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  /** Whether the switcher wears its own search box (docs/SHELL.md 第三轮
      第 3 条:「切换器头上加搜索框」). */
  search?: boolean;
}) {
  const [query, setQuery] = useState("");
  // By what a row shows — the name and where it is — and nothing else.
  const q = query.trim().toLowerCase();
  const shown = q === "" ? faces : faces.filter((f) => `${f.label} ${f.path}`.toLowerCase().includes(q));
  return (
    <div data-tab-switcher className="flex h-full min-h-0 flex-col">
      {search && <PaneBar searchLabel={gui.search_tabs} query={query} onQuery={setQuery} />}
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
            <TabOpen menu={f.menu}>
            <button
              type="button"
              data-switch-open={f.id}
              onClick={() => onFocus(f.id)}
              // The name and the path are what a person aims at here;
              // at content height the row's padding looked like part of
              // the target and was the parent's.
              className="flex min-h-touch min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
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
            <Button
              size="icon"
              variant="ghost"
              data-switch-close={f.id}
              aria-label={gui.close_tab}
              // Always drawn here, unlike the wide strip: a phone has no
              // pointer to summon a control with, and a close that only
              // appears on hover is a close that does not exist
              // (`FilesPane`'s pin, same rule) — and on the face that
              // has no pointer, a target under the floor is the same
              // kind of nearly-not-there.
              className="min-h-touch min-w-touch flex-none"
              onClick={() => onClose(f.id)}
            >
              <IconClose />
            </Button>
          </div>
        ))}
      </div>
    </div>
  );
}
