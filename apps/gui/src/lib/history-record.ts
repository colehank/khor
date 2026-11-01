import type { HistoryRecordChunk } from '@/gen/bindings/HistoryRecordChunk';
export function decodeRecordSection(chunk: HistoryRecordChunk, decoder: TextDecoder): string {
  const bytes = Uint8Array.from(atob(chunk.data), character => character.charCodeAt(0));
  return decoder.decode(bytes, { stream: chunk.next !== null });
}
