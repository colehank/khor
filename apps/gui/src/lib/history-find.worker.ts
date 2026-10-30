import { historyFindBatch, type FindWorkerInput, type FindWorkerOutput } from './history-find-model';

let batch: ReturnType<typeof historyFindBatch> | null = null;
self.onmessage = (event: MessageEvent<FindWorkerInput>) => {
  try {
    const message = event.data;
    if (message.kind === 'query') {
      batch = historyFindBatch(message.query);
      return;
    }
    if (!batch) throw new Error('history search query is missing');
    const answer: FindWorkerOutput = message.kind === 'page' ? batch.page(message.page) : batch.next();
    self.postMessage(answer);
  } catch (failure) {
    self.postMessage({ kind: 'error', error: String(failure) } satisfies FindWorkerOutput);
  }
};
