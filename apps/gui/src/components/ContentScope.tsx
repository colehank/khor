/** Both browsing sources share one leading seat, including their menu. */
import {IconBrowser,IconFiles,IconChevronUp} from '@/components/icons';
import {GlassCircle} from '@/components/Glass';
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuTrigger} from '@/components/ui/dropdown-menu';
import {gui} from '@/gen/catalog';
import type {ContentSource} from '@/hooks/use-content-navigation';

export function ContentScope({value,onChange,inPill=false}:{value:ContentSource;onChange:(source:ContentSource)=>void;inPill?:boolean}) {
  const glyph=value==='workspace'?<IconFiles/>:<IconBrowser/>;
  return <DropdownMenu><DropdownMenuTrigger asChild>
    {inPill?<button type="button" data-band-control data-content-scope data-in-pill className="content-source-inline" aria-label={gui.content_scope}>{glyph}<IconChevronUp centred className="content-source-chevron"/></button>:<GlassCircle data-band-control data-content-scope aria-label={gui.content_scope}>{glyph}</GlassCircle>}
  </DropdownMenuTrigger><DropdownMenuContent side={inPill?"top":"bottom"} align="start" onCloseAutoFocus={e=>{if(inPill)e.preventDefault();}}>
    <DropdownMenuItem data-content-source="workspace" className="content-source-choice" onSelect={()=>onChange('workspace')}><span className="content-source-glyph"><IconFiles/></span>{gui.workspaces_tab}</DropdownMenuItem>
    <DropdownMenuItem data-content-source="browser" className="content-source-choice" onSelect={()=>onChange('browser')}><span className="content-source-glyph"><IconBrowser/></span>{gui.browser_tab}</DropdownMenuItem>
  </DropdownMenuContent></DropdownMenu>;
}
