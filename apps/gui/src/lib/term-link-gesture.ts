import type { ILink, Terminal } from '@xterm/xterm';

const activations = new WeakMap<object, 'claimed' | 'cancelled'>();
export const terminalLinkGesture = (event: object) => activations.get(event);

type Cell = { col: number; row: number };
type Press = { link: ILink; down: MouseEvent; phase: 'link' | 'program' | 'cancelled' };

/** xterm 6 exposes the resolved provider result only through its core linkifier.
 * Read that same result so OSC8, URLs and verified paths keep one priority order. */
function currentLink(term: Terminal): ILink | undefined {
  return (term as unknown as { _core?: { linkifier?: { currentLink?: { link: ILink } } } })._core?.linkifier?.currentLink?.link;
}

function contains(link: ILink, cell: Cell | null): boolean {
  if (!cell) return false;
  const { start, end } = link.range, x = cell.col + 1, y = cell.row + 1;
  return y >= start.y && y <= end.y && (y > start.y || x >= start.x) && (y < end.y || x <= end.x);
}

/** Claim a resolved link before xterm sends its button-down to the program.
 * A drag gives the original press back; it must never also activate the link. */
export function claimTerminalLinkClicks(term: Terminal, box: HTMLElement, at: (x: number, y: number) => Cell | null, slop: number): () => void {
  const doc = box.ownerDocument, view = doc.defaultView;
  const capture = { capture: true };
  let press: Press | undefined, replaying = false;
  const consume = (event: MouseEvent) => { event.preventDefault(); event.stopImmediatePropagation(); };
  const down = (event: MouseEvent) => {
    if (press?.phase === 'cancelled' && event.button === 0) press = undefined;
    if (replaying || press || event.defaultPrevented || event.button !== 0 || event.shiftKey || event.altKey || term.hasSelection() || term.modes.mouseTrackingMode === 'none') return;
    const link = currentLink(term);
    if (!link || !contains(link, at(event.clientX, event.clientY))) return;
    press = { link, down: event, phase: 'link' };
    consume(event);
    term.focus();
  };
  const moved = (event: MouseEvent) => {
    if (!press || press.phase !== 'link') return;
    if (Math.hypot(event.clientX - press.down.clientX, event.clientY - press.down.clientY) <= slop) {
      consume(event);
      return;
    }
    press.phase = 'program';
    const Mouse = view?.MouseEvent;
    if (!Mouse) return;
    const original = press.down;
    replaying = true;
    try {
      original.target?.dispatchEvent(new Mouse('mousedown', {
        bubbles: true, cancelable: true, button: original.button, buttons: original.buttons,
        detail: original.detail, view: original.view,
        clientX: original.clientX, clientY: original.clientY,
        metaKey: original.metaKey, ctrlKey: original.ctrlKey, shiftKey: original.shiftKey, altKey: original.altKey,
      }));
    } finally { replaying = false; }
  };
  const up = (event: MouseEvent) => {
    if (!press || event.button !== 0) return;
    const saved = press;
    press = undefined;
    if (saved.phase === 'program') {
      activations.set(event, 'cancelled');
      return;
    }
    consume(event);
    if (saved.phase === 'cancelled') return;
    if (!box.contains(event.target as Node) || term.hasSelection() || currentLink(term) !== saved.link || !contains(saved.link, at(event.clientX, event.clientY))) return;
    if (Math.hypot(event.clientX - saved.down.clientX, event.clientY - saved.down.clientY) > slop) return;
    activations.set(event, 'claimed');
    saved.link.activate(event, saved.link.text);
  };
  const cancel = () => {
    if (press?.phase === 'link') press.phase = 'cancelled';
    else if (press?.phase === 'program') press = undefined;
  };
  box.addEventListener('mousedown', down, capture);
  doc.addEventListener('mousemove', moved, capture);
  doc.addEventListener('mouseup', up, capture);
  doc.addEventListener('pointercancel', cancel, capture);
  view?.addEventListener('blur', cancel);
  return () => {
    press = undefined;
    box.removeEventListener('mousedown', down, capture);
    doc.removeEventListener('mousemove', moved, capture);
    doc.removeEventListener('mouseup', up, capture);
    doc.removeEventListener('pointercancel', cancel, capture);
    view?.removeEventListener('blur', cancel);
  };
}
