import type { ReactNode } from "react";

/**
 * **One section's heading row, on a board with no boxes on it.**
 *
 * 2026-09-28, the owner: the machine card's parts stop being cards on
 * the board — they are sections of the board itself, one hairline
 * between them, each with a small heading row, and the section's own
 * control (the figure's •••) at the right end of that row. The 额度
 * card's usage half is the same product (2026-09-08 「现在看起来是两个
 * 产品」, 2026-09-09 「为什么机器和余量页三个点的位置还不一样」), so it
 * takes the same shape — the lead's call on the same day, for the owner
 * to confirm.
 *
 * The name is in the board's section-heading style — `group` 11/600 in
 * `tertiary`, what a settings group's name wears over its card
 * (components/SettingsGroup) — so a section's name reads as a name and
 * not as another line of the content under it. The row is
 * `--slot-head-h` tall: the height the machine card's 24-hour section
 * draws its own heading row at (the day panel's first line, `day.css`),
 * so every section's heading sits on one rhythm.
 *
 * ⚠️ 墓碑 (2026-09-28): `components/Panel` — a hairline border, the card
 * ground, `--radius-md` and `--gap-row` of inset — was what these two
 * surfaces drew their parts in, from 2026-09-08. It has no users left.
 */
export function SectionHead({
  title,
  mark,
  children,
}: {
  title: string;
  /** Extra anchors on the name (a gate that reads it by its old mark). */
  mark?: Record<string, string>;
  /** The section's own control, at the row's right end. */
  children?: ReactNode;
}) {
  return (
    <div data-section-head className="flex flex-none items-center gap-in" style={{ height: "var(--slot-head-h)" }}>
      <h2 {...mark} className="min-w-0 flex-1 truncate text-group text-tertiary">
        {title}
      </h2>
      {children}
    </div>
  );
}
