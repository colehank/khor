import { useEffect, useRef, useState } from 'react';

import { fetchDevices, type DeviceRow, type SessionRow } from '@/api';
import { DirPicker } from '@/components/DirPicker';
import { gui } from '@/gen/catalog';
import { useSessionRecovery, type RecoveryOpened } from '@/hooks/use-session-recovery';
import { offerNotice } from '@/lib/pill-notices';
import { recoveryRelationship, shouldContinueOnEntry } from '@/lib/recovery-relationship';

export function RecoveryActions({ row, onOpened }: { row: SessionRow; onOpened: (opened: RecoveryOpened) => void }) {
  const home = row.execution_home;
  if (!home || !/^[0-9a-f]{64}$/.test(home)) return <UnknownHome row={row}/>;
  return <RecoverySurface key={JSON.stringify([home, row.id])} row={row} home={home} onOpened={onOpened}/>;
}

function UnknownHome({ row }: { row: SessionRow }) {
  useEffect(() => {
    if (row.kind !== 'gui' && row.kind !== 'shell') return;
    return offerNotice(row.id, { id:'relationship', kind:'unconfirmed', layout:'compact', short:[{name:row.title},' · ',gui.recovery_pill_unconfirmed], whole:gui.recovery_pill_unconfirmed, at:Date.now(), acts:[] });
  }, [row.id, row.kind, row.title]);
  return null;
}

function RecoverySurface({ row, home, onOpened }: { row: SessionRow; home: string; onOpened: (opened: RecoveryOpened) => void }) {
  const recovery = useSessionRecovery({ id:row.id, home,
    revision:JSON.stringify([row.word, row.can_control, row.attachable, row.ended]), onOpened });
  const [picker, setPicker] = useState<DeviceRow | null>(null);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const sameIntent = recovery.intent?.home === home && recovery.intent.id === row.id;
  const facts = { kind:row.kind, offer:recovery.offer, intentOperation:sameIntent ? recovery.intent!.operation : null,
    outcome:recovery.outcome?.kind ?? null, ready:recovery.intentReady, busy:recovery.busy || picking,
    fault:!!(recovery.fault || recovery.problem || pickerError) };
  const presentation = recoveryRelationship(facts);
  const current = useRef({ recovery, presentation });
  current.current = { recovery, presentation };
  const entered = useRef(false);
  useEffect(() => {
    if (entered.current || !recovery.intentReady || !recovery.offer) return;
    entered.current = true;
    if (shouldContinueOnEntry(facts)) void current.current.recovery.act();
  }, [recovery.intentReady, recovery.offer, sameIntent, row.kind]);
  useEffect(() => {
    if (recovery.outcome?.kind !== 'pending' || !sameIntent || !recovery.intentReady || facts.busy || facts.fault) return;
    const timer = window.setTimeout(() => { void current.current.recovery.act(); }, 1000);
    return () => window.clearTimeout(timer);
  }, [recovery.outcome, sameIntent, recovery.intentReady, facts.busy, facts.fault]);

  const run = useRef<() => void>(() => {});
  run.current = () => {
    if (facts.busy) return;
    setPickerError(null);
    const action = current.current.presentation.action;
    if (action === 'directory') {
      setPicking(true);
      void fetchDevices().then(devices => {
        if (!alive.current) return;
        const device = devices.find(value => value.id === home);
        if (!device) throw new Error('execution home is unavailable');
        setPicker(device);
      }).catch(error => { if (alive.current) setPickerError(String(error)); }).finally(() => { if (alive.current) setPicking(false); });
    } else if (action === 'refresh') void current.current.recovery.refresh();
    else if (action) void current.current.recovery.act();
  };
  const words = {
    quiet:'', connecting:gui.recovery_pill_connecting, continuing:gui.recovery_pill_continuing,
    opening:gui.web_opening_here, ended:gui.term_over, unconfirmed:gui.recovery_pill_unconfirmed,
    failed:gui.recovery_pill_failed, directory:gui.recovery_pill_directory, unsupported:gui.recovery_pill_unsupported,
  };
  const whole = presentation.state === 'failed' && (recovery.problem || pickerError) ? gui.recovery_pill_unsupported : words[presentation.state];
  const actionWord = presentation.action === 'directory' ? gui.recovery_choose_directory
    : presentation.action === 'new_shell' ? gui.dial_new_terminal : gui.region_retry;
  useEffect(() => {
    if (presentation.state === 'quiet') return;
    const working = ['connecting','continuing','opening'].includes(presentation.state);
    return offerNotice(row.id, {
      id:'relationship', kind:working ? 'rejoin' : presentation.state === 'ended' ? 'ended' : 'unconfirmed', layout:'compact',
      short:[{name:row.title},' · ',whole], whole, at:Date.now(),
      acts:presentation.action ? [{key:'retry', word:actionWord, run:() => run.current()}] : [],
    });
  }, [row.id, row.title, presentation.state, presentation.action, whole, actionWord]);

  return picker && <DirPicker open fixed machine={picker.name} onOpenChange={open => { if (!open) setPicker(null); }} onPick={async pick => {
    if (pick.machine !== picker.name || picker.id !== home) throw new Error(gui.recovery_no_home);
    setPicker(null);
    await current.current.recovery.act(pick.path);
  }}/>;
}
