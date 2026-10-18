/** Machine identity and searchable choice use one field across creation flows. */
import type { DeviceRow } from '@/api';
import { MachineAvatar } from '@/components/Avatar';
import { ChoiceField, type ChoiceOption } from '@/components/ui/choice-field';
import { gui } from '@/gen/catalog';
import { connectionWord } from '@/words';

type Identity = Pick<DeviceRow, 'name' | 'shown' | 'face'> & { id?: string };
export function machineChoice(machine: Identity, id = machine.name, aside?: string): ChoiceOption {
  return {
    id,
    label: machine.shown,
    terms: `${machine.name} ${machine.id ?? ''}`,
    aside,
    icon: <MachineAvatar face={machine.face} className="size-kind-mark" />,
  };
}

export function MachineField({
  devices,
  value,
  onPick,
  onPendingChange,
  disabled = false,
}: {
  devices: DeviceRow[];
  value: string;
  onPick: (name: string) => void;
  onPendingChange?: (pending: boolean) => void;
  disabled?: boolean;
}) {
  const picked = devices.find(d => d.name === value || d.id === value || (value === '' && d.me));
  return (
    <ChoiceField
      label={gui.new_session_machine}
      value={picked?.shown ?? value}
      selected={picked?.name}
      placeholder={gui.page_choose_machine}
      disabled={disabled}
      options={devices.map(d => machineChoice(d, d.name, connectionWord(d) || undefined))}
      onPick={onPick}
      onPendingChange={onPendingChange}
    />
  );
}
