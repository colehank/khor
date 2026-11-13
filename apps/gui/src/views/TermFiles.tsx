/** Attachment browser in the existing key-panel band. Browsing never
 * sends: files open read-only; selection is sent through the pane. */
import { useCallback, useEffect, useRef, useState } from "react";
import { fetchDevices, fetchLs, nativeFolders, said, termDropForget, termDropStage, termPickFolders, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu } from "@/components/RowMenu";
import { IconBack, IconCheck, IconClose, IconFile, IconFiles } from "@/components/icons";
import { FileKindIcon } from "@/components/FileKindIcon";
import { Button } from "@/components/ui/button";
import { gui, cli } from "@/gen/catalog";
import type { DirListing } from "@/gen/bindings/DirListing";
import { deviceAttachments, deviceFolderEntries, includesAttachment, joinAttachmentPath, parentAttachmentPath, pathAttachment, type Attachment } from "@/lib/term-attachments";
import { cn } from "@/lib/utils";
import { isDown } from "@/lib/topology";
import { hopWord } from "@/words";
import { sizeWord } from "@/views/FilesPane";
import { AttachmentPreview } from "@/views/AttachmentPreview";
import { Dialog, TaskOverlay, DialogTitle, TaskActions } from "@/components/ui/dialog";
import { FileCard, FilePreview, SendInto } from "@/views/chat-files";

export type FilesRoad = {
  machine: string | null; cwd: string | null;
  selected: Attachment[];
  add: (entries: Attachment[]) => void;
  remove: (entry: Attachment) => void;
  replace?: (entry: Attachment, file: File) => void;
  open?: () => void;
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
  const [queue, setQueue] = useState(road.selected.length>0), [query, setQuery] = useState("");
  const [localInto,setLocalInto]=useState<string|null>(null);
  const into=road.into===undefined?localInto:road.into;
  const setInto=road.onInto??setLocalInto;
  const [localLook,setLocalLook]=useState<Attachment|null>(null);
  const [look, setLook] = useState<Look | null>(null);
  const [folder, setFolder] = useState<{root: Attachment; prefix: string} | null>(null);
  const preview = useRef<{turn:number;path:string|null}>({turn:0,path:null});
  const [previewing,setPreviewing]=useState<string|null>(null);
  const closePreview=()=>{
    preview.current.turn++;
    const path=preview.current.path;preview.current.path=null;
    if(path)void termDropForget([path]).catch(()=>{});
    setLook(null);setPreviewing(null);
  };
  const [error, setError] = useState<string | null>(null), [result, setResult] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const generation = useRef(0), mounted = useRef(false), sending=useRef(false);
  const trail = useRef<{ at: Place | null; query: string }[]>([]);
  const refreshDevices = useCallback(() => {
    const mark = ++deviceGeneration.current;
    void fetchDevices().then((d) => {
      if (mounted.current && mark === deviceGeneration.current) { setDevices(d); setSourceError(null); }
    }).catch((e: unknown) => { if (mounted.current && mark === deviceGeneration.current) setSourceError(said(e)); });
  }, []);
  useEffect(() => {
    mounted.current = true;
    const visible = () => { if (document.visibilityState === "visible") refreshDevices(); };
    window.addEventListener("focus", visible);
    document.addEventListener("visibilitychange", visible);
    return () => {
      mounted.current = false; generation.current++; deviceGeneration.current++;
      preview.current.turn++;
      if(preview.current.path)void termDropForget([preview.current.path]).catch(()=>{});
      window.removeEventListener("focus", visible);
      document.removeEventListener("visibilitychange", visible);
    };
  }, [refreshDevices]);
  const sources = !at && !queue && !folder;
  useEffect(() => { if (sources) refreshDevices(); }, [sources, refreshDevices]);
  const open = (machine: DeviceRow, path: string, remember = true) => {
    setFolder(null);
    if (remember) trail.current.push({ at, query });
    const mark = ++generation.current;
    if (remember) setQuery("");
    setAt({ machine, path, listing: null });
    void fetchLs(machine.name, path).then((listing) => {
      if (mark === generation.current) setAt({ machine, path: listing.path, listing });
    }).catch((e: unknown) => { if (mark === generation.current) setAt({ machine, path, listing: null, error: said(e) }); });
  };
  const selected = (e: Attachment) => road.selected.some((x) => x.key === e.key);
  const inspect = async (entry: Attachment) => {
    closePreview();setError(null);
    if(entry.device&&!entry.dir){setLocalLook(entry);return;}
    if(entry.dir&&entry.device){setFolder({root:entry,prefix:""});setQueue(false);setQuery("");return;}
    const machine=devices.find(device=>entry.from?device.name===entry.from:device.me);
    if(!machine){setError(gui.content_machine_gone);return;}
    if(entry.dir&&!entry.device){setQueue(false);open(machine,entry.path);return;}
    if(!entry.device){setLook({entry,machine});return;}
    const turn=preview.current.turn;
    setPreviewing(entry.key);
    try{
      const path=await termDropStage(entry.device.files[0].file);
      if(!mounted.current||preview.current.turn!==turn){void termDropForget([path]).catch(()=>{});return;}
      preview.current.path=path;setLook({entry,machine,path});
    }catch(error){if(mounted.current&&preview.current.turn===turn)setError(said(error));}
    finally{if(mounted.current&&preview.current.turn===turn)setPreviewing(null);}
  };
  const included = (e: Attachment) => road.selected.some((x) => includesAttachment(x, e));
  const toggle = (e: Attachment) => {
    if (road.busy || included(e)) return;
    if (selected(e)) road.remove(e); else road.add([e]);
    setResult(null);
  };
  const addDevice = (files: File[], dirs: boolean) => {
    if (!files.length || road.busy) return;
    try { road.add(deviceAttachments(files, dirs, gui.machine_here)); setQueue(true); setError(null); setResult(null); }
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
      if (paths.length) { setQueue(true); setResult(null); }
    } catch (e) { if (mounted.current) setError(said(e)); }
    finally { if (mounted.current) setPicking(false); }
  };
  const target=devices.find(device=>road.machine?device.name===road.machine:device.me);
  const destination = into ?? target?.receive_at ?? null;
  const send = async (entries: Attachment[]) => {
    if (road.busy || sending.current || !entries.length) return;
    sending.current=true;setError(null);closePreview();
    try{
      const ok = await road.send(entries, destination);
      if(!ok&&mounted.current)setError("附件未能发送，已保留所选文件，可重试");
      if (!mounted.current) return;
      setResult(ok ? gui.files_sent(entries.length) : gui.files_retry_ready);
      if (ok) {setQueue(false);onSent();}
    }catch(error){if(mounted.current)setError(said(error));}
    finally{sending.current=false;}
  };
  const back = () => {
    if(folder){
      if(folder.prefix){const at=folder.prefix.lastIndexOf('/');setFolder({...folder,prefix:at<0?"":folder.prefix.slice(0,at)});}
      else{setFolder(null);setQueue(true);}
      setQuery("");return;
    }
    if (queue) { setQueue(false); return; }
    generation.current++;
    const previous = trail.current.pop();
    setAt(previous?.at ?? null);
    setQuery(previous?.query ?? "");
  };
  const entries = at?.listing?.entries.filter((e) => e.name.toLocaleLowerCase().includes(query.toLocaleLowerCase())) ?? [];
  const localEntries = folder ? deviceFolderEntries(folder.root,folder.prefix) : [];
  return <div data-files-panel className="flex min-h-0 flex-1 flex-col">
    <DialogTitle>{road.action==='添加'?'添加附件':'发送附件'}</DialogTitle>
    <TaskActions busy={road.busy} close={<Button data-files-cancel disabled={road.stopping} onClick={road.busy?road.cancel:onClose}>{road.busy?'取消传输':'关闭'}</Button>}
      primary={<Button data-files-send disabled={road.busy||!road.selected.length||picking} variant={road.selected.length&&!road.busy?'primary':'default'} onClick={()=>void send(road.selected)}>{road.busy?'正在发送':`${road.action??'发送'}${road.selected.length?' ('+road.selected.length+')':''}`}</Button>}/>
    <div data-files-target className="flex flex-none flex-col gap-in border-b pb-in text-body">
      <div className="flex min-w-0 items-center gap-in"><span className="text-muted-foreground">发到</span>{target&&<MachineAvatar face={target.face} className="size-kind-mark flex-none"/>}<span className="min-w-0 truncate">{target?.shown??road.machine??'会话所在机器'}</span></div>
      <SendInto machine={road.machine??''} name={target?.name??road.machine??''} into={into} onInto={setInto}/>
      {road.cwd&&road.onInto&&<Button size="sm" variant="ghost" disabled={road.busy} className="self-start" onClick={()=>setInto(road.cwd)}>使用会话目录</Button>}
    </div>
    <div data-key-panel-scroll className="min-h-0 flex-1 touch-pan-y overflow-y-auto overscroll-contain">
      {road.selected.length>0&&<section data-files-cards className="flex flex-col gap-in py-row">
        <div className="flex items-center justify-between gap-in"><h3 className="text-body font-label">待发送 · {road.selected.length}</h3><Button data-files-add-more size="sm" onClick={()=>{setQueue(false);setAt(null);setFolder(null);}}>继续添加</Button></div>
        <div className="flex flex-wrap gap-in">{road.selected.map(entry=><div data-files-selected={entry.name} key={entry.key} className="relative w-64 max-w-full flex-none rounded-lg bg-fill pe-touch">
          {entry.device||entry.dir?<AttachmentCard entry={entry} pending={previewing===entry.key} onOpen={()=>void inspect(entry)}/>:<FileCard machine={entry.from??''} shown={entry.shown} path={entry.path} onOpen={()=>void inspect(entry)}/>}
          <button type="button" disabled={road.busy} aria-label={`移除 ${entry.name}`} onClick={()=>{if(look?.entry.key===entry.key||previewing===entry.key||folder?.root.key===entry.key)closePreview();if(folder?.root.key===entry.key){setFolder(null);setQueue(true);}road.remove(entry);}} className="absolute right-0 top-0 grid min-h-touch min-w-touch place-items-center disabled:opacity-40"><IconClose className="size-4"/></button>
        </div>)}</div>
      </section>}
      {(at||folder)&&!queue&&<div className="flex min-h-touch items-center gap-in border-b"><button type="button" data-files-up aria-label="返回来源" onClick={back} className="grid min-h-touch min-w-touch place-items-center"><IconBack className="size-5"/></button><span className="min-w-0 flex-1 truncate text-callout">{folder?folder.root.name:at?.machine.shown+' · '+at?.path}</span><Button size="sm" onClick={()=>{setFolder(null);setAt(null);setQueue(true);}}>查看已选</Button></div>}
      {folder ? <div data-files-device-folder={folder.root.name}>
        {localEntries.map(row=><button key={row.relative} type="button" data-files-local-entry={row.relative} data-files-local-kind={row.dir?'dir':'file'}
          onClick={()=>{if(row.dir){setFolder({...folder,prefix:row.relative});setQuery("");}else if(row.file)void inspect({...folder.root,key:folder.root.key+'/'+row.relative,name:row.name,path:folder.root.name+'/'+row.relative,dir:false,size:row.file.size,device:{files:[{file:row.file,relative:row.file.name}],dirs:[]}});}}
          className="flex min-h-touch w-full min-w-0 items-center gap-row border-b py-in text-left">
          {row.dir?<IconFiles className="size-kind-mark flex-none text-muted-foreground"/>:<FileKindIcon name={row.name} className="size-kind-mark flex-none"/>}
          <span className="min-w-0 flex-1 truncate text-body">{row.name}</span>{row.file&&<span className="text-caption tabular-nums text-muted-foreground">{sizeWord(row.file.size)}</span>}
        </button>)}
        {!localEntries.length&&<p className="py-pane text-center text-callout text-muted-foreground">{gui.empty_dir}</p>}
      </div> : queue ? !road.selected.length&&<p className="py-pane text-center text-callout text-muted-foreground">选择文件，发送前可预览或移除</p> : !at ? <>
        <h3 className="pb-in pt-in text-body font-label">从哪添加</h3>
        <div className="flex gap-in pb-row"><Button data-files-pick="device" disabled={road.busy} className="min-h-touch flex-1" onClick={() => picker.current?.click()}><span data-files-pick-glyph className="grid size-affordance place-items-center [&>svg]:size-affordance"><IconFile /></span>{gui.files_local_file}</Button>{!road.filesOnly&&<Button data-files-pick="folder" disabled={road.busy || picking} className="min-h-touch flex-1" onClick={() => void pickFolders()}><span data-files-pick-glyph className="grid size-affordance place-items-center [&>svg]:size-affordance"><IconFiles /></span>{picking ? gui.files_reading_folder : gui.ws_kind_folder}</Button>}</div>
        <section data-files-mesh><h3 className="pb-in text-group text-muted-foreground">{gui.devices_tab}</h3>
          {machinesFor(devices, road.machine).map((d) => <button key={d.id} type="button" data-files-machine={d.name} data-files-session={(road.machine === null ? d.me : d.name === road.machine) || undefined} onClick={() => open(d, "")} className="flex min-h-touch w-full items-center gap-row border-t text-left text-body"><MachineAvatar face={d.face} className="size-kind-mark flex-none" /><span className="min-w-0 flex-1 truncate">{d.shown}</span>{!d.me && d.hop === "quiet" && isDown(d.down_ms) && <span data-files-unreachable className="flex-none text-caption text-muted-foreground">{hopWord("down")}</span>}{(road.machine === null ? d.me : d.name === road.machine) && <span className="text-caption text-muted-foreground">{gui.files_session}</span>}</button>)}
          {sourceError && <div className="py-in text-callout"><p role="alert">{sourceError}</p><Button onClick={refreshDevices}>{gui.region_retry}</Button></div>}
        </section>
      </> : at.error ? <div className="py-row text-callout"><p role="alert">{gui.files_failed_list(at.error)}</p><Button onClick={() => open(at.machine, at.path, false)}>{gui.region_retry}</Button></div> : !at.listing ? <p role="status" className="py-pane text-center text-callout text-muted-foreground">{gui.files_reading_folder}</p> : <>
        {parentAttachmentPath(at.listing.path) && <button type="button" data-files-parent onClick={() => open(at.machine, parentAttachmentPath(at.listing!.path)!)} className="flex min-h-touch w-full items-center gap-row border-b py-in text-left text-body"><IconBack className="size-kind-mark flex-none rotate-90" /><span>{gui.up_dir}</span></button>}
        {entries.length === 0 && <p className="py-pane text-center text-callout text-muted-foreground">{query ? gui.files_no_matches : gui.empty_dir}</p>}
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
              <button type="button" onClick={() => entry.dir ? open(at.machine, entry.path) : view()} className="flex min-h-touch min-w-0 flex-1 items-center gap-row py-in text-left"><Mark entry={entry} /><span className="min-w-0 flex-1 truncate text-body">{entry.name}</span>{entry.size !== null && <span className="text-caption tabular-nums text-muted-foreground">{sizeWord(entry.size)}</span>}</button>
              <button type="button" data-files-select={entry.name} aria-label={`${own ? gui.files_deselect : gui.files_select} ${entry.name}`} aria-pressed={own || inherited} disabled={inherited || road.busy || (road.filesOnly && entry.dir)} title={inherited ? gui.files_included : undefined} onClick={() => toggle(entry)} className="grid min-h-touch min-w-touch place-items-center disabled:opacity-40"><span className={cn("grid size-5 place-items-center rounded-full border", (own || inherited) && "bg-fill-strong")}>{(own || inherited) && <IconCheck className="size-3.5" />}</span></button>
            </div>
          </RowMenu>;
        })}
        {at.listing.truncated && <p role="status" className="py-in text-callout text-muted-foreground">{cli.dir_truncated(at.listing.entries.length)}</p>}
      </>}
    </div>
    <input data-files-input ref={picker} type="file" multiple hidden onChange={(e) => { addDevice([...(e.target.files ?? [])], false); e.target.value = ""; }} />
    <input ref={folders} type="file" multiple hidden {...{ webkitdirectory: "" }} onChange={(e) => { addDevice([...(e.target.files ?? [])], true); e.target.value = ""; }} />
    <div className="flex flex-none flex-col gap-in border-t py-in text-callout">
      {error&&<p role="alert" className="text-destructive">{error}</p>}
      {road.busy?<div data-files-transfer role="status">{road.stopping?'正在取消':`正在发送 · ${Math.round((road.progress??0)*100)}%`}</div>:<span className="text-muted-foreground">{road.action==='添加'?'添加后可在输入区补充说明，再一起发送':'发送后将文件路径放入终端，回车由你确认'}</span>}
    </div>
    <AttachmentPreview file={localLook?.device?.files[0]?.file??null} onClose={()=>setLocalLook(null)} onSave={road.replace&&localLook?file=>road.replace!(localLook,file):undefined}/>
    <FilePreview machine={look?.machine.name ?? ""} shown={look?.machine.shown ?? ""} here={look?.machine.me ?? false} path={look?.path??look?.entry.path ?? null} initialFace={look?.face} onClose={closePreview} footer={look && !folder && <><Button disabled={road.busy || included(look.entry)} onClick={() => toggle(look.entry)}>{selected(look.entry) ? gui.files_deselect : gui.files_select}</Button></>} />
  </div>;
}
function AttachmentCard({entry,pending,onOpen}:{entry:Attachment;pending:boolean;onOpen:()=>void}){
  const file=entry.dir?null:entry.device?.files[0]?.file;
  const [thumb,setThumb]=useState<string|null>(null);
  useEffect(()=>{if(!file?.type.startsWith('image/')||file.size>1024*1024)return;const url=URL.createObjectURL(file);setThumb(url);return()=>URL.revokeObjectURL(url);},[file]);
  return <button type="button" data-files-card-preview aria-busy={pending||undefined} onClick={onOpen} className="flex min-h-touch w-full min-w-0 items-center gap-in p-in text-left aria-busy:animate-pulse motion-reduce:aria-busy:animate-none">
    {thumb?<img src={thumb} alt="" className="size-10 flex-none rounded-md object-cover"/>:<Mark entry={entry}/>}
    <span className="min-w-0 flex-1"><span className="block truncate text-body">{entry.name}</span>{entry.size!==null&&<span className="block text-caption tabular-nums text-muted-foreground">{sizeWord(entry.size)}</span>}<span className="block truncate text-caption text-muted-foreground">{entry.shown}</span></span>
  </button>;
}
function Mark({ entry }: { entry: Attachment }) {
  return entry.dir ? <IconFiles className="size-kind-mark flex-none text-muted-foreground" /> : <FileKindIcon name={entry.name} className="size-kind-mark flex-none" />;
}
