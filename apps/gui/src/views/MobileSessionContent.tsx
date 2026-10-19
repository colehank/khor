/** The mobile session's current content owns one bottom deck. The
 * terminal's measured deck room is retained behind the sheet. */
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { fetchLs, searchIn, forgetRecentByDevice, said, type DeviceRow, type SessionRow } from '@/api';
import { PageField } from '@/browse/PageField';
import { NewTabPage } from '@/browse/NewTabPage';
import { usePlaces } from '@/browse/use-places';
import { PAGES } from '@/views/PagesKind';
import { ContentScope } from '@/components/ContentScope';
import { ContentTabsGlyph, DeckBand, TabsGlyph, TabSheetRows } from '@/components/SessionBar';
import { SessionFace, fileKind, pageKind, workspaceKind } from '@/components/SessionFace';
import { GlassCircle } from '@/components/Glass';
import { IconBack, IconClose, IconMore, IconSearch } from '@/components/icons';
import { Input } from '@/components/ui/input';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { SideFind } from '@/components/SideFind';
import { gui } from '@/gen/catalog';
import type { ContentNavigation, ContentSource } from '@/hooks/use-content-navigation';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
import { useContentSheet } from '@/hooks/use-content-sheet';
import { directoryOf } from '@/lib/paths';
import { imeKey } from '@/lib/ime';
import { sessionCwdOnScreen } from '@/lib/session-cwd';
import { SideWorkspace } from '@/views/SideWorkspace';
import type { PlaceDetent } from '@/views/SessionPlaceSheet';
import { contentName, contentView } from '@/lib/content';
import { TabAvatar, type TabFace } from '@/browse/TabStrip';

const quiet = (work: Promise<unknown>) => { void work.catch(() => {}); };
const leaf = (path: string) => path.replace(/[\\/]$/, '').split(/[\\/]/).pop() || path;
export function contentContext(tab:ContentTab,row:SessionRow|undefined,devices:DeviceRow[]) {
  if(contentView(tab)==='page')return tab.url.replace(/^https?:\/\//,'').replace(/\/$/,'');
  const root=sessionCwdOnScreen(row),dir=contentView(tab)==='file'?directoryOf(tab.url):tab.url;
  const folder=root&&(dir===root||dir.startsWith(root+'/')||dir.startsWith(root+'\\'))?leaf(root)+dir.slice(root.length):dir;
  return [devices.find(d=>d.id===tab.device)?.name,folder].filter(Boolean).join(' · ');
}
/** One content identity for the desktop strip and mobile tab overview. */
export function contentTabFace(tab:ContentTab,devices:DeviceRow[],iconOf:(url:string)=>string|undefined):TabFace {
  return {id:tab.id,label:contentName(tab),path:tab.url,...(contentView(tab)==='workspace'?{hideFace:true}:{}),face:devices.find(d=>d.id===tab.device)?.face||null,
    mark:contentView(tab)==='file'?fileKind(tab.url):pageKind(iconOf(tab.url))};
}
export type DeckOwner={face:TabFace;content:SessionContent;navigation:ContentNavigation;onClose:()=>void};
export function ContentOwnerIdentity({face,title,children,editing=false}: {face:TabFace;title?:string;children?:ReactNode;editing?:boolean}) {
  return <span className="content-owner-identity" data-content-editing={editing||undefined}><span className="content-owner-face">{face.session&&<SessionFace face={face.face} {...face.session} size="tab"/>}</span><span className="content-owner-name">{face.label}</span><span className="content-owner-slash">/</span>{children||<span className="content-owner-title">{title}</span>}</span>;
}
export function ContentTabButton({session,content,navigation}: Pick<DeckOwner,'content'|'navigation'> & {session:string}) {
  const count=content.allTabs.filter(t=>t.session===session).length;
  return <GlassCircle data-content-overview-button aria-label={gui.content_tabs_count(count)} aria-expanded={navigation.get(session).scene==='overview'} onClick={()=>navigation.overview(session)}><ContentTabsGlyph n={count}/></GlassCircle>;
}
/** The nested pill uses the terminal band's exact threshold, flick and cancellation. */
export function ContentDocumentBand({session,face,content,navigation,title,onTitle}: Omit<DeckOwner,'onClose'> & {session:string;title:string;onTitle:()=>void}) {
  const tabs=content.allTabs.filter(t=>t.session===session);
  const active=navigation.get(session).key;
  const faces=tabs.map(t=>({...face,session:undefined,hideFace:contentView(t)==='workspace',mark:contentView(t)==='file'?fileKind(t.url):pageKind(content.pageIcon(t.url)),id:content.handle(t),label:contentView(t)==='workspace'?gui.content_new:contentName(t)}));
  return <DeckBand faces={faces} active={active||''} onFocus={key=>{const tab=tabs.find(t=>content.handle(t)===key);if(tab){navigation.select(tab);quiet(content.focus(tab.id));}}} onTitle={onTitle} renderFace={(item,role)=><ContentOwnerIdentity face={face} title={role==='cur'?title:item.label}/>}/>;
}
export function MobileContentDeck({session,content,navigation,onClose,children,editing=false,onEditEnd}: DeckOwner & {session:string;children:ReactNode;editing?:boolean;onEditEnd?:()=>void}) {
  return <div className="mobile-content-deck" data-content-deck data-content-editing={editing||undefined}>
    {!editing&&<GlassCircle aria-label={gui.back} onClick={()=>{if(!navigation.back(session))onClose();}}><IconBack/></GlassCircle>}
    <div className="glass mobile-content-pill">{children}</div>
    {editing?<GlassCircle data-content-edit-end aria-label={gui.edit_done} onPointerDown={e=>e.preventDefault()} onClick={onEditEnd}><IconClose/></GlassCircle>:<ContentTabButton session={session} content={content} navigation={navigation}/>}
  </div>;
}
export function contentFileDeck({session,face,content,navigation,title}: DeckOwner & {session:string;title:string}) {
  return {identity:<div className="mobile-file-identity"><ContentDocumentBand session={session} face={face} content={content} navigation={navigation} title={title} onTitle={()=>navigation.browse(session)}/></div>,folded:<span className="mobile-fold-identity"><ContentOwnerIdentity face={face} title={title}/></span>,trail:<ContentTabButton session={session} content={content} navigation={navigation}/>};
}
/** Search has one clear button inside and an independent end-editing control outside. */
export function ContentSearch({label,value,onChange,onEditing,onEnter}: {label:string;value:string;onChange:(value:string)=>void;onEditing:(editing:boolean)=>void;onEnter?:()=>void}) {
  const input=useRef<HTMLInputElement>(null);
  return <div className="content-search-field"><IconSearch/><Input ref={input} autoFocus type="search" aria-label={label} placeholder={label} value={value} onFocus={()=>onEditing(true)} onBlur={()=>onEditing(false)} onChange={e=>onChange(e.target.value)} onKeyDown={e=>{if(imeKey(e))return;if(e.key==='Enter')onEnter?.();if(e.key==='Escape'){e.preventDefault();e.stopPropagation();input.current?.blur();}}}/>{value&&<button data-content-search-clear aria-label={gui.search_clear} onPointerDown={e=>e.preventDefault()} onClick={()=>{onChange('');input.current?.focus();}}><IconClose/></button>}</div>;
}
function PickerBand({source,onSource,onEdit,text}: {source:ContentSource;onSource:(source:ContentSource)=>void;onEdit:()=>void;text:string}) {
  const faces:TabFace[]=[{id:'workspace',label:gui.tree_search,path:'',face:null},{id:'browser',label:gui.page_address,path:'',face:null}];
  return <><ContentScope value={source} onChange={onSource} inPill/><DeckBand faces={faces} active={source} onFocus={id=>onSource(id as ContentSource)} onTitle={onEdit} renderFace={(f,role)=><span className="content-picker-prompt">{role==='cur'?(text||f.label):f.label}</span>}/></>;
}
function WorkspaceContext({root, device, repo, history, make}: {root: string; device: DeviceRow; repo:boolean; history: () => void; make: (kind:'file'|'dir') => void}) {
  const pending=useRef<'file'|'dir'|null>(null);
  return <div className="mobile-content-location"><TabAvatar f={{id:device.id,label:leaf(root),path:root,face:device.face,mark:workspaceKind(repo)}} bare="size-list-face" seat="row"/><strong title={root}>{leaf(root)}</strong>
    <DropdownMenu><DropdownMenuTrigger asChild><button aria-label={gui.content_workspace_actions}><IconMore/></button></DropdownMenuTrigger>
      <DropdownMenuContent align="end" onCloseAutoFocus={e=>{if(!pending.current)return;e.preventDefault();const kind=pending.current;pending.current=null;make(kind);}}>
        <DropdownMenuItem onSelect={()=>{pending.current='file';}}>{gui.file_new}</DropdownMenuItem>
        <DropdownMenuItem onSelect={()=>{pending.current='dir';}}>{gui.dir_new}</DropdownMenuItem>
        <DropdownMenuSeparator/><DropdownMenuItem onSelect={history}>{gui.file_history}</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>;
}
function BrowserPicker({session, device, devices, show, onOpen,face,content,navigation,onClose,desktop=false}: DeckOwner & {desktop?:boolean;session: string; device: DeviceRow; devices: DeviceRow[]; show: boolean; onOpen: (device: DeviceRow,url: string) => void}) {
  const state=navigation.get(session),seed=state.address,exit=devices.find(d=>d.id===state.exit)||device;
  const fieldKey='picker:'+session+':'+(state.key||'legacy'),field=navigation.editor(fieldKey),editing=field.editing??false,machinePrompt=field.exitPrompt;
  const [listSeat,setListSeat]=useState<HTMLDivElement|null>(null),[error,setError]=useState<string|null>(null);
  const setSeed=(address:string)=>navigation.update(session,{address});
  const endEdit=()=>{(document.activeElement as HTMLElement|null)?.blur();edit(false);};
  const places=usePlaces(PAGES);
  const open=(url:string,d:DeviceRow)=>{navigation.edit(fieldKey,{exitPrompt:undefined});setError(null);navigation.update(session,{address:url,exit:d.id});onOpen(d,url);};
  const setExit=(d:DeviceRow)=>{navigation.update(session,{exit:d.id});if(navigation.editor(fieldKey).exitPrompt!==undefined)open(navigation.get(session).address,d);};
  const edit=(value:boolean)=>navigation.edit(fieldKey,{editing:value,...(!value?{exitPrompt:undefined}:{})});
  const revisit=(id:string,url:string)=>{
    const d=devices.find(d=>d.id===id);
    if(d){open(url,d);return;}
    setSeed(url);navigation.edit(fieldKey,{editing:true,exitPrompt:(machinePrompt??0)+1});
  };
  const forget=async (device:string,path:string)=>{setError(null);try{await forgetRecentByDevice('page',device,path);places.reload();}catch(e){setError(said(e));}};
  return <div className="mobile-content-browser" hidden={!show}>
    {desktop&&<div data-content-browser-field className="session-content-picker-bar"><ContentScope value={state.source} onChange={source=>navigation.update(session,{source})}/><PageField tab={'content-picker:'+session+':'+(state.key||'legacy')} machine={exit} devices={devices} at={seed} onDraft={setSeed} first={device.id} machinePrompt={machinePrompt} steps={false} autoFocus={false} onEditingChange={edit} onMachine={setExit} onGo={open}/></div>}
    {machinePrompt!==undefined&&<p role="status" className="px-row py-in text-callout text-muted-foreground">{gui.content_choose_exit}</p>}
    {error&&<p role="alert" className="px-row py-in text-callout text-destructive">{error}</p>}
    <div className="mobile-content-places"><NewTabPage kind={PAGES} places={places} devices={devices} narrow onOpen={revisit} onForget={p=>void forget(p.device,p.place)}/></div>
    {!desktop&&<><div ref={setListSeat} className="mobile-content-page-list"/><MobileContentDeck session={session} face={face} content={content} navigation={navigation} onClose={onClose} editing={editing} onEditEnd={endEdit}>{editing?<PageField tab={'content-picker:'+session+':'+(state.key||'legacy')} machine={exit} devices={devices} at={seed} onDraft={setSeed} first={device.id} machinePrompt={machinePrompt} steps={false} autoFocus inPill listSeat={listSeat} onEditingChange={edit} onMachine={setExit} onGo={open}/>:<PickerBand source={state.source} onSource={source=>{endEdit();navigation.update(session,{source});}} onEdit={()=>edit(true)} text={seed}/>}</MobileContentDeck></>}

  </div>;
}
export function ContentPicker({row, devices, navigation, onOpen, face, content, onClose,desktop=false}: DeckOwner & {
  desktop?:boolean;
  row: SessionRow; devices: DeviceRow[]; navigation: ContentNavigation;
  onOpen: (device: DeviceRow,path: string,view:'file'|'page',line?: number|null) => void;
}) {
  const state=navigation.get(row.id);
  const device=devices.find(d=>state.scope?d.id===state.scope.device:d.name===row.home);
  const root=state.scope?.root || sessionCwdOnScreen(row);
  const query=state.query,setQuery=(query:string)=>navigation.update(row.id,{query});
  const [make,setMake]=useState<{kind:'file'|'dir';at:number}>();
  const [searching,setSearching]=useState(false);
  if(!device)return <div className="mobile-content-picker"><div className="session-content-unknown">{gui.content_machine_gone}</div>{!desktop&&<MobileContentDeck session={row.id} face={face} content={content} navigation={navigation} onClose={onClose}><ContentOwnerIdentity face={face} title={gui.content_machine_gone}/></MobileContentDeck>}</div>;
  const source=(source: ContentSource)=>navigation.update(row.id,{source});
  const enter = async () => {
    (document.activeElement as HTMLElement|null)?.blur();
    const raw=query.trim();
    if(!raw||(!desktop&&root))return;
    if(desktop&&/^(https?:\/\/|localhost(?::|\/|$))/i.test(raw)){navigation.update(row.id,{source:'browser',address:raw});return;}
    const absolute=/^(\/|[a-z]:[\\/])/i.test(raw);
    const path=absolute||!root?raw:root.replace(/[\\/]$/,'')+'/'+raw;
    try{const ls=await fetchLs(device.name,path);navigation.update(row.id,{scope:{device:device.id,root:ls.path}});setQuery('');}
    catch{if(!root)onOpen(device,path,'file');else try{const found=await searchIn(device.name,root,raw);if(found.names.length===1)onOpen(device,found.names[0],'file');}catch{/* The search view reports its own failure. */}}
  };
  return <div className="mobile-content-picker" data-desktop-picker={desktop||undefined}>
    <div className="mobile-content-workspace" hidden={state.source!=='workspace'}>
      {root?<SideWorkspace key={device.id+' '+root} machine={device.name} here={device.me} root={root} query={query} onQuery={setQuery} findLeading={desktop?<ContentScope value={state.source} onChange={source}/>:undefined} makeRequest={make} onSearchEnter={()=>void enter()} onOpenContent={(path,line)=>onOpen(device,path,'file',line)} treeHead={(history,repo)=><WorkspaceContext repo={repo} root={root} device={device} history={history} make={kind=>{setQuery('');setMake({kind,at:Date.now()});}}/>}/>:<><SideFind label={gui.tree_search} query={query} onQuery={setQuery} leading={desktop?<ContentScope value={state.source} onChange={source}/>:undefined} onEnter={()=>void enter()}/><div className="session-content-unknown">{gui.content_no_directory}</div></>}
    </div>
    {!desktop&&state.source==='workspace'&&<MobileContentDeck session={row.id} face={face} content={content} navigation={navigation} onClose={onClose} editing={searching} onEditEnd={()=>{(document.activeElement as HTMLElement|null)?.blur();setSearching(false);}}>{searching?<ContentSearch label={gui.tree_search} value={query} onChange={setQuery} onEditing={setSearching} onEnter={()=>void enter()}/>:<PickerBand source={state.source} onSource={source} onEdit={()=>setSearching(true)} text={query}/>}</MobileContentDeck>}

    <BrowserPicker desktop={desktop} face={face} content={content} navigation={navigation} onClose={onClose} session={row.id} device={device} devices={devices} show={state.source==='browser'} onOpen={(d,url)=>onOpen(d,url,'page')}/>
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
  const tabFaces=tabs.map(t=>({...contentTabFace(t,devices,content.pageIcon),...(contentView(t)==='workspace'?{label:gui.content_new,hideFace:true}:{})}));
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
  return <div className="mobile-content-overview"><div className="mobile-content-overview-list"><TabSheetRows faces={shown} active={active} onFocus={onFocus} onClose={onCloseSession} onNew={onNew} newLabel={gui.new_session}/>{!shown.length&&<p className="mobile-content-empty">{gui.content_no_match}</p>}</div><div data-content-deck className="mobile-content-deck" data-content-editing={editing||undefined}>{!editing&&<GlassCircle aria-label={gui.back} onClick={onClose}><IconBack/></GlassCircle>}<div className="glass mobile-content-pill">{editing?<ContentSearch label={gui.search_sessions} value={query} onChange={onQuery} onEditing={setEditing}/>:<button data-session-overview-search className="content-search-prompt" onClick={()=>setEditing(true)}><IconSearch/>{query||gui.search_sessions}</button>}</div>{editing?<GlassCircle aria-label={gui.edit_done} onPointerDown={e=>e.preventDefault()} onClick={()=>{(document.activeElement as HTMLElement|null)?.blur();setEditing(false);}}><IconClose/></GlassCircle>:<GlassCircle aria-label={gui.switch_tabs(faces.length)} onClick={onClose}><TabsGlyph n={faces.length}/></GlassCircle>}</div></div>;
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
    const expand=(e:Event)=>{if(e.target instanceof Element&&e.target.closest('input,textarea,[contenteditable=true]'))setDetent('full');};
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
