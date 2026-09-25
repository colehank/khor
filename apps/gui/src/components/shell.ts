// **The shell's one word about moving the window** (docs/SHELL.md 第十五轮
// 第 3 条, docs/DESIGN.md 第五节「拖窗口由外壳一处声明」).
//
// Three places in the window are its handle — the left block's first row
// (L0), the rail (L1) and the right side's first row (R0) — and each is
// marked once, **by the shell, on the container**, with this. `deep`: any
// blank inside moves the window, and tauri's own rule keeps every control
// inside a control (a `<button>`, an `<input>`, anything with a role or a
// tabindex answers its press instead). The list (L2) and the board (R1)
// are not marked, the way Finder's and System Settings' are not.
//
// **Why the attribute is spelled only here.** Until 第十五轮 eleven
// components each marked 「their own row's blank」, and every round that
// moved a row took a handle away with it: 第十三轮's search filled L0's
// row, 第十四轮 took the empty band off the pages without tabs, and the
// window could not be moved from most pages (SHELL.md 挂账 1). A handle
// that lives on the shell's three containers cannot be moved away by a
// page. `check-face` (第十五轮 e) reads `src/` and goes red on the
// attribute anywhere else — `LockScreen` is the one named exception,
// because a locked window has no shell at all and must still move.
//
// If a blank inside one of the three does not drag, the fix is not a
// `"false"` or a second mark in a component: it is to find what is
// standing there and give it the semantics it lacks (a `role`, a
// `tabindex`) — or take it out.
export const WINDOW_DRAG = { "data-tauri-drag-region": "deep" } as const;
