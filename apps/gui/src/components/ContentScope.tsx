/** Both browsing sources share one leading seat, including their menu. */
import {IconBrowser,IconFiles} from '@/components/icons';
import {GlassCircle} from '@/components/Glass';
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuTrigger} from '@/components/ui/dropdown-menu';
import {gui} from '@/gen/catalog';
import type {ContentSource} from '@/hooks/use-content-navigation';

export function ContentScope({value,onChange,inPill=false}:{value:ContentSource;onChange:(source:ContentSource)=>void;inPill?:boolean}) {
  return <DropdownMenu><DropdownMenuTrigger asChild>
    <GlassCircle data-band-control data-content-scope data-in-pill={inPill||undefined} aria-label={gui.content_scope}>
      {value==='workspace'?<IconFiles/>:<IconBrowser/>}
    </GlassCircle>
  </DropdownMenuTrigger><DropdownMenuContent side={inPill?"top":"bottom"} align="start" onCloseAutoFocus={e=>{if(inPill)e.preventDefault();}}>
    <DropdownMenuItem data-content-source="workspace" className="content-source-choice" onSelect={()=>onChange('workspace')}><span className="content-source-glyph"><IconFiles/></span>{gui.workspaces_tab}</DropdownMenuItem>
    <DropdownMenuItem data-content-source="browser" className="content-source-choice" onSelect={()=>onChange('browser')}><span className="content-source-glyph"><IconBrowser/></span>{gui.browser_tab}</DropdownMenuItem>
  </DropdownMenuContent></DropdownMenu>;
}
