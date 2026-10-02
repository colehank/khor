/**
 * **A dialog's one act is always pressable** (the owner, 2026-10-02):
 * neither hidden nor greyed while something it needs is missing. Pressed
 * then, it does not act: the caret goes to what is missing and a line
 * says why, where the person is already looking. ⚠️ 墓碑 (the audit fix
 * plan F1.9, one commit): the act was not drawn until its input was
 * valid — a dialog with no way forward and no word about the way.
 */
import { useState } from "react";

export function useWhy() {
  const [why, setWhy] = useState<string | null>(null);
  return {
    why,
    /** Say why the act cannot go yet, and put the caret where it is
        needed (`field`: a selector inside the page). */
    ask: (text: string, field?: string) => {
      setWhy(text);
      if (field) requestAnimationFrame(() => document.querySelector<HTMLElement>(field)?.focus());
    },
    clear: () => setWhy(null),
  };
}

export function Why({ text }: { text: string | null }) {
  if (!text) return null;
  return (
    <p data-dialog-why role="alert" className="text-callout text-destructive">
      {text}
    </p>
  );
}
