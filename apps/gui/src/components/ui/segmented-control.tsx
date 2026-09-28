// SegmentedControl — one of a few fixed choices, all of them in view
// (docs/design-system components/SegmentedControl; macOS's
// NSSegmentedControl). One `fill` track at `radius-sm`, `ctl-md` tall;
// the chosen segment is a `segment` face at `radius-xs` lifted by the
// `knob` shadow — a face sitting in a track, the switch's knob's
// relative — and a hairline stands between two segments neither of
// which is chosen.
//
// **When this, and when a popup** (the owner, 2026-09-28): up to about
// four choices that do not grow — this; a list that grows (machines, a
// registry) — `PopupButton`. A caller whose list is usually short but
// open (the agents) draws this while it is short and the popup past
// `SEGMENTS_MAX`, the same row either way.
//
// **It is a radiogroup**: one tab stop (the chosen segment), the arrow
// keys move the choice and the focus together, Home/End go to the ends,
// and a disabled segment is stepped over. A segment is a choice and
// nothing else — an act that opens something (「添加…」) stands after
// the control as its own button, never as one more segment.
import { useRef, type KeyboardEvent, type ReactNode } from "react";

import { cn } from "@/lib/utils";

/** Past this many choices a row draws a popup instead (the README's
    「≤ ~4 fixed choices」). */
export const SEGMENTS_MAX = 4;

export type Segment<K extends string = string> = {
  key: K;
  label: ReactNode;
  disabled?: boolean;
  /** The segment's full name where `label` may be cut short. */
  title?: string;
  /** Anchors on this segment's button. */
  mark?: Record<string, string | boolean | undefined>;
};

export function SegmentedControl<K extends string>({
  label,
  value,
  options,
  onChange,
  mark,
  className,
}: {
  /** What the group is called, for a screen reader — the row's label. */
  label: string;
  value: K;
  options: Segment<K>[];
  onChange: (key: K) => void;
  /** Anchors on the group. */
  mark?: Record<string, string | boolean | undefined>;
  className?: string;
}) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const at = options.findIndex((o) => o.key === value);

  const step = (from: number, by: 1 | -1 | "home" | "end") => {
    const live = options.map((o, i) => (o.disabled ? -1 : i)).filter((i) => i >= 0);
    if (live.length === 0) return;
    let next: number;
    if (by === "home") next = live[0];
    else if (by === "end") next = live[live.length - 1];
    else {
      const k = live.indexOf(from);
      next = live[(((k === -1 ? 0 : k) + by) % live.length + live.length) % live.length];
    }
    buttons.current[next]?.focus();
    if (next !== at) onChange(options[next].key);
  };

  const onKey = (i: number) => (e: KeyboardEvent<HTMLButtonElement>) => {
    const by =
      e.key === "ArrowRight" || e.key === "ArrowDown"
        ? 1
        : e.key === "ArrowLeft" || e.key === "ArrowUp"
          ? -1
          : e.key === "Home"
            ? "home"
            : e.key === "End"
              ? "end"
              : null;
    if (by === null) return;
    e.preventDefault();
    step(i, by);
  };

  return (
    <div
      role="radiogroup"
      aria-label={label}
      data-segmented
      {...mark}
      className={cn("inline-flex h-ctl-md max-w-full min-w-0 items-center rounded-sm bg-fill", className)}
    >
      {options.map((o, i) => {
        const on = i === at;
        // The hairline between two segments, only where neither side
        // is the chosen one — the chosen face is its own edge.
        const rule = i > 0 && !on && i - 1 !== at;
        return (
          <button
            key={o.key}
            ref={(el) => {
              buttons.current[i] = el;
            }}
            type="button"
            role="radio"
            aria-checked={on}
            // One tab stop: the chosen segment, or the first live one
            // when nothing is chosen.
            tabIndex={on || (at === -1 && i === options.findIndex((x) => !x.disabled)) ? 0 : -1}
            disabled={o.disabled}
            title={o.title}
            data-segment={o.key}
            data-on={on}
            onClick={() => !on && onChange(o.key)}
            onKeyDown={onKey(i)}
            {...o.mark}
            className={cn(
              // **The button is the hit area, the face is drawn inside
              // it** — the switch's arrangement (`settings/Switch`). With
              // a mouse the floor (`--hit`) is the track's own `ctl-md`;
              // a finger gets `--touch` around the same drawing.
              "relative flex min-h-touch min-w-touch flex-initial items-center justify-center rounded-sm px-row text-body text-foreground",
              "focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring disabled:opacity-40",
              rule &&
                "before:absolute before:top-1/2 before:left-0 before:h-3.5 before:w-px before:-translate-y-1/2 before:bg-border before:content-['']",
            )}
          >
            {on && (
              <span
                aria-hidden
                data-segment-face
                className="absolute inset-x-0.75 top-1/2 h-ctl-sm -translate-y-1/2 rounded-xs bg-segment shadow-knob"
              />
            )}
            <span className="relative min-w-0 truncate">{o.label}</span>
          </button>
        );
      })}
    </div>
  );
}
