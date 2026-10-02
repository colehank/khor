// A settings slider (the redesign, 2026-10-02: 外观's two text sizes):
// a small mark, the track, a large mark — iOS's text size control.
//
// The control is the native range input, so the keyboard, a screen
// reader and a finger all get the platform's own behaviour; it lies over
// the drawn track with its own track clear (`app.css` `[data-slider]`).
// The fill and the ticks are positioned on the knob's centre line, which
// runs from half a knob in from one end to half a knob in from the other.
import type { CSSProperties, ReactNode } from "react";


export function Slider({
  label,
  min,
  max,
  step = 1,
  value,
  ticks,
  onChange,
  small,
  large,
  mark,
}: {
  label: string;
  min: number;
  max: number;
  step?: number;
  value: number;
  /** How many tick marks under the track, ends included. */
  ticks: number;
  onChange: (value: number) => void;
  /** The marks at the two ends: the smallest and the largest size. */
  small: ReactNode;
  large: ReactNode;
  mark?: Record<string, string>;
}) {
  const at = max > min ? (value - min) / (max - min) : 0;
  return (
    <div
      data-slider
      {...mark}
      className="flex min-h-touch flex-1 items-center gap-3.5"
      style={{ "--slider-at": at } as CSSProperties}
    >
      <span aria-hidden="true" className="flex-none">
        {small}
      </span>
      <div className="relative h-slider-knob min-w-0 flex-1">
        <div className="absolute inset-x-0 top-1/2 h-1 -translate-y-1/2 rounded-full bg-fill-strong" />
        <div
          className="absolute left-0 top-1/2 h-1 w-[calc(var(--slider-half)+(100%-var(--slider-knob))*var(--slider-at))] -translate-y-1/2 rounded-full bg-brand-strong"
        />
        <div className="absolute inset-x-slider-half top-1/2 flex -translate-y-1/2 justify-between">
          {Array.from({ length: ticks }, (_, i) => (
            <span key={i} className="-mx-px h-2 w-0.5 rounded-full bg-fill-strong" />
          ))}
        </div>
        <input
          type="range"
          aria-label={label}
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          className="absolute inset-0 h-full w-full"
        />
      </div>
      <span aria-hidden="true" className="flex-none">
        {large}
      </span>
    </div>
  );
}
