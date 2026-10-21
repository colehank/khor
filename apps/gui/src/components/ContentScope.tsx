/** Both browsing sources share one leading seat, including their menu. */
import {IconBrowser,IconFiles,IconChevronUp,IconChevronDown} from '@/components/icons';
import {GlassCircle} from '@/components/Glass';
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuTrigger} from '@/components/ui/dropdown-menu';
import {SegmentedControl} from '@/components/ui/segmented-control';
import {gui} from '@/gen/catalog';
import type {ContentSource} from '@/hooks/use-content-navigation';

export function ContentScope({value,onChange,inPill=false,segmented=false}:{value:ContentSource;onChange:(source:ContentSource)=>void;inPill?:boolean;segmented?:boolean}) {
  if(segmented)return <SegmentedControl label={gui.content_scope} value={value} onChange={onChange} mark={{'data-content-scope':true,'data-content-scope-segmented':true}} className="flex-none" options={[
    {key:'workspace',label:<span className="inline-flex items-center gap-in"><span className="content-source-glyph"><IconFiles/></span>{gui.workspaces_tab}</span>,mark:{'data-content-source':'workspace'}},
    {key:'browser',label:<span className="inline-flex items-center gap-in"><span className="content-source-glyph"><IconBrowser/></span>{gui.browser_tab}</span>,mark:{'data-content-source':'browser'}},
  ]}/>;
  const glyph=value==='workspace'?<IconFiles/>:<IconBrowser/>;
  return <DropdownMenu><DropdownMenuTrigger asChild>
    {inPill?<button type="button" data-band-control data-content-scope data-in-pill className="content-source-inline" aria-label={gui.content_scope}>{glyph}<IconChevronUp centred className="content-source-chevron"/></button>:<GlassCircle className="content-source-desktop" data-band-control data-content-scope aria-label={gui.content_scope}>{glyph}<IconChevronDown centred className="content-source-chevron"/></GlassCircle>}
  </DropdownMenuTrigger><DropdownMenuContent side={inPill?"top":"bottom"} align="start" onCloseAutoFocus={e=>{if(inPill)e.preventDefault();}}>
    <DropdownMenuItem data-content-source="workspace" className="content-source-choice" onSelect={()=>onChange('workspace')}><span className="content-source-glyph"><IconFiles/></span>{gui.workspaces_tab}</DropdownMenuItem>
    <DropdownMenuItem data-content-source="browser" className="content-source-choice" onSelect={()=>onChange('browser')}><span className="content-source-glyph"><IconBrowser/></span>{gui.browser_tab}</DropdownMenuItem>
  </DropdownMenuContent></DropdownMenu>;
}
