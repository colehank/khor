// Session owns common preferences and the Terminal settings subpage.
// Config metadata remains canonical; Terminal display/input are local,
// while agent recognition keeps its explicit target-machine picker.
import type { DeviceRow } from "@/api";
import { AgentRecognition } from "./AgentRecognition";
import { ConfigSections } from "@/views/more/ConfigRows";
import { FinishedNotifyRow, TerminalSection } from "@/views/more/AppearanceRows";
import { MoreFrame,MoreSection } from "@/views/more/MorePage";
import {Button} from "@/components/ui/button";
import {IconBack,IconChevronRight,IconTerminal} from "@/components/icons";
import {useNarrow} from "@/hooks/use-narrow";
import {config,gui} from "@/gen/catalog";

export function TerminalPage({ onClose, devices, machine, onMachine }: { onClose: () => void; devices: DeviceRow[]; machine: string | null; onMachine: (id: string) => void }) {
  const narrow=useNarrow();
  return (
    <MoreFrame page="terminal" onClose={onClose}>
      {!narrow && <Button variant="ghost" data-settings-parent onClick={onClose} className="min-h-touch self-start">
        <IconBack className="size-affordance"/>{config.session}
      </Button>}
      <TerminalSection />
      <AgentRecognition devices={devices} machine={machine} onMachine={onMachine} />
    </MoreFrame>
  );
}

export function SessionPage({ onClose, onSettingChanged,onTerminal }: { onClose: () => void; onSettingChanged: () => void;onTerminal:()=>void }) {
  return (
    <MoreFrame page="session" onClose={onClose}>
      <ConfigSections page="session" onChanged={onSettingChanged} after={{ session: <FinishedNotifyRow /> }} />
      <MoreSection name="session-terminal">
        <button type="button" data-setting-row="nav" data-settings-child="terminal" onClick={onTerminal}
          className="flex min-h-touch w-full items-center gap-row px-row py-in text-start hover:bg-fill focus-visible:-outline-offset-2">
          <IconTerminal className="size-ctl-glyph flex-none"/>
          <span className="min-w-0 flex-1 text-body">{gui.view_terminal}</span>
          <IconChevronRight className="size-affordance flex-none text-tertiary"/>
        </button>
      </MoreSection>
    </MoreFrame>
  );
}

export function WorkspacePage({ onClose, onSettingChanged }: { onClose: () => void; onSettingChanged: () => void }) {
  return (
    <MoreFrame page="workspace" onClose={onClose}>
      <ConfigSections page="workspace" onChanged={onSettingChanged} />
    </MoreFrame>
  );
}
