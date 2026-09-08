import { useState } from "react";

import { IconChevronDown } from "@/components/icons";
import { cn } from "@/lib/utils";

/** **One axis's label, which is also the control that changes it**
 * (2026-09-05:「散点图搞成带X和Y label的。通过点击label,去切换X或y的值」).
 *
 * # Why it is a label and not a row of four words
 *
 * It used to be four words per axis, all eight sitting in the figure's
 * foot under a 「横」/「纵」. Two things were wrong with that and only one
 * of them was the position: **an axis with four quantities beside it
 * does not say what it is.** A reader glancing at the vertical ruler
 * saw four candidates and had to work out which one the picture was
 * drawn with; now the axis carries one word, and it is the right one.
 *
 * # Why it can be pressed, without a ground or a border
 *
 * The chevron. **Not a new mark** — it is the same one the column heads
 * next door wear to say 「this word is the control」, so the card is not
 * being taught a second vocabulary for the same idea. A capsule or a
 * bordered box would say 「button」 louder and would also be the first
 * such box on this card, which is the wrong thing to be first at.
 *
 * # Why the height cannot move
 *
 * Collapsed it is one button, open it is four, and **both are a row of
 * `min-h-touch` buttons** — so opening the picker cannot push the
 * figure down or squeeze it. That is the same rule the 24-hour block's
 * three reading words and the drawer's ⊙ title follow: one place, the
 * contents swapped, the height fixed.
 *
 * The floor is on the hit area rather than on the word: each of these
 * is 44 tall and 44 wide to a finger while the ink stays the
 * footnote's size (`day.css`'s `.sw button`, the same arrangement) —
 * which is also why the gap between the four is zero. At the floor's
 * width they are already far enough apart.
 *
 * # ⚠️ Shared, and the markup is the contract
 *
 * Moved out of `MachineCard` on 2026-09-08 so the 额度 page's usage half
 * can wear the same control rather than a second one that does the same
 * job — that page had a row of native `<select>`s, 23px tall, sitting
 * in a strip above the figure, and a reader arriving from the machine
 * card met a different idea of 「how you change an axis」 on the second
 * screen.
 *
 * **What was generalised is the wording, not the shape**: the option
 * list and the ruler word are arguments now, and every `data-plot-*`
 * attribute, every class and the open/closed structure are byte for
 * byte what they were. The machine card's gate reads those attributes,
 * so a change to any of them here is a change to that gate — and the
 * point of moving this was that there is one of it, not two.
 */
export function AxisPick({
  which,
  chosen,
  options,
  rulerWord,
  onPick,
  align,
}: {
  which: string;
  chosen: string;
  /** The quantities this axis can be, in the order they are offered. */
  options: { key: string; word: string }[];
  /** Which ruler this axis is on, printed **beside** the label rather
      than somewhere else on the axis.

      It used to be its own `<text>` inside the drawing, at the far end
      of each axis. With the label moved into the same corner the two
      would have sat on top of each other, so they are read as one
      thing: 「内存 (log)」. **This is not the mark going back where it
      came from** — it was moved off the tick numbers for a measured
      reason (a ruler word through 「606.8M」) and it is still inside the
      plot, on its own axis.

      **Outside the button on purpose.** The button's contents are
      replaced by the candidates while the picker is open; a ruler mark
      inside it would blink out exactly while somebody is choosing which
      ruler they are about to read. */
  rulerWord: string;
  onPick: (k: string) => void;
  /** Which corner it is tucked into — the vertical label opens
      rightwards from the top-left, the horizontal one leftwards from
      the bottom-right, so neither set of words can run off the
      drawing. */
  align: "left" | "right";
}) {
  const [open, setOpen] = useState(false);
  return (
    <span
      data-plot-pick={which}
      data-open={open ? "" : undefined}
      className={cn(
        "flex items-baseline gap-0 text-aux",
        align === "right" ? "flex-row-reverse" : "flex-row",
      )}
    >
      <span
        data-plot-ruler={which}
        aria-hidden="true"
        className="px-in text-muted-foreground/70"
      >
        {rulerWord}
      </span>
      {open
        ? options.map((o) => (
            <button
              key={o.key}
              type="button"
              data-plot-axis={o.key}
              data-on={o.key === chosen ? "true" : "false"}
              aria-pressed={o.key === chosen}
              onClick={() => {
                onPick(o.key);
                setOpen(false);
              }}
              className={cn(
                "inline-flex min-h-touch min-w-touch items-center justify-center px-in hover:text-foreground",
                o.key === chosen ? "font-title text-foreground" : "text-muted-foreground",
              )}
            >
              {o.word}
            </button>
          ))
        : (
            <button
              type="button"
              data-plot-label={which}
              aria-expanded={false}
              onClick={() => setOpen(true)}
              className="inline-flex min-h-touch min-w-touch items-center justify-center gap-in px-in font-title text-foreground hover:text-foreground"
            >
              {options.find((o) => o.key === chosen)?.word ?? chosen}
              <IconChevronDown className="size-3" />
            </button>
          )}
    </span>
  );
}
