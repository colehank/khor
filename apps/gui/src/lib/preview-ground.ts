// **预览背景** (round UX-4, the file's 「…」): the ground a rendered
// document is read on — the app's own, or always light, or always dark —
// per device, like the file's 字号. Worn as `data-ground` on the
// preview's root, which `tokens.css` answers with that theme's ground
// tokens only (the one scoped exception to a root-only palette).
import { useEffect, useState } from "react";

export type Ground = "auto" | "light" | "dark";
export const GROUNDS: Ground[] = ["auto", "light", "dark"];

const KEY = "khor.preview-ground";
const CHANGED = "khor:preview-ground";

function stored(): Ground {
  try {
    const v = window.localStorage.getItem(KEY);
    return v === "light" || v === "dark" ? v : "auto";
  } catch {
    return "auto";
  }
}

/** The ground and a way to change it; every open preview follows. */
export function usePreviewGround(): { ground: Ground; set: (g: Ground) => void } {
  const [ground, setGround] = useState<Ground>(stored);
  useEffect(() => {
    const again = () => setGround(stored());
    window.addEventListener(CHANGED, again);
    return () => window.removeEventListener(CHANGED, again);
  }, []);
  const set = (g: Ground) => {
    try {
      window.localStorage.setItem(KEY, g);
    } catch {
      // A private window keeps it for this page only.
    }
    setGround(g);
    window.dispatchEvent(new Event(CHANGED));
  };
  return { ground, set };
}

/** The attribute a preview root wears: nothing when it follows the app. */
export function groundAttr(g: Ground): { "data-ground"?: "light" | "dark" } {
  return g === "auto" ? {} : { "data-ground": g };
}
