type Consumer<T> = { read: () => Promise<T>; land: (value: T) => void; fail: (error: unknown) => void };
type Operation = { stale: boolean };

/** Invalidations coalesce without queuing another physical read behind a departed view. */
export class WorkspaceRefresh<T> {
  private consumer: Consumer<T> | null = null;
  private operation: Operation | null = null;
  private needed = false;

  attach(read: () => Promise<T>, land: (value: T) => void, fail: (error: unknown) => void): () => void {
    const consumer = { read, land, fail };
    this.consumer = consumer;
    this.invalidate();
    return () => {
      if (this.consumer !== consumer) return;
      this.consumer = null;
      if (this.operation) this.operation.stale = true;
    };
  }

  invalidate(): void {
    this.needed = true;
    if (this.operation) this.operation.stale = true;
    this.start();
  }

  private start(): void {
    const consumer = this.consumer;
    if (!consumer || !this.needed || this.operation) return;
    this.needed = false;
    const operation: Operation = { stale: false };
    this.operation = operation;
    const owns = () => this.consumer === consumer && !operation.stale;
    void Promise.resolve().then(async () => {
      if (!owns()) return;
      try {
        const value = await consumer.read();
        if (owns()) consumer.land(value);
      } catch (error) {
        if (owns()) consumer.fail(error);
      }
    }).finally(() => {
      this.operation = null;
      this.start();
    });
  }
}
