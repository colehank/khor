import type { Track } from "@/components/Ring";
import { TRACK_OPACITY } from "@/components/Ring";
import { cli } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * A machine's readings as bars, for the two screens that show every
 * machine at once.
 *
 * # Why the overviews stopped printing the numbers
 *
 * Both of them used to: the star map printed `CPU 24% 内存 80% GPU 63%`
 * under every face, the network's now printed the same string on every
 * row. Measured on the map before this: **sixteen pieces of text, every
 * one at the same size and the same weight** — a machine's name, how old
 * its answer was and three readings all arriving at once, with nothing
 * saying which to read first. That is what "看着费劲" is made of.
 *
 * **An overview is asked "who is busy", not "how busy, exactly".** A bar
 * answers the first question without being read; the second question has
 * an answer one press away, on the machine's own card, where the rings
 * carry the percentages and the byte counts underneath them. So the
 * numbers did not go missing — they stopped being said in the place that
 * was not asking.
 *
 * They are still *said* here, just not drawn: the strip carries the same
 * sentence the card would print as its `aria-label` and its `title`, so
 * a screen reader hears the percentages and a pointer recovers them
 * without navigating anywhere.
 *
 * # One arithmetic
 *
 * The fractions and the colours are `vitalsTracks`' — the same values
 * the card's rings and the list row's stack draw from. A machine that
 * looked busy on one screen and calm on another would be two machines.
 * Strain arrives already decided (the node's `Strain`, never worked out
 * here) and paints the fill; utilisation is neutral, because a busy CPU
 * is a machine doing what you asked.
 */
/**
 * One reading as a bar: a track, and however much of it is filled.
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

export function VitalBars({
  tracks,
  len,
  thick,
  className,
}: {
  tracks: Track[];
  /** The length of one bar, when the caller's picture scales (the map's
      captions do). Defaults to the token. */
  len?: string;
  /** Likewise its thickness. */
  thick?: string;
  className?: string;
}) {
  if (tracks.length === 0) return null;
  return (
    <span
      data-vital-bars
      role="img"
      aria-label={sentence(tracks)}
      title={sentence(tracks)}
      className={cn("flex items-center gap-in", className)}
    >
      {tracks.map((t) => (
        <Bar
          key={t.id}
          anchor={t.id}
          fraction={t.fraction}
          color={t.color}
          len={len}
          thick={thick}
        />
      ))}
    </span>
  );
}

/** The name of a track, in the words both faces already use for it. */
function trackName(id: string): string {
  switch (id) {
    case "cpu":
      return cli.vitals_name_cpu;
    case "mem":
      return cli.vitals_name_mem;
    case "disk":
      return cli.vitals_name_disk;
    case "gpu":
      return cli.vitals_name_gpu;
    default:
      return id;
  }
}

/**
 * What the bars would say if they were text — the string the overviews
 * used to draw, now spoken instead.
 *
 * A track with no reading is left out rather than given a placeholder
 * percentage: "we do not know" is what the empty bar beside it says, and
 * inventing a number for the label would make the two disagree.
 */
function sentence(tracks: Track[]): string {
  return tracks
    .filter((t) => t.fraction !== null)
    .map((t) => `${trackName(t.id)} ${Math.round(t.fraction! * 100)}%`)
    .join(" · ");
}
