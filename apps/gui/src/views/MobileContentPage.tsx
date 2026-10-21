/** A session's webpage keeps the browser's address, exit choice and
 * native history. Its one deck carries the owning session identity. */
import {useState,type ReactNode} from 'react';
import {forgetRecentByDevice,said,type DeviceRow} from '@/api';
import {ContentToolbar} from '@/components/ContentToolbar';
import {NewTabPage} from '@/browse/NewTabPage';
import {usePlaces} from '@/browse/use-places';
import {PAGES} from '@/views/PagesKind';
import {PageField} from '@/browse/PageField';
import {GlassCircle} from '@/components/Glass';
import {IconBack,IconClose} from '@/components/icons';
import {PageHistory,PageReload,usePageSteps} from '@/browse/PageControls';
import {gui} from '@/gen/catalog';
import type {ContentTab} from '@/hooks/use-session-content';
import {ContentDocumentBand,ContentTabButton,type DeckOwner} from '@/views/MobileSessionContent';

export function MobileContentPage({tab,handle,device,devices,face,content,navigation,onClose,onPoint,children,desktop=false}: DeckOwner & {
  desktop?:boolean;
  tab:ContentTab;handle:string;device:DeviceRow;devices:DeviceRow[];
  onPoint:(device:DeviceRow,path:string)=>void;children:(onBusy:(busy:boolean)=>void,suspended?:boolean)=>ReactNode;
}) {
  const places=usePlaces(PAGES);
  const [error,setError]=useState('');
  const field=navigation.editor(handle),editing=!!field.editing,draft=editing?field.address??tab.url:tab.url;
  const setEditing=(editing:boolean)=>navigation.edit(handle,{editing,...(editing?{address:field.address??tab.url}:{})});
  const go=(device:DeviceRow,url:string)=>{navigation.edit(handle,{editing:false,address:url});onPoint(device,url);};
  const setDraft=(address:string)=>navigation.edit(handle,{editing:true,address});
  const [busy,setBusy]=useState(false),steps=usePageSteps(handle);
  const [listSeat,setListSeat]=useState<HTMLDivElement|null>(null);
  const browse=(source:'workspace'|'browser')=>{navigation.update(tab.session,{source});navigation.browse(tab.session);};
  const close=()=>{if(!navigation.closePicker(tab.session))onClose();};
  const back=()=>navigation.returnToPicker(tab.session);
  const revisit=(id:string,url:string)=>{const d=devices.find(d=>d.id===id);if(d)go(d,url);else navigation.edit(handle,{editing:true,address:url,exitPrompt:(field.exitPrompt??0)+1});};
  const forget=async(id:string,path:string)=>{try{await forgetRecentByDevice('page',id,path);places.reload();}catch(e){setError(said(e));}};
  return <div className="mobile-content-page">
    <div className="session-content-web" hidden={editing}>{children(setBusy,editing)}</div>
    {editing&&<div data-content-page-landing className="mobile-content-places"><NewTabPage kind={PAGES} places={places} devices={devices} narrow={!desktop} onOpen={revisit} onForget={p=>void forget(p.device,p.place)}/></div>}
    {error&&<p role="alert" className="session-content-error">{error}</p>}
    <div ref={setListSeat} className="mobile-content-page-list"/>
    {desktop?<ContentToolbar kind="page" source="browser" onSource={browse} onBack={back} onClose={close} pinned={tab.pinned}>
      <PageHistory tab={handle} steps={steps}/>
      <PageField inPill suggestAddresses={false} steps={false} tab={handle} machine={device} devices={devices} at={draft} onDraft={setDraft} first={device.id} autoFocus={editing} onEditingChange={setEditing} onLeave={()=>setEditing(false)} onMachine={d=>onPoint(d,draft)} onGo={(url,d)=>go(d,url)}/>
      <PageReload tab={handle} busy={busy}/>
    </ContentToolbar>:<div data-content-deck className="mobile-content-deck">
      {!editing&&<GlassCircle data-content-back aria-label={gui.back} onClick={back}><IconBack/></GlassCircle>}
      <div className="glass mobile-content-pill">
        {!editing&&<PageHistory tab={handle} steps={steps}/>}
        {editing?<div className="mobile-content-page-editor"><PageField suggestAddresses={false} tab={handle} machine={device} devices={devices} at={draft} onDraft={setDraft} first={device.id} steps={false} autoFocus inPill listSeat={listSeat} onLeave={()=>setEditing(false)} onMachine={d=>onPoint(d,draft)} onGo={(url,d)=>go(d,url)}/></div>:<>
          <div className="mobile-page-identity"><ContentDocumentBand session={tab.session} face={face} content={content} navigation={navigation} title={tab.title||tab.url.replace(/^https?:\/\//,'').replace(/\/$/,'')} onTitle={()=>setEditing(true)}/></div>
        </>}
        {!editing&&<PageReload tab={handle} busy={busy}/>}
      </div>
      {editing?<GlassCircle data-content-edit-end aria-label={gui.edit_done} onPointerDown={e=>e.preventDefault()} onClick={()=>{(document.activeElement as HTMLElement|null)?.blur();setEditing(false);}}><IconClose/></GlassCircle>:<ContentTabButton session={tab.session} content={content} navigation={navigation}/>}
    </div>}
  </div>;
}
