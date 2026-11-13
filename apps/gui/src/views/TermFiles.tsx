/** Shared source browser and attachment draft. Only the task submit
 * sends its complete selection; previews cannot bypass that draft. */
import { useCallback, useEffect, useRef, useState } from "react";
import { fetchDevices, fetchLs, nativeFolders, said, termDropForget, termPickFolders, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu } from "@/components/RowMenu";
import { IconBack, IconCheck, IconFile, IconFiles, IconChevronDown } from "@/components/icons";
import { FileKindIcon } from "@/components/FileKindIcon";
import { Button } from "@/components/ui/button";
import { gui, cli } from "@/gen/catalog";
import type { DirListing } from "@/gen/bindings/DirListing";
import { deviceAttachments, deviceFolderEntries, includesAttachment, joinAttachmentPath, parentAttachmentPath, pathAttachment, type Attachment } from "@/lib/term-attachments";
import { cn } from "@/lib/utils";
import { isDown } from "@/lib/topology";
import { hopWord } from "@/words";
import { AttachmentPreview } from "@/views/AttachmentPreview";
import { AttachmentList } from "@/views/AttachmentList";
import { Dialog, TaskOverlay, DialogTitle, TaskActions } from "@/components/ui/dialog";
import { FilePreview, SendInto } from "@/views/chat-files";

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
type Place = { machine: DeviceRow; path: string; listing: DirListing | null; error?: string };
/** `face` left out: the file's own default (`defaultFaceOf`). */
type Look = { entry: Attachment; machine: DeviceRow; path?: string; face?: "source" | "preview" };
/** One retained task: source browsing above, selected rows below. */
export function AttachmentPicker({open,onClose,road,onSent}:{open:boolean;onClose:()=>void;road:FilesRoad;onSent:()=>void}){
 return <Dialog open={open} onOpenChange={value=>{if(!value&&!road.busy)onClose();}}><TaskOverlay purpose="edit" initialHeight="large" maxHeight="large" data-attachment-picker onOpenAutoFocus={event=>event.preventDefault()}>
  <TermFiles road={road} onClose={onClose} onSent={onSent}/>
 </TaskOverlay></Dialog>;
}
export function TermFiles({ road, onClose, onSent }: { road: FilesRoad; onClose: () => void; onSent: () => void }) {
  const picker = useRef<HTMLInputElement>(null), folders = useRef<HTMLInputElement>(null);
  const [devices, setDevices] = useState<DeviceRow[]>([]);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const deviceGeneration = useRef(0);
  const [at, setAt] = useState<Place | null>(null);
  const [look, setLook] = useState<Look | null>(null);
  const closePreview=()=>setLook(null);
  const [error,setError]=useState<string|null>(null);
  const [picking, setPicking] = useState(false);
  const generation = useRef(0), mounted = useRef(false), sending=useRef(false);
  const trail = useRef<(Place|null)[]>([]);
  const refreshDevices = useCallback(() => {
    const mark = ++deviceGeneration.current;
    void fetchDevices().then((d) => {
      if (mounted.current && mark === deviceGeneration.current) { setDevices(d); setSourceError(null); }
    }).catch((e: unknown) => { if (mounted.current && mark === deviceGeneration.current) setSourceError(said(e)); });
  }, []);
  useEffect(() => {
    mounted.current = true;
    refreshDevices();
    const visible = () => { if (document.visibilityState === "visible") refreshDevices(); };
    window.addEventListener("focus", visible);
    document.addEventListener("visibilitychange", visible);
    return () => {
      mounted.current = false; generation.current++; deviceGeneration.current++;
      window.removeEventListener("focus", visible);
      document.removeEventListener("visibilitychange", visible);
    };
  }, [refreshDevices]);
  const sources = !at;
  useEffect(() => { if (sources) refreshDevices(); }, [sources, refreshDevices]);
  const open = (machine: DeviceRow, path: string, remember = true) => {
    if (remember) trail.current.push(at);
    const mark = ++generation.current;
    setAt({ machine, path, listing: null });
    void fetchLs(machine.name, path).then((listing) => {
      if (mark === generation.current) setAt({ machine, path: listing.path, listing });
    }).catch((e: unknown) => { if (mark === generation.current) setAt({ machine, path, listing: null, error: said(e) }); });
  };
  const selected = (e: Attachment) => road.selected.some((x) => x.key === e.key);
  const included = (e: Attachment) => road.selected.some((x) => includesAttachment(x, e));
  const toggle = (e: Attachment) => {
    if (road.busy || included(e)) return;
    if (selected(e)) road.remove(e); else road.add([e]);
  };
  const addDevice = (files: File[], dirs: boolean) => {
    if (!files.length || road.busy) return;
    try { road.add(deviceAttachments(files, dirs, gui.machine_here)); setError(null); }
    catch (e) { setError(said(e)); }
  };
  const pickFolders = async () => {
    setError(null);
    if (!nativeFolders()) {
      if (folders.current && "webkitdirectory" in folders.current) folders.current.click();
      else setError(gui.files_browser_dir);
      return;
    }
    setPicking(true);
    try {
      const paths = await termPickFolders();
      if (!mounted.current) { void termDropForget(paths).catch(() => {}); return; }
      road.add(paths.map((p) => pathAttachment(p, null, gui.machine_here, true, null, true)));
    } catch (e) { if (mounted.current) setError(said(e)); }
    finally { if (mounted.current) setPicking(false); }
  };
  const send = async (entries: Attachment[]) => {
    if (road.busy || sending.current || !entries.length) return;
    sending.current=true;setError(null);closePreview();
    try{
      const ok = await road.send(entries,road.into);
      if(!ok&&mounted.current)setError("附件未能发送，已保留所选文件，可重试");
      if (!mounted.current) return;
      if (ok) onSent();
    }catch(error){if(mounted.current)setError(said(error));}
    finally{sending.current=false;}
  };
  const back=()=>{generation.current++;setAt(trail.current.pop()??null);};
  const entries=at?.listing?.entries??[];
  return <div data-files-panel className="flex min-h-0 flex-1 flex-col">
    <DialogTitle>附件</DialogTitle>
    <TaskActions close={!road.busy?<Button data-files-cancel onClick={onClose}>关闭</Button>:undefined}
      cancel={road.busy?<Button data-files-cancel disabled={road.stopping} onClick={road.cancel}>{road.stopping?'正在取消':'取消传输'}</Button>:undefined}
      primary={<Button data-files-send disabled={road.busy||!road.selected.length||picking} variant="primary" className={!road.selected.length||road.busy?"bg-fill text-muted-foreground":undefined} onClick={()=>void send(road.selected)}>{road.busy?'正在发送':`${road.action??'发送'}${road.selected.length?' ('+road.selected.length+')':''}`}</Button>}/>
    <div data-attachment-split inert={road.busy||undefined} className="flex min-h-0 flex-1 flex-col">
     <section data-files-source data-files-source-path={at?.path??""} aria-label="选择文件" className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-touch flex-none items-center gap-in">
       {at?<><button type="button" data-files-up aria-label="返回来源" onClick={back} className="grid min-h-touch min-w-touch place-items-center"><IconBack className="size-5"/></button><span className="min-w-0 flex-1 truncate text-callout">{at.machine.shown+' · '+at.path}</span><Button data-files-pick="device" size="sm" onClick={()=>picker.current?.click()}>本机</Button></>:<h3 className="text-body font-label">选择来源</h3>}
      </div>
      <div data-files-source-scroll className="min-h-0 flex-1 touch-pan-y overflow-y-auto overscroll-contain">
      {!at ? <>
        <div className="flex gap-in pb-row"><Button data-files-pick="device" disabled={road.busy} className="min-h-touch flex-1" onClick={()=>picker.current?.click()}><span data-files-pick-glyph className="grid size-affordance place-items-center [&>svg]:size-affordance"><IconFile/></span>本机</Button>{!road.filesOnly&&<Button data-files-pick="folder" disabled={road.busy||picking} className="min-h-touch flex-1" onClick={()=>void pickFolders()}><IconFiles className="size-affordance"/>{picking?gui.files_reading_folder:gui.ws_kind_folder}</Button>}</div>
        <section data-files-mesh><h3 className="pb-in text-group text-muted-foreground">{gui.devices_tab}</h3>
          {machinesFor(devices, road.machine).filter(d=>!road.filesOnly||d.me||d.name===road.machine).map((d) => <button key={d.id} type="button" data-files-machine={d.name} data-files-session={(road.machine === null ? d.me : d.name === road.machine) || undefined} onClick={() => open(d, "")} className="flex min-h-touch w-full items-center gap-row border-t text-left text-body"><MachineAvatar face={d.face} className="size-kind-mark flex-none" /><span className="min-w-0 flex-1 truncate">{d.shown}</span>{!d.me && d.hop === "quiet" && isDown(d.down_ms) && <span data-files-unreachable className="flex-none text-caption text-muted-foreground">{hopWord("down")}</span>}{(road.machine === null ? d.me : d.name === road.machine) && <span className="text-caption text-muted-foreground">{gui.files_session}</span>}</button>)}
          {sourceError && <div className="py-in text-callout"><p role="alert">{sourceError}</p><Button onClick={refreshDevices}>{gui.region_retry}</Button></div>}
        </section>
      </> : at.error ? <div className="py-row text-callout"><p role="alert">{gui.files_failed_list(at.error)}</p><Button onClick={() => open(at.machine, at.path, false)}>{gui.region_retry}</Button></div> : !at.listing ? <p role="status" className="py-pane text-center text-callout text-muted-foreground">{gui.files_reading_folder}</p> : <>
        {parentAttachmentPath(at.listing.path) && <button type="button" data-files-parent onClick={() => open(at.machine, parentAttachmentPath(at.listing!.path)!)} className="flex min-h-touch w-full items-center gap-row border-b py-in text-left text-body"><IconBack className="size-kind-mark flex-none rotate-90" /><span>{gui.up_dir}</span></button>}
        {entries.length === 0 && <p className="py-pane text-center text-callout text-muted-foreground">{gui.empty_dir}</p>}
        {entries.map((row) => {
          const entry = pathAttachment(joinAttachmentPath(at.listing!.path, row.name), at.machine.me ? null : at.machine.name, at.machine.shown, row.dir, row.dir ? null : row.size);
          const own = selected(entry), inherited = included(entry);
          const view = (face?: "source" | "preview") => {
            closePreview();
            setLook({ entry, machine: at.machine, face });
          };
          return <RowMenu key={entry.key} items={[
            ...(entry.dir ? [] : [{ key: "preview", label: gui.file_face_preview, run: () => view("preview") }]),
            ...(!inherited && !road.busy && (!road.filesOnly || !entry.dir) ? [{ key: "select", label: own ? gui.files_deselect : gui.files_select, run: () => toggle(entry) }] : []),
            { key: "copy", label: gui.tab_copy_path, run: () => { void navigator.clipboard.writeText(entry.path).catch((e: unknown) => setError(said(e))); } },
          ]}>
            <div data-files-entry={entry.dir ? "dir" : "file"} data-files-name={entry.name} className={cn("flex min-h-touch items-center border-b", own && "bg-fill-strong")}>
              <button type="button" onClick={() => entry.dir ? open(at.machine, entry.path) : toggle(entry)} className="flex min-h-touch min-w-0 flex-1 items-center gap-row py-in text-left"><Mark entry={entry} /><span className="min-w-0 flex-1 truncate text-body">{entry.name}</span>{entry.size !== null && <span className="text-caption tabular-nums text-muted-foreground">{sizeWord(entry.size)}</span>}</button>
              <button type="button" data-files-select={entry.name} aria-label={`${own ? gui.files_deselect : gui.files_select} ${entry.name}`} aria-pressed={own || inherited} disabled={inherited || road.busy || (road.filesOnly && entry.dir)} title={inherited ? gui.files_included : undefined} onClick={() => toggle(entry)} className="grid min-h-touch min-w-touch place-items-center disabled:opacity-40"><span className={cn("grid size-5 place-items-center rounded-full border", (own || inherited) && "bg-fill-strong")}>{(own || inherited) && <IconCheck className="size-3.5" />}</span></button>
            </div>
          </RowMenu>;
        })}
        {at.listing.truncated && <p role="status" className="py-in text-callout text-muted-foreground">{cli.dir_truncated(at.listing.entries.length)}</p>}
      </>}
    </div>
     </section>
     <section data-files-selection aria-label="已选附件" className="flex min-h-0 flex-1 flex-col border-t">
      <h3 data-files-count className="flex min-h-touch flex-none items-center text-body font-label">已选 {road.selected.length} 项</h3>
      <div data-files-selection-scroll className="min-h-0 flex-1 touch-pan-y overflow-y-auto overscroll-contain">
       {road.selected.length?<AttachmentDraftRows road={road}/>:<p className="py-in text-callout text-muted-foreground">尚未选择文件或目录</p>}
      </div>
     </section>
    </div>
    <input data-files-input ref={picker} type="file" multiple hidden onChange={event=>{addDevice([...(event.target.files??[])],false);event.target.value="";}}/>
    <input ref={folders} type="file" multiple hidden {...{webkitdirectory:""}} onChange={event=>{addDevice([...(event.target.files??[])],true);event.target.value="";}}/>
    <AttachmentDestination machine={road.machine} into={road.into??null} onInto={road.onInto??(()=>{})} busy={road.busy}/>
    {error&&<p role="alert" className="text-callout text-destructive">{error}</p>}
    {road.busy&&<p data-files-transfer role="status" className="text-caption tabular-nums text-muted-foreground">正在发送 · {Math.round((road.progress??0)*100)}%</p>}
    <FilePreview machine={look?.machine.name ?? ""} shown={look?.machine.shown ?? ""} here={look?.machine.me ?? false} path={look?.path??look?.entry.path ?? null} initialFace={look?.face} onClose={closePreview} footer={look && <><Button disabled={road.busy || included(look.entry)} onClick={() => toggle(look.entry)}>{selected(look.entry) ? gui.files_deselect : gui.files_select}</Button></>} />
  </div>;
}
function Mark({ entry }: { entry: Attachment }) {
  return entry.dir ? <IconFiles className="size-kind-mark flex-none text-muted-foreground" /> : <FileKindIcon name={entry.name} className="size-kind-mark flex-none" />;
}

/** Destination is contextual to the open conversation, not another form. */
export function AttachmentDestination({machine,into,onInto,busy=false}:{machine:string|null;into:string|null;onInto:(path:string|null)=>void;busy?:boolean}){
 const [target,setTarget]=useState<DeviceRow|null>(null);
 useEffect(()=>{let live=true;void fetchDevices().then(rows=>{if(live)setTarget(rows.find(row=>machine?row.name===machine:row.me)??null);},()=>{});return()=>{live=false;};},[machine]);
 return <SendInto machine={machine??''} name={target?.name??machine??''} into={into} onInto={onInto} render={({path,pick,reset})=><div data-attachment-destination className="flex w-full min-w-0 flex-none items-center gap-in">
  <button type="button" data-send-into-change aria-label="更改附件保存位置" disabled={busy} onClick={pick} className="flex min-h-touch min-w-0 flex-1 items-center gap-in text-left text-callout text-muted-foreground">
   {target&&<MachineAvatar face={target.face} className="size-kind-mark flex-none"/>}<span className="min-w-0 truncate"><span>{target?.shown??machine??'会话所在机器'}</span><span aria-hidden> · </span><span data-send-into-path>{path}</span></span><IconChevronDown className="size-3.5 flex-none"/>
  </button>
  {reset&&<Button size="sm" data-send-into-reset disabled={busy} onClick={reset}>恢复默认</Button>}
 </div>}/>;
}
/** Optional preview belongs to selected rows, independent of source browsing. */
function AttachmentDraftRows({road}:{road:FilesRoad}){
 const [look,setLook]=useState<Attachment|null>(null),[local,setLocal]=useState<File|null>(null),[folder,setFolder]=useState<{entry:Attachment;prefix:string}|null>(null);
 const [thumbs,setThumbs]=useState<Record<string,string>>({});
 useEffect(()=>{
  const urls:Record<string,string>={};for(const item of road.selected){const file=item.device?.files[0]?.file;if(!item.dir&&file?.type.startsWith('image/')&&file.size<=1024*1024)urls[item.key]=URL.createObjectURL(file);}
  setThumbs(urls);return()=>Object.values(urls).forEach(url=>URL.revokeObjectURL(url));
 },[road.selected]);
 const inspect=(item:Attachment)=>{setLocal(null);setFolder(null);setLook(item);if(item.dir)setFolder({entry:item,prefix:''});else if(item.device)setLocal(item.device.files[0]?.file??null);};
 const remove=(key:string)=>{const item=road.selected.find(item=>item.key===key);if(item){road.remove(item);if(look?.key===key){setLook(null);setLocal(null);setFolder(null);}}};
 if(!road.selected.length)return null;
 return <>
  <AttachmentList items={road.selected.map(item=>({...item,thumb:thumbs[item.key],detail:item.shown,busy:road.busy}))} onOpen={key=>{const item=road.selected.find(item=>item.key===key);if(item)inspect(item);}} onRemove={remove} busy={road.busy}/>
  <AttachmentPreview file={local} onClose={()=>{setLocal(null);setLook(folder?.entry??null);}} onSave={look&&!folder&&road.replace?file=>road.replace!(look,file):undefined}/>
  <FilePreview machine={look?.from??''} shown={look?.shown??''} here={look?.from===null} path={look&&!look.dir&&!look.device?look.path:null} onClose={()=>setLook(folder?.entry??null)}/>
  <Dialog open={folder!==null&&!local&&!!look?.dir} onOpenChange={value=>{if(!value){setFolder(null);setLook(null);}}}><TaskOverlay data-attachment-folder purpose="preview" initialHeight="large" maxHeight="large" scrollable>
   <DialogTitle>{folder?.entry.name??'文件夹'}</DialogTitle><TaskActions close={<Button onClick={()=>{setFolder(null);setLook(null);}}>关闭</Button>}/>
   {folder&&<AttachmentFolder entry={folder.entry} prefix={folder.prefix} onPrefix={prefix=>setFolder({...folder,prefix})} onFile={item=>{setLook(item);if(item.device)setLocal(item.device.files[0]?.file??null);}}/>}

  </TaskOverlay></Dialog>
 </>;
}

function AttachmentFolder({entry,prefix,onPrefix,onFile}:{entry:Attachment;prefix:string;onPrefix:(prefix:string)=>void;onFile:(entry:Attachment)=>void}){
 const [listing,setListing]=useState<DirListing|null>(null),[error,setError]=useState<string|null>(null);
 const path=prefix?joinAttachmentPath(entry.path,prefix):entry.path;
 useEffect(()=>{if(entry.device)return;let live=true;setListing(null);setError(null);void fetchLs(entry.from??'',path).then(value=>{if(live)setListing(value);},error=>{if(live)setError(said(error));});return()=>{live=false;};},[entry,path]);
 const rows=entry.device?deviceFolderEntries(entry,prefix):listing?.entries.map(row=>({name:row.name,relative:prefix?prefix+'/'+row.name:row.name,dir:row.dir,file:undefined}));
 return <>
  {prefix&&<Button className="self-start" onClick={()=>onPrefix(prefix.split('/').slice(0,-1).join('/'))}>上一级</Button>}
  {error?<p role="alert" className="text-callout text-destructive">{error}</p>:!rows?<p role="status">{gui.files_reading_folder}</p>:rows.length===0?<p className="text-callout text-muted-foreground">{gui.empty_dir}</p>:rows.map(row=><button type="button" data-attachment-folder-entry={row.dir?"dir":"file"} key={row.relative} onClick={()=>{
   if(row.dir){onPrefix(row.relative);return;}
   if(row.file){const item=deviceAttachments([row.file],false,entry.shown)[0];onFile(item);}
   else onFile(pathAttachment(joinAttachmentPath(entry.path,row.relative),entry.from,entry.shown));
  }} className="flex min-h-touch w-full items-center gap-in border-b text-left text-body">{row.dir?<IconFiles className="size-kind-mark"/>:<FileKindIcon name={row.name} className="size-kind-mark"/>}<span className="min-w-0 truncate">{row.name}</span></button>)}
 </>;
}
