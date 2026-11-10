import { useEffect, useState } from "react";
import { onWindowFocus } from "@/api";

/** Focus of the viewing window, including its sibling native page. */
export function useWindowFocus(): boolean {
  const [focused, setFocused] = useState(() => !document.hidden && document.hasFocus());
  useEffect(() => onWindowFocus(setFocused), []);
  return focused;
}
