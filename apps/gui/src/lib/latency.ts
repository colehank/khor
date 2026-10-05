// **Latency bands, one scale for the whole app** (UX-26, the owner
// 2026-10-06): a round trip is read as one of four words wherever a
// light sits beside a number — the machine row's 「延迟灯 · 23 ms」, the
// 我的 khor dial's seats, its roads and its matrix. Written once so two
// surfaces cannot draw one reading two colours.
//
// The three inks are three of the six state words (`tokens.css`): `fast`
// wears 完成's green, `mid` 出错's amber, `slow` 失败's red; `none` is
// `tertiary`'s grey. A number is always beside a light; never the colour
// alone (docs/design-system 六个词只作墨).

export type Band = "fast" | "mid" | "slow" | "none";

/** Under this, a road is `fast`. */
export const FAST_UNDER_MS = 30;
/** Under this (and not `fast`), a road is `mid`; from here up, `slow`. */
export const MID_UNDER_MS = 120;

/** The band a round trip falls in; no reading is `none`. */
export function band(ms: number | null | undefined): Band {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return "none";
  return ms < FAST_UNDER_MS ? "fast" : ms < MID_UNDER_MS ? "mid" : "slow";
}

/** The ink a band wears, as the token it reads — the one place the
    four colours are named, so a surface writes `var(…)` of this and
    nothing of its own. */
export const BAND_INK: Record<Band, string> = {
  fast: "var(--state-done)",
  mid: "var(--state-errored)",
  slow: "var(--state-failed)",
  none: "var(--tertiary)",
};
