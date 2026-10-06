// The soft keyboard, said to the stylesheet (2026-10-02, iOS 第三步 A).
//
// A phone's keyboard does not make the page shorter. WebKit keeps the
// layout viewport — and with it `100%`, `100dvh` and every box sized off
// them — the height of the whole screen, slides the keyboard over its
// foot, and pans the page so the focused field shows. On the session page
// that is the wrong answer three times: the bar at the foot (the tabs, the
// keys a soft keyboard lacks) goes under the keys, the terminal keeps
// rows nobody can see, and the page scrolls when the shell is nailed down
// (`tokens.css`, 「the shell is nailed down」).
//
// The one thing that does shrink is the **visual** viewport. So the shell
// is sized to it instead: while it is shorter than the layout viewport,
// the root wears `data-keyboard` and `--app-h` (its height), `app.css`
// makes `#root` that tall, and everything inside lays out in what is left
// — the bar lands on top of the keys, the terminal's box shrinks and its
// own resize observer refits it, so `cols`/`rows` follow and the far pty
// is told. Any pan the webview made is taken back, because nothing in the
// shell is out of view any more.
//
// A desktop window's two viewports are the same height, so this stamps
// nothing there; a browser without `visualViewport` keeps the old shell.
//
// **khor's own panel in the keyboard's place** (UX-31, the owner
// 2026-10-06: the key row's 「…」 is a keyboard, not a sheet). The panel
// (`lib/key-panel`) asks this file to **hold the foot**: while held, the
// root stays stamped as if a keyboard of that height were up, whether or
// not the system's is — so swapping the keyboard for the panel, and back,
// moves nothing above them. `keyboardShown()` keeps meaning the system's
// keyboard; `footCovered()` is either.

/** Whole pixels of cover before it counts — rounding between the two
    viewports is not a keyboard. */
const SLACK = 1;

/** **The keyboard's height before one was ever measured** (UX-31): a
    phone's portrait keyboard with its suggestion bar and the home
    indicator under it, as an iPhone of the owner's size draws it. The
    first real keyboard replaces it, and the number is kept. */
const DEFAULT_KEYBOARD = 336;
const KEYBOARD_KEY = "khor.keyboard-h";

/** The foot khor's panel holds, in px, or none (`holdFoot`). */
let held: number | null = null;
/** The last settled keyboard height (`keyboardHeight`). */
let remembered = (() => {
  try {
    const n = Number(window.localStorage.getItem(KEYBOARD_KEY));
    return Number.isFinite(n) && n > 0 ? n : DEFAULT_KEYBOARD;
  } catch {
    return DEFAULT_KEYBOARD;
  }
})();
/** `followVisualViewport`'s apply, for a hold to re-stamp through. */
let reapply: (() => void) | null = null;

/** **The last keyboard's height** (UX-31): what a panel raised with the
    keyboard down is as tall as. */
export function keyboardHeight(): number {
  return remembered;
}

function remember(px: number) {
  if (px === remembered) return;
  remembered = px;
  try {
    window.localStorage.setItem(KEYBOARD_KEY, String(px));
  } catch {
    // Kept for this session.
  }
}

/**
 * **Hold the foot at `px`, or let it go** (UX-31, `lib/key-panel`): the
 * root is stamped as if a keyboard that tall were up — the shell lays out
 * above it, `--kb` says how much — until the hold is released. A system
 * keyboard taller than the hold still wins while it is up. Re-stamps at
 * once, so the caller's own change (a panel mounted, a keyboard lowered)
 * lands in the same frame.
 */
export function holdFoot(px: number | null) {
  if (px === held) return;
  held = px;
  reapply?.();
  window.dispatchEvent(new Event("khor:soft-keyboard"));
}

/** What the foot is held at, or null. */
export function footHeld(): number | null {
  return held;
}

/** **The foot is covered — by the system keyboard, settled, or by khor's
    panel** (UX-31): what a deck that must not fold, and a screen laid
    out above the keys, ask. */
export function footCovered(): boolean {
  return keyboardShown() || held !== null;
}

/** How long the visual viewport must hold still before the keyboard
    counts as up or down: past its animation (item 28). */
const SETTLED_MS = 120;

/** The keyboard as the screen last settled on it (item 28): up when the
    visual viewport is shorter than the layout one and has held still. */
let shown = false;
let settle = 0;
/** When the visual viewport last moved (ms, `performance.now`): a resize
    of the terminal's box inside this window is the keyboard animating
    (item 29). */
let steppedAt = -Infinity;

/** Whether the soft keyboard is up, as the screen says — not as a field
    asked (item 28: iOS lowers the keyboard on its own, and a state kept
    from the request said "up" after it had gone). */
export function keyboardShown(): boolean {
  return forced ?? shown;
}

// ⚠️ Tombstone (item 42 → item 49): `keyboardEdge`, the keyboard's top
// edge from the visual viewport, was where the lowering swipe had to
// reach; any downward swipe lowers now (`lib/term-swipe`).

// The gates' seam, in the dev server only: a desktop browser has no soft
// keyboard to bring up, so a walk under touch says one is up by hand.
let forced: boolean | null = null;
if (import.meta.env.DEV && typeof window !== "undefined") {
  (window as unknown as { __khorKeyboard?: (on: boolean | null) => void }).__khorKeyboard = (on) => {
    forced = on;
    window.dispatchEvent(new Event("khor:soft-keyboard"));
  };
}

/** Whether the visual viewport moved within the last `ms`. */
export function viewportMoving(ms: number): boolean {
  return performance.now() - steppedAt < ms;
}

/** A zero-size `fixed` box on the layout viewport's bottom edge. */
let probe: HTMLElement | null = null;

/**
 * **How much of the layout viewport the soft keyboard covers**, in CSS
 * pixels — 0 where there is no keyboard.
 *
 * Measured against the layout viewport's own bottom edge (a `fixed`
 * probe, and `clientHeight` beside it), **not `window.innerHeight`**
 * (2026-10-03, the owner on a real iPhone: the chat capsule, the address
 * row and their tab rows stayed under the keys). In an app's WKWebView
 * `innerHeight` can follow the visual viewport down when the keyboard
 * comes up, and then 「innerHeight − visual height」 is 0 while the bottom
 * of the shell is behind the keys: nothing was stamped, nothing rode.
 * The layout viewport's bottom is where `fixed` and `100%` boxes end,
 * which is the thing the keys cover.
 */
export function keyboardCover(): number {
  const vv = window.visualViewport;
  if (!vv) return 0;
  return Math.max(0, Math.round(layoutBottom() - (vv.offsetTop + vv.height)));
}

/** The layout viewport's bottom edge in client px: where `fixed` and
    `100%` boxes end. */
function layoutBottom(): number {
  if (!probe) {
    probe = document.createElement("div");
    probe.setAttribute("aria-hidden", "true");
    probe.style.cssText = "position:fixed;left:0;bottom:0;width:0;height:0;visibility:hidden;pointer-events:none";
    document.body.append(probe);
  }
  return Math.max(probe.getBoundingClientRect().bottom, document.documentElement.clientHeight);
}

export function followVisualViewport(): () => void {
  const vv = window.visualViewport;
  if (!vv) return () => {};
  const root = document.documentElement;
  // The pan WebKit made to show the field, taken back: the field is in
  // view without it once the shell is as tall as what is left, and a
  // panned shell is a page that scrolled. **Once after the keyboard rose
  // — the focus's late beat, and the settled state flipping — never on
  // every visual-viewport scroll** (the owner, 2026-10-05, iPhone build
  // 5: 「只要在输入框往上滑，就会抖动」): a drag on the composer or the
  // deck was taken by the webview's own scroll view, which bounced while
  // this took the pan back frame by frame, and the two fought until the
  // finger lifted. The bounce is off on the phone (`hold_the_face_still`).
  const takeBack = () => {
    if (window.scrollY !== 0 || window.scrollX !== 0) window.scrollTo(0, 0);
  };
  /** The system keyboard's cover, as measured now: a pinch shrinks the
      visual viewport too, and that is somebody looking closer, not a
      keyboard — only an unzoomed page counts. */
  const measured = () => {
    const covered = keyboardCover();
    return covered > SLACK && Math.abs(vv.scale - 1) < 0.01 ? covered : 0;
  };
  const apply = () => {
    steppedAt = performance.now();
    const keys = measured();
    // **The foot held by khor's panel counts as a keyboard** (UX-31): the
    // larger of the two is what the shell lays out above, so the panel
    // taking the keyboard's place — or giving it back — changes nothing.
    const covered = held === null ? keys : Math.max(held, keys);
    if (covered > 0) {
      root.setAttribute("data-keyboard", "");
      root.style.setProperty("--app-h", `${Math.round(held === null ? vv.height : layoutBottom() - covered)}px`);
      root.style.setProperty("--kb", `${covered}px`);
    } else {
      root.removeAttribute("data-keyboard");
      root.style.removeProperty("--app-h");
      root.style.removeProperty("--kb");
    }
    // The keyboard's state, read once it has stopped moving, and said to
    // whoever follows it (`lib/soft-keyboard`'s `onKeyboardChange`). The
    // system's own, not the hold: a panel is not a keyboard to them.
    window.clearTimeout(settle);
    settle = window.setTimeout(() => {
      const keysNow = measured();
      const now = keysNow > 0;
      if (now) remember(keysNow);
      if (now === shown) return;
      shown = now;
      takeBack();
      window.dispatchEvent(new Event("khor:soft-keyboard"));
    }, SETTLED_MS);
  };
  reapply = apply;
  apply();
  takeBack();
  vv.addEventListener("resize", apply);
  vv.addEventListener("scroll", apply);
  // Focus moves before the keyboard has finished rising on some
  // versions; asking again a beat later costs nothing.
  const late = () =>
    window.setTimeout(() => {
      apply();
      takeBack();
    }, 350);
  window.addEventListener("focusin", late);
  window.addEventListener("focusout", late);
  return () => {
    if (reapply === apply) reapply = null;
    vv.removeEventListener("resize", apply);
    vv.removeEventListener("scroll", apply);
    window.removeEventListener("focusin", late);
    window.removeEventListener("focusout", late);
  };
}
