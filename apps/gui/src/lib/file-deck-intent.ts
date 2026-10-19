/** A file face restores its reading position asynchronously. Those scrolls
 * are not reading gestures. Re-arm folding only after actual scroll input;
 * momentum keeps that intent until a face change or explicit unfold. */
export class FileDeckIntent {
  private reading = false;
  private from: { x: number; y: number } | null = null;
  reset() { this.reading = false; this.from = null; }
  start(x: number, y: number) { this.from = { x, y }; }
  move(x: number, y: number) {
    if (!this.from) return;
    const dx = Math.abs(x - this.from.x), dy = Math.abs(y - this.from.y);
    if (dy >= 8 && dy > dx) this.reading = true;
  }
  end() { this.from = null; }
  wheel(dx: number, dy: number) { if (dy !== 0 && Math.abs(dy) >= Math.abs(dx)) this.reading = true; }
  key(key: string) { if (['ArrowUp','ArrowDown','PageUp','PageDown','Home','End'].includes(key)) this.reading = true; }
  allows() { return this.reading; }
}
