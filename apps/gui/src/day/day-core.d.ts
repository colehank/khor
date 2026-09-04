// The transplanted 24-hour panel's shape, for the one file that mounts
// it. **A declaration, not a re-typing of the module**: the module is
// `card-whole.html`'s `KhorDay` moved across whole, and it is JavaScript
// because that is what was moved — see its head.
import type { CpuDay } from "@/gen/bindings/CpuDay";

/** Every word the drawing says, out of the catalog. */
export type DayWords = {
  cpu: string;
  gpu: string;
  mem: string;
  disk: string;
  vram: string;
  heat: string;
  ridge: string;
  para: string;
  gap: string;
  now: string;
  shade: string;
  strainLegend: string;
  peakAt: (pct: string, at: string) => string;
  brushHead: (what: string, n: number, hours: string) => string;
  brushMedian: (name: string, pct: string) => string;
  brushNone: (name: string) => string;
  brushEmpty: (name: string, band: string) => string;
};

export type DayPanel = {
  update: (day: CpuDay, words: DayWords) => void;
  destroy: () => void;
};

export function mount(host: HTMLElement, opts?: Record<string, unknown>): DayPanel;
