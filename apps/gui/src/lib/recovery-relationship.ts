import type { SessionRecovery } from '@/api';

export type RecoveryRelationship = {
  state: 'quiet' | 'connecting' | 'continuing' | 'opening' | 'ended' | 'unconfirmed' | 'failed' | 'directory' | 'unsupported';
  action: 'retry' | 'refresh' | 'new_shell' | 'directory' | null;
};
type Facts = {
  kind: string;
  offer: SessionRecovery | null;
  intentOperation: 'native' | 'new_shell' | null;
  outcome: string | null;
  ready: boolean;
  busy: boolean;
  fault: boolean;
};

const owned = (kind: string) => kind === 'gui' || kind === 'shell';

export function recoveryRelationship(facts: Facts): RecoveryRelationship {
  const { kind, offer, intentOperation, outcome, ready, busy, fault } = facts;
  const available = !!offer && offer.action !== 'none' && offer.execution !== 'offline' && !!offer.next_attempt_epoch;
  const retry = ready && (intentOperation || available) ? 'retry' : 'refresh';
  if (busy || outcome === 'pending' && !fault) return {
    state: intentOperation === 'new_shell' || offer?.action === 'new_shell' ? 'opening' : offer?.action === 'join' ? 'connecting' : 'continuing', action: null,
  };
  if (fault) return { state: 'failed', action: retry };
  if (outcome === 'directory_required' && !intentOperation) return { state: 'directory', action: 'directory' };
  if (intentOperation) return { state: 'unconfirmed', action: ready ? 'retry' : 'refresh' };
  if (outcome === 'opened' || outcome === 'join') return { state: 'quiet', action: null };
  if (!owned(kind)) return { state: 'quiet', action: null };
  if (offer?.execution === 'live') return { state: 'quiet', action: null };
  if (outcome === 'refused' || outcome === 'unavailable' || outcome === 'unknown') return { state: 'failed', action: retry };
  if (!offer) return { state: 'quiet', action: null };
  if (offer.execution !== 'ended') return { state: 'unconfirmed', action: 'refresh' };
  if (offer.action === 'new_shell') return { state: 'ended', action: available && ready ? 'new_shell' : null };
  return { state: 'unsupported', action: retry };
}

/** Called only for the first settled offer on entry, never for a later exit. */
export function shouldContinueOnEntry(facts: Facts): boolean {
  if (!owned(facts.kind) || !facts.ready || facts.busy || facts.fault || !facts.offer) return false;
  if (facts.intentOperation) return true; // Reconcile the same durable, previously authorized token.
  const { offer } = facts;
  return facts.kind === 'gui' && offer.execution === 'ended' && offer.action === 'native' && !!offer.next_attempt_epoch
    && (offer.native?.support === 'resume' || offer.native?.support === 'load');
}
