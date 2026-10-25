import type { Pending } from '@/views/chat-files';

export type ChatDraftOwner = { readonly session: string; readonly key: symbol };
export type ChatDraft = {
  owner: ChatDraftOwner;
  reader: object;
  session: string;
  token: string;
  text: string;
  full: string;
  images: readonly [string, string][];
  documents: readonly [string, string][];
  attachments: readonly Pending[];
};
export type RecoverableChatDraft = {
  draft: ChatDraft;
  reason: 'cancelled' | 'failed' | 'unconfirmed';
};

const owners = new Map<string, ChatDraftOwner>();
const copyPair = (pair: readonly [string, string]): [string, string] => [pair[0], pair[1]];
const timers = new Map<ChatDraftOwner, Map<string, ReturnType<typeof setTimeout>>>();
const pending = new Map<string, Map<string, ChatDraft>>();
const saved = new Map<string, readonly RecoverableChatDraft[]>();
const empty: readonly RecoverableChatDraft[] = [];
const listeners = new Set<() => void>();
let allowed: ReadonlySet<string> | null = null;
const accepts = (session: string) => allowed === null || allowed.has(session);
const changed = () => { for (const listener of listeners) listener(); };

/** Open tabs own recovery data. Remember the boundary too: a late receipt
 * after close must not recreate an entry that this pass just released. */
export function retainCancelledChatDrafts(sessions: readonly string[]): void {
  allowed = new Set(sessions);
  let removed = false;
  for (const [session, owner] of owners) {
    if (!allowed.has(session)) {
      for (const timer of timers.get(owner)?.values() ?? []) clearTimeout(timer);
      timers.delete(owner);
      owners.delete(session);
    }
  }
  for (const session of pending.keys()) if (!allowed.has(session)) pending.delete(session);
  for (const session of saved.keys()) {
    if (!allowed.has(session)) { saved.delete(session); removed = true; }
  }
  if (removed) changed();
}

/** The open-tab lifetime, shared by its mounted views. Closing and reopening
 * the same id creates another owner; an old callback cannot claim it again. */
export function chatDraftOwner(session: string): ChatDraftOwner | null {
  if (!accepts(session)) return null;
  let owner = owners.get(session);
  if (!owner) owners.set(session, owner = {session, key: Symbol(session)});
  return owner;
}

/** Strings are shared, including base64. Only metadata/tuple containers are
 * copied; the original thumbnail URL is owned and revoked by the composer. */
export function rememberChatDraft(draft: ChatDraft): ChatDraft | null {
  if (!accepts(draft.session) || owners.get(draft.session) !== draft.owner || !/^[A-Za-z0-9._:-]{1,128}$/.test(draft.token)) return null;
  let sends = pending.get(draft.session);
  if (sends?.has(draft.token) || recoverableChatDrafts(draft.session).some(entry => entry.draft.token === draft.token)) return null;
  const copy: ChatDraft = {
    ...draft,
    images: draft.images.map(copyPair),
    documents: draft.documents.map(copyPair),
    attachments: draft.attachments.map(item => ({ ...item, thumb: null, inline: item.inline ? copyPair(item.inline) : null })),
  };
  if (!sends) pending.set(draft.session, sends = new Map());
  sends.set(draft.token, copy);
  return copy;
}

function clearReceiptTimer(owner: ChatDraftOwner, token: string): void {
  const waiting = timers.get(owner);
  const timer = waiting?.get(token);
  if (timer !== undefined) clearTimeout(timer);
  waiting?.delete(token);
  if (waiting?.size === 0) timers.delete(owner);
}

function removePending(session: string, token: string, owner: ChatDraftOwner): void {
  clearReceiptTimer(owner, token);
  const sends = pending.get(session);
  sends?.delete(token);
  if (sends?.size === 0) pending.delete(session);
}

/** Only the token-bearing commit receipt confirms a write on every
 * transport: the HTTP command reply acknowledges admission, not commit. */
export function completeChatDraft(session: string, token: string, owner: ChatDraftOwner, reader?: object): void {
  if (owners.get(session) !== owner) return;
  if (reader && pending.get(session)?.get(token)?.reader !== reader) return;
  removePending(session, token, owner);
  const before = recoverableChatDrafts(session);
  const after = before.filter(entry => !(entry.draft.token === token && entry.reason === 'unconfirmed'));
  if (after.length !== before.length) {
    if (after.length) saved.set(session, after); else saved.delete(session);
    changed();
  }
}

function save(draft: ChatDraft, reason: RecoverableChatDraft['reason']): void {
  const before = recoverableChatDrafts(draft.session);
  const index = before.findIndex(entry => entry.draft.token === draft.token);
  if (index >= 0 && before[index].reason === reason) return;
  const entry = {draft, reason};
  saved.set(draft.session, index < 0 ? [...before, entry] : before.map((old, at) => at === index ? entry : old));
  changed();
}

/** A rejection alone cannot distinguish cancellation from transport failure.
 * Keep its data until a receipt lands; make missing confirmation visible. */
export function awaitChatDraftReceipt(session: string, token: string, owner: ChatDraftOwner, after = 1000): void {
  if (owners.get(session) !== owner || !pending.get(session)?.has(token)) return;
  clearReceiptTimer(owner, token);
  let waiting = timers.get(owner);
  if (!waiting) timers.set(owner, waiting = new Map());
  waiting.set(token, setTimeout(() => {
    clearReceiptTimer(owner, token);
    if (owners.get(session) !== owner) return;
    const draft = pending.get(session)?.get(token);
    if (draft) save(draft, 'unconfirmed');
  }, after));
}

/** A lost/restarted/ended attachment cannot promise to replay old receipts. */
export function unconfirmedChatDrafts(session: string, owner: ChatDraftOwner | null, scope?: {reader: object; except?: boolean}): void {
  if (!owner || owners.get(session) !== owner) return;
  for (const draft of pending.get(session)?.values() ?? []) {
    if (scope && (scope.except ? draft.reader === scope.reader : draft.reader !== scope.reader)) continue;
    clearReceiptTimer(owner, draft.token);
    save(draft, 'unconfirmed');
  }
}

export function settleChatDraft(session: string, token: string | null | undefined, reason: 'cancelled' | 'failed', owner: ChatDraftOwner | null, reader?: object): ChatDraft | null {
  if (!accepts(session) || !token || !owner || owners.get(session) !== owner) return null;
  const draft = pending.get(session)?.get(token);
  if (!draft || (reader && draft.reader !== reader)) return null;
  removePending(session, token, owner);
  save(draft, reason);
  return draft;
}

export function recoverableChatDrafts(session: string): readonly RecoverableChatDraft[] {
  return saved.get(session) ?? empty;
}

export function discardChatDraft(session: string, token: string, owner: ChatDraftOwner): void {
  if (owners.get(session) !== owner) return;
  const before = recoverableChatDrafts(session);
  const after = before.filter(entry => entry.draft.token !== token);
  if (after.length === before.length) return;
  removePending(session, token, owner);
  if (after.length) saved.set(session, after); else saved.delete(session);
  changed();
}

export function subscribeChatDrafts(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
