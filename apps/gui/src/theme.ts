// 外观 (docs/MORE.md 设置): 跟系统 / 浅 / 深.
//
// # Who resolves `auto`, and why it is here rather than in Rust
//
// The store keeps the **choice** — `auto`, `light` or `dark`
// (`khor_node::config`) — and never the theme it resolves to. A store
// holding 「dark」 because the desktop was dark that evening is a machine
// that quietly stopped following the desktop, and nobody said so.
//
// So resolving is the face's job, because `matchMedia` is the only place
// the question 「what is the desktop wearing *now*」 can be asked at all,
// and because the answer changes while khor is running.
//
// # The stamp is the whole mechanism
//
// Every stylesheet keys its dark half off `:root[data-theme="dark"]`
// rather than off a media query (tokens.css has the full argument: the
// alternative was a second copy of a hundred and sixty lines of
// palette). So this file setting one attribute *is* the theme switch,
// and four gates depend on the `change` listener below re-stamping when
// `emulateMedia({colorScheme})` flips the desktop under them.
//
// # The mirror
//
// `index.html` stamps before the bundle exists, and it cannot ask khor —
// the store is behind an async call. So the resolved **choice** is
// mirrored into `localStorage` here and read there. It is a cache, never
// the preference: `khor config` remains the one place that lives, a
// browser that refuses storage simply follows the desktop, and the cost
// of a stale mirror is one quarter-second flash on the first launch
// after somebody changes the setting.
import { inApp, isMenuFace, setWindowTheme } from "@/api";

/** The key `index.html`'s pre-bundle script reads. **Written in two
    places by necessity** — that script runs before any module — and
    `check-boot` asserts the two spellings agree, the same way it
    guards the four colours that frame repeats. */
export const THEME_MIRROR = "khor.theme";

/** What a face can be told to wear. `auto` is a choice, not a theme. */
export type ThemePref = "auto" | "light" | "dark";

/** The last choice khor handed over, so the `change` listener can
    re-resolve without asking again. */
let pref: ThemePref = "auto";

function systemIsDark(): boolean {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  } catch {
    // No `matchMedia` at all is a face that cannot ask, and light is
    // what an unstamped root has always painted.
    return false;
  }
}

/** The choice as it was last mirrored, for the stamp before khor has
    answered. Anything that is not a choice khor takes reads as `auto` —
    including a mirror written by a newer build. */
export function mirroredPref(): ThemePref {
  try {
    const was = window.localStorage.getItem(THEME_MIRROR);
    if (was === "light" || was === "dark" || was === "auto") return was;
  } catch {
    // Private window, blocked site data.
  }
  return "auto";
}

/**
 * Wears a choice: stamps the root, mirrors it, and — in the app — hands
 * the resolved theme to the window itself, whose title bar and ground
 * are AppKit's and not the page's.
 *
 * Handed `null` (khor has not answered, or has no such key) it keeps the
 * choice it already had rather than inventing one: a theme that snapped
 * back to `auto` because a read was late would be a window that changes
 * colour on its own.
 */
export function wearTheme(next: ThemePref | null): void {
  // **The overlay follows the app; it never steers it** (2026-10-01, the
  // owner: 浅色下左块是深色材质、字暗). The overlay menu window loads this
  // same page (`?face=menu`) at startup and never reads the store, so a
  // choice of its own could only be the mirror as it stood when it
  // booted. It used to wear that choice like the face — on NSApp, which
  // is the whole app's — so two pages handed the app two answers and the
  // last one won: launched on 浅色 over a dark desktop, the overlay's
  // stale 「auto」 landed after the face's 「light」 and the sidebar
  // material stayed dark under a light page (`npm run native`, the
  // launch against the desktop: 1 launch in 2 before this). Its `change`
  // listener below would hand it over again on every appearance change.
  // NSApp already wears the face's choice and the overlay's
  // `prefers-color-scheme` is NSApp's, so the overlay reads it as `auto`
  // and writes neither the mirror nor the window.
  if (isMenuFace) {
    document.documentElement.setAttribute("data-theme", systemIsDark() ? "dark" : "light");
    return;
  }
  if (next !== null) pref = next;
  const dark = pref === "dark" || (pref === "auto" && systemIsDark());
  document.documentElement.setAttribute("data-theme", dark ? "dark" : "light");
  try {
    window.localStorage.setItem(THEME_MIRROR, pref);
  } catch {
    // Only the head start on the next launch is lost.
  }
  // **The window is not the page.** Its ground is painted from Rust
  // before any HTML exists and its title bar is AppKit's, so neither
  // follows an attribute on a document. Nothing outside the app has a
  // window of its own (`api.ts` `setWindowTheme` is a no-op there).
  //
  // **The choice, not the resolved theme** (2026-09-27): the app wears
  // `dark` / `light` on NSApp and clears it for `auto`, so AppKit follows
  // the desktop by itself — see `set_window_theme`.
  if (inApp) void setWindowTheme(pref).catch(() => {});
}

/**
 * Follows the desktop while the choice is `auto`.
 *
 * **Not optional.** `check-day`, `check-palette`, `check-rail` and
 * `smoke` all flip the theme with `emulateMedia({colorScheme})` and
 * then read colours back; what turns that into a repaint is this
 * listener re-stamping. Armed once, for the life of the face.
 */
export function followSystemTheme(): void {
  try {
    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", () => wearTheme(null));
  } catch {
    // A face that cannot ask cannot follow; the stamp stands.
  }
}
