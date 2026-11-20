/** An attachment selection panel with the existing file browser and preview. */
import {useEffect,useLayoutEffect,useMemo,useRef,useState,useId,type RefObject} from 'react';
import {createPortal} from 'react-dom';
import {fetchDevices,nativeFolders,said,termDropForget,termPickFolders,type DeviceRow} from '@/api';
import {MachineAvatar} from '@/components/Avatar';
import {IconBack,IconFile,IconFiles,IconMore} from '@/components/icons';
import {ContentSearch} from '@/components/ContentSearch';
import {ContentDeck,ContentPill,PillFace} from '@/components/ContentDeck';
import {MachineFace} from '@/components/SessionFace';
import {Button} from '@/components/ui/button';
import {DropdownMenu,DropdownMenuTrigger,DropdownMenuContent,DropdownMenuItem} from '@/components/ui/dropdown-menu';
import {ContentOverlay} from '@/components/ContentOverlay';
import {GlassCircle} from '@/components/Glass';
import {gui} from '@/gen/catalog';
import type {DirListing} from '@/gen/bindings/DirListing';
import {deviceAttachments,deviceFolderEntries,includesAttachment,pathAttachment,type Attachment} from '@/lib/term-attachments';
import {keyboardCover,keyboardHeight,holdFoot,footHeld} from '@/lib/viewport';
import {lowerKeyboard,raiseKeyboard} from '@/lib/soft-keyboard';
import {isDown} from '@/lib/topology';
import {hopWord} from '@/words';
import {cn} from '@/lib/utils';
import {useNarrow} from '@/hooks/use-narrow';
import {FilesPane} from '@/views/FilesPane';
import {AttachmentList} from '@/views/AttachmentList';
import {AttachmentFile} from '@/views/AttachmentFile';
import {SendInto} from '@/views/chat-files';
export type FilesRoad = {
  machine: string | null; cwd: string | null;
  selected: Attachment[];
  add: (entries: Attachment[]) => void;
  remove: (entry: Attachment) => void;
  replace?: (entry: Attachment, file: File) => void;
  open?: () => void; close?: () => void; opened?: boolean;
  into?: string | null; onInto?: (path: string | null) => void;
  filesOnly?: boolean; action?: string;
  send: (entries: Attachment[], into?: string | null) => Promise<boolean>;
  dropFiles: (files: File[]) => void;
  busy: boolean; stopping?: boolean; progress: number | null; cancel?: () => void;
};
export function machinesFor(devices: DeviceRow[], machine: string | null): DeviceRow[] {
  const ours = (d: DeviceRow) => (machine === null ? d.me : d.name === machine);
  const hosting = devices.filter((d) => d.hosts);
  return [...hosting.filter(ours), ...hosting.filter((d) => !ours(d))];
}

type Place={machine:DeviceRow;path:string;root?:Attachment};
type PickerProps = {
 open: boolean;
 onClose: () => void;
 road: FilesRoad;
 onSent: () => void;
 viewport?: RefObject<HTMLElement | null>;
 terminalOverlay?: boolean;
 keyboardBand?: number;
 onBeforeSend?: () => void;
};

export function AttachmentPicker({open,...props}:PickerProps){
 return open?<TermFiles {...props}/>:null;
}
export function TermFiles({road,onClose,onSent,viewport,terminalOverlay=false,keyboardBand=0,onBeforeSend}:Omit<PickerProps,'open'>){
 const narrow=useNarrow();
 const sourceTitle=useId(),selectedTitle=useId();
 const picker=useRef<HTMLInputElement>(null),folders=useRef<HTMLInputElement>(null),panel=useRef<HTMLDivElement>(null);
 const [devices,setDevices]=useState<DeviceRow[]>([]),[error,setError]=useState<string|null>(null),[picking,setPicking]=useState(false);
 const [at,setAt]=useState<Place|null>(null),[look,setLook]=useState<Attachment|null>(null);
 const sending=useRef(false),mounted=useRef(true);
 const restore=useRef<HTMLElement|null>(null),oldHold=useRef<number|null>(null);
 const [height]=useState(()=>keyboardBand || Math.min(keyboardCover()||footHeld()||keyboardHeight(),window.innerHeight*0.6));
 useLayoutEffect(()=>{
  const sheet=panel.current;if(!sheet)return;
  let watched:HTMLElement|null=null;
  const sync=()=>{
   const host=viewport?.current??null;
   if(host!==watched){if(watched)observer.unobserve(watched);if(host)observer.observe(host);watched=host;}
   const rect=host?.getBoundingClientRect(),left=Math.max(0,rect?.left??0),right=Math.min(window.innerWidth,rect?.right??window.innerWidth);
   const width=right-left;
   sheet.style.left=(width>0?left:0)+'px';sheet.style.width=(width>0?width:window.innerWidth)+'px';
   // With no reserved keyboard space, cover part of the terminal while
   // keeping its key row reachable and its viewport at full height.
   if(terminalOverlay&&!keyboardBand&&rect){
    const panelHeight=Math.min(height,rect.height);
    sheet.style.top=`${rect.bottom-panelHeight}px`;
    sheet.style.height=`${panelHeight}px`;
    sheet.style.setProperty('--attachment-panel-height',`${panelHeight}px`);
   }
  };
  const observer=new ResizeObserver(sync);sync();
  const frame=requestAnimationFrame(sync);
  window.addEventListener('resize',sync);window.addEventListener('transitionend',sync,true);window.addEventListener('animationend',sync,true);window.visualViewport?.addEventListener('resize',sync);
  return()=>{cancelAnimationFrame(frame);observer.disconnect();window.removeEventListener('resize',sync);window.removeEventListener('transitionend',sync,true);window.removeEventListener('animationend',sync,true);window.visualViewport?.removeEventListener('resize',sync);};
 },[height,keyboardBand,terminalOverlay,viewport]);
 useLayoutEffect(()=>{
  if(terminalOverlay)return;
  const field=document.activeElement;oldHold.current=footHeld();
  // Move chrome to the keyboard's edge while the terminal retains its grid.
  // The terminal's fit guard covers this hold and every nested content page.
  const covered=keyboardCover()>0;
  if(narrow||matchMedia('(pointer: coarse)').matches)holdFoot(height);
  if(covered&&field instanceof HTMLElement){restore.current=field;lowerKeyboard(field);}
  else if(field instanceof HTMLElement&&field.matches('input,textarea'))field.blur();
  return()=>{holdFoot(oldHold.current);};
 },[height,narrow,terminalOverlay]);
 const finish=(sent=false)=>{if(!terminalOverlay&&restore.current?.isConnected)raiseKeyboard(restore.current);sent?onSent():onClose();};
 useEffect(()=>{mounted.current=true;let live=true;void fetchDevices().then(rows=>{if(live)setDevices(rows);},e=>{if(live)setError(said(e));});return()=>{live=false;mounted.current=false;};},[]);
 const sources=machinesFor(devices,road.machine).filter(device=>!device.me);
 const own=(entry:Attachment)=>road.selected.some(item=>item.key===entry.key);
 const inherited=(entry:Attachment)=>road.selected.some(item=>includesAttachment(item,entry));
 const toggle=(entry:Attachment)=>{if(road.busy||inherited(entry))return;own(entry)?road.remove(entry):road.add([entry]);};
 const addDevice=(files:File[],dirs:boolean)=>{if(!files.length||road.busy)return;try{road.add(deviceAttachments(files,dirs,gui.machine_here));setError(null);}catch(e){setError(said(e));}};
 const pickFolders=async()=>{
  if(!nativeFolders()){folders.current?.click();return;}
  setPicking(true);try{const paths=await termPickFolders();if(!mounted.current){void termDropForget(paths).catch(()=>{});return;}road.add(paths.map(p=>pathAttachment(p,null,gui.machine_here,true,null,true)));}catch(e){if(mounted.current)setError(said(e));}finally{if(mounted.current)setPicking(false);}
 };
 const machineFor=(entry:Attachment)=>devices.find(d=>entry.from?d.name===entry.from:d.me);
 const browse=(entry:Attachment)=>{const machine=machineFor(entry);if(machine)setAt({machine,path:entry.device?'/':entry.path,root:entry.device?entry:undefined});};
 const send=async()=>{
  if(road.busy||sending.current||!road.selected.length)return;
  sending.current=true;setError(null);
  try{
   // A native picker or preview can have taken focus. Restore it during
   // the send press, while the browser can still open the soft keyboard.
   onBeforeSend?.();
   if(await road.send(road.selected,road.into)){if(mounted.current)finish(true);}
   else if(mounted.current)setError('附件未能发送，已保留所选文件，可重试');
  }catch(e){if(mounted.current)setError(said(e));}
  finally{sending.current=false;}
 };
 return <>
 {createPortal(<div ref={panel} data-attachment-picker data-files-panel style={{height,'--attachment-panel-height':height+'px'} as import('react').CSSProperties} className={cn("fixed z-40 flex min-h-0 flex-col gap-row border-t bg-(--canvas-solid) px-base pt-in pb-safe-bottom shadow-(--shadow-float)",(!terminalOverlay||keyboardBand>0)&&"bottom-0")} onMouseDown={event=>{
  if(!terminalOverlay||event.button!==0||!document.activeElement?.matches('.xterm-helper-textarea'))return;
  if(event.target instanceof Element&&!event.target.closest('input,textarea,select,[contenteditable=true]'))event.preventDefault();
 }} onKeyDown={e=>{if(e.key==='Escape'&&!road.busy){e.preventDefault();finish();}}}>
  <div data-files-actions className="flex min-h-touch flex-none items-center justify-between">
   <Button data-files-cancel variant="ghost" className="min-h-touch" disabled={road.stopping||(road.busy&&!road.cancel)} onClick={()=>road.busy?road.cancel?.():finish()}>{road.busy&&road.cancel?'取消传输':'取消'}</Button>
   <Button data-files-send compactTouch variant={road.selected.length?"primary":"default"} className={road.selected.length?"min-h-touch":"min-h-touch text-muted-foreground"} disabled={road.busy||picking||!road.selected.length} onClick={()=>void send()}>{road.busy?'正在发送':`发送${road.selected.length?' ('+road.selected.length+')':''}`}</Button>
  </div>
  <div data-attachment-split data-files-body className="flex min-h-0 flex-1 flex-col gap-row overflow-y-auto overscroll-contain pb-in">
   <section data-files-source className="flex flex-none flex-col gap-in" aria-labelledby={sourceTitle}>
    <h3 id={sourceTitle} className="text-headline">选择文件</h3>
    <div className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-row gap-y-in">
     <span className="text-callout text-muted-foreground">本机</span>
     <div data-files-local className="flex min-w-0 items-center gap-in">
      <Button data-files-pick="device" disabled={road.busy} className="min-h-touch gap-in text-callout" onClick={()=>picker.current?.click()}><IconFile className="size-kind-mark"/>文件</Button>
      {!road.filesOnly&&<Button data-files-pick="folder" disabled={road.busy||picking} className="min-h-touch gap-in text-callout" onClick={()=>void pickFolders()}><IconFiles className="size-kind-mark"/>目录</Button>}
     </div>
     <span className="text-callout text-muted-foreground">其他机器</span>
     <AttachmentMachines sources={sources} machine={road.machine} busy={road.busy} onPick={device=>setAt({machine:device,path:''})}/>
    </div>
   </section>
   {road.selected.length>0&&<section data-files-selection aria-labelledby={selectedTitle} className="flex flex-none flex-col gap-in">
    <h3 id={selectedTitle} className="flex items-center gap-in text-headline">已选文件<span className="tabular-nums text-muted-foreground">{road.selected.length}</span></h3>
    <div data-files-selection-scroll className="min-h-touch max-h-[calc(var(--attachment-panel-height)/2)] overflow-y-auto overscroll-contain rounded-md">
     <AttachmentList items={road.selected.map(entry=>({...entry,source:entry,detail:entry.shown}))} busy={road.busy} onOpen={key=>{const entry=road.selected.find(item=>item.key===key);if(entry)entry.dir?browse(entry):setLook(entry);}} onRemove={key=>{const entry=road.selected.find(item=>item.key===key);if(entry)toggle(entry);}}/>
    </div>
   </section>}
   <section data-files-destination className="flex flex-none flex-col gap-in" aria-label="发送到">
    <h3 className="text-headline">发送到</h3>
    <AttachmentDestination className="border-0" machine={road.machine} into={road.into??null} onInto={road.onInto??(()=>{})} busy={road.busy}/>
   </section>
  </div>
  <input data-files-input ref={picker} type="file" multiple hidden onChange={e=>{addDevice([...(e.target.files??[])],false);e.target.value='';}}/>
  <input data-files-folder-input ref={folders} type="file" multiple hidden {...{webkitdirectory:''}} onChange={e=>{addDevice([...(e.target.files??[])],true);e.target.value='';}}/>
  {error&&<p role="alert" className="text-caption text-destructive">{error}</p>}
  {road.busy&&<p data-files-transfer role="status" className="text-caption text-muted-foreground">{road.progress===null?'正在发送':`正在发送 · ${Math.round(road.progress*100)}%`}</p>}
 </div>,document.body)}
 <AttachmentDirectory place={at} onClose={()=>setAt(null)} selected={own} inherited={inherited} onToggle={toggle} onOpen={setLook} busy={road.busy} filesOnly={road.filesOnly}/>
 <AttachmentFile entry={look} devices={devices} onClose={()=>setLook(null)}/>
 </>;
}
/** Measure complete chip widths; only the suffix that cannot fit goes in More. */
function AttachmentMachines({sources,machine,busy,onPick}:{sources:DeviceRow[];machine:string|null;busy:boolean;onPick:(device:DeviceRow)=>void}){
 const row=useRef<HTMLDivElement>(null),more=useRef<HTMLButtonElement>(null);
 const [count,setCount]=useState(0);
 useLayoutEffect(()=>{
  const root=row.current;if(!root)return;
  let live=true;
  const measure=()=>{
   if(!live)return;
   const widths=[...root.querySelectorAll<HTMLElement>('[data-files-machine-chip]')].map(node=>node.getBoundingClientRect().width);
   const gap=parseFloat(getComputedStyle(root).columnGap)||0,room=root.clientWidth;
   if(widths.reduce((a,b)=>a+b,0)+Math.max(0,widths.length-1)*gap<=room){setCount(widths.length);return;}
   const reserve=(more.current?.getBoundingClientRect().width??0)+gap;
   let used=0,visible=0;
   for(const width of widths){const next=used+(visible?gap:0)+width;if(next+reserve>room)break;used=next;visible++;}
   setCount(visible);
  };
  const observer=new ResizeObserver(measure);observer.observe(root);
  for(const node of root.querySelectorAll<HTMLElement>('button'))observer.observe(node);
  measure();void document.fonts.ready.then(measure);
  return()=>{live=false;observer.disconnect();};
 },[sources]);
 const hidden=sources.slice(count);
 return <div ref={row} data-files-machine-row data-files-source-count={sources.length} className="relative flex min-h-touch min-w-0 items-center gap-in overflow-hidden">
  {!sources.length&&<span className="text-callout text-muted-foreground">暂无其他机器</span>}
  {sources.map((device,index)=><Button key={device.id} data-files-machine-chip={device.name} aria-hidden={index>=count||undefined} tabIndex={index>=count?-1:undefined} title={device.shown} disabled={busy} className={cn('w-max min-h-touch max-w-nav-menu min-w-0 flex-none gap-in text-callout',index>=count&&'invisible pointer-events-none absolute left-0 top-0')} onClick={()=>onPick(device)}><MachineAvatar face={device.face} className="size-kind-mark flex-none"/><span className="truncate">{device.shown}</span></Button>)}
  <DropdownMenu modal={false}>
   <DropdownMenuTrigger asChild><Button ref={more} data-files-machines data-files-machine-count={hidden.length} aria-label={`更多机器，${hidden.length} 台`} aria-hidden={!hidden.length||undefined} tabIndex={hidden.length?undefined:-1} variant="ghost" disabled={busy||!hidden.length} className={cn('size-touch flex-none p-0',!hidden.length&&'invisible pointer-events-none absolute left-0 top-0')}><IconMore className="size-affordance"/></Button></DropdownMenuTrigger>
   <DropdownMenuContent data-files-machines-menu align="end" aria-label="选择来源机器" className="min-w-nav-menu max-w-[calc(100vw-2*var(--gap-row))]">
    {hidden.map(device=><DropdownMenuItem key={device.id} data-files-machine={device.name} data-files-session={device.name===machine||undefined} onSelect={()=>onPick(device)}><MachineAvatar face={device.face} className="size-kind-mark flex-none"/><span className="min-w-0 flex-1 truncate">{device.shown}</span>{device.name===machine?<span className="text-caption text-muted-foreground">{gui.files_session}</span>:device.hop==='quiet'&&isDown(device.down_ms)&&<span className="text-caption text-muted-foreground">{hopWord('down')}</span>}</DropdownMenuItem>)}
   </DropdownMenuContent>
  </DropdownMenu>
 </div>;
}
function localEntry(root:Attachment,path:string,dir:boolean):Attachment{
 const relative=path.replace(/^\//,'');const name=relative.split('/').pop()??root.name;
 if(!relative)return root;
 const file=root.device!.files.find(item=>item.relative===relative)?.file;
 const prefix=relative+'/';
 return {key:root.key+'/'+relative,name,path,from:root.from,shown:root.shown,dir,size:file?.size??null,device:dir?{files:root.device!.files.filter(item=>item.relative.startsWith(prefix)).map(item=>({...item,relative:item.relative.slice(prefix.length)})),dirs:root.device!.dirs.filter(item=>item.startsWith(prefix)).map(item=>item.slice(prefix.length))}: {files:file?[{file,relative:name}]:[],dirs:[]}};
}
function AttachmentDirectory({place,onClose,selected,inherited,onToggle,onOpen,busy,filesOnly}:{place:Place|null;onClose:()=>void;selected:(a:Attachment)=>boolean;inherited:(a:Attachment)=>boolean;onToggle:(a:Attachment)=>void;onOpen:(a:Attachment)=>void;busy:boolean;filesOnly?:boolean}){
 return <ContentOverlay open={place!==null} title={place?place.machine.shown+'文件':''} onClose={onClose} data-attachment-directory>{place&&
  <DirectoryBody key={place.machine.id+' '+(place.root?.key??'')} place={place} onClose={onClose} selected={selected} inherited={inherited} onToggle={onToggle} onOpen={onOpen} busy={busy} filesOnly={filesOnly}/>
 }</ContentOverlay>;
}
function DirectoryBody({place,onClose,selected,inherited,onToggle,onOpen,busy,filesOnly}:Omit<Parameters<typeof AttachmentDirectory>[0],'place'>&{place:Place}){
 const [path,setPath]=useState(place.path),[query,setQuery]=useState(place.path?place.path.replace(/\/$/,'')+'/':''),[filter,setFilter]=useState('');
 const navigate=(path:string)=>{setPath(path);setQuery(path.replace(/\/$/,'')+'/');setFilter('');};
 const entry=(full:string,dir:boolean,size:number|null=null)=>place.root?localEntry(place.root,full,dir):pathAttachment(full,place.machine.me?null:place.machine.name,place.machine.shown,dir,size);
 const listing=useMemo<DirListing|undefined>(()=>place.root?{path,entries:deviceFolderEntries(place.root,path.replace(/^\//,'')).map(row=>({name:row.name,dir:row.dir,size:row.file?.size??0,at_ms:row.file?.lastModified??0,from:null,received_ms:null})),truncated:false}:undefined,[place.root,path]);
 return <div data-files-directory-body className="flex min-h-0 flex-1 flex-col">
  <div className="flex min-h-0 flex-1 flex-col"><FilesPane machine={place.machine.name} here={place.machine.me} path={path} onPath={navigate} onLanded={resolved=>{if(!query)navigate(resolved);}} onFile={()=>{}} open={null} sourceListing={listing} filter={filter} selection={{filesOnly,state:(full,dir)=>{const item=entry(full,dir);return {selected:selected(item)||inherited(item),disabled:busy||inherited(item)};},toggle:(full,dir,size)=>onToggle(entry(full,dir,size)),open:(full,dir)=>onOpen(entry(full,dir))}}/></div>
  <ContentDeck data-files-directory-deck>
   <GlassCircle aria-label="返回附件" data-files-directory-back onClick={onClose}><IconBack/></GlassCircle>
   <ContentPill data-files-search><ContentSearch leading={<PillFace data-files-directory-avatar role="img" aria-label={place.machine.shown} title={place.machine.shown}><MachineFace face={place.machine.face} system={place.machine.vitals?.vitals.system??null} size="tab"/></PillFace>} label="搜索或输入路径" value={query} autoFocus={false} onEditing={()=>{}} onChange={value=>{setQuery(value);const split=value.lastIndexOf('/');if(split>=0){setPath(value.slice(0,split)||'/');setFilter(value.slice(split+1));}else setFilter(value);}} onEnter={()=>{if(query.endsWith('/'))navigate(query);if(document.activeElement instanceof HTMLElement)document.activeElement.blur();}}/></ContentPill>
  </ContentDeck>
 </div>;
}
/** Destination is contextual to the open conversation, not another form. */
export function AttachmentDestination({machine,into,onInto,busy=false,className}:{machine:string|null;into:string|null;onInto:(path:string|null)=>void;busy?:boolean;className?:string}){
 const [target,setTarget]=useState<DeviceRow|null>(null);
 useEffect(()=>{let live=true;void fetchDevices().then(rows=>{if(live)setTarget(rows.find(row=>machine?row.name===machine:row.me)??null);},()=>{});return()=>{live=false;};},[machine]);
 return <SendInto machine={machine??''} name={target?.name??machine??''} into={into} onInto={onInto} render={({path,pick,reset})=><div data-attachment-destination className={cn("flex w-full min-w-0 flex-none items-center gap-in border-t",className)}>
  <div data-send-to-machine role="group" aria-label={`发送到 ${target?.shown??machine??'会话所在机器'}，保存文件夹：${path}`} title={`${target?.shown??machine??'会话所在机器'}：${path}`} className="flex min-w-0 flex-1 items-center gap-xxs text-callout text-muted-foreground">
   <MachineAvatar face={target?.face??null} className="size-kind-mark flex-none"/>
   <span aria-hidden="true">：</span><span data-send-into-path className="min-w-0 flex-1 truncate">{path}</span>
  </div>
  <Button variant="ghost" className="min-h-touch flex-none" data-send-into-change aria-label="更改附件保存位置" disabled={busy} onClick={pick}>更改</Button>
  {reset&&<Button variant="ghost" className="min-h-touch flex-none" data-send-into-reset disabled={busy} onClick={reset}>恢复默认</Button>}
 </div>}/>;
}
