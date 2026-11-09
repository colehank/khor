import type {ReactNode} from "react";
import type {DeviceRow} from "@/api";
import {MachineFace} from "@/components/SessionFace";
import {NetworkMark} from "@/components/NetworkMark";
import {IconChevronRight,IconDevices,IconNetwork,IconShield} from "@/components/icons";
import {cn} from "@/lib/utils";
import {gui} from "@/gen/catalog";
import {MoreFrame,MoreSection} from "./MorePage";

function Link({name,label,glyph,onPress,danger=false}: {name:string;label:string;glyph?:ReactNode;onPress:()=>void;danger?:boolean}) {
  return <button type="button" data-setting-row="nav" data-network-action={name} onClick={onPress}
    className={cn("flex min-h-touch w-full items-center gap-row px-row py-in text-start hover:bg-fill focus-visible:-outline-offset-2",danger && "text-destructive")}>
    {glyph && <span className="flex flex-none items-center justify-center [&>svg]:size-ctl-glyph">{glyph}</span>}
    <span className="min-w-0 flex-1 text-body">{label}</span>
    <IconChevronRight className="size-affordance flex-none text-tertiary"/>
  </button>;
}

/** Network membership is the identity here; this machine is one member. */
export function NetworkOverview({devices,me,holdsRoot,onClose,onMachine,onAdd,onConnections,onTopology,onWords,onLeave,onReset,status}: {
  devices:DeviceRow[];me:DeviceRow|null;holdsRoot:boolean;onClose:()=>void;
  onMachine:(row:DeviceRow,at:DOMRect)=>void;onAdd:()=>void;onConnections:()=>void;
  onTopology:()=>void;onWords:()=>void;onLeave?:()=>void;onReset:()=>void;status:string[]|null;
}) {
  return <MoreFrame page="me" onClose={onClose}>
    {status?.map(line=><p key={line} role="status" className="text-callout text-muted-foreground">{line}</p>)}
    <header data-network-profile className="flex flex-col items-center gap-in py-group text-center">
      <NetworkMark large/>
      <h2 className="text-title-lg font-title">{gui.khor_network}</h2>
      <p className="text-body text-muted-foreground">{gui.khor_network_members(devices.length)}</p>
      {me && <p className="text-callout text-muted-foreground">{gui.khor_network_current(me.shown)}</p>}
    </header>
    <MoreSection name="network-members" label={gui.devices_tab}>
      {devices.map(row=><button key={row.id} type="button" data-setting-row="nav" data-network-member={row.id}
        onClick={event=>onMachine(row,event.currentTarget.getBoundingClientRect())}
        className="flex min-h-row-2 w-full items-center gap-row px-row py-in text-start hover:bg-fill focus-visible:-outline-offset-2">
        <MachineFace face={row.face} system={row.vitals?.vitals.system ?? null} className="flex-none"/>
        <span className="min-w-0 flex-1 text-body">{row.shown}</span>
        {row.me && <span className="text-callout text-muted-foreground">{gui.machine_here}</span>}
        <IconChevronRight className="size-affordance flex-none text-tertiary"/>
      </button>)}
      <Link name="add" label={gui.add_machine} glyph={<IconDevices/>} onPress={onAdd}/>
    </MoreSection>
    <MoreSection name="network-connections" label={gui.network}>
      <Link name="connections" label={gui.khor_network_connections} glyph={<IconNetwork/>} onPress={onConnections}/>
      <Link name="topology" label={gui.khor_network_topology} glyph={<IconNetwork/>} onPress={onTopology}/>
      {holdsRoot && <Link name="recovery" label={gui.root_words_title} glyph={<IconShield/>} onPress={onWords}/>}
    </MoreSection>
    {me && <MoreSection name="network-current-machine" label={gui.khor_network_this_machine}>
      {onLeave && <Link name="leave" label={gui.identity_leave_do} onPress={onLeave} danger/>}
      <Link name="reset" label={gui.khor_network_reset_this_machine} onPress={onReset} danger/>
    </MoreSection>}
  </MoreFrame>;
}
