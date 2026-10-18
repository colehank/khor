/** Local receipts cover the row version the person opened, never a future
 * completion. Keep them after persistence: an older list may still arrive. */
export type ReadReceipt = { id: string; at_ms: number };
type ReadRow = ReadReceipt & { unread: number };

export class SessionReads {
  private seen = new Map<string, number>();
  private queued = new Map<string, ReadReceipt>();

  read(row: ReadRow): ReadReceipt | null {
    // The shared conversation acknowledges individual rendered message versions,
    // not a session watermark (including messages that were never on screen).
    if (row.id === "wide/all" || row.unread === 0 || !Number.isSafeInteger(row.at_ms) || row.at_ms < 0) return null;
    const at = this.seen.get(row.id);
    if (at !== undefined && at >= row.at_ms) return null;
    const receipt = { id: row.id, at_ms: row.at_ms };
    this.seen.set(row.id, row.at_ms);
    this.queued.set(row.id, receipt);
    return receipt;
  }

  project<T extends ReadRow>(rows: T[]): T[] {
    return rows.map((row) => {
      const at = this.seen.get(row.id);
      return at !== undefined && row.at_ms <= at && row.unread > 0 ? { ...row, unread: 0 } : row;
    });
  }

  pending(): ReadReceipt[] { return [...this.queued.values()]; }

  confirmed(receipt: ReadReceipt): void {
    // An earlier request completing cannot drop the newer queued receipt.
    if (this.queued.get(receipt.id)?.at_ms === receipt.at_ms) this.queued.delete(receipt.id);
  }
}
