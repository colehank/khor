import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { IconMore } from "@/components/icons";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

/**
 * **A figure's axis label, and the one button that changes what the
 * figure shows.**
 *
 * # ⚠️ Tombstone: `components/AxisPick` was these two in one, until 2026-09-08
 *
 * That component made **the axis's own word the control**: press
 * 「内存」 and four candidates replaced it in place. It was built for
 * 2026-09-05:「散点图搞成带X和Y label的。通过点击label,去切换X或y的值」,
 * and its head argued the chevron, the fixed height, and the two
 * corners at length.
 *
 * **The user overrode the second half of that ruling on 2026-09-08**:
 * 「所有设置图的,比如面积(机器没有,得加)、X和y,**都做成一个小的设置按钮**,
 * 点击后进行选择」. So switching moved off the label and into a button.
 *
 * ⚠️ **The first half of the 2026-09-05 ruling still stands, and that is
 * why the label did not go with the control.** Its reason was that
 * 「an axis with four quantities beside it does not say what it is」 —
 * a reader glancing at the vertical ruler had to work out which of four
 * the picture was drawn with. That problem is not solved by a settings
 * button; it is solved by the axis carrying **one** word, which it
 * still does. **Do not read this tombstone as 「the 09-05 ruling was
 * reversed」** — half of it is the reason `AxisLabel` exists.
 *
 * What the label lost is the chevron. That mark said 「this word is the
 * control」 and it no longer is; leaving it would be the card teaching a
 * gesture that does nothing.
 *
 * # What the markup owes the gates
 *
 * `data-plot-pick`, `data-plot-ruler` and `data-plot-label` are
 * unchanged and still on the label, because two measurements read them
 * for **position** rather than for pressability: the two labels tucked
 * into the corners a scatter is least likely to put a point in, and the
 * card's own layout landmark. `data-plot-axis` is unchanged too — it
 * moved into the panel, so every floor and every says-so rule that
 * already applied to those buttons still applies.
 */
export function AxisLabel({
  which,
  word,
  rulerWord,
}: {
  which: string;
  /** The quantity this axis is drawn with — **one word, never four**. */
  word: string;
  /** Which ruler this axis is on, printed beside the word rather than
      somewhere else on the axis. It used to be its own `<text>` at the
      far end; with the label in the same corner the two are read as one
      thing: 「内存 (log)」. */
  rulerWord: string;
  /** Which corner it is tucked into — the vertical label sits at the
      top-left, the horizontal one at the bottom-right, so neither runs
      off the drawing.

      ⚠️ **Tombstone: `align` left this component on 2026-09-10.** It
      flipped the two words' order so the pair would read outward from
      whichever corner it sat in — and the brackets made the order fixed,
      so there was nothing left for it to decide. The seat still decides
      where the label sits; `labelSeat` was always the one doing that. */
}) {
  return (
    //   **The name first, the ruler in brackets after it** (the user,
    //   2026-09-10:「表的单位表示方式改一下,比如总量(√)这样」). It was
    //   two words side by side, and which came first depended on which
    //   end of the axis the label sat at — `flex-row-reverse` on the
    //   right-hand one — so the same pair read 「总量 √」 at one corner
    //   and 「√ 总量」 at the other. **A bracket says 「this belongs to
    //   that」 in a way a space cannot**, and it reads the same at
    //   either corner, so the order stops depending on the seat.
    <span data-plot-pick={which} className="flex items-baseline gap-0 text-callout">
      <span data-plot-label={which} className="px-in font-title text-foreground">
        {word}
      </span>
      {/* An empty ruler prints nothing at all rather than an empty pair
          of brackets: 「linear」 has no mark on purpose (the axis is not
          bent), and 「总量()」 would read as a word with something
          missing from it. */}
      {rulerWord !== "" && (
        <span data-plot-ruler={which} aria-hidden="true" className="pe-in text-muted-foreground/70">
          ({rulerWord})
        </span>
      )}
    </span>
  );
}

/**
 * **One button for everything a figure can be set to.**
 *
 * 2026-09-08, the user:「都做成一个小的设置按钮,点击后进行选择」.
 *
 * # Why `IconMore` and not a gear
 *
 * **No new mark.** This app teaches ⋯ as 「this thing's options」 on
 * every row menu; a gear would be a second vocabulary for the same
 * idea, and `check-icons` exists because a second icon family reads the
 * way two typefaces in one paragraph read. The cost is that ⋯ is
 * slightly less specific than a gear — and the panel it opens says what
 * it is, so the specificity is one press away rather than absent.
 *
 * # The rows are given, not assumed
 *
 * Which quantities a row may offer is the **caller's** to decide, and
 * that is not tidiness: on the machine card a row's third quantity is
 * missing on most machines (`gpu_mem` is null on 200 of 200 rows on two
 * of the three), and a quantity nobody reported must not be offered as
 * one that shrinks a dot to its floor — the floor reads as 「smallest」,
 * which is the neighbouring-value error `top-plot-off` was written
 * against. So this component draws what it is handed and judges
 * nothing.
 */
export function PlotSettings({
  rows,
  onPick,
}: {
  rows: { key: string; word: string; chosen: string; options: { key: string; word: string }[] }[];
  onPick: (row: string, key: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const panel = useRef<HTMLDivElement | null>(null);
  const here = useRef<HTMLSpanElement | null>(null);
  //   **A press anywhere else puts it away** (2026-09-10, the user:
  //   「点击空白区域可以关掉这三个点弹出的候选」). Without this the
  //   only way out was the same button, which is a rule the reader has
  //   to be told — every other menu in this app closes by looking away.
  //
  //   `pointerdown` in the capture phase, so it lands before whatever
  //   was pressed acts: pressing a control behind the panel should
  //   both close the panel and reach that control, which is what a
  //   listener that runs first and does not swallow the event gives.
  //   Escape too, since the panel takes focus.
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (!here.current?.contains(e.target as Node)) setOpen(false);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", away, true);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerdown", away, true);
      document.removeEventListener("keydown", key);
    };
  }, [open]);
  //   **Re-fit when the contents change, not only when it opens.**
  //   Picking a quantity re-weights one word (the chosen one is
  //   `font-title`), and a row whose offer list changed is a different
  //   width — both move the panel's edge while it is open.
  const shape = rows.map((r) => `${r.key}:${r.chosen}:${r.options.length}`).join("|");
  useLayoutEffect(() => {
    const el = panel.current;
    if (!open || !el) return;
    //   **Measured, not declared.** The first version of this panel
    //   said `left-0 min-w-max` and let the caller place the button;
    //   the user's first press found it cut off (2026-09-08:
    //   「三个点点击了弹出的窗口都显示不全」). Measured that night at
    //   three widths: the 額度 trend panel hung 172.6px past the edge
    //   **at every one of them** — the same number, because the button
    //   sits at the right end of its row and the panel grew rightwards
    //   from it, which has nothing to do with how wide the screen is —
    //   and the stats panel, 474 across for eight quantities, hung
    //   155px out at 390 where **no side to anchor to would have
    //   fitted it**.
    //
    //   So two things are read off the glass rather than written down:
    //   how wide it may be, and how far it has to come back. A caller
    //   cannot answer either — 「which side has room」 is a fact about
    //   this width, and the caller that got it wrong had the button on
    //   the left when it was written.
    el.style.maxWidth = "";
    el.style.transform = "";
    //   The box it must stay inside: the nearest ancestor that clips
    //   (the 額度 detail pane is `overflow:auto`), else the window.
    let n: HTMLElement | null = el.parentElement;
    let left = 0;
    let right = window.innerWidth;
    let top = 0;
    let floor = window.innerHeight;
    while (n) {
      const cs = getComputedStyle(n);
      if (cs.overflowX !== "visible" || cs.overflowY !== "visible") {
        const b = n.getBoundingClientRect();
        left = Math.max(left, b.left);
        right = Math.min(right, b.right);
        top = Math.max(top, b.top);
        floor = Math.min(floor, b.bottom);
        break;
      }
      n = n.parentElement;
    }
    //   **The air beside it is the air under it**, read from the
    //   stylesheet rather than copied into this file: `mt-in` is
    //   already the gap between the panel and its button, and a second
    //   number here would be a mirror nothing keeps in step.
    const air = parseFloat(getComputedStyle(el).marginTop) || 0;
    const room = right - left - 2 * air;
    if (room > 0 && el.getBoundingClientRect().width > room) {
      //   Wider than the room: it wraps. `w-max` keeps it as narrow as
      //   its contents when there is space, and the rows already carry
      //   `flex-wrap` — it was `min-w-max` that made that wrap
      //   unreachable, since a min-width outranks a max-width.
      el.style.maxWidth = `${Math.round(room)}px`;
    }
    const r = el.getBoundingClientRect();
    const past = r.right - (right - air);
    const short = left + air - r.left;
    const dx = past > 0 ? -past : short > 0 ? short : 0;
    //   **And the same question downward** (2026-09-10, when the button
    //   moved to the figure's top-right and the panel started opening
    //   down). The gate measured it the first run: 89.6 and 70.8 past
    //   the bottom of a 390-wide pane — 「弹出的窗口显示不全」 again,
    //   the failure this whole measurement exists for, now on the other
    //   axis. **A cut-off panel gives no sign it is cut off**, which is
    //   why this is arithmetic and not a guess about how much room
    //   there usually is.
    //
    //   Moved with a transform rather than by re-rendering on the other
    //   side: a state flip would re-run this effect, and a panel that
    //   fits in neither direction would flip forever.
    const below = r.bottom - (floor - air);
    let dy = 0;
    if (below > 0) {
      //   Above the button instead, if it fits there whole. The lift is
      //   the panel, the button and the air on both sides of it.
      const seat = here.current?.getBoundingClientRect();
      const lift = seat ? r.height + seat.height + 2 * air : 0;
      dy = lift > 0 && r.top - lift >= top + air ? -lift : -Math.min(below, Math.max(0, r.top - (top + air)));
    }
    if (dx || dy) el.style.transform = `translate(${Math.round(dx)}px, ${Math.round(dy)}px)`;
  }, [open, shape]);
  return (
    <span ref={here} data-plot-settings-at className="relative inline-flex">
      {/* A glyph-only control names itself (W-V7); round, so its ring is
          (W-V6). */}
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            data-plot-settings
            data-open={open ? "" : undefined}
            aria-expanded={open}
            aria-label={rows.map((r) => r.word).join(" / ")}
            onClick={() => setOpen(!open)}
            className="inline-flex min-h-touch min-w-touch items-center justify-center rounded-full text-muted-foreground hover:text-foreground"
          >
            <IconMore className="size-4" />
          </button>
        </TooltipTrigger>
        {!open && <TooltipContent side="top">{rows.map((r) => r.word).join(" / ")}</TooltipContent>}
      </Tooltip>
      {open && (
        // **On the figure, not beside it.** The panel is absolute so
        // opening it cannot push the drawing down — the same rule the
        // label it replaces followed ("one place, the contents swapped,
        // the height fixed").
        <div
          ref={panel}
          data-plot-settings-open
          //   ⚠️ **`w-max`, not `min-w-max`.** A min-width outranks a
          //   max-width, so the second spelling makes the cap above
          //   unreachable and the panel can be wider than the screen.
          //   **Downward, from a button in the figure's top-right**
          //   (2026-09-10; it opened upward from the bottom-left until
          //   then). **Vertical is not clamped**, and that is still a
          //   decision rather than an oversight: the tallest this has
          //   been measured is 306 — the narrow face's stats panel,
          //   after the width cap above makes it wrap — in a pane
          //   running 45 to 847, so it has room below as it had above.
          //   The honest answer to a panel with no room is the other
          //   direction, a different gesture rather than a nudge, and
          //   the gate measures both.
          //   **It floats over the figure** (2026-09-28): `popover`,
          //   `float`, `radius-lg` — it was the card's own `card`, flat
          //   on the drawing it covers.
          className="absolute left-0 right-auto top-full z-10 mt-in flex w-max flex-col gap-in rounded-lg border bg-popover px-row py-row shadow-(--shadow-float)"
        >
          {rows.map((r) => (
            <div key={r.key} data-plot-set={r.key} className="flex items-center gap-in">
              <span className="shrink-0 text-callout text-muted-foreground">{r.word}</span>
              <span className="flex flex-wrap">
                {r.options.map((o) => (
                  <button
                    key={o.key}
                    type="button"
                    data-plot-axis={o.key}
                    data-on={o.key === r.chosen ? "true" : "false"}
                    aria-pressed={o.key === r.chosen}
                    onClick={() => onPick(r.key, o.key)}
                    className={cn(
                      "inline-flex min-h-touch min-w-touch items-center justify-center rounded-sm px-in text-callout hover:text-foreground",
                      o.key === r.chosen ? "font-title text-foreground" : "text-muted-foreground",
                    )}
                  >
                    {o.word}
                  </button>
                ))}
              </span>
            </div>
          ))}
        </div>
      )}
    </span>
  );
}
