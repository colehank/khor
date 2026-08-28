import { TRACK_OPACITY } from "@/components/Ring";
import { cn } from "@/lib/utils";

/**
 * One quantity as a bar: a track, and however much of it is filled.
 *
 * # What this file used to be, and why one shape survived
 *
 * It drew `VitalBars` too — a machine's CPU / 内存 / GPU as three small
 * bars, under every face on the star map and beside every name on the
 * network's now. The user's word for them was「看不懂,可以取消」
 * (2026-08-28) and they are gone from both.
 *
 * **The difference between those and this one is a denominator that is
 * said out loud.** Three unlabelled bars under a name answer 「how full
 * is *what*」 with nothing; a reader has to be told which is which and
 * the seat had no room to tell them. This bar has one meaning per
 * caller and the caller writes it down next to it (今天's is「和最忙的
 * 一天比」). So the rule that removed them is not「bars are bad」, it is
 * **an overview carries no readings** plus **a bar has to name what it
 * is a fraction of** — and where a real 「full」 exists, a ring says it
 * better still, because the shape itself carries the whole
 * (`components/Ring`).
 *
 * **Sizes arrive as lengths, never as classes.** A caller that wanted a
 * full-width bar would have to beat `w-gauge-bar-len` in a `cn()` merge,
 * and a custom spacing name is exactly what tailwind-merge cannot be
 * trusted to override (docs/handoff 坑节) — the failure is silent and
 * leaves both classes on the element. So the width is a style, and the
 * default is the token spelled out.
 */
export function Bar({
  fraction,
  color,
  len,
  thick,
  anchor,
  className,
}: {
  /** 0–1, or null for a reading khor could not take (track alone). */
  fraction: number | null;
  color?: string;
  len?: string;
  thick?: string;
  anchor?: string;
  className?: string;
}) {
  return (
    <span
      data-bar={anchor}
      className={cn("relative block rounded-full", className)}
      style={{
        width: len ?? "var(--gauge-bar-len)",
        height: thick ?? "var(--gauge-bar)",
      }}
    >
      {/* The unfilled part, at the ring's own track opacity — one
          judgment about how present an empty track is, not two. A
          sibling rather than a parent so the fill does not inherit the
          fade. */}
      <span
        className="absolute inset-0 rounded-full bg-muted-foreground"
        style={{ opacity: TRACK_OPACITY }}
      />
      {fraction !== null && (
        <span
          className="absolute inset-y-0 left-0 rounded-full"
          style={{
            width: `${Math.min(100, Math.max(0, fraction * 100))}%`,
            background: color ?? "var(--muted-foreground)",
          }}
        />
      )}
    </span>
  );
}
