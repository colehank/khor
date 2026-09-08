import type { ReactNode } from "react";

/**
 * **One block of readings, on a card.**
 *
 * A hairline border, the card ground, `--radius-md` and `--gap-row` of
 * air on every side. Everything a surface says about one subject goes
 * inside one of these; two subjects are two panels, and two *parts* of
 * one subject are one panel with a `--gap-group` seam through it
 * (`data-top-seam` next door is that seam).
 *
 * # ⚠️ Shared, and it was two shapes before
 *
 * Moved out of `MachineCard` on 2026-09-08. Until that day the 額度
 * card's usage half was **not** a card at all: two flat sections with a
 * top border, transparent, square-cornered, 12px of air above and below
 * but 16 at the sides. Measured beside this one they shared no value —
 * and 「现在看起来是两个产品」 is what that reads as on a screen, from
 * the user who met both.
 *
 * **The radius is `rounded-md`, which is `--radius` minus 2 — 8px, not
 * 10.** Both numbers are real and neither is a mistake: `--radius` is
 * the base the scale is derived from (`--radius-sm/-md/-lg/-xl` in
 * `tokens.css`), and a card takes the `md` rung. Reading the base off
 * the token file and calling it 「the card's radius」 is off by two, and
 * two pixels of corner is exactly the kind of difference that is
 * visible and unattributable.
 */
export function Panel({ children }: { children: ReactNode }) {
  return <div className="rounded-md border bg-card px-row py-row">{children}</div>;
}
