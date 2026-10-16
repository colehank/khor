/** The mobile session's current content owns one bottom deck. The
 * terminal's measured deck room is retained behind the sheet. */
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { fetchLs, searchIn, forgetRecent, type DeviceRow, type SessionRow } from '@/api';
import { PageField } from '@/browse/PageField';
import { NewTabPage } from '@/browse/NewTabPage';
import { usePlaces } from '@/browse/use-places';
import { PAGES } from '@/views/PagesKind';
import { ContentScope } from '@/components/ContentScope';
import { ContentTabsGlyph, TabSheetRows } from '@/components/SessionBar';
import { SessionFace, fileKind, pageKind, workspaceKind } from '@/components/SessionFace';
import { FileKindIcon } from '@/components/FileKindIcon';
import { GlassCircle } from '@/components/Glass';
import { IconBack, IconBrowser, IconFiles, IconForward, IconMore, IconPlus } from '@/components/icons';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { SegmentedControl } from '@/components/ui/segmented-control';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { SideFind } from '@/components/SideFind';
import { gui } from '@/gen/catalog';
import type { ContentNavigation, ContentSource } from '@/hooks/use-content-navigation';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
import { useContentSheet } from '@/hooks/use-content-sheet';
import { directoryOf } from '@/lib/paths';
import { sessionCwdOnScreen } from '@/lib/session-cwd';
import { SideWorkspace } from '@/views/SideWorkspace';
import type { PlaceDetent } from '@/views/SessionPlaceSheet';
import { contentName, contentView } from '@/lib/content';
import type { TabFace } from '@/browse/TabStrip';

const quiet = (work: Promise<unknown>) => { void work.catch(() => {}); };
const leaf = (path: string) => path.replace(/[\\/]$/, '').split(/[\\/]/).pop() || path;
export function ContentGlyph({tab}: {tab: ContentTab}) {
  return <span className="mobile-content-glyph">{contentView(tab)==='file'?<FileKindIcon name={leaf(tab.url)}/>:contentView(tab)==='page'?<IconBrowser/>:<IconFiles/>}</span>;
}
export function contentContext(tab:ContentTab,row:SessionRow|undefined,devices:DeviceRow[]) {
  if(contentView(tab)==='page')return tab.url.replace(/^https?:\/\//,'').replace(/\/$/,'');
  const root=sessionCwdOnScreen(row),dir=contentView(tab)==='file'?directoryOf(tab.url):tab.url;
  const folder=root&&(dir===root||dir.startsWith(root+'/')||dir.startsWith(root+'\\'))?leaf(root)+dir.slice(root.length):dir;
  return [devices.find(d=>d.id===tab.device)?.name,folder].filter(Boolean).join(' · ');
}
export type DeckOwner={face:TabFace;content:SessionContent;navigation:ContentNavigation;onClose:()=>void};
export function ContentOwnerIdentity({face,title,children}: {face:TabFace;title?:string;children?:ReactNode}) {
  return <span className="content-owner-identity"><span className="content-owner-face">{face.session&&<SessionFace face={face.face} {...face.session} size="tab"/>}</span><span className="content-owner-name">{face.label}</span><span className="content-owner-slash">/</span>{children||<span className="content-owner-title">{title}</span>}</span>;
}
export function ContentTabButton({session,content,navigation}: Pick<DeckOwner,'content'|'navigation'> & {session:string}) {
  const count=content.tabs.filter(t=>t.session===session).length;
  return <GlassCircle data-content-overview-button aria-label={gui.content_tabs_count(count)} aria-expanded={navigation.get(session).scene==='overview'} onClick={()=>navigation.overview(session)}><ContentTabsGlyph n={count}/></GlassCircle>;
}
export function MobileContentDeck({session,content,navigation,onClose,children}: DeckOwner & {session:string;children:ReactNode}) {
  return <div className="mobile-content-deck" data-content-deck>
    <GlassCircle aria-label={gui.back} onClick={()=>{if(!navigation.back(session))onClose();}}><IconBack/></GlassCircle>
    <div className="glass mobile-content-pill">{children}</div>
    <ContentTabButton session={session} content={content} navigation={navigation}/>
  </div>;
}
export function contentFileDeck({session,face,content,navigation,title}: DeckOwner & {session:string;title:string}) {
  return {identity:<button className="mobile-file-identity" aria-label={gui.content_find} onClick={()=>navigation.browse(session)}><ContentOwnerIdentity face={face} title={title}/></button>,folded:<span className="mobile-fold-identity"><ContentOwnerIdentity face={face} title={title}/></span>,trail:<ContentTabButton session={session} content={content} navigation={navigation}/>};
}
function WorkspaceContext({root, device, history, make}: {root: string; device: DeviceRow; history: () => void; make: (kind:'file'|'dir') => void}) {
  const pending=useRef<'file'|'dir'|null>(null);
  return <div className="mobile-content-location"><strong title={root}>{leaf(root)}</strong><span>{device.name}</span>
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
  const setExit=(d:DeviceRow)=>navigation.update(session,{exit:d.id});
  const setSeed=(address:string)=>navigation.update(session,{address});
  const places=usePlaces(PAGES);
  const open=(url: string,d: DeviceRow)=>{navigation.update(session,{address:url,exit:d.id});onOpen(d,url);};
  return <div className="mobile-content-browser" hidden={!show}>
    {desktop&&<div data-content-browser-field className="session-content-picker-bar"><ContentScope value={state.source} onChange={source=>navigation.update(session,{source})}/><PageField tab={'content-picker:'+session} machine={exit} devices={devices} at={seed} onDraft={setSeed} first={device.id} steps={false} autoFocus={false} onMachine={setExit} onGo={open}/></div>}
    <div className="mobile-content-places"><NewTabPage kind={PAGES} places={places} devices={devices} narrow onOpen={(id,url)=>{const d=devices.find(d=>d.id===id);if(d)open(url,d);}} onForget={p=>quiet(forgetRecent('page',p.name,p.place).then(places.reload))}/></div>
    {!desktop&&<MobileContentDeck session={session} face={face} content={content} navigation={navigation} onClose={onClose}><ContentOwnerIdentity face={face}><PageField tab={'content-picker:'+session} machine={devices.find(d=>d.id===exit.id)||exit} devices={devices} at={seed} onDraft={setSeed} first={device.id} steps={false} autoFocus={false} inPill onMachine={setExit} onGo={open}/></ContentOwnerIdentity></MobileContentDeck>}
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
    {!desktop&&<div className="mobile-content-picker-head"><SegmentedControl label={gui.content_scope} value={state.source} options={[{key:'workspace',label:gui.workspaces_tab},{key:'browser',label:gui.browser_tab}]} onChange={source}/></div>}
    <div className="mobile-content-workspace" hidden={state.source!=='workspace'}>
      {root?<SideWorkspace key={device.id+' '+root} machine={device.name} here={device.me} root={root} query={query} onQuery={setQuery} findLeading={desktop?<ContentScope value={state.source} onChange={source}/>:undefined} makeRequest={make} onSearchEnter={()=>void enter()} onOpenContent={(path,line)=>onOpen(device,path,'file',line)} treeHead={history=><WorkspaceContext root={root} device={device} history={history} make={kind=>{setQuery('');setMake({kind,at:Date.now()});}}/>}/>:<><SideFind label={gui.search_file} query={query} onQuery={setQuery} leading={desktop?<ContentScope value={state.source} onChange={source}/>:undefined} onEnter={()=>void enter()}/><div className="session-content-unknown">{gui.content_no_directory}</div></>}
    </div>
    {!desktop&&state.source==='workspace'&&<MobileContentDeck session={row.id} face={face} content={content} navigation={navigation} onClose={onClose}><ContentOwnerIdentity face={face}><Input data-content-search type="search" aria-label={gui.search_file} placeholder={gui.search_file} value={query} onChange={e=>setQuery(e.target.value)} onKeyDown={e=>{if(e.key==='Enter')void enter();}}/></ContentOwnerIdentity></MobileContentDeck>}
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

export function ContentDocumentNavigation({tab, machine, temporary, onBrowse, onKeep, onOpen}: {tab: ContentTab; machine: string; temporary: boolean; onBrowse: () => void; onKeep: () => void; onOpen: (path:string) => void}) {
  const {files,at}=useContentNeighbors(tab,machine);
  return <div className="mobile-content-document-head">
    <Button variant="ghost" size="sm" className="min-h-touch" aria-label={gui.content_return} onClick={onBrowse}><IconBack/>{contentView(tab)==='page'?gui.browser_tab:gui.workspaces_tab}</Button>
    <div className="mobile-content-document-actions">{contentView(tab)==='file'&&<div className="mobile-content-neighbors">
      <button aria-label={gui.content_previous} title={files[at-1]&&leaf(files[at-1])} disabled={at<=0} onClick={()=>onOpen(files[at-1])}><IconBack/></button>
      <button aria-label={gui.content_next} title={files[at+1]&&leaf(files[at+1])} disabled={at<0||at>=files.length-1} onClick={()=>onOpen(files[at+1])}><IconForward/></button>
    </div>}{temporary&&<Button variant="ghost" size="sm" className="min-h-touch" onClick={onKeep}><IconPlus/>{gui.content_keep}</Button>}</div>
  </div>;
}

export function ContentOverview({session,row,devices,content,navigation,onPick,face,onClose}: DeckOwner & {session:string;row:SessionRow|undefined;devices:DeviceRow[];onPick:(tab:ContentTab)=>void}) {
  const state=navigation.get(session),q=state.filter.toLocaleLowerCase();
  const tabs=content.allTabs.filter(t=>t.session===session&&(contentName(t)+' '+t.url+' '+contentContext(t,row,devices)).toLocaleLowerCase().includes(q));
  const tabFaces=tabs.map(t=>({id:t.id,label:contentName(t),path:t.url,face:devices.find(d=>d.id===t.device)?.face||null,mark:contentView(t)==='file'?fileKind(t.url):contentView(t)==='workspace'?workspaceKind(false):pageKind(undefined)}));
  const current=content.allTabs.find(t=>t.session===session&&content.handle(t)===(state.recent||state.key));
  return <div className="mobile-content-overview"><div className="mobile-content-overview-head"><h2>{gui.content_tabs_for(face.label)}</h2><Button variant="ghost" size="sm" className="min-h-touch" onClick={()=>{if(!navigation.back(session))onClose();}}>{gui.edit_done}</Button></div>
    <div className="mobile-content-overview-list">
      {!tabs.length&&<p className="mobile-content-empty">{q?gui.content_no_match:gui.content_empty}</p>}
      <TabSheetRows faces={tabFaces} active={current?.id||''} onFocus={id=>{const tab=tabs.find(t=>t.id===id);if(tab)onPick(tab);}} onClose={id=>quiet(content.close(id))} canClose={f=>!tabs.find(t=>t.id===f.id)?.pinned} actions={f=>{const tab=tabs.find(t=>t.id===f.id);return tab&&content.isPreview(tab)?<Button variant="ghost" size="sm" className="min-h-touch" onClick={()=>quiet(content.keep(tab.id))}>{gui.content_keep}</Button>:null;}} rowData={f=>({'data-content-overview-tab':f.id,'data-temporary':!!content.previews[session]&&content.previews[session].id===f.id})} onNew={()=>navigation.browse(session)} newLabel={gui.content_find}/>
    </div>
    <MobileContentDeck session={session} face={face} content={content} navigation={navigation} onClose={onClose}><ContentOwnerIdentity face={face}><Input type="search" aria-label={gui.content_tabs_search} placeholder={gui.content_tabs_search} value={state.filter} onChange={e=>navigation.update(session,{filter:e.target.value})}/></ContentOwnerIdentity></MobileContentDeck>
  </div>;
}
/** In-place sheet shared by content and session overviews. Its deck is
 * inside the current page; the retained terminal never changes geometry. */
export function MobileContentSheet({children,onClose}: {children:ReactNode;onClose:()=>void}) {
  const [el,setEl]=useState<HTMLDivElement|null>(null),[detent,setDetent]=useState<PlaceDetent>('half');
  const overlay=useRef<HTMLDivElement>(null);
  const screen=()=>overlay.current?.closest<HTMLElement>('[data-session-screen]');
  useContentSheet(el,setDetent,onClose,()=>({height:screen()?.clientHeight||innerHeight,foot:0}));
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
