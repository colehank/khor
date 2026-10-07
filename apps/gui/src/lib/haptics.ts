// **Haptics where iOS plays them** (UX-10; ux-9 B5). HIG Playing haptics:
// 「Use system-provided haptic patterns according to their documented
// meanings」, consistently, with a clear cause — and standard controls
// already play them, which is the gap: khor's switches, segments and
// confirmations are drawn by the page, so they were silent where UIKit's
// would tap. Three meanings, each one place:
//
// - `selection` — a value a person picks moved: a switch, a segment
//   (UISelectionFeedbackGenerator, what UIKit's pickers play);
// - `success` — a task they started completed: a pairing took
//   (UINotificationFeedbackGenerator.success);
// - `warning` — the act was refused and said why: a join or a delete
//   the other side turned down (…warning);
// - `threshold` — a drag crossed the line where letting go acts, either
//   way: the deck pill's swipe between tabs (UX-49 §5; a light
//   UIImpactFeedbackGenerator, the snap HIG gives impacts to).
//
// The terminal's bell keeps its own light impact (`bell_haptic`). Only
// under a finger and only in the app — or the dev bridge beside it, where
// a gate can count what was asked: a Mac has no engine to reach, a
// browser no command.
import { haptic, onWebFace } from "@/api";

export type Feel = "selection" | "success" | "warning" | "threshold";

export function feel(kind: Feel): void {
  if (onWebFace || !window.matchMedia("(pointer: coarse)").matches) return;
  haptic(kind).catch(() => {});
}
