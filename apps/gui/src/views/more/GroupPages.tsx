// Session owns general options and expanded Terminal settings.
// Native configuration metadata remains canonical.
import {ConfigSections} from "@/views/more/ConfigRows";
import {FinishedNotifyRow,TerminalSection} from "@/views/more/AppearanceRows";
import {MoreFrame,MoreSectionHeading} from "@/views/more/MorePage";
import {gui} from "@/gen/catalog";

export function SessionPage({onClose,onSettingChanged}: {onClose:()=>void;onSettingChanged:()=>void}) {
  return <MoreFrame page="session" onClose={onClose}>
    <section data-session-general className="flex min-w-0 flex-col gap-xxs">
      <MoreSectionHeading>{gui.settings}</MoreSectionHeading>
      <ConfigSections page="session" onChanged={onSettingChanged} after={{session:<FinishedNotifyRow/>}}/>
    </section>
    <section data-terminal-settings className="flex min-w-0 flex-col gap-group">
      <TerminalSection/>
    </section>
  </MoreFrame>;
}

export function WorkspacePage({onClose,onSettingChanged}: {onClose:()=>void;onSettingChanged:()=>void}) {
  return <MoreFrame page="workspace" onClose={onClose}>
    <ConfigSections page="workspace" onChanged={onSettingChanged}/>
  </MoreFrame>;
}
