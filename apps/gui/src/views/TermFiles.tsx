/** A keyboard-position selection panel, with the existing file browser/pages above it. */
import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {fetchDevices,nativeFolders,said,termDropForget,termPickFolders,type DeviceRow} from '@/api';
import {MachineAvatar} from '@/components/Avatar';
import {IconBack,IconFile,IconFiles,IconChevronDown,IconDevices} from '@/components/icons';
import {ContentSearch} from '@/components/ContentSearch';
import {MachineIdentity} from '@/components/MachineIdentity';
import {Button} from '@/components/ui/button';
import {DropdownMenu,DropdownMenuTrigger,DropdownMenuContent,DropdownMenuItem} from '@/components/ui/dropdown-menu';
import {Dialog,TaskOverlay,DialogTitle} from '@/components/ui/dialog';
import {GlassCapsule,GlassCircle} from '@/components/Glass';
import {gui} from '@/gen/catalog';
import type {DirListing} from '@/gen/bindings/DirListing';
import {deviceAttachments,deviceFolderEntries,includesAttachment,pathAttachment,type Attachment} from '@/lib/term-attachments';
import {keyboardCover,keyboardHeight,holdFoot,footHeld} from '@/lib/viewport';
import {lowerKeyboard,raiseKeyboard} from '@/lib/soft-keyboard';
import {isDown} from '@/lib/topology';
import {hopWord} from '@/words';
import {useNarrow} from '@/hooks/use-narrow';
import {FilesPane,FileChoiceRow} from '@/views/FilesPane';
import {AttachmentFile} from '@/views/AttachmentFile';
import {SendInto} from '@/views/chat-files';
export type FilesRoad = {
  machine: string | null; cwd: string | null;
  selected: Attachment[];
  add: (entries: Attachment[]) => void;
  remove: (entry: Attachment) => void;
  replace?: (entry: Attachment, file: File) => void;
  open?: () => void; opened?: boolean;
  into?: string | null; onInto?: (path: string | null) => void;
  filesOnly?: boolean; action?: string;
  send: (entries: Attachment[], into?: string | null) => Promise<boolean>;
  dropFiles: (files: File[]) => void;
  busy: boolean; stopping?: boolean; progress: number | null; cancel: () => void;
};
export function machinesFor(devices: DeviceRow[], machine: string | null): DeviceRow[] {
  const ours = (d: DeviceRow) => (machine === null ? d.me : d.name === machine);
  const hosting = devices.filter((d) => d.hosts);
  return [...hosting.filter(ours), ...hosting.filter((d) => !ours(d))];
}

type Place={machine:DeviceRow;path:string;root?:Attachment};
export function AttachmentPicker({open,onClose,road,onSent}:{open:boolean;onClose:()=>void;road:FilesRoad;onSent:()=>void}){
 return open?<TermFiles road={road} onClose={onClose} onSent={onSent}/>:null;
}
export function TermFiles({road,onClose,onSent}:{road:FilesRoad;onClose:()=>void;onSent:()=>void}){
 const narrow=useNarrow();
 const picker=useRef<HTMLInputElement>(null),folders=useRef<HTMLInputElement>(null),panel=useRef<HTMLDivElement>(null);
 const [devices,setDevices]=useState<DeviceRow[]>([]),[error,setError]=useState<string|null>(null),[picking,setPicking]=useState(false);
 const [at,setAt]=useState<Place|null>(null),[look,setLook]=useState<Attachment|null>(null);
 const sending=useRef(false),mounted=useRef(true);
 const restore=useRef<HTMLElement|null>(null),oldHold=useRef<number|null>(null);
 const [height]=useState(()=>Math.min(keyboardCover()||footHeld()||keyboardHeight(),window.innerHeight*0.6));
 useLayoutEffect(()=>{
  const field=document.activeElement;oldHold.current=footHeld();
  // Move chrome to the keyboard's edge while the terminal retains its grid.
  // The terminal's fit guard covers this hold and every nested content page.
  const covered=keyboardCover()>0;
  if(narrow||matchMedia('(pointer: coarse)').matches)holdFoot(height);
  if(covered&&field instanceof HTMLElement){restore.current=field;lowerKeyboard(field);}
  else if(field instanceof HTMLElement&&field.matches('input,textarea'))field.blur();
  return()=>{holdFoot(oldHold.current);};
 },[]);
 const finish=(sent=false)=>{if(restore.current?.isConnected)raiseKeyboard(restore.current);sent?onSent():onClose();};
 useEffect(()=>{mounted.current=true;let live=true;void fetchDevices().then(rows=>{if(live)setDevices(rows);},e=>{if(live)setError(said(e));});return()=>{live=false;mounted.current=false;};},[]);
 const sources=machinesFor(devices,road.machine);
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
 const send=async()=>{if(road.busy||sending.current||!road.selected.length)return;sending.current=true;setError(null);try{if(await road.send(road.selected,road.into)){if(mounted.current)finish(true);}else if(mounted.current)setError('附件未能发送，已保留所选文件，可重试');}catch(e){if(mounted.current)setError(said(e));}finally{sending.current=false;}};
 return <>
 {createPortal(<div ref={panel} data-attachment-picker data-files-panel style={{height}} className="fixed inset-x-0 bottom-0 z-40 flex min-h-0 flex-col border-t bg-background px-row pb-safe-bottom shadow-(--shadow-float) sm:left-auto sm:right-row sm:w-dialog" onKeyDown={e=>{if(e.key==='Escape'&&!road.busy){e.preventDefault();finish();}}}>
  <div data-files-actions className="flex min-h-touch flex-none items-center justify-between py-xxs">
   <Button data-files-cancel variant="ghost" className="min-h-touch" disabled={road.stopping} onClick={()=>road.busy?road.cancel():finish()}>{road.busy?'取消传输':'取消'}</Button>
   <Button data-files-send variant="primary" className={road.selected.length?"min-h-touch":"min-h-touch bg-fill text-muted-foreground"} disabled={road.busy||picking||!road.selected.length} onClick={()=>void send()}>{road.busy?'正在发送':`发送${road.selected.length?' ('+road.selected.length+')':''}`}</Button>
  </div>
  <div data-attachment-split className="flex min-h-0 flex-1 flex-col">
   <section data-files-source className="flex flex-none flex-col" aria-label="选择文件">
    <div className="flex gap-in"><Button data-files-pick="device" disabled={road.busy} className="min-h-touch flex-1" onClick={()=>picker.current?.click()}><span className="grid size-affordance place-items-center [&>svg]:size-affordance"><IconFile/></span>本机文件</Button>{!road.filesOnly&&<Button data-files-pick="folder" disabled={road.busy||picking} className="min-h-touch flex-1" onClick={()=>void pickFolders()}><span className="grid size-affordance place-items-center [&>svg]:size-affordance"><IconFiles/></span>本机文件夹</Button>}</div>
    <DropdownMenu modal={false}>
     <DropdownMenuTrigger asChild><Button data-files-machines data-files-machine-count={sources.length} variant="ghost" disabled={road.busy||!sources.length} className="min-h-touch w-full justify-start"><span className="grid size-affordance flex-none place-items-center [&>svg]:size-affordance"><IconDevices/></span><span className="min-w-0 flex-1 text-left">其他机器</span><span className="tabular-nums text-muted-foreground">{sources.length} 台</span><IconChevronDown className="size-affordance"/></Button></DropdownMenuTrigger>
     <DropdownMenuContent data-files-machines-menu align="start" aria-label="选择来源机器" className="min-w-(--radix-dropdown-menu-trigger-width)">
      {sources.map(d=><DropdownMenuItem key={d.id} data-files-machine={d.name} data-files-session={(road.machine===null?d.me:d.name===road.machine)||undefined} onSelect={()=>setAt({machine:d,path:''})}>
       <MachineAvatar face={d.face} className="size-kind-mark flex-none"/><span className="min-w-0 flex-1 truncate">{d.shown}</span>{(road.machine===null?d.me:d.name===road.machine)?<span className="text-caption text-muted-foreground">{gui.files_session}</span>:!d.me&&d.hop==='quiet'&&isDown(d.down_ms)&&<span className="text-caption text-muted-foreground">{hopWord('down')}</span>}
      </DropdownMenuItem>)}
     </DropdownMenuContent>
    </DropdownMenu>
   </section>
   <section data-files-selection aria-label="已选附件" className="flex min-h-0 flex-1 flex-col border-t">
    <div data-files-selection-scroll className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
     {road.selected.length?road.selected.map(entry=><div key={entry.key} data-files-selected={entry.name}><FileChoiceRow name={entry.name} dir={entry.dir} size={entry.size} detail={entry.shown} selected disabled={road.busy} onChoose={()=>toggle(entry)} onEnter={()=>entry.dir?browse(entry):toggle(entry)} onOpen={!entry.dir?()=>setLook(entry):undefined}/></div>):<p className="py-in text-callout text-muted-foreground">尚未选择文件或目录</p>}
    </div>
   </section>
  </div>
  <input data-files-input ref={picker} type="file" multiple hidden onChange={e=>{addDevice([...(e.target.files??[])],false);e.target.value='';}}/>
  <input data-files-folder-input ref={folders} type="file" multiple hidden {...{webkitdirectory:''}} onChange={e=>{addDevice([...(e.target.files??[])],true);e.target.value='';}}/>
  <AttachmentDestination machine={road.machine} into={road.into??null} onInto={road.onInto??(()=>{})} busy={road.busy}/>
  {error&&<p role="alert" className="text-caption text-destructive">{error}</p>}
  {road.busy&&<p data-files-transfer role="status" className="text-caption text-muted-foreground">正在发送 · {Math.round((road.progress??0)*100)}%</p>}
 </div>,document.body)}
 <AttachmentDirectory place={at} onClose={()=>setAt(null)} selected={own} inherited={inherited} onToggle={toggle} onOpen={setLook} busy={road.busy} filesOnly={road.filesOnly}/>
 <AttachmentFile entry={look} devices={devices} onClose={()=>setLook(null)}/>
 </>;
}
function localEntry(root:Attachment,path:string,dir:boolean):Attachment{
 const relative=path.replace(/^\//,'');const name=relative.split('/').pop()??root.name;
 if(!relative)return root;
 const file=root.device!.files.find(item=>item.relative===relative)?.file;
 const prefix=relative+'/';
 return {key:root.key+'/'+relative,name,path,from:root.from,shown:root.shown,dir,size:file?.size??null,device:dir?{files:root.device!.files.filter(item=>item.relative.startsWith(prefix)).map(item=>({...item,relative:item.relative.slice(prefix.length)})),dirs:root.device!.dirs.filter(item=>item.startsWith(prefix)).map(item=>item.slice(prefix.length))}: {files:file?[{file,relative:name}]:[],dirs:[]}};
}
function AttachmentDirectory({place,onClose,selected,inherited,onToggle,onOpen,busy,filesOnly}:{place:Place|null;onClose:()=>void;selected:(a:Attachment)=>boolean;inherited:(a:Attachment)=>boolean;onToggle:(a:Attachment)=>void;onOpen:(a:Attachment)=>void;busy:boolean;filesOnly?:boolean}){
 return <Dialog open={place!==null} onOpenChange={open=>{if(!open)onClose();}}>{place&&<TaskOverlay layout="content" purpose="preview" initialHeight="large" maxHeight="large" data-attachment-directory className="flex min-h-0 flex-col gap-0 overflow-hidden p-0" onOpenAutoFocus={event=>{event.preventDefault();(event.target as HTMLElement).focus({preventScroll:true});}} onCloseAutoFocus={event=>event.preventDefault()}>
  <DialogTitle className="sr-only">{place.machine.shown}文件</DialogTitle>
  <DirectoryBody key={place.machine.id+' '+(place.root?.key??'')} place={place} onClose={onClose} selected={selected} inherited={inherited} onToggle={onToggle} onOpen={onOpen} busy={busy} filesOnly={filesOnly}/>
 </TaskOverlay>}</Dialog>;
}
function DirectoryBody({place,onClose,selected,inherited,onToggle,onOpen,busy,filesOnly}:Omit<Parameters<typeof AttachmentDirectory>[0],'place'>&{place:Place}){
 const [path,setPath]=useState(place.path),[query,setQuery]=useState(place.path?place.path.replace(/\/$/,'')+'/':''),[filter,setFilter]=useState('');
 const navigate=(path:string)=>{setPath(path);setQuery(path.replace(/\/$/,'')+'/');setFilter('');};
 const entry=(full:string,dir:boolean,size:number|null=null)=>place.root?localEntry(place.root,full,dir):pathAttachment(full,place.machine.me?null:place.machine.name,place.machine.shown,dir,size);
 const listing=useMemo<DirListing|undefined>(()=>place.root?{path,entries:deviceFolderEntries(place.root,path.replace(/^\//,'')).map(row=>({name:row.name,dir:row.dir,size:row.file?.size??0,at_ms:row.file?.lastModified??0,from:null,received_ms:null})),truncated:false}:undefined,[place.root,path]);
 return <div data-files-directory-body className="flex min-h-0 flex-1 flex-col">
  <header data-files-directory-head className="flex min-h-touch flex-none items-center gap-row px-row py-in"><MachineIdentity row={place.machine}/></header>
  <div className="flex min-h-0 flex-1 flex-col"><FilesPane machine={place.machine.name} here={place.machine.me} path={path} onPath={navigate} onLanded={resolved=>{if(!query)navigate(resolved);}} onFile={()=>{}} open={null} sourceListing={listing} filter={filter} selection={{filesOnly,state:(full,dir)=>{const item=entry(full,dir);return {selected:selected(item)||inherited(item),disabled:busy||inherited(item)};},toggle:(full,dir,size)=>onToggle(entry(full,dir,size)),open:(full,dir)=>onOpen(entry(full,dir))}}/></div>
  <div data-files-directory-deck className="flex min-w-0 flex-none items-center gap-in px-row py-in pb-safe-bottom">
   <GlassCircle aria-label="返回附件" data-files-directory-back onClick={onClose}><IconBack/></GlassCircle>
   <GlassCapsule data-files-search className="min-w-0 flex-1"><ContentSearch label="搜索或输入路径" value={query} autoFocus={false} onEditing={()=>{}} onChange={value=>{setQuery(value);const split=value.lastIndexOf('/');if(split>=0){setPath(value.slice(0,split)||'/');setFilter(value.slice(split+1));}else setFilter(value);}} onEnter={()=>{if(query.endsWith('/'))navigate(query);if(document.activeElement instanceof HTMLElement)document.activeElement.blur();}}/></GlassCapsule>
  </div>
 </div>;
}
/** Destination is contextual to the open conversation, not another form. */
export function AttachmentDestination({machine,into,onInto,busy=false}:{machine:string|null;into:string|null;onInto:(path:string|null)=>void;busy?:boolean}){
 const [target,setTarget]=useState<DeviceRow|null>(null);
 useEffect(()=>{let live=true;void fetchDevices().then(rows=>{if(live)setTarget(rows.find(row=>machine?row.name===machine:row.me)??null);},()=>{});return()=>{live=false;};},[machine]);
 return <SendInto machine={machine??''} name={target?.name??machine??''} into={into} onInto={onInto} render={({path,pick,reset})=><div data-attachment-destination className="flex w-full min-w-0 flex-none items-center gap-in border-t">
  <div data-send-to-machine role="group" aria-label={`发送到 ${target?.shown??machine??'会话所在机器'}，保存文件夹：${path}`} title={`${target?.shown??machine??'会话所在机器'}：${path}`} className="flex min-w-0 flex-1 items-center gap-xxs text-callout text-muted-foreground">
   <MachineAvatar face={target?.face??null} className="size-kind-mark flex-none"/>
   <span aria-hidden="true">：</span><span data-send-into-path className="min-w-0 flex-1 truncate">{path}</span>
  </div>
  <Button variant="ghost" className="min-h-touch flex-none" data-send-into-change aria-label="更改附件保存位置" disabled={busy} onClick={pick}>更改</Button>
  {reset&&<Button variant="ghost" className="min-h-touch flex-none" data-send-into-reset disabled={busy} onClick={reset}>恢复默认</Button>}
 </div>}/>;
}
