/** A session column and its optional content column share one top edge.
 * Each column owns its own header; navigation never resizes the window. */
import {createContext,useContext,useState,type ComponentProps,type ReactNode} from 'react';
import {Tabs} from '@/browse/Tabs';
import {SessionScreen} from '@/components/SessionBar';
import {SessionHeader} from '@/components/SessionHeader';

const SettingsSeat=createContext<HTMLElement|null|undefined>(undefined);
export const useSessionSettingsSeat=()=>useContext(SettingsSeat);

export function SessionFrame({aside,children,tail,mobile=false,contentMode=false,onBack,onTitle,...tabs}:ComponentProps<typeof Tabs>&{
  aside?:ReactNode;mobile?:boolean;contentMode?:boolean;onBack?:()=>void;onTitle?:()=>void;
}) {
  const [settings,setSettings]=useState<HTMLSpanElement|null>(null);
  if(mobile){
    const chat=tabs.faces.find(face=>face.id===tabs.active)?.session?.kind==='gui';
    return <SettingsSeat.Provider value={settings}><SessionScreen faces={tabs.faces} active={tabs.active}
      onFocus={tabs.onFocus} onClose={tabs.onClose} onNew={tabs.onNew} onBack={onBack??(()=>tabs.onClose(tabs.active))}
      onTitle={onTitle} contentMode={contentMode} renderHeader={chat?props=><SessionHeader {...props}/>:undefined} deckAt={chat?'top':'bottom'}>
      {children}
    </SessionScreen></SettingsSeat.Provider>;
  }
  return <SettingsSeat.Provider value={settings}><div data-session-panes data-session-frame className="flex min-h-0 min-w-0 flex-1 flex-row">
    <div data-session-primary style={aside?{minWidth:"min(var(--session-side-min),50%)"}:undefined} className="flex min-h-0 min-w-0 flex-1 flex-col">
      <Tabs {...tabs} tail={<><span ref={setSettings} data-lab-session-settings-slot className="empty:hidden"/>{tail}</>} windowControls={tabs.windowControls??!aside}>{children}</Tabs>
    </div>
    {aside}
  </div></SettingsSeat.Provider>;
}
