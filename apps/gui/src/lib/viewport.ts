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

export function followVisualViewport(): () => void {
  const vv = window.visualViewport;
  if (!vv) return () => {};
  const root = document.documentElement;
  const apply = () => {
    const covered = window.innerHeight - vv.height;
    // A pinch shrinks the visual viewport too, and that is somebody
    // looking closer, not a keyboard: only an unzoomed page counts.
    if (covered > SLACK && Math.abs(vv.scale - 1) < 0.01) {
      root.setAttribute("data-keyboard", "");
      root.style.setProperty("--app-h", `${Math.round(vv.height)}px`);
    } else {
      root.removeAttribute("data-keyboard");
      root.style.removeProperty("--app-h");
    }
    // The pan WebKit made to show the field: the field is in view now
    // without it, and a panned shell is a page that scrolled.
    if (window.scrollY !== 0 || window.scrollX !== 0) window.scrollTo(0, 0);
  };
  apply();
  vv.addEventListener("resize", apply);
  vv.addEventListener("scroll", apply);
  return () => {
    vv.removeEventListener("resize", apply);
    vv.removeEventListener("scroll", apply);
  };
}
