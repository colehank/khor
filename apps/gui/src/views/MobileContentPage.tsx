/** A session's webpage keeps the browser's address, exit choice and
 * native history. Its one deck carries the owning session identity. */
import {useEffect,useState,type ReactNode} from 'react';
import {onPageWord,pageStep,type DeviceRow} from '@/api';
import {ContentScope} from '@/components/ContentScope';
import {PageField} from '@/browse/PageField';
import {GlassCircle} from '@/components/Glass';
import {IconBack,IconClose,IconForward,IconMore,IconReload} from '@/components/icons';
import {DropdownMenu,DropdownMenuContent,DropdownMenuItem,DropdownMenuTrigger} from '@/components/ui/dropdown-menu';
import {gui} from '@/gen/catalog';
import type {ContentTab} from '@/hooks/use-session-content';
import {ContentDocumentBand,ContentTabButton,type DeckOwner} from '@/views/MobileSessionContent';

export function MobileContentPage({tab,handle,device,devices,face,content,navigation,onClose,onPoint,children,desktop=false}: DeckOwner & {
  desktop?:boolean;
  tab:ContentTab;handle:string;device:DeviceRow;devices:DeviceRow[];
  onPoint:(device:DeviceRow,path:string)=>void;children:(onBusy:(busy:boolean)=>void)=>ReactNode;
}) {
  const field=navigation.editor(handle),editing=!!field.editing,draft=editing?field.address??tab.url:tab.url;
  const setEditing=(editing:boolean)=>navigation.edit(handle,{editing,...(editing?{address:field.address??tab.url}:{})});
  const go=(device:DeviceRow,url:string)=>{navigation.edit(handle,{editing:false,address:url});onPoint(device,url);};
  const setDraft=(address:string)=>navigation.edit(handle,{editing:true,address});
  const [busy,setBusy]=useState(false),[steps,setSteps]=useState<{back:boolean;forward:boolean}|null>(null);
  const [listSeat,setListSeat]=useState<HTMLDivElement|null>(null);
  useEffect(()=>onPageWord('page:steps',handle,text=>setSteps({back:text[0]==='1',forward:text[1]==='1'})),[handle]);
  const step=(way:'back'|'forward'|'reload')=>void pageStep(handle,way).catch(()=>{});
  const browse=(source:'workspace'|'browser')=>{navigation.update(tab.session,{source});navigation.browse(tab.session);};
  const close=()=>{if(!navigation.closePicker(tab.session))onClose();};
  return <div className="mobile-content-page">
    <div className="session-content-web">{children(setBusy)}</div>
    <div ref={setListSeat} className="mobile-content-page-list"/>
    {desktop?<div data-content-page-bar className="session-content-page-bar">
      <ContentScope value="browser" onChange={browse}/>
      <PageField steps={false} tab={handle} machine={device} devices={devices} at={draft} onDraft={setDraft} first={device.id} autoFocus={editing} onEditingChange={setEditing} onMachine={d=>onPoint(d,draft)} onGo={(url,d)=>go(d,url)}/>
      <DropdownMenu><DropdownMenuTrigger asChild><GlassCircle data-content-page-more aria-label={gui.rail_more}><IconMore/></GlassCircle></DropdownMenuTrigger><DropdownMenuContent className="[&_svg]:size-nav-glyph" align="end">
        <DropdownMenuItem disabled={steps?.back===false} onSelect={()=>step('back')}><IconBack/>{gui.page_back}</DropdownMenuItem>
        <DropdownMenuItem disabled={steps?.forward===false} onSelect={()=>step('forward')}><IconForward/>{gui.page_forward}</DropdownMenuItem>
        <DropdownMenuItem aria-busy={busy} onSelect={()=>step('reload')}><IconReload/>{gui.page_reload}</DropdownMenuItem>
      </DropdownMenuContent></DropdownMenu>
      <GlassCircle data-content-close disabled={tab.pinned} aria-label={gui.close_tab} onClick={close}><IconClose/></GlassCircle>
    </div>:<div data-content-deck className="mobile-content-deck">
      <div className="glass mobile-content-pill">
        {editing?<div className="mobile-content-page-editor"><PageField tab={handle} machine={device} devices={devices} at={draft} onDraft={setDraft} first={device.id} steps={false} autoFocus inPill listSeat={listSeat} onLeave={()=>setEditing(false)} onMachine={d=>onPoint(d,draft)} onGo={(url,d)=>go(d,url)}/></div>:<>
          <div className="mobile-page-identity"><ContentDocumentBand session={tab.session} face={face} content={content} navigation={navigation} title={tab.title||tab.url.replace(/^https?:\/\//,'').replace(/\/$/,'')} onTitle={()=>setEditing(true)}/></div>
        </>}
      </div>
      {!editing&&<DropdownMenu><DropdownMenuTrigger asChild><GlassCircle data-content-page-more aria-label={gui.rail_more}><IconMore/></GlassCircle></DropdownMenuTrigger><DropdownMenuContent className="[&_svg]:size-nav-glyph" side="top" align="end">
        <DropdownMenuItem disabled={steps?.back===false} onSelect={()=>step('back')}><IconBack/>{gui.page_back}</DropdownMenuItem>
        <DropdownMenuItem disabled={steps?.forward===false} onSelect={()=>step('forward')}><IconForward/>{gui.page_forward}</DropdownMenuItem>
        <DropdownMenuItem aria-busy={busy} onSelect={()=>step('reload')}><IconReload/>{gui.page_reload}</DropdownMenuItem>
        <DropdownMenuItem disabled={tab.pinned} onSelect={close}><IconClose/>{gui.close_tab}</DropdownMenuItem>
      </DropdownMenuContent></DropdownMenu>}
      {editing?<GlassCircle data-content-edit-end aria-label={gui.edit_done} onPointerDown={e=>e.preventDefault()} onClick={()=>{(document.activeElement as HTMLElement|null)?.blur();setEditing(false);}}><IconClose/></GlassCircle>:<ContentTabButton session={tab.session} content={content} navigation={navigation}/>}
    </div>}
  </div>;
}
