/** Session content on a narrow screen: browse, read, then keep.
 * The shared content model owns node handles; this file owns presentation. */
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { fetchLs, forgetRecent, type DeviceRow, type SessionRow } from '@/api';
import { PageField } from '@/browse/PageField';
import { NewTabPage } from '@/browse/NewTabPage';
import { usePlaces } from '@/browse/use-places';
import { PAGES } from '@/views/PagesKind';
import { BarSeat, TabsGlyph } from '@/components/SessionBar';
import { SessionFace } from '@/components/SessionFace';
import { FileKindIcon } from '@/components/FileKindIcon';
import { GlassCircle } from '@/components/Glass';
import { IconBack, IconBrowser, IconCheck, IconClose, IconFiles, IconForward, IconMore, IconPlus } from '@/components/icons';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { SegmentedControl } from '@/components/ui/segmented-control';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { SideFind } from '@/components/SideFind';
import { gui } from '@/gen/catalog';
import type { ContentNavigation, ContentSource } from '@/hooks/use-content-navigation';
import type { ContentTab, SessionContent } from '@/hooks/use-session-content';
import { directoryOf } from '@/lib/paths';
import { sessionCwdOnScreen } from '@/lib/session-cwd';
import { SideWorkspace } from '@/views/SideWorkspace';
import { useDetents, type PlaceDetent } from '@/views/SessionPlaceSheet';
import { contentName, contentView } from '@/lib/content';
import type { ReactNode } from 'react';
import type { TabFace } from '@/browse/TabStrip';

const quiet = (work: Promise<unknown>) => { void work.catch(() => {}); };
const leaf = (path: string) => path.replace(/[\\/]$/, '').split(/[\\/]/).pop() || path;
export function ContentGlyph({tab}: {tab: ContentTab}) {
  return <span className="mobile-content-glyph">{contentView(tab)==='file'?<FileKindIcon name={leaf(tab.url)}/>:contentView(tab)==='page'?<IconBrowser/>:<IconFiles/>}</span>;
}
function context(tab: ContentTab, row: SessionRow | undefined, devices: DeviceRow[]) {
  if(contentView(tab)==='page') return tab.url.replace(/^https?:\/\//, '').replace(/\/$/, '');
  const root=sessionCwdOnScreen(row), dir=contentView(tab)==='file'?directoryOf(tab.url):tab.url;
  const folder=root && (dir===root || dir.startsWith(root+'/') || dir.startsWith(root+'\\')) ? leaf(root)+dir.slice(root.length) : dir;
  return [devices.find(d=>d.id===tab.device)?.name, folder].filter(Boolean).join(' · ');
}
export const contentContext = context;

export function MobileContentBar({session, face, content, navigation, onClose}: {
  session: string; face: TabFace; content: SessionContent; navigation: ContentNavigation; onClose: () => void;
}) {
  const state=navigation.get(session), scene=state.scene==='overview'?state.from:state.scene;
  const tab=scene==='document'?content.allTabs.find(t=>t.session===session&&content.handle(t)===state.key):null;
  const name=tab?contentName(tab):state.source==='workspace'?gui.workspaces_tab:gui.browser_tab;
  const count=content.tabs.filter(t=>t.session===session).length;
  return <><BarSeat at="pill"><div className="mobile-content-group">
    <button className="mobile-content-parent" aria-label={gui.content_return_session(face.label)} aria-expanded onClick={onClose} title={face.label}>
      {face.session&&<SessionFace face={face.face} {...face.session} size="tab"/>}<span>{face.label}</span>
    </button>
    <button className="mobile-content-current" data-temporary={tab?content.isPreview(tab):undefined} aria-label={tab?gui.content_current(name):name} onClick={e=>{if(state.scene==='document'||state.scene==='overview')navigation.overview(session);else e.currentTarget.closest('[data-session-screen]')?.querySelector<HTMLInputElement>('[data-content-picker="'+CSS.escape(session)+'"] '+(state.source==='workspace'?'[data-side-find-field]':'[data-page-address]'))?.focus();}} title={name}>
      {tab?<ContentGlyph tab={tab}/>:state.source==='workspace'?<IconFiles/>:<IconBrowser/>}<span>{name}</span>
    </button>
    <button className="mobile-content-add" aria-label={gui.content_find} onClick={()=>navigation.browse(session)}><IconPlus/></button>
  </div></BarSeat><BarSeat at="trail"><GlassCircle data-content-overview-button data-on={state.scene==='overview'} aria-label={gui.content_tabs_count(count)} aria-expanded={state.scene==='overview'} onClick={()=>navigation.overview(session)}><TabsGlyph n={count}/></GlassCircle></BarSeat></>;
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
function BrowserPicker({session, device, devices, show, onOpen}: {session: string; device: DeviceRow; devices: DeviceRow[]; show: boolean; onOpen: (device: DeviceRow,url: string) => void}) {
  const [exit,setExit]=useState(device),[seed,setSeed]=useState('');
  const written=useRef('');
  const places=usePlaces(PAGES);
  const open=(url: string,d: DeviceRow)=>{written.current=url;setSeed(url);onOpen(d,url);};
  return <div className="mobile-content-browser" hidden={!show}
    onInputCapture={e=>{if(e.target instanceof HTMLInputElement&&e.target.matches('[data-page-address]'))written.current=e.target.value;}}
    onBlurCapture={e=>{if(e.target instanceof HTMLInputElement&&e.target.matches('[data-page-address]'))setSeed(written.current);}}>
    <div className="mobile-content-places"><NewTabPage kind={PAGES} places={places} devices={devices} narrow onOpen={(id,url)=>{const d=devices.find(d=>d.id===id);if(d)open(url,d);}} onForget={p=>quiet(forgetRecent('page',p.name,p.place).then(places.reload))}/></div>
    <div className="mobile-content-address"><PageField tab={'content-picker:'+session} machine={devices.find(d=>d.id===exit.id)||exit} devices={devices} at={seed} first={device.id} steps={false} autoFocus={false} onMachine={setExit} onGo={open}/></div>
  </div>;
}
export function ContentPicker({row, devices, navigation, onOpen}: {
  row: SessionRow; devices: DeviceRow[]; navigation: ContentNavigation;
  onOpen: (device: DeviceRow,path: string,view:'file'|'page',line?: number|null) => void;
}) {
  const state=navigation.get(row.id);
  const device=devices.find(d=>state.scope?d.id===state.scope.device:d.name===row.home);
  const root=state.scope?.root || sessionCwdOnScreen(row);
  const [query,setQuery]=useState(''),[make,setMake]=useState<{kind:'file'|'dir';at:number}>();
  if(!device)return <div className="session-content-unknown">{gui.content_machine_gone}</div>;
  const source=(source: ContentSource)=>navigation.update(row.id,{source});
  const enter = async () => {
    (document.activeElement as HTMLElement|null)?.blur();
    const path=query.trim();
    if (!path || root) return;
    try { const ls=await fetchLs(device.name,path); navigation.update(row.id,{scope:{device:device.id,root:ls.path}});setQuery(''); }
    catch { onOpen(device,path,'file'); }
  };
  return <div className="mobile-content-picker">
    <div className="mobile-content-picker-head"><SegmentedControl label={gui.content_scope} value={state.source} options={[{key:'workspace',label:gui.workspaces_tab},{key:'browser',label:gui.browser_tab}]} onChange={source}/></div>
    <div className="mobile-content-workspace" hidden={state.source!=='workspace'}>
      {root?<SideWorkspace key={device.id+' '+root} machine={device.name} here={device.me} root={root} query={query} onQuery={setQuery} makeRequest={make} onSearchEnter={()=>void enter()} onOpenContent={(path,line)=>onOpen(device,path,'file',line)} treeHead={history=><WorkspaceContext root={root} device={device} history={history} make={kind=>{setQuery('');setMake({kind,at:Date.now()});}}/>}/>:<><SideFind label={gui.search_file} query={query} onQuery={setQuery} onEnter={()=>void enter()}/><div className="session-content-unknown">{gui.content_no_directory}</div></>}
    </div>
    <BrowserPicker session={row.id} device={device} devices={devices} show={state.source==='browser'} onOpen={(d,url)=>onOpen(d,url,'page')}/>
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

export function ContentOverview({session, row, devices, content, navigation, onPick}: {session:string;row:SessionRow|undefined;devices:DeviceRow[];content:SessionContent;navigation:ContentNavigation;onPick:(tab:ContentTab)=>void}) {
  const state=navigation.get(session),q=state.filter.toLocaleLowerCase();
  const matches=(tab:ContentTab)=>(contentName(tab)+' '+tab.url+' '+context(tab,row,devices)).toLocaleLowerCase().includes(q);
  const tabs=content.tabs.filter(t=>t.session===session),found=tabs.filter(matches),preview=content.previews[session];
  const previewMatches=preview&&matches(preview);
  const render=(tab:ContentTab,temp=false)=><div key={tab.id} data-content-overview-tab={tab.id} className="mobile-content-overview-row" data-active={state.from==='document'&&state.key===content.handle(tab)}>
    <button onClick={()=>onPick(tab)}><ContentGlyph tab={tab}/><span className="mobile-content-document-label"><strong>{contentName(tab)}</strong><small>{context(tab,row,devices)}</small></span>{state.from==='document'&&state.key===content.handle(tab)&&<IconCheck/>}</button>
    {temp&&<Button variant="ghost" size="sm" className="min-h-touch" onClick={()=>quiet(content.keep(tab.id))}>{gui.content_keep}</Button>}
    {!tab.pinned&&<button aria-label={gui.close_tab} title={contentName(tab)} onClick={()=>quiet(content.close(tab.id))}><IconClose/></button>}
  </div>;
  return <div className="mobile-content-overview"><div className="mobile-content-overview-head"><h2>{gui.content_tabs}</h2><Button variant="ghost" size="sm" className="min-h-touch" onClick={()=>navigation.overview(session)}>{gui.edit_done}</Button></div>
    <div className="mobile-content-overview-list">{previewMatches&&<div className="mobile-content-pending"><p>{gui.content_temporary}</p>{render(preview,true)}</div>}{found.map(t=>render(t))}
      {!tabs.length&&!previewMatches&&<p className="mobile-content-empty">{gui.content_empty}</p>}{tabs.length>0&&!found.length&&!previewMatches&&<p className="mobile-content-empty">{gui.content_no_match}</p>}
    </div>
    <div className="mobile-content-overview-search"><Input type="search" aria-label={gui.content_tabs_search} placeholder={gui.content_tabs_search} value={state.filter} onChange={e=>navigation.update(session,{filter:e.target.value})}/></div>
  </div>;
}

/** In-place sibling of the deck; no Dialog focus trap can steal the pill.
 * Native WebViews are bounded to the content body above these DOM controls. */
export function MobileContentSheet({children,onClose}: {children: ReactNode;onClose:()=>void}) {
  const [el,setEl]=useState<HTMLDivElement|null>(null),[handle,setHandle]=useState<HTMLElement|null>(null),[detent,setDetent]=useState<PlaceDetent>('half');
  const overlay=useRef<HTMLDivElement>(null);
  const screen=()=>overlay.current?.closest<HTMLElement>('[data-session-screen]');
  const bounds=()=>{const host=screen(),deck=host?.querySelector<HTMLElement>('[data-session-deck]');return {height:host?.clientHeight??window.innerHeight,foot:host&&deck?host.getBoundingClientRect().bottom-deck.getBoundingClientRect().top:0};};
  useDetents(el,setDetent,onClose,bounds,handle);
  useLayoutEffect(()=>{
    const host=screen(),deck=host?.querySelector<HTMLElement>('[data-session-deck]');if(!host||!deck||!overlay.current)return;
    const read=()=>{setHandle(deck.querySelector<HTMLElement>('.mobile-content-group'));const box=bounds();overlay.current?.style.setProperty('--mobile-content-height',box.height+'px');overlay.current?.style.setProperty('--mobile-content-foot',box.foot+'px');};
    const observer=new ResizeObserver(read);observer.observe(host);observer.observe(deck);read();return()=>observer.disconnect();
  },[]);
  useEffect(()=>{const back=(e:KeyboardEvent)=>{if(e.key==='Escape'&&!document.querySelector('[role="dialog"], [role="menu"], [data-radix-popper-content-wrapper]')){e.preventDefault();onClose();}};document.addEventListener('keydown',back);return()=>document.removeEventListener('keydown',back);},[onClose]);
  return <div ref={overlay} data-mobile-content-sheet className="mobile-content-overlay"><button className="mobile-content-scrim" aria-label={gui.side_hide} onClick={onClose}/>
    <div ref={setEl} data-detent={detent} data-content-panel className="mobile-content-sheet" onFocusCapture={e=>{if((e.target as Element).closest('input,textarea,[contenteditable=true]'))setDetent('full');}}>
      <button data-place-grab className="mobile-content-grab" aria-label={detent==='half'?gui.content_expand_sheet:gui.content_collapse_sheet} onClick={e=>{if(e.detail>0&&el?.hasAttribute('data-sheet-dragged')){el.removeAttribute('data-sheet-dragged');return;}setDetent(detent==='half'?'full':'half');}}><span/></button>
      {children}
    </div>
  </div>;
}
