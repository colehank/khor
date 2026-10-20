// When the editor stops folding long lines into the width (the owner,
// 2026-10-20). Wrapping is the default (「文件显示,要随屏幕宽度自动换行」,
// 2026-09-15), and it is what makes one enormous line expensive: the
// editor measures every row the line folds into before it can draw. A
// notebook's saved plot is one base64 line, and measured in WebKit with
// wrapping on, a 100 KB line opened in 84 ms, 300 KB in 0.54 s, 1 MB in
// 5.7 s, 2 MB in 22 s; unwrapped, every one of them in about 10 ms
// (lane nb-view). So a file with a line past this opens unwrapped, and
// the only thing a person sees change is that it does not fold.
//
// **64 KB**: past the 100 KB step that already costs a frame, and far
// beyond any line a person wrote — the longest source line in this
// repository is under 2 KB; what crosses it is data (base64, minified
// bundles, one-line JSON).
//
// **Decided when a text is loaded, not while it is typed**: on opening,
// and when the file is read again from its machine (the host hands over
// a text that is not the one typed). Typing past the limit, or deleting
// back under it, leaves the wrapping as it was — a view that reflowed
// under the caret in the middle of a paste would be the worse surprise.
export const WRAP_LINE_MAX = 64 * 1024;

/** Whether no line of `text` is longer than `WRAP_LINE_MAX` characters. */
export function wrapsAt(text: string): boolean {
  if (text.length <= WRAP_LINE_MAX) return true;
  let from = 0;
  for (;;) {
    const nl = text.indexOf("\n", from);
    const end = nl === -1 ? text.length : nl;
    if (end - from > WRAP_LINE_MAX) return false;
    if (nl === -1) return true;
    from = nl + 1;
  }
}
