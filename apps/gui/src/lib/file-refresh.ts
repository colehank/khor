/** A background disk read may replace a clean buffer only while the
 * document has had no intervening edit, composition or newer read. */
export function fileRefresh() {
  let revision = 0;
  let request = 0;
  let composing = false;
  const invalidate = () => { revision += 1; };
  return {
    invalidate,
    setComposing(next: boolean) {
      if (composing === next) return;
      composing = next;
      invalidate();
    },
    async read<T>(load: () => Promise<T | null>, accept: (value: T) => void) {
      if (composing) return;
      const startedAt = revision;
      const id = ++request;
      const value = await load();
      if (value !== null && startedAt === revision && id === request && !composing) accept(value);
    },
  };
}
