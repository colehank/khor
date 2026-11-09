import type {IDisposable,Terminal} from "@xterm/xterm";
import type {Block,Marks} from "@/lib/term-marks";

/**
 * **Command blocks, drawn** (roadmap 2.3): a hairline where each prompt
 * starts, and once a command has ended, its exit status as a 2 px stripe
 * in the left margin beside the whole block — the state inks, done green
 * for 0 and failed red otherwise. xterm decorations pinned to the marks'
 * markers, so they ride the scrollback; VS Code draws its command
 * decorations the same way (a gutter mark per command, coloured by exit).
 */
export function drawBlocks(term: Terminal, marks: Marks): { dispose: () => void; size: () => number } {
  const drawn = new Map<Block, IDisposable[]>();
  const draw = () => {
    // A block whose lines left the scrollback took its markers, and with
    // them its decorations; its entry goes too, or one stays per command
    // for the pane's life (the load pass, 2026-10-04).
    for (const [b, ds] of drawn) {
      if (b.prompt.isDisposed) {
        ds.forEach((d) => d.dispose());
        drawn.delete(b);
      }
    }
    for (const b of marks.blocks()) {
      const has = drawn.get(b) ?? [];
      if (has.length === 0) {
        const hair = term.registerDecoration({ marker: b.prompt, width: term.cols, height: 1 });
        hair?.onRender((el) => el.classList.add("term-block-hair"));
        if (hair) has.push(hair);
      }
      if (b.end && has.length === 1) {
        const height = Math.max(1, b.end.line - b.prompt.line);
        const tint = term.registerDecoration({ marker: b.prompt, x: 0, width: 1, height });
        tint?.onRender((el) => {
          el.classList.add("term-block-tint");
          el.dataset.exit = String(b.exit ?? "");
        });
        if (tint) has.push(tint);
      }
      drawn.set(b, has);
    }
  };
  const changed = marks.onChange(draw);
  draw();
  return {
    dispose: () => {
      changed.dispose();
      drawn.forEach((ds) => ds.forEach((d) => d.dispose()));
      drawn.clear();
    },
    size: () => drawn.size,
  };
}
