// Which hand is on the app: the keyboard or a pointer.
//
// **The focus ring is the keyboard's** (the audit fix plan F1.3 / F4.5,
// 2026-10-02): a ring that appears because a finger tapped a field, or
// before anybody has pressed a key at all, is noise on a phone and the
// web's tell on a Mac. `:focus-visible` alone cannot say it — a text
// field matches it on any focus — so the root carries the last input's
// kind (`data-modality`) and the one focus rule in `app.css` reads it.
//
// A key that moves focus or acts on it is the keyboard; a press of a
// pointer (mouse, pen, finger) is a pointer. Typing letters into a field
// does not flip it, so a ring does not appear under the caret.
const KEYS = new Set(["Tab", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", " ", "Escape", "Home", "End"]);

export function followModality(): void {
  const root = document.documentElement;
  const set = (m: "keyboard" | "pointer") => {
    if (root.dataset.modality !== m) root.dataset.modality = m;
  };
  set("pointer");
  window.addEventListener(
    "keydown",
    (e) => {
      if (KEYS.has(e.key) || e.metaKey || e.ctrlKey) set("keyboard");
    },
    true,
  );
  window.addEventListener("pointerdown", () => set("pointer"), true);
}
