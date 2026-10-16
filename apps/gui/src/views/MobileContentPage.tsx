/** A session's webpage keeps the browser's address, exit choice and
 * native history. Its one deck carries the owning session identity. */
import {useEffect,useState,type ReactNode} from 'react';
import {onPageWord,pageStep,type DeviceRow} from '@/api';
import {PageField} from '@/browse/PageField';
import {GlassCircle,GlassItem} from '@/components/Glass';
import {IconBack,IconForward,IconMore,IconReload} from '@/components/icons';
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuSeparator,DropdownMenuTrigger} from '@/components/ui/dropdown-menu';
import {gui} from '@/gen/catalog';
import type {ContentTab} from '@/hooks/use-session-content';
import {ContentOwnerIdentity,ContentTabButton,type DeckOwner} from '@/views/MobileSessionContent';

export function MobileContentPage({tab,handle,device,devices,face,content,navigation,onClose,onPoint,onKeep,children,desktop=false}: DeckOwner & {
  desktop?:boolean;
  tab:ContentTab;handle:string;device:DeviceRow;devices:DeviceRow[];
  onPoint:(device:DeviceRow,path:string)=>void;onKeep:()=>void;children:(onBusy:(busy:boolean)=>void)=>ReactNode;
}) {
  const [editing,setEditing]=useState(false),[busy,setBusy]=useState(false),[steps,setSteps]=useState<{back:boolean;forward:boolean}|null>(null);
  const [listSeat,setListSeat]=useState<HTMLDivElement|null>(null);
  useEffect(()=>onPageWord('page:steps',handle,text=>setSteps({back:text[0]==='1',forward:text[1]==='1'})),[handle]);
  const step=(way:'back'|'forward'|'reload')=>void pageStep(handle,way).catch(()=>{});
  const browse=()=>{navigation.update(tab.session,{source:'browser'});navigation.browse(tab.session);};
  return <div className="mobile-content-page">
    <div className="session-content-web">{children(setBusy)}</div>
    <div ref={setListSeat} className="mobile-content-page-list"/>
    {desktop?<div data-content-page-bar className="session-content-page-bar">
      <GlassCircle aria-label={gui.back} onClick={()=>{if(!navigation.back(tab.session))onClose();}}><IconBack/></GlassCircle>
      <PageField tab={handle} machine={device} devices={devices} at={tab.url} first={device.id} onMachine={d=>onPoint(d,tab.url)} onGo={(url,d)=>onPoint(d,url)}/>
      <DropdownMenu><DropdownMenuTrigger asChild><GlassCircle data-content-page-more aria-label={gui.rail_more}><IconMore/></GlassCircle></DropdownMenuTrigger><DropdownMenuContent className="[&_svg]:size-nav-glyph" align="end">
        {content.isPreview(tab)&&<DropdownMenuItem onSelect={onKeep}>{gui.content_keep}</DropdownMenuItem>}
        <DropdownMenuItem onSelect={browse}>{gui.content_find}</DropdownMenuItem>
      </DropdownMenuContent></DropdownMenu>
    </div>:<div data-content-deck className="mobile-content-deck">
      <GlassCircle aria-label={gui.back} onClick={()=>{if(!navigation.back(tab.session))onClose();}}><IconBack/></GlassCircle>
      <div className="glass mobile-content-pill">
        {editing?<div className="mobile-content-page-editor"><ContentOwnerIdentity face={face}/><PageField tab={handle} machine={device} devices={devices} at={tab.url} first={device.id} steps={false} autoFocus inPill listSeat={listSeat} onLeave={()=>setEditing(false)} onMachine={d=>onPoint(d,tab.url)} onGo={(url,d)=>{onPoint(d,url);setEditing(false);}}/></div>:<>
          <GlassItem data-page-history-back aria-label={gui.page_back} disabled={steps?.back===false} onClick={()=>step('back')}><IconBack/></GlassItem>
          <button className="mobile-file-identity" aria-label={gui.page_address} onClick={()=>setEditing(true)}><ContentOwnerIdentity face={face} title={tab.url.replace(/^https?:\/\//,'').replace(/\/$/,'')}/></button>
          <GlassItem aria-label={gui.page_reload} aria-busy={busy} onClick={()=>step('reload')}><IconReload/></GlassItem>
        </>}
      </div>
      <DropdownMenu><DropdownMenuTrigger asChild><GlassCircle data-content-page-more aria-label={gui.rail_more}><IconMore/></GlassCircle></DropdownMenuTrigger><DropdownMenuContent className="[&_svg]:size-nav-glyph" side="top" align="end">
        {content.isPreview(tab)&&<DropdownMenuItem onSelect={onKeep}>{gui.content_keep}</DropdownMenuItem>}
        <DropdownMenuItem disabled={steps?.back===false} onSelect={()=>step('back')}><IconBack/>{gui.page_back}</DropdownMenuItem>
        <DropdownMenuItem disabled={steps?.forward===false} onSelect={()=>step('forward')}><IconForward/>{gui.page_forward}</DropdownMenuItem>
        <DropdownMenuSeparator/><DropdownMenuItem onSelect={browse}>{gui.content_find}</DropdownMenuItem>
      </DropdownMenuContent></DropdownMenu>
      <ContentTabButton session={tab.session} content={content} navigation={navigation}/>
    </div>}
  </div>;
}
