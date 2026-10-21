/** Both browsing sources share one leading seat, including their menu. */
import {IconBrowser,IconFiles,IconChevronUp,IconChevronDown} from '@/components/icons';
import {useRef} from 'react';
import {GlassCircle,GlassCapsule,GlassItem} from '@/components/Glass';
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuTrigger} from '@/components/ui/dropdown-menu';
import {gui} from '@/gen/catalog';
import type {ContentSource} from '@/hooks/use-content-navigation';

export function ContentScope({value,onChange,inPill=false,segmented=false}:{value:ContentSource;onChange:(source:ContentSource)=>void;inPill?:boolean;segmented?:boolean}) {
  const buttons=useRef<(HTMLButtonElement|null)[]>([]);
  const sources=['workspace','browser'] as const;
  if(segmented)return <GlassCapsule data-content-scope data-content-scope-nav data-content-scope-segmented role="radiogroup" aria-label={gui.content_scope}>
    {sources.map((source,index)=><GlassItem key={source} ref={el=>{buttons.current[index]=el;}} role="radio" data-content-source={source} aria-label={source==='workspace'?gui.workspaces_tab:gui.browser_tab} aria-checked={source===value} tabIndex={source===value?0:-1} on={source===value} tipSide="top" onClick={()=>onChange(source)} onKeyDown={e=>{
      const next=e.key==='Home'?0:e.key==='End'?1:['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key)?1-index:null;
      if(next===null)return;e.preventDefault();buttons.current[next]?.focus();if(sources[next]!==value)onChange(sources[next]);
    }}>{source==='workspace'?<IconFiles/>:<IconBrowser/>}</GlassItem>)}
  </GlassCapsule>;
  const glyph=value==='workspace'?<IconFiles/>:<IconBrowser/>;
  return <DropdownMenu><DropdownMenuTrigger asChild>
    {inPill?<button type="button" data-band-control data-content-scope data-in-pill className="content-source-inline" aria-label={gui.content_scope}>{glyph}<IconChevronUp centred className="content-source-chevron"/></button>:<GlassCircle className="content-source-desktop" data-band-control data-content-scope aria-label={gui.content_scope}>{glyph}<IconChevronDown centred className="content-source-chevron"/></GlassCircle>}
  </DropdownMenuTrigger><DropdownMenuContent side={inPill?"top":"bottom"} align="start" onCloseAutoFocus={e=>{if(inPill)e.preventDefault();}}>
    <DropdownMenuItem data-content-source="workspace" className="content-source-choice" onSelect={()=>onChange('workspace')}><span className="content-source-glyph"><IconFiles/></span>{gui.workspaces_tab}</DropdownMenuItem>
    <DropdownMenuItem data-content-source="browser" className="content-source-choice" onSelect={()=>onChange('browser')}><span className="content-source-glyph"><IconBrowser/></span>{gui.browser_tab}</DropdownMenuItem>
  </DropdownMenuContent></DropdownMenu>;
}
