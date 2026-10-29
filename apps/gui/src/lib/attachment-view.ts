import type { AttachmentKind } from '@/gen/bindings/AttachmentKind';

type Transport = {
  begin: (id: string, kind: AttachmentKind) => Promise<string>;
  leave: (id: string, kind: AttachmentKind, lease: string) => Promise<void>;
};
export type AttachmentCleanup = { failures: number; last: string | null };

/** A view owns one Begin ACK and releases it even when the ACK arrives late. */
export function attachmentView<Opened>(id: string, kind: AttachmentKind, transport: Transport) {
  let stopped = false;
  let lease: string | null = null;
  let opening: Promise<Opened> | null = null;
  let releasing: Promise<void> | null = null;
  let failures = 0;
  let last: string | null = null;
  const closed = () => new Error('attachment view is closed');
  const release = () => {
    if (lease === null) return Promise.resolve();
    const key = lease;
    return releasing ??= Promise.resolve().then(() => transport.leave(id, kind, key)).catch(error => {
      failures++;
      last = String(error).slice(0, 512);
    });
  };
  const begun = Promise.resolve().then(() => {
    if (stopped) throw closed();
    return transport.begin(id, kind);
  }).then(async key => {
    lease = key;
    if (stopped) { await release(); throw closed(); }
    return key;
  });
  // Construction and disposal may happen without an open call (StrictMode).
  void begun.catch(() => {});
  const dispose = async (): Promise<AttachmentCleanup> => {
    stopped = true;
    await begun.catch(() => {});
    await release();
    return { failures, last };
  };
  return {
    id,
    open(work: (lease: string) => Promise<Opened>): Promise<Opened> {
      return opening ??= begun.then(key => {
        if (stopped) throw closed();
        return work(key);
      }).catch(async error => { await dispose(); throw error; });
    },
    async run<Result>(work: (lease: string) => Promise<Result>): Promise<Result> {
      if (!opening) throw new Error('attachment is not open');
      await opening;
      if (stopped || lease === null) throw closed();
      return work(lease);
    },
    dispose,
  };
}
