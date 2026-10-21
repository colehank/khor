/** Every field shares one native presenter. Serialize IPC mutations so a
 * delayed open cannot overtake a newer field's open. Superseded queued
 * updates are accepted without presentation, not reported as unsupported. */
export function nativeMenuQueue<Request extends { id: string }>(transport: {
  show: (request: Request) => Promise<boolean>;
  hide: (id: string) => Promise<unknown>;
}) {
  let tail: Promise<unknown> = Promise.resolve();
  let desired: { id: string; ticket: number } | null = null;
  let ticket = 0;
  const serial = <T>(run: () => Promise<T>): Promise<T> => {
    const result = tail.then(run, run);
    // A refused presenter must not poison the following field's queue.
    tail = result.catch(() => {});
    return result;
  };
  return {
    show(request: Request): Promise<boolean> {
      const thisTicket = ++ticket;
      desired = { id: request.id, ticket: thisTicket };
      return serial(() => desired?.ticket === thisTicket
        ? transport.show(request)
        : Promise.resolve(true));
    },
    hide(id: string): Promise<unknown> {
      if (desired?.id === id) desired = null;
      return serial(() => transport.hide(id));
    },
  };
}
