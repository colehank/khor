import { useState } from "react";

import { fetchDevices, type DeviceRow, type SessionRow } from "@/api";
import { DirPicker } from "@/components/DirPicker";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { useSessionRecovery, type RecoveryOpened } from "@/hooks/use-session-recovery";
import { HeaderSeat } from "@/views/SessionPreferences";

export function RecoveryActions({ row, previous, onOpened, onSession }: {
  row: SessionRow;
  previous: string | null;
  onOpened: (opened: RecoveryOpened) => void;
  onSession?: (id: string) => void;
}) {
  // A display name or a readable content locator is never execution authority.
  const home = row.execution_home;
  if (!home || !/^[0-9a-f]{64}$/.test(home)) return <HeaderSeat>
    <DropdownMenu><DropdownMenuTrigger asChild><Button variant="ghost" size="sm" className="min-h-touch">{gui.recovery_title}</Button></DropdownMenuTrigger>
      <DropdownMenuContent><DropdownMenuLabel>{gui.recovery_no_home}</DropdownMenuLabel></DropdownMenuContent>
    </DropdownMenu>
  </HeaderSeat>;
  return <RecoverySurface key={JSON.stringify([home, row.id])} row={row} home={home} previous={previous} onOpened={onOpened} onSession={onSession} />;
}
function RecoverySurface({ row, home, previous, onOpened, onSession }: {
  row: SessionRow; home: string; previous: string | null;
  onOpened: (opened: RecoveryOpened) => void; onSession?: (id: string) => void;
}) {
  const recovery = useSessionRecovery({ id: row.id, home,
    revision: JSON.stringify([row.word, row.can_control, row.attachable, row.ended]), onOpened });
  const [picker, setPicker] = useState<DeviceRow | null>(null);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const { offer, outcome, intent, busy } = recovery;
  const sameIntent = intent?.home === home && intent.id === row.id;
  const prior = offer?.previous ?? previous;
  const chooseDirectory = async () => {
    if (busy || picking) return;
    setPicking(true); setPickerError(null);
    try {
      const devices = await fetchDevices();
      const device = devices.find(value => value.id === home);
      if (!device) { setPickerError(gui.recovery_no_home); return; }
      setPicker(device);
    } catch (error) { setPickerError(String(error)); }
    finally { setPicking(false); }
  };
  const action = sameIntent ? gui.recovery_retry_intent : offer?.action === "join" ? gui.recovery_join
    : offer?.action === "new_shell" ? gui.recovery_new_shell
    : offer?.native?.support === "unknown" ? gui.recovery_check
    : offer?.native?.support === "unsupported" ? gui.recovery_recheck : gui.recovery_resume;
  const state = outcome?.kind === "pending" ? gui.recovery_pending
    : outcome?.kind === "unknown" ? gui.recovery_unknown
    : outcome?.kind === "directory_required" ? gui.recovery_directory_required
    : outcome?.kind === "unavailable" ? gui.recovery_unavailable
    : outcome?.kind === "refused" ? gui.recovery_refused
    : sameIntent ? gui.recovery_unresolved : null;
  const problem = recovery.problem && {
    storage: gui.recovery_storage, invalid: gui.recovery_invalid_intent,
    occupied: gui.recovery_conflict, capacity: gui.recovery_capacity, unavailable: gui.recovery_unavailable,
  }[recovery.problem];
  const execution = offer && {
    live: gui.recovery_live, ended: gui.recovery_ended,
    offline: gui.recovery_offline, unknown: gui.recovery_execution_unknown,
  }[offer.execution];
  return <>
    <HeaderSeat><DropdownMenu>
      <DropdownMenuTrigger asChild><Button variant="ghost" size="sm" className="min-h-touch" aria-busy={busy || undefined}>{gui.recovery_title}</Button></DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="max-w-dialog">
        {execution && <DropdownMenuLabel>{execution}</DropdownMenuLabel>}
        {offer?.native && <DropdownMenuLabel className="break-all whitespace-normal">{offer.native.agent} · {offer.native.native_id}</DropdownMenuLabel>}
        {offer?.native?.support === "unsupported" && <DropdownMenuLabel className="whitespace-normal">{gui.recovery_unsupported}</DropdownMenuLabel>}
        {prior && <DropdownMenuItem disabled={!onSession} onSelect={() => onSession?.(prior)}>{gui.recovery_previous} <code>{prior}</code></DropdownMenuItem>}
        {state && <DropdownMenuLabel role="status" className="whitespace-normal">{state}</DropdownMenuLabel>}
        {outcome && "reason" in outcome && <DropdownMenuLabel className="break-words whitespace-normal">{outcome.reason}</DropdownMenuLabel>}
        {offer?.reason && <DropdownMenuLabel className="break-words whitespace-normal">{offer.reason}</DropdownMenuLabel>}
        {(problem || recovery.fault || pickerError) && <DropdownMenuLabel role="alert" className="break-words whitespace-normal text-destructive">{problem ?? recovery.fault ?? pickerError}</DropdownMenuLabel>}
        <DropdownMenuSeparator />
        {outcome?.kind === "directory_required" && !intent ?
          <DropdownMenuItem disabled={busy || picking} onSelect={event => { event.preventDefault(); void chooseDirectory(); }}>{gui.recovery_choose_directory}</DropdownMenuItem>
          : <DropdownMenuItem disabled={busy || !recovery.intentReady || (!sameIntent && (!offer || offer.action === "none" || offer.execution === "offline" || !offer.next_attempt_epoch))}
              onSelect={event => { event.preventDefault(); void recovery.act(); }}>{action}</DropdownMenuItem>}
        <DropdownMenuItem disabled={busy} onSelect={event => { event.preventDefault(); void recovery.refresh(); }}>{gui.file_reread}</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu></HeaderSeat>
    {picker && <DirPicker open fixed machine={picker.name} onOpenChange={open => { if (!open) setPicker(null); }} onPick={async pick => {
      // Only the selected path crosses the action boundary. The picker cannot
      // replace the authoritative execution home with its display/channel name.
      if (pick.machine !== picker.name || picker.id !== home) throw new Error(gui.recovery_no_home);
      await recovery.act(pick.path); // fresh offer/epoch/nonce after DirectoryRequired
    }} />}
  </>;
}
