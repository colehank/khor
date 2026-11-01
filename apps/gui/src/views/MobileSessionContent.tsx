import { readCommitAddress } from '@/lib/content-commit';
import type { GitCommitSummary } from '@/gen/bindings/GitCommitSummary';
/** The mobile session's current content owns one bottom deck. The
 * terminal's measured deck room is retained behind the sheet. */
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { fetchLs, forgetRecentByDevice, said, type DeviceRow, type SessionRow } from '@/api';
import { PageField } from '@/browse/PageField';
import { NewTabPage } from '@/browse/NewTabPage';
import { usePlaces } from '@/browse/use-places';
import { PAGES } from '@/views/PagesKind';
import {ContentToolbar} from '@/components/ContentToolbar';
import { ContentScope } from '@/components/ContentScope';
import { DeckBand, TabSheetRows } from '@/components/SessionBar';
import { SessionFace, fileKind, pageKind, newTabKind, historyKind } from '@/components/SessionFace';
import { GlassCircle, GlassCapsule, GlassItem } from '@/components/Glass';
import { IconBack, IconClose, IconPlus, IconSearch } from '@/components/icons';
import { Input } from '@/components/ui/input';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { ContentPathField } from '@/views/ContentPathField';
import { gui } from '@/gen/catalog';
import type { ContentNavigation, ContentSource } from '@/hooks/use-content-navigation';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
import { useContentSheet } from '@/hooks/use-content-sheet';
import { directoryOf } from '@/lib/paths';
import { imeKey } from '@/lib/ime';
import { useFieldDismiss } from '@/hooks/use-field-dismiss';
import { sessionCwdOnScreen } from '@/lib/session-cwd';
import { SideWorkspace } from '@/views/SideWorkspace';
import type { PlaceDetent } from '@/views/SessionPlaceSheet';
import { contentName, contentView, contentTabTargets } from '@/lib/content';
import { TabAvatar, type TabFace } from '@/browse/TabStrip';

const quiet = (work: Promise<unknown>) => { void work.catch(() => {}); };
const leaf = (path: string) => path.replace(/[\\/]$/, '').split(/[\\/]/).pop() || path;
export function contentContext(tab:ContentTab,row:SessionRow|undefined,devices:DeviceRow[]) {
  if(contentView(tab)==='page')return tab.url.replace(/^https?:\/\//,'').replace(/\/$/,'');
  const root=sessionCwdOnScreen(row),dir=contentView(tab)==='commit'?readCommitAddress(tab.url)?.root||'':contentView(tab)==='file'?directoryOf(tab.url):tab.url;
  const folder=root&&(dir===root||dir.startsWith(root+'/')||dir.startsWith(root+'\\'))?leaf(root)+dir.slice(root.length):dir;
  return [devices.find(d=>d.id===tab.device)?.name,folder].filter(Boolean).join(' · ');
}
/** One content identity for the desktop strip and mobile tab overview. */
export function contentTabFace(tab:ContentTab,devices:DeviceRow[],iconOf:(url:string)=>string|undefined):TabFace {
  return {id:tab.id,label:contentView(tab)==='workspace'?gui.content_new:contentName(tab),path:tab.url,face:contentView(tab)==='workspace'?null:devices.find(d=>d.id===tab.device)?.face||null,
    mark:contentView(tab)==='workspace'?newTabKind():contentView(tab)==='file'?fileKind(tab.url):contentView(tab)==='commit'?historyKind():pageKind(iconOf(tab.url))};
}
export type DeckOwner={face:TabFace;content:SessionContent;navigation:ContentNavigation;onClose:()=>void};
export function ContentOwnerIdentity({face,title,children,editing=false}: {face:TabFace;title?:string;children?:ReactNode;editing?:boolean}) {
  return <span className="content-owner-identity" data-content-editing={editing||undefined}><span className="content-owner-face">{face.session&&<SessionFace face={face.face} {...face.session} size="tab"/>}</span><span className="content-owner-name">{face.label}</span><span className="content-owner-slash">/</span>{children||<span className="content-owner-title">{title}</span>}</span>;
}
/** The nested pill uses the terminal band's exact threshold, flick and cancellation. */
export function ContentDocumentBand({session,face,content,navigation,title,onTitle}: Omit<DeckOwner,'onClose'> & {session:string;title:string;onTitle:()=>void}) {
  const tabs=content.allTabs.filter(t=>t.session===session);
  const active=navigation.get(session).key;
  const targets=contentTabTargets(tabs,content.handle);
  const faces=[...targets].map(([key,t])=>({...face,session:undefined,face:contentView(t)==='workspace'?null:face.face,hideFace:false,mark:contentView(t)==='workspace'?newTabKind():contentView(t)==='file'?fileKind(t.url):contentView(t)==='commit'?historyKind():pageKind(content.pageIcon(t.url)),id:key,label:contentView(t)==='workspace'?gui.content_new:contentName(t)}));
  return <DeckBand faces={faces} active={active||''} onFocus={key=>{const tab=targets.get(key);if(tab){navigation.select(tab);quiet(content.focus(tab.id));}}} onTitle={onTitle} onOverview={()=>navigation.overview(session)} renderFace={(item,role)=>item.mark?.key==='new-tab'||role==='cur'&&navigation.get(session).scene==='picker'?<span className="content-new-identity"><span className="content-owner-face"><TabAvatar f={{...item,face:null,mark:newTabKind()}} bare="size-tab-face" seat="tab"/></span><span className="content-new-title">{gui.content_new}</span></span>:<ContentOwnerIdentity face={face} title={role==='cur'?title:item.label}/>}/>;
}
export function MobileContentDeck({session,navigation,onClose,children,editing=false,onEditEnd,leading,actions}: DeckOwner & {session:string;children:ReactNode;editing?:boolean;onEditEnd?:()=>void;leading?:ReactNode;actions?:ReactNode}) {
  return <div className="mobile-content-deck" data-content-deck data-content-editing={editing||undefined}>
    {!editing&&leading}
    <div className="glass mobile-content-pill">{children}</div>
    {editing?<GlassCircle data-content-edit-end aria-label={gui.edit_done} onPointerDown={e=>e.preventDefault()} onClick={onEditEnd}><IconClose/></GlassCircle>:<>{actions}{navigation.get(session).scene==='overview'&&<GlassCircle aria-label={gui.side_hide} onClick={onClose}><IconClose/></GlassCircle>}</>}
  </div>;
}
function ContentFileIdentity({session,face,content,navigation,title,devices,tab}: DeckOwner & {session:string;title:string;devices:DeviceRow[];tab:ContentTab}) {
 const [editing,setEditing]=useState(false),device=devices.find(d=>d.id===tab.device);
 return editing&&device?<ContentPathField device={device} devices={devices} path={tab.url} base={directoryOf(tab.url)} mobile onLeave={()=>setEditing(false)} onDirectory={(d,path)=>{setEditing(false);navigation.browse(session,{device:d.id,root:path});}} onFile={(d,path)=>{setEditing(false);quiet(content.point(tab.id,d.id,path,'file'));}}/>:<ContentDocumentBand session={session} face={face} content={content} navigation={navigation} title={title} onTitle={()=>setEditing(true)}/>;
}
export function contentFileDeck(props: DeckOwner & {session:string;title:string;devices:DeviceRow[];tab:ContentTab}) {
 const {face,title}=props;
 return {identity:<div className="mobile-file-identity"><ContentFileIdentity {...props}/></div>,folded:<span className="mobile-fold-identity"><ContentOwnerIdentity face={face} title={title}/></span>};
}
/** Search has one clear button inside and an independent end-editing control outside. */
export function ContentSearch({label,value,onChange,onEditing,onEnter,onDismiss}: {label:string;value:string;onChange:(value:string)=>void;onEditing:(editing:boolean)=>void;onEnter?:()=>void;onDismiss?:()=>void}) {
  const input=useRef<HTMLInputElement>(null),region=useRef<HTMLDivElement>(null);
  useFieldDismiss(onDismiss!==undefined,()=>[region.current],()=>onDismiss?.());
  return <div ref={region} className="content-search-field"><IconSearch/><Input ref={input} autoFocus type="search" aria-label={label} placeholder={label} value={value} onFocus={()=>onEditing(true)} onBlur={()=>onEditing(false)} onChange={e=>onChange(e.target.value)} onKeyDown={e=>{if(imeKey(e))return;if(e.key==='Enter')onEnter?.();if(e.key==='Escape'){e.preventDefault();e.stopPropagation();input.current?.blur();onDismiss?.();}}}/>{value&&<button data-content-search-clear aria-label={gui.search_clear} onPointerDown={e=>e.preventDefault()} onClick={()=>{onChange('');input.current?.focus();}}><IconClose/></button>}</div>;
}
/** One location toolbar for both sizes; the content below keeps its own scroll. */
export function ContentPicker({row,devices,navigation,onOpen,onWorkspaceFile,onOpenCommit,onFilesMoved,onFilesGone,face,content,onClose,desktop=false,active=true}: DeckOwner & {
 active?:boolean;desktop?:boolean;row:SessionRow;devices:DeviceRow[];
 onFilesMoved?:(device:DeviceRow,from:string,to:string)=>void;onFilesGone?:(device:DeviceRow,path:string)=>void;
 onOpen:(device:DeviceRow,path:string,view:'file'|'page',line?:number|null)=>void;
 onWorkspaceFile:(device:DeviceRow,path:string,line:number|null)=>void;
 onOpenCommit:(device:DeviceRow,root:string,commit:GitCommitSummary)=>void;
}) {
 const state=navigation.get(row.id),session=row.id;
 const home=devices.find(d=>d.name===row.home),device=state.scope?devices.find(d=>d.id===state.scope!.device):home;
 const editDevice=device||home||devices.find(d=>d.me)||devices[0];
 const root=state.scope?.root||sessionCwdOnScreen(row)||'';
 const canReturnWorkspace=state.source==='workspace'&&!!home&&!!state.scope&&(state.scope.device!==home.id||state.scope.root!==sessionCwdOnScreen(row));
 const exit=devices.find(d=>d.id===state.exit)||device;
 const fieldKey='picker:'+session+':'+(state.key||'legacy'),field=navigation.editor(fieldKey),editing=!!field.editing||field.exitPrompt!==undefined;
 const [history,setHistory]=useState(false),[historyQuery,setHistoryQuery]=useState('');
 useEffect(()=>{setHistory(false);setHistoryQuery('');},[device?.id,root]);
 const [searching,setSearching]=useState(false),[make,setMake]=useState<{kind:'file'|'dir';at:number}>(),[error,setError]=useState<string|null>(null);
 const pending=useRef<'file'|'dir'|null>(null);
 const [listSeat,setListSeat]=useState<HTMLDivElement|null>(null);
 const places=usePlaces(PAGES);
 const edit=(editing:boolean)=>navigation.edit(fieldKey,{editing});
 const endEdit=()=>{const active=document.activeElement;if(active instanceof HTMLElement&&active.matches('input,textarea,[contenteditable="true"]'))active.blur();navigation.edit(fieldKey,{editing:false,exitPrompt:undefined});};
 const directory=(d:DeviceRow,path:string)=>{stopSearch();navigation.update(session,{scope:{device:d.id,root:path},query:''});endEdit();};
 const open=(d:DeviceRow,path:string,view:'file'|'page',line?:number|null)=>{stopSearch();endEdit();onOpen(d,path,view,line);};
 const source=(source:ContentSource)=>{stopSearch();endEdit();navigation.update(session,{source});};
 const setQuery=(query:string)=>history?setHistoryQuery(query):navigation.update(session,{query});
 const stopSearch=()=>{(document.activeElement as HTMLElement|null)?.blur();setSearching(false);setQuery('');};
 const returnWorkspace=()=>{stopSearch();endEdit();navigation.update(session,{scope:null,source:'workspace',query:''});};
 const scope=<ContentScope value={state.source} onChange={source} inPill={!desktop} segmented={desktop}/>;
 const newAction=<DropdownMenu><DropdownMenuTrigger asChild><GlassItem data-content-directory-new aria-label={gui.new_item}><IconPlus/></GlassItem></DropdownMenuTrigger><DropdownMenuContent side={desktop?'bottom':'top'} align="end" onCloseAutoFocus={e=>{const kind=pending.current;if(!kind)return;e.preventDefault();pending.current=null;setQuery('');setSearching(false);setHistory(false);setMake({kind,at:Date.now()});}}><DropdownMenuItem onSelect={()=>{pending.current='file';}}>{gui.file_new}</DropdownMenuItem><DropdownMenuItem onSelect={()=>{pending.current='dir';}}>{gui.dir_new}</DropdownMenuItem></DropdownMenuContent></DropdownMenu>;
 const directoryActions=state.source==='workspace'&&<GlassCapsule role="group" aria-label={gui.content_workspace_actions}><GlassItem data-content-directory-search aria-label={searching?gui.search_clear:gui.tree_search} onClick={()=>{if(searching)stopSearch();else{edit(false);setSearching(true);}}}>{searching?<IconClose/>:<IconSearch/>}</GlassItem>{newAction}</GlassCapsule>;
 const address=exit&&<PageField suggestAddresses={false} tab={fieldKey} machine={exit} devices={devices} at={state.address} onDraft={address=>navigation.update(session,{address})} first={home?.id} machinePrompt={field.exitPrompt} steps={false} autoFocus={!desktop&&editing} inPill listSeat={desktop?undefined:listSeat} onLeave={()=>edit(false)} onEditingChange={edit} onMachine={d=>{navigation.update(session,{exit:d.id});if(field.exitPrompt!==undefined)open(d,navigation.get(session).address,'page');}} onGo={(url,d)=>{navigation.update(session,{exit:d.id,address:url});open(d,url,'page');}}/>;
 const location=state.source==='browser'?address:editDevice&&<ContentPathField key={editDevice.id+root} device={editDevice} devices={devices} path={device?root:''} mobile={!desktop} onDirectory={directory} onFile={(d,path)=>open(d,path,'file')} onLeave={endEdit}/>;
 const search=<div data-content-directory-search-field><ContentSearch label={history?gui.git_history_search_page:gui.tree_search} value={history?historyQuery:state.query} onChange={setQuery} onEditing={()=>{}} onDismiss={stopSearch}/></div>;
 const fieldBody=searching?search:location;
 const close=()=>{stopSearch();if(!navigation.closePicker(session))onClose();};
 const revisit=(id:string,url:string)=>{const d=devices.find(d=>d.id===id);if(d)open(d,url,'page');else{navigation.update(session,{source:'browser',address:url});navigation.edit(fieldKey,{editing:true,exitPrompt:(field.exitPrompt??0)+1});}};
 const forget=async(id:string,path:string)=>{try{await forgetRecentByDevice('page',id,path);places.reload();}catch(e){setError(said(e));}};
 return <div className="mobile-content-picker" data-desktop-picker={desktop||undefined}>
   {desktop&&<ContentToolbar kind="picker" onBack={canReturnWorkspace?returnWorkspace:undefined} source={state.source} onSource={source} actions={directoryActions} pinned={content.allTabs.find(t=>content.handle(t)===state.key)?.pinned} onClose={close}>{state.source==='workspace'&&!editing&&!searching?<button data-content-location-edit className="content-location-rest" onClick={()=>edit(true)}><span className="content-location-face">{device&&<TabAvatar f={{id:device.id,label:device.shown,path:root,face:device.face}} bare="size-tab-face" seat="tab"/>}</span><span>{root||'~/'}</span></button>:fieldBody}</ContentToolbar>}
   {!desktop&&canReturnWorkspace&&<div className="flex shrink-0 px-row py-in"><GlassCircle data-content-workspace-return aria-label={gui.back} tip={sessionCwdOnScreen(row)||undefined} onClick={returnWorkspace}><IconBack/></GlassCircle></div>}
   {error&&<p role="alert" className="session-content-error">{error}</p>}
   <div className="mobile-content-workspace" hidden={state.source!=='workspace'}>{device?<SideWorkspace active={active&&state.source==='workspace'} hideFind key={device.id+' '+root} machine={device.name} here={device.me} root={root||'~'} historyView={history} onHistoryView={setHistory} query={history?historyQuery:state.query} onQuery={setQuery} makeRequest={make} onContentMoved={(from,to)=>onFilesMoved?.(device,from,to)} onContentGone={path=>onFilesGone?.(device,path)} onOpenCommit={commit=>onOpenCommit(device,root||'~',commit)} onOpenContent={(path,line)=>onWorkspaceFile(device,path,line)}/>:<div className="session-content-unknown">{gui.content_machine_gone}</div>}</div>
   <div className="mobile-content-browser" hidden={state.source!=='browser'}><div className="mobile-content-places"><NewTabPage kind={PAGES} places={places} devices={devices} narrow onOpen={revisit} onForget={p=>void forget(p.device,p.place)}/></div></div>
   {!desktop&&<div ref={setListSeat} className="mobile-content-page-list"/>}
   {!desktop&&<MobileContentDeck session={session} face={face} content={content} navigation={navigation} onClose={onClose} editing={editing} onEditEnd={endEdit} leading={searching?<GlassCircle aria-label={gui.back} onClick={stopSearch}><IconBack/></GlassCircle>:<div className="glass content-source-capsule">{scope}</div>} actions={searching?undefined:directoryActions}>{editing||searching?fieldBody:<div className="mobile-file-identity"><ContentDocumentBand session={session} face={face} content={content} navigation={navigation} title={gui.content_new} onTitle={()=>edit(true)}/></div>}</MobileContentDeck>}
 </div>;
}

export function useContentNeighbors(tab:ContentTab,machine:string,enabled=true) {
  const [near,setNear]=useState<{key:string;files:string[]}>({key:'',files:[]});
  const key=tab.device+' '+tab.url;
  useEffect(()=>{
    let live=true;if(!enabled||contentView(tab)!=='file')return;
    const dir=directoryOf(tab.url),sep=dir.endsWith('/')||dir.endsWith('\\')?'':dir.includes('\\')?'\\':'/';
    fetchLs(machine,dir).then(ls=>{if(live)setNear({key,files:ls.entries.filter(e=>!e.dir).map(e=>dir+sep+e.name)});},()=>{if(live)setNear({key,files:[]});});
    return()=>{live=false;};
  },[key,machine,enabled]);
  const files=near.key===key?near.files:[],at=files.indexOf(tab.url);
  return {files,at};
}

export function ContentOverview({session,row,devices,content,navigation,onPick,face,onClose}: DeckOwner & {session:string;row:SessionRow|undefined;devices:DeviceRow[];onPick:(tab:ContentTab)=>void}) {
  const state=navigation.get(session),q=state.filter.toLocaleLowerCase();
  const [searching,setSearching]=useState(false);
  const tabs=content.allTabs.filter(t=>t.session===session&&(contentName(t)+' '+t.url+' '+contentContext(t,row,devices)).toLocaleLowerCase().includes(q));
  const tabFaces=tabs.map(t=>({...contentTabFace(t,devices,content.pageIcon),...(contentView(t)==='workspace'?{label:gui.content_new}:{})}));
  const current=content.allTabs.find(t=>t.session===session&&content.handle(t)===(state.recent||state.key));
  return <div className="mobile-content-overview">
    <div className="mobile-content-overview-list">
      {!tabs.length&&<p className="mobile-content-empty">{q?gui.content_no_match:gui.content_empty}</p>}
      <TabSheetRows faces={tabFaces} active={current?.id||''} onFocus={id=>{const tab=tabs.find(t=>t.id===id);if(tab)onPick(tab);}} onClose={id=>quiet(content.close(id))} canClose={f=>!tabs.find(t=>t.id===f.id)?.pinned} rowData={f=>({'data-content-overview-tab':f.id})} onNew={()=>navigation.add(session)} newLabel={gui.content_new}/>
    </div>
    <MobileContentDeck session={session} face={face} content={content} navigation={navigation} onClose={onClose} editing={searching} onEditEnd={()=>{(document.activeElement as HTMLElement|null)?.blur();setSearching(false);}}>{searching?<ContentSearch label={gui.content_tabs_search} value={state.filter} onChange={filter=>navigation.update(session,{filter})} onEditing={setSearching}/>:<button data-content-overview-search className="content-search-prompt" onClick={()=>setSearching(true)}><IconSearch/>{state.filter||gui.content_tabs_search}</button>}</MobileContentDeck>
  </div>;
}
export function ContentSessionOverview({faces,active,onFocus,onCloseSession,onNew,onClose,query,onQuery}: {faces:TabFace[];active:string;onFocus:(id:string)=>void;onCloseSession:(id:string)=>void;onNew:()=>void;onClose:()=>void;query:string;onQuery:(q:string)=>void}) {
  const [editing,setEditing]=useState(false);
  const q=query.trim().toLocaleLowerCase(),shown=faces.filter(f=>(f.title||f.label).toLocaleLowerCase().includes(q));
  return <div className="mobile-content-overview"><div className="mobile-content-overview-list"><TabSheetRows faces={shown} active={active} onFocus={onFocus} onClose={onCloseSession} onNew={onNew} newLabel={gui.new_session}/>{!shown.length&&<p className="mobile-content-empty">{gui.content_no_match}</p>}</div><div data-content-deck className="mobile-content-deck" data-content-editing={editing||undefined}><div className="glass mobile-content-pill">{editing?<ContentSearch label={gui.search_sessions} value={query} onChange={onQuery} onEditing={setEditing}/>:<button data-session-overview-search className="content-search-prompt" onClick={()=>setEditing(true)}><IconSearch/>{query||gui.search_sessions}</button>}</div>{editing?<GlassCircle aria-label={gui.edit_done} onPointerDown={e=>e.preventDefault()} onClick={()=>{(document.activeElement as HTMLElement|null)?.blur();setEditing(false);}}><IconClose/></GlassCircle>:<GlassCircle aria-label={gui.side_hide} onClick={onClose}><IconClose/></GlassCircle>}</div></div>;
}
/** In-place sheet shared by content and session overviews. Its deck is
 * inside the current page; the retained terminal never changes geometry. */
export function MobileContentSheet({children,onClose}: {children:ReactNode;onClose:()=>void}) {
  const [el,setEl]=useState<HTMLDivElement|null>(null),[detent,setDetent]=useState<PlaceDetent>('half');
  const overlay=useRef<HTMLDivElement>(null);
  const screen=()=>overlay.current?.closest<HTMLElement>('[data-session-screen]');
  useContentSheet(el,setDetent,onClose,()=>{
    const host=screen(),top=host?.querySelector<HTMLElement>('[data-session-top]');
    const gap=parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--gap-in'))||0;
    return {height:host?.clientHeight??innerHeight,foot:0,top:(top?.getBoundingClientRect().height||0)+gap};
  });
  useLayoutEffect(()=>{const host=screen();if(!host)return;const read=()=>overlay.current?.style.setProperty('--mobile-content-height',host.clientHeight+'px');read();const observer=new ResizeObserver(read);observer.observe(host);return()=>observer.disconnect();},[]);
  // Content is portaled into this DOM seat; React events bubble through
  // the portal's owner, so listen on the physical sheet for input focus.
  useEffect(()=>{
    if(!el)return;
    const expand=(e:Event)=>{if(e.target instanceof Element&&e.target.closest('input,textarea,[contenteditable=true]')&&!e.target.closest('[data-content-directory-search-field]'))setDetent('full');};
    el.addEventListener('focusin',expand);el.addEventListener('click',expand);
    return()=>{el.removeEventListener('focusin',expand);el.removeEventListener('click',expand);};
  },[el]);
  useEffect(()=>{
    const modal=()=>[...document.querySelectorAll<HTMLElement>('[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]')].some(n=>n.getClientRects().length&&getComputedStyle(n).visibility!=='hidden');
    const escape=(e:KeyboardEvent)=>{
      if(e.key!=='Escape'||e.defaultPrevented||e.isComposing||modal())return;
      e.preventDefault();onClose();
    };
    // The retained terminal can regain focus while its deck is hidden.
    // Intercept its Escape before xterm consumes it or sends it to the PTY.
    const terminalEscape=(e:KeyboardEvent)=>{
      if(e.key==='Escape'&&!e.isComposing&&!modal()&&e.target instanceof Element&&e.target.closest('[data-terminal]')){
        e.preventDefault();e.stopPropagation();onClose();
      }
    };
    document.addEventListener('keydown',terminalEscape,true);
    document.addEventListener('keydown',escape);
    return()=>{document.removeEventListener('keydown',terminalEscape,true);document.removeEventListener('keydown',escape);};
  },[onClose]);
  return <div ref={overlay} data-mobile-content-sheet className="mobile-content-overlay"><button className="mobile-content-scrim" aria-label={gui.side_hide} onClick={onClose}/><div ref={setEl} data-detent={detent} data-content-panel className="mobile-content-sheet"><button data-place-grab className="mobile-content-grab" aria-label={detent==='half'?gui.content_expand_sheet:gui.content_collapse_sheet} onClick={()=>setDetent(detent==='half'?'full':'half')}><span/></button>{children}</div></div>;
}
