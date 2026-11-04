export type DraftStorage = Pick<Storage, 'getItem' | 'setItem'>;
const KEY = 'khor.session-drafts.v1';
const LIMIT = 512 * 1024;

export class SessionDraftStore {
  private readonly storage: DraftStorage | null;
  private readonly entries = new Map<string, string>();
  private saved: string | null = null;
  private readable = true;

  constructor(storage: DraftStorage | null) {
    this.storage = storage;
    if (!storage) return;
    try {
      const raw = storage.getItem(KEY);
      if (raw === null) { this.saved = '[]'; return; }
      if (raw.length > LIMIT) throw new Error('draft snapshot exceeds its limit');
      const values: unknown = JSON.parse(raw);
      if (!Array.isArray(values) || values.length > 512) throw new Error('invalid draft snapshot');
      for (const entry of values) {
        if (!Array.isArray(entry) || entry.length !== 2 || typeof entry[0] !== 'string' ||
          !/^(gui|khor):.+/.test(entry[0]) || entry[0].length > 1024 || typeof entry[1] !== 'string') {
          throw new Error('invalid draft entry');
        }
      }
      for (const [key, text] of values as [string, string][]) if (text) this.entries.set(key, text);
      this.saved = raw;
    } catch { this.readable = false; }
  }

  get(key: string): string { return this.entries.get(key) ?? ''; }

  set(key: string, text: string): void {
    if (text) this.entries.set(key, text); else this.entries.delete(key);
    this.checkpoint();
  }

  retain(sessions: readonly string[]): void {
    const open = new Set(sessions);
    for (const key of this.entries.keys()) if (!open.has(key.slice(key.indexOf(':') + 1))) this.entries.delete(key);
    this.checkpoint();
  }

  checkpoint(): boolean {
    if (!this.storage) return this.entries.size === 0;
    if (!this.readable) return false;
    const encoded = JSON.stringify([...this.entries]);
    if (encoded === this.saved) return true;
    if (encoded.length > LIMIT || this.entries.size > 512) return false;
    try { this.storage.setItem(KEY, encoded); this.saved = encoded; return true; }
    catch { return false; }
  }
}
