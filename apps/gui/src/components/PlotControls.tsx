import { useState } from "react";

import { IconMore } from "@/components/icons";
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
  align,
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
      off the drawing. */
  align: "left" | "right";
}) {
  return (
    <span
      data-plot-pick={which}
      className={cn(
        "flex items-baseline gap-0 text-aux",
        align === "right" ? "flex-row-reverse" : "flex-row",
      )}
    >
      <span data-plot-ruler={which} aria-hidden="true" className="px-in text-muted-foreground/70">
        {rulerWord}
      </span>
      <span data-plot-label={which} className="px-in font-title text-foreground">
        {word}
      </span>
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
  return (
    <span data-plot-settings-at className="relative inline-flex">
      <button
        type="button"
        data-plot-settings
        data-open={open ? "" : undefined}
        aria-expanded={open}
        aria-label={rows.map((r) => r.word).join(" / ")}
        onClick={() => setOpen(!open)}
        className="inline-flex min-h-touch min-w-touch items-center justify-center text-muted-foreground hover:text-foreground"
      >
        <IconMore className="size-4" />
      </button>
      {open && (
        // **On the figure, not beside it.** The panel is absolute so
        // opening it cannot push the drawing down — the same rule the
        // label it replaces followed ("one place, the contents swapped,
        // the height fixed").
        <div
          data-plot-settings-open
          className="absolute bottom-full left-0 z-10 mb-in flex min-w-max flex-col gap-in rounded-md border bg-card px-row py-row"
        >
          {rows.map((r) => (
            <div key={r.key} data-plot-set={r.key} className="flex items-center gap-in">
              <span className="shrink-0 text-aux text-muted-foreground">{r.word}</span>
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
                      "inline-flex min-h-touch min-w-touch items-center justify-center px-in text-aux hover:text-foreground",
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
