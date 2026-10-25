import type { Pending } from '@/views/chat-files';

export type ChatDraftOwner = { readonly session: string; readonly key: symbol };
export type ChatDraft = {
  owner: ChatDraftOwner;
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
  reason: 'cancelled' | 'failed';
};

const owners = new Map<string, ChatDraftOwner>();
const copyPair = (pair: readonly [string, string]): [string, string] => [pair[0], pair[1]];
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
  for (const session of owners.keys()) if (!allowed.has(session)) owners.delete(session);
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

/** A rejected write is not enough to identify cancellation. Its snapshot
 * stays pending until the token-bearing cancellation/failure receipt lands. */
export function completeChatDraft(session: string, token: string, owner: ChatDraftOwner): void {
  if (owners.get(session) !== owner) return;
  const sends = pending.get(session);
  sends?.delete(token);
  if (sends?.size === 0) pending.delete(session);
}

export function settleChatDraft(session: string, token: string | null | undefined, reason: RecoverableChatDraft['reason'], owner: ChatDraftOwner | null): ChatDraft | null {
  if (!accepts(session) || !token || !owner || owners.get(session) !== owner) return null;
  const draft = pending.get(session)?.get(token);
  if (!draft) return null;
  completeChatDraft(session, token, owner);
  saved.set(session, [...recoverableChatDrafts(session), {draft, reason}]);
  changed();
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
  if (after.length) saved.set(session, after); else saved.delete(session);
  changed();
}

export function subscribeChatDrafts(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
