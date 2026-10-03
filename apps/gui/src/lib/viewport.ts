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

/** Whole pixels of cover before it counts — rounding between the two
    viewports is not a keyboard. */
const SLACK = 1;

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
  if (!probe) {
    probe = document.createElement("div");
    probe.setAttribute("aria-hidden", "true");
    probe.style.cssText = "position:fixed;left:0;bottom:0;width:0;height:0;visibility:hidden;pointer-events:none";
    document.body.append(probe);
  }
  const bottom = Math.max(probe.getBoundingClientRect().bottom, document.documentElement.clientHeight);
  return Math.max(0, Math.round(bottom - (vv.offsetTop + vv.height)));
}

export function followVisualViewport(): () => void {
  const vv = window.visualViewport;
  if (!vv) return () => {};
  const root = document.documentElement;
  const apply = () => {
    // The pan WebKit made to show the field, taken back first: the field
    // is in view without it once the shell is as tall as what is left,
    // and a panned shell is a page that scrolled.
    if (window.scrollY !== 0 || window.scrollX !== 0) window.scrollTo(0, 0);
    const covered = keyboardCover();
    // A pinch shrinks the visual viewport too, and that is somebody
    // looking closer, not a keyboard: only an unzoomed page counts.
    if (covered > SLACK && Math.abs(vv.scale - 1) < 0.01) {
      root.setAttribute("data-keyboard", "");
      root.style.setProperty("--app-h", `${Math.round(vv.height)}px`);
      root.style.setProperty("--kb", `${covered}px`);
    } else {
      root.removeAttribute("data-keyboard");
      root.style.removeProperty("--app-h");
      root.style.removeProperty("--kb");
    }
  };
  apply();
  vv.addEventListener("resize", apply);
  vv.addEventListener("scroll", apply);
  // Focus moves before the keyboard has finished rising on some
  // versions; asking again a beat later costs nothing.
  const late = () => window.setTimeout(apply, 350);
  window.addEventListener("focusin", late);
  window.addEventListener("focusout", late);
  return () => {
    vv.removeEventListener("resize", apply);
    vv.removeEventListener("scroll", apply);
    window.removeEventListener("focusin", late);
    window.removeEventListener("focusout", late);
  };
}
