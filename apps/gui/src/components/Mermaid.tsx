// A ```mermaid block, drawn (lane md-render).
//
// **Loaded on first need, drawn once the fence has closed.** Mermaid is
// the heaviest thing the renderer can reach for — far heavier than the
// rest of it together — so it is its own chunk and only a closed mermaid
// fence fetches it; a block still arriving is a growing code block like
// any other (`Markdown`'s `codePart`), and a diagram half-typed is never
// laid out.
//
// **Its colours are this app's.** Mermaid's themes are its own palette;
// `base` with its variables read off the tokens at draw time puts the
// diagram on the ground it sits on — the app's, or the preview's own
// (预览背景) — and redraws it when that changes.
//
// **Strict.** `securityLevel: "strict"` encodes every label and refuses
// click handlers; what comes back is an SVG mermaid has already passed
// through its own sanitiser, and the gate reads the drawn block back for
// a script or an `on*` attribute (`scripts/check-md.mjs`). A diagram that
// will not parse stays its source.
import { useEffect, useRef, useState, type ReactNode } from "react";

type Mermaid = typeof import("mermaid")["default"];

let lib: Promise<Mermaid> | null = null;
/** Draws one at a time: `initialize` is global, and two grounds drawing
    at once would trade colours. */
let queue: Promise<unknown> = Promise.resolve();
const DRAWN = new Map<string, string | null>();
let seq = 0;

function vars(at: Element) {
  const css = getComputedStyle(at);
  const v = (name: string) => css.getPropertyValue(name).trim();
  return {
    darkMode: css.colorScheme === "dark",
    background: v("--background"),
    fontFamily: v("--font-sans"),
    primaryColor: v("--muted"),
    primaryTextColor: v("--foreground"),
    primaryBorderColor: v("--muted-foreground"),
    secondaryColor: v("--fill"),
    tertiaryColor: v("--fill-strong"),
    lineColor: v("--muted-foreground"),
    textColor: v("--foreground"),
    noteBkgColor: v("--fill"),
    noteTextColor: v("--foreground"),
  };
}

function draw(code: string, at: Element): Promise<string | null> {
  const theme = vars(at);
  const key = `${JSON.stringify(theme)}\n${code}`;
  if (DRAWN.has(key)) return Promise.resolve(DRAWN.get(key) ?? null);
  lib ??= import("mermaid").then((m) => m.default);
  const job = queue.then(async () => {
    const mermaid = await lib!;
    mermaid.initialize({ startOnLoad: false, securityLevel: "strict", theme: "base", themeVariables: theme });
    try {
      const { svg } = await mermaid.render(`khor-mermaid-${(seq += 1)}`, code);
      DRAWN.set(key, svg);
      return svg;
    } catch {
      DRAWN.set(key, null);
      return null;
    }
  });
  queue = job.catch(() => {});
  return job;
}

/** The ground the block sits on, as a string that changes when it does:
    the app's theme, and the preview's own (`data-ground`). */
function useGround(at: React.RefObject<HTMLElement | null>): string {
  const read = () => `${document.documentElement.dataset.theme}/${at.current?.closest("[data-ground]")?.getAttribute("data-ground") ?? ""}`;
  const [ground, setGround] = useState(read);
  useEffect(() => {
    const look = () => setGround(read());
    const mo = new MutationObserver(look);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    const preview = at.current?.closest("[data-ground]");
    if (preview) mo.observe(preview, { attributes: true, attributeFilter: ["data-ground"] });
    look();
    return () => mo.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return ground;
}

export function MermaidBlock({ code, children }: { code: string; children: ReactNode }) {
  const box = useRef<HTMLDivElement>(null);
  const ground = useGround(box);
  const [svg, setSvg] = useState<string | null>(null);
  useEffect(() => {
    if (box.current === null) return;
    let live = true;
    void draw(code, box.current).then((s) => live && setSvg(s));
    return () => {
      live = false;
    };
  }, [code, ground]);
  return svg === null ? (
    <div ref={box} data-mermaid="source">
      {children}
    </div>
  ) : (
    <div ref={box} data-mermaid="drawn" dangerouslySetInnerHTML={{ __html: svg }} />
  );
}
