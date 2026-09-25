// Whether this window is the one in front — macOS's 「key window」
// (docs/SHELL.md 第十五轮 第 2 条, D6).
//
// A selected list row is the brand's solid green while the window is in
// front and grey the moment it is not, the way every sidebar on macOS
// behaves: the green says 「this is where your keys go」, and in a window
// behind another that stops being true. The root carries
// `data-window-key="false"` while the window is behind; `app.css` keys
// the grey off it.
//
// **The document's own focus events, not tauri's.** A webview hears
// `blur` / `focus` on its window when the app loses and regains the
// front (WKWebView and WebView2 both pass them through), and a browser
// tab hears the same on the web face — one road for every face, with no
// round trip. `document.hasFocus()` seeds it, since a window that opens
// behind another never gets a `blur` to say so.
export function followWindowKey(): void {
  const root = document.documentElement;
  const wear = (key: boolean) => root.setAttribute("data-window-key", key ? "true" : "false");
  wear(document.hasFocus());
  window.addEventListener("focus", () => wear(true));
  // Asked again a moment later rather than taken at its word: focus
  // moving into a frame inside this page (a PDF's viewer) blurs the
  // window too, and the document still has the focus then.
  window.addEventListener("blur", () => setTimeout(() => wear(document.hasFocus()), 0));
}
