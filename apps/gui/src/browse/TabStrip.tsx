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
import { useEffect, useRef, useState } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { SessionFace } from "@/components/SessionFace";
import { IconClose, IconPlus } from "@/components/icons";
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
  /** Present on a session tab: the row's identity facts, drawn exactly
      as the list row draws them (`SessionFace` — the halo is the state,
      the corner is the kind; 2026-09-02:「标签页的标签头像应该和
      session列表一致」). A browsing tab has none and keeps the bare
      machine face. */
  session?: { word: string; kind: string; category: string | null };
};

/** The face a tab wears: the session assembly when it is a session,
    the bare machine face when it is a place. One decision, both the
    strip and the switcher read it. */
function TabAvatar({ f, bare }: { f: TabFace; bare: string }) {
  return f.session ? (
    <SessionFace face={f.face} {...f.session} />
  ) : (
    <MachineAvatar face={f.face} className={bare} />
  );
}

export function TabStrip({
  faces,
  active,
  onFocus,
  onClose,
  onNew,
  onSwitch,
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
}) {
  const strip = useRef<HTMLDivElement>(null);
  // Whether the strip has given up on its own promise. Measured off the
  // element, not counted from `faces.length` and a width: how many tabs
  // fit depends on the region, which the rail, the window and the
  // landing all have a say in, and a number derived here would be a
  // second opinion about a layout the browser has already decided.
  const [crowded, setCrowded] = useState(false);
  useEffect(() => {
    const el = strip.current;
    if (!el) return;
    // A pixel of tolerance: sub-pixel flex widths make an exact
    // comparison flap between renders on a strip that fits.
    const measure = () => setCrowded(el.scrollWidth - el.clientWidth > 1);
    measure();
    // The observer catches the region changing; the dependency catches
    // tabs opening and closing. Neither alone sees both.
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [faces.length]);

  return (
    <div
      data-tab-bar
      // **The band is darker and the open tab is the page's own
      // colour**, which is the whole of how a tab says it is in front:
      // its bottom edge has no edge, so it reads as continuous with what
      // is under it, and the ones that are not open sit *in* the band
      // behind it. That is Chrome's arrangement, and it is not a
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
      // **The band and the scrolling row are two elements now, and the
      // reason is a measurement**: with everything in one scroll
      // container the "+" scrolled away with the tabs — at 20 tabs on a
      // 1400 window it sat at x=1912 while the strip's visible edge was
      // 1400, **512 past the end of the world**, so the way to open a
      // tab was to scroll right until you found it. Chrome pins it, and
      // the only way to pin anything against a scroll container is to
      // be outside it. The band keeps the ground and the padding; the
      // row inside it is what scrolls; the buttons ride at the end and
      // never move.
      className="flex flex-none items-end gap-1 bg-muted/40 px-2 pt-1"
    >
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
              title={f.path || undefined}
              className={cn(
                // `h-touch`, not `h-ctl-md`: the tab is the target, so the
                // tab is what has to be tall enough to land on — and it is
                // also what paints "this one is in front", which is the
                // half that would have been lost by growing an invisible
                // box inside a short pill. A tab whose colour stops short
                // of where it can be pressed is a tab that lies about
                // where it is.
                "group flex h-touch flex-none items-center gap-1 rounded-t-md pr-0.5 pl-2",
                // **Two widths, and each tab wears exactly one.** The one
                // in front takes its own name's width — never less than a
                // stub, so it does not read as the smallest tab on the
                // strip, and up to the ceiling; every other is the stub.
                // Spelled as two branches rather than a base plus an
                // override because `w-tab` is a custom spacing class and
                // `cn()` cannot lift one of those off another (button.tsx
                // measured that) — both would survive and the
                // stylesheet's order would pick.
                on
                  ? "min-w-tab max-w-tab-on bg-background text-foreground"
                  : "w-tab text-muted-foreground hover:bg-background/50",
              )}
            >
              <button
                type="button"
                data-tab-open={f.id}
                onClick={() => onFocus(f.id)}
                // `h-full`: the whole tab opens it, not the line of text
                // in the middle of it. Left at content height this was 18
                // tall inside a tab it filled the width of — the band
                // around the label looked pressable and was not.
                className="flex h-full min-w-0 flex-1 cursor-pointer items-center gap-1 text-left"
              >
                <TabAvatar f={f} bare="size-kind-mark flex-none" />
                <span data-tab-label className="min-w-0 flex-1 truncate text-sm">
                  {f.label}
                </span>
              </button>
              <Button
                size="icon"
                variant="ghost"
                data-tab-close={f.id}
                aria-label={gui.close_tab}
                // Drawn on the open tab and under the pointer, the session
                // rows' rule: a close on every tab at once is the strip's
                // loudest repetition, and the open one needs it always
                // because that is the tab a person is finished with.
                //
                // **`min-*`, not a `size-*`** — and the class it replaces
                // is the proof: `size-ctl-sm` never took effect here.
                // `cn()` cannot lift `h-ctl-md`/`w-ctl-md` off the icon
                // variant (tailwind-merge does not read `ctl-*` as a
                // size, `button.tsx` says so with its own measurement),
                // so both survived and the stylesheet's order decided —
                // this button measured 30, the icon size, not the 24 it
                // asked for. A floor beats a height without having to
                // remove it. The glyph keeps its own size; what grows is
                // the box, and the ghost wash paints that box, so what
                // lights up is what can be pressed.
                //
                // What it costs, said out loud: the close takes 44 of a
                // stub's 160 where it took 30, and the half that holds
                // the face and the name gives them up. On the open tab
                // it costs nothing — that tab is as wide as its name
                // plus this.
                //
                // **A close that outweighs the open is the tab telling a
                // finger the wrong thing**, and at 160 the open half is
                // still the bigger one (102 was the width where they
                // crossed, back when tabs shrank; fixed stubs never reach
                // it).
                className={cn(
                  "min-h-touch min-w-touch flex-none",
                  !on && "opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100",
                )}
                onClick={() => onClose(f.id)}
              >
                <IconClose />
              </Button>
            </div>
          );
        })}
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
      {onSwitch && crowded && <TabCount n={faces.length} onPress={onSwitch} />}
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
export function TabCount({ n, onPress }: { n: number; onPress: () => void }) {
  return (
    <Button
      variant="ghost"
      size="icon"
      data-tab-count
      aria-label={gui.switch_tabs(n)}
      onClick={onPress}
      // The count keeps its size; the box around it takes the floor.
      // This is the only way to every other tab, and on narrow it sits
      // at the end of the address bar where a thumb reaches — the one
      // control on this landing where being under the floor would be
      // felt every time.
      className="min-h-touch min-w-touch flex-none"
    >
      <span className="rounded-sm border px-1 text-xs tabular-nums">{n}</span>
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
  onNew,
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  /** Mint a tab. Absent on a switcher whose tabs are not made here —
      the session tabs' rule, same as the strip's own `onNew`: a session
      tab *is* its session, so there is nothing a "new tab" here could
      open, and the foot stays off. */
  onNew?: () => void;
}) {
  return (
    <div data-tab-switcher className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {faces.map((f) => (
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
            <button
              type="button"
              data-switch-open={f.id}
              onClick={() => onFocus(f.id)}
              // The name and the path are what a person aims at here;
              // at content height the row's padding looked like part of
              // the target and was the parent's.
              className="flex min-h-touch min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
            >
              <TabAvatar f={f} bare="size-avatar flex-none" />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm font-label">{f.label}</span>
                {/* The whole path under the name: on a phone the strip
                    is not there to give the tab context, so the row has
                    to carry it. Truncated from the tail — a slashed
                    path read right to left is not a path. */}
                {f.path && (
                  <span className="truncate text-xs text-muted-foreground">{f.path}</span>
                )}
              </span>
            </button>
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
      {onNew && (
        <div className="flex-none border-t p-2">
          {/* `size="sm"` is the look — a filled word, not a bar across the
              foot — and the floor is the hit area under it. The two are
              allowed to disagree: `--ctl-*` is what a control *is* beside
              text, `--touch` is what a finger needs (tokens.css). */}
          <Button
            variant="secondary"
            size="sm"
            data-tab-new
            className="min-h-touch min-w-touch"
            onClick={onNew}
          >
            {gui.new_tab}
          </Button>
        </div>
      )}
    </div>
  );
}
