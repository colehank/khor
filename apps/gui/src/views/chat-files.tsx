import { useFileHandoff } from "@/hooks/use-file-handoff";
import { FileHandoffAction, FileHandoffStatus } from "@/components/FileHandoff";
import { fileManagerLabel } from "@/lib/file-actions";
// **Files in an agent conversation** (round CHAT, 2026-10-03, the owner:
// 「gui 对话得做到像 WeChat 一样：发文件、预览文件、还得显示文件在哪个路径」).
//
// # Where a file goes, and what the agent is told
//
// A file from the device — a phone's photo, a Mac's desktop file, a drop —
// is put on the machine the conversation runs on, in that machine's
// receive directory (`~/khor-drop` unless set in 存储, under its own name,
// `khor_gui_core::chat::chat_put`), the very folder a `khor send` to that
// machine lands in — or in a directory there the person chose above the
// box (「存到」, `SendInto`, STORE-BRIEF phase 2: 指定位置就传到指定位置);
// the agent is told the landed path. A file already on that machine
// (「这台机器上的文件」) is
// not copied at all. Either way the agent is told the **path**, one line
// `@<path>` per file after the person's words (`lib/paths`): what Claude
// Code and Codex read. A small picture also rides inline when the agent
// said it takes pictures, as pictures did before this round (`INLINE_MAX`).
//
// # Composing
//
// Each file is put on the machine as soon as it is chosen, so sending
// waits on nothing that could have happened earlier; its chip says so
// while it travels (busy), and says why when it could not.
//
// # Looking at one
//
// A pressed file opens in `FilePreview`, which is the workspace's own
// `FileView` in a window — pictures, PDFs, markdown, code, and the same
// sentences for what it will not draw — read only, with the three things
// a person does next: open it where it lives, copy its path, and hand it
// on (Finder on the Mac, the system's share sheet on a phone).
import { useEffect, useState, useSyncExternalStore, type ReactNode } from "react";

import { fetchDevices, fetchFileText, fetchLs, fetchPreview, fetchRevealHere, pullFile, reveal } from "@/api";
import { DirPicker } from "@/components/DirPicker";
import { FileKindIcon } from "@/components/FileKindIcon";
import { PathCut } from "@/components/PathCut";
import { PathField } from "@/components/PathField";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, TaskActions, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { baseName, shownName } from "@/lib/paths";
import { cn } from "@/lib/utils";
import { sizeWord } from "@/views/FilesPane";
import { defaultFaceOf, FileView, kindOf } from "@/views/FileView";
import { useShowFile } from "@/views/linkHome";

/** One file going along with the line being written. */
export type Pending = import('@/lib/composer-files').PendingFile;
import {composerFiles,composerReady} from '@/lib/composer-files';

/** How large a picture still rides inline beside its path: 5 MB, the
    old picture road's limit again. A turn with pictures goes in slices,
    off the main thread, with its progress shown (`chat_say_with`, the
    net lane's c85d45d; the host takes 64 MB a turn) — ⚠️ 墓碑: 1 MiB
    while one 8.5 MB frame could hold the dev bridge's only request loop
    for minutes (round CHAT). */
const INLINE_MAX = 5 * 1024 * 1024;
/** How large a picture still gets a thumbnail on its chip. */
const THUMB_MAX = 1024 * 1024;

const why = (e: unknown) => String((e as { message?: string })?.message ?? e);

async function base64Of(f: File): Promise<string> {
  const buf = new Uint8Array(await f.arrayBuffer());
  // Chunked: spreading a multi-megabyte array into one call overflows the
  // stack at a size a test image never reaches.
  let bin = "";
  for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
  return btoa(bin);
}

/** `into`: the directory on the conversation's machine the files go
    to, as chosen when each is added; `null` is its receive location. */
export function usePendingFiles(upload: {
  put: (file: File, into: string | null) => Promise<string>;
  putPaths: (paths: string[], into: string | null) => Promise<string[]>;
}, into: string | null = null, session = 'composer') {
  const store=composerFiles(session);
  const items=useSyncExternalStore(store.subscribe,store.snapshot);
  const addFiles=(files:File[])=>{
    for(const file of files){
      const key=store.key(),picture=file.type.startsWith('image/'),encoding=picture&&file.size<=INLINE_MAX;
      store.add([{key,name:file.name||gui.chat_image(key),size:file.size,
        thumb:picture&&file.size<=THUMB_MAX?URL.createObjectURL(file):null,
        inline:null,path:null,error:null,encoding}]);
      if(encoding)void base64Of(file).then(data=>store.patch(key,{inline:[file.type,data],encoding:false}),
        error=>store.patch(key,{encoding:false,error:why(error)}));
      void upload.put(file,into).then(path=>store.patch(key,{path}),error=>store.patch(key,{error:why(error)}));
    }
  };
  const addLocalPaths=(paths:string[])=>{
    const keys=paths.map(()=>store.key());
    store.add(paths.map((path,index)=>({key:keys[index],name:baseName(path),size:null,thumb:null,inline:null,path:null,error:null})));
    void upload.putPaths(paths,into).then(landed=>keys.forEach((key,index)=>store.patch(key,landed[index]
      ?{path:landed[index]}:{error:gui.history_read_failed})),error=>keys.forEach(key=>store.patch(key,{error:why(error)})));
  };
  const addPaths=(paths:string[])=>store.add(paths.map(path=>({key:store.key(),name:baseName(path),size:null,thumb:null,inline:null,path,error:null})));
  const restore=(remembered:readonly Pending[])=>store.add(remembered.map(item=>({...item,key:store.key(),thumb:null})));
  return {items,addFiles,addLocalPaths,addPaths,remove:(key:string)=>store.remove(key),clear:()=>store.clear(),restore,
    busy:items.some(item=>!item.error&&(item.path===null||item.encoding)),ready:composerReady(items)};
}

/** Unsent files retain their independent preview and removal controls. */
export function PendingChips({ items, onRemove, machine, here }: {
  items: Pending[];
  onRemove: (key: string) => void;
  machine: string;
  here: boolean;
}) {
  const [looking, setLooking] = useState<string | null>(null);
  if (items.length === 0) return null;
  const path = items.find(item => item.key === looking)?.path ?? null;
  return (
    <div data-chat-pending className="flex flex-wrap items-center gap-in px-2 pb-in">
      {items.map(item => (
        <div
          key={item.key}
          data-chat-pending-file={item.name}
          data-placed={item.path !== null || undefined}
          aria-busy={(!item.error && (item.path === null || item.encoding)) || undefined}
          title={item.error ?? item.path ?? undefined}
          className="flex max-w-full min-w-0 items-center rounded-lg bg-fill text-callout aria-busy:animate-pulse motion-reduce:aria-busy:animate-none"
        >
          <button
            type="button"
            data-chat-pending-preview
            disabled={item.path === null}
            onClick={() => setLooking(item.key)}
            className="flex min-h-touch min-w-0 items-center gap-in px-in text-left"
          >
            {item.thumb ? <img src={item.thumb} alt="" className="size-7 flex-none rounded-md object-cover" />
              : <FileKindIcon name={item.name} className="size-7 flex-none" />}
            <span className="flex min-w-0 flex-col">
              <span className="max-w-48 truncate">{item.name}</span>
              {(item.error || item.size !== null) && (
                <span className={cn("text-caption tabular-nums", item.error ? "text-state-failed" : "text-muted-foreground")}>
                  {item.error ? gui.chat_put_failed : sizeWord(item.size ?? 0)}
                </span>
              )}
            </span>
          </button>
          <button
            type="button"
            data-chat-pending-drop
            aria-label={`${gui.remove} ${item.name}`}
            onClick={() => {
              if (looking === item.key) setLooking(null);
              onRemove(item.key);
            }}
            className="grid min-h-touch min-w-touch flex-none place-items-center text-muted-foreground pointer-fine:min-h-ctl-md pointer-fine:min-w-ctl-md"
          >
            ×
          </button>
        </div>
      ))}
      {path !== null && <FilePreview machine={machine} shown={machine} here={here} path={path} onClose={() => setLooking(null)} />}
    </div>
  );
}

/**
 * **Where a file sent from here lands** (STORE-BRIEF phase 2): 「存到」 and
 * the receiving machine's receive location, or the directory picked on
 * it (更改…, `DirPicker` fixed to that machine); 恢复默认 once one is
 * picked. One line for both conversations — a machine's (`MachineChat`)
 * and an agent's (`ChatView`) — so the two cannot say it two ways.
 */
export type SendIntoControl = {path:string;pick:()=>void;reset:(()=>void)|null};

export function SendInto({
  machine,
  name,
  into,
  onInto,
  className,
  render,
}: {
  /** The API's spelling, for the picker: `""` is this machine. */
  machine: string;
  /** The machine's name as its device row reads it. */
  name: string;
  /** The picked directory; `null` is the receive location. */
  into: string | null;
  onInto: (into: string | null) => void;
  className?: string;
  /** Reuse this location and picker from an action menu. */
  render?: (control:SendIntoControl)=>ReactNode;
}) {
  const [at, setAt] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  useEffect(() => {
    let live = true;
    fetchDevices().then(
      (rows) => live && setAt(rows.find((r) => r.name === name)?.receive_at ?? null),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [name]);
  return (
    <>
      {render?render({path:into??at??"~/khor-drop",pick:()=>setPicking(true),reset:into===null?null:()=>onInto(null)}):<div
        data-send-into={into ?? ""}
        className={cn("flex min-w-0 items-center gap-in text-caption text-muted-foreground", className)}
      >
        <span className="flex-none">{gui.send_into}</span>
        {/* `~/khor-drop` where the row has no `receive_at` (an older khor). */}
        <PathCut data-send-into-path text={into ?? at ?? "~/khor-drop"} className="min-w-0 flex-1" />
        <Button size="sm" variant="ghost" data-send-into-change onClick={() => setPicking(true)}>
          {gui.store_place_change}
        </Button>
        {into !== null && (
          <Button size="sm" variant="ghost" data-send-into-reset onClick={() => onInto(null)}>
            {gui.store_place_reset}
          </Button>
        )}
      </div>}
      <DirPicker open={picking} onOpenChange={setPicking} machine={machine} fixed onPick={(p) => onInto(p.path)} />
    </>
  );
}

/** 「<machine> 上的文件…」: a path on the conversation's machine, typed
    with the file completion every directory box has (`PathField`). Not
    copied anywhere — the agent is told where it already is. */
export function MachineFilePick({
  open,
  onOpenChange,
  machine,
  shown,
  onPick,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The API's spelling: `""` is this machine. */
  machine: string;
  /** The name a person reads. */
  shown: string;
  onPick: (path: string) => void;
}) {
  const [value, setValue] = useState("~/");
  useEffect(() => {
    if (open) setValue("~/");
  }, [open]);
  const take = () => {
    const v = value.trim();
    if (!v || v.endsWith("/")) return;
    onOpenChange(false);
    // **Whole, never `~`-led**: the agent is told this path, and so is
    // the card's listing; only the machine knows its home, and its
    // listing of `""` says it (`fetchLs`).
    if (!v.startsWith("~")) return onPick(v);
    void fetchLs(machine, "").then(
      (home) => onPick(`${home.path.replace(/\/+$/, "")}${v.slice(1)}`),
      () => onPick(v),
    );
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <TaskOverlay purpose="selection" initialHeight="fit" data-chat-machine-pick>
        <DialogHeader>
          <DialogTitle>{gui.chat_attach_machine_title(shown)}</DialogTitle>
        </DialogHeader>
        <PathField machine={machine} value={value} onValueChange={setValue} onSubmit={take} files autoFocus />
        <TaskActions dirty={!!value} cancel={<Button data-dialog-cancel onClick={() => onOpenChange(false)}>
            {gui.delete_keep}
          </Button>} primary={<Button variant="primary" data-chat-machine-pick-add onClick={take}>
            {gui.watch_add}
          </Button>}/>
      </TaskOverlay>
    </Dialog>
  );
}

/** Whether Finder can show this machine's files, asked once a page. */
let revealing: Promise<boolean> | null = null;
export const revealHere = () => (revealing ??= fetchRevealHere().catch(() => false));

/** The file's bytes for the share sheet, from the reads the preview
    already makes: a picture or a PDF as the preview carries it, text as
    text. `null` for anything else, which is not offered. */
export async function bytesOf(machine: string, path: string): Promise<File | null> {
  const name = shownName(path);
  const kind = kindOf(path);
  if (kind === "text") {
    const f = await fetchFileText(machine, path);
    return f.binary || f.cut ? null : new File([f.text], name, { type: "text/plain" });
  }
  if (kind === "doc") return null;
  const look = await fetchPreview(machine, path);
  if (look.data === "" || look.cut) return null;
  const raw = atob(look.data);
  const buf = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) buf[i] = raw.charCodeAt(i);
  return new File([buf], name, { type: look.media });
}

/** The system's share sheet takes files here (a phone's WebKit; a desktop
    browser may take only links). */
export const shares = () =>
  typeof navigator.share === "function" &&
  typeof navigator.canShare === "function" &&
  navigator.canShare({ files: [new File([""], "x.txt", { type: "text/plain" })] });

/** One file of a conversation, looked at (round CHAT item 3). */
export function FilePreview({
  machine,
  shown,
  path,
  here,
  onClose,
  footer,
  initialFace,
}: {
  /** The API's spelling of the conversation's machine. */
  machine: string;
  /** That machine is this one: its files are already on this disk. */
  here: boolean;
  /** Its name as a person reads it; empty draws no name. */
  shown: string;
  /** Absolute on that machine; `null` is closed. */
  path: string | null;
  onClose: () => void;
  footer?: ReactNode;
  /** The face it opens on; left out, the file's own (`defaultFaceOf`). */
  initialFace?: "source" | "preview";
}) {
  const handoff=useFileHandoff(machine,path??"",path!==null);
  const opening = initialFace ?? (path === null ? "source" : defaultFaceOf(path));
  const [face, setFace] = useState(opening);
  useEffect(() => setFace(opening), [machine, path, opening]);
  const show = useShowFile();
  const [finder, setFinder] = useState(false);
  const [said, setSaid] = useState<"copied" | "failed" | string | null>(null);
  useEffect(() => {
    void revealHere().then(setFinder);
  }, []);
  useEffect(() => setSaid(null), [path]);
  const kind = path === null ? "text" : kindOf(path);
  const nativePreview = (kind === "doc" || kind === "pdf") && handoff.previewSupported;
  const sharing = path !== null && kind !== "doc" && shares();
  const copy = () => {
    if (path === null) return;
    navigator.clipboard.writeText(path).then(
      () => setSaid("copied"),
      () => setSaid("failed"),
    );
  };
  // A far machine's file is taken to this Mac first (the downloads,
  // `pullFile`) and that copy is the one Finder shows.
  const toFinder = () => {
    if (path === null) return;
    const local = here ? Promise.resolve(path) : pullFile(machine, path);
    void local
      .then((p) => reveal(p)).catch((e: unknown) => setSaid(String((e as Error)?.message ?? e)));
  };
  const share = () => {
    if (path === null) return;
    void bytesOf(machine, path)
      .then((f) => (f ? navigator.share({ files: [f] }) : undefined))
      .catch(() => {
        // Dismissing the sheet rejects too; nothing to say about it.
      });
  };
  return (
    <Dialog open={path !== null} onOpenChange={(o) => !o && onClose()}>
      <TaskOverlay
        data-file-preview={path ?? undefined}
        // The phone's sheet (grabber, pulled down to put away); a window
        // over the scrim elsewhere, closed by a press outside or Esc.
        purpose="preview"

      >
        {path !== null && (
          <>
            <div
              data-file-preview-head
              className="flex flex-none flex-col gap-in"
            >
              <DialogTitle>{shownName(path)}</DialogTitle>
              <TaskActions close={<Button data-file-preview-close onClick={onClose}>{gui.close}</Button>}/>
              <div className="flex min-w-0 text-caption text-muted-foreground">
                {shown && <span className="flex-none">{shown}&nbsp;·&nbsp;</span>}
                <PathCut text={path} className="min-w-0 flex-1" />
              </div>
              <div className="flex items-center justify-between gap-in">
                <SegmentedControl label={gui.file_face_preview} value={face} onChange={setFace}
                  options={[{ key: "source", label: gui.files_original }, { key: "preview", label: gui.file_face_preview }]} />
                <span className="text-caption text-muted-foreground">{gui.file_read_only}</span>
              </div>
              <div className="flex flex-wrap items-center gap-in">
                {show && (
                  <Button
                    size="sm"
                    data-file-preview-open
                    onClick={() => {
                      onClose();
                      show(machine, path);
                    }}
                  >
                    {gui.chat_file_open}
                  </Button>
                )}
                <Button size="sm" data-file-preview-copy onClick={copy}>
                  {gui.tab_copy_path}
                </Button>
                {finder && (
                  <Button size="sm" data-file-preview-reveal onClick={toFinder}>
                    {fileManagerLabel()}
                  </Button>
                )}
                {!finder && handoff.supported && !nativePreview && <FileHandoffAction handoff={handoff}/>}
                {!finder && !handoff.supported && sharing && (
                  <Button size="sm" data-file-preview-share onClick={share}>
                    {gui.chat_file_share}
                  </Button>
                )}
                {said !== null && (
                  <span data-file-preview-said className="text-callout text-muted-foreground">
                    {said === "copied" ? gui.copied : said === "failed" ? gui.copy_failed : said}
                  </span>
                )}
              </div>
            </div>
            <div ref={handoff.owner} data-file-preview-body className="relative flex min-h-0 flex-1 flex-col border-t">
              <FileHandoffStatus handoff={handoff}/>
              {/* The workspace's own view of a file, read only: editing is
                  the file view's, one press away above. */}
              <FileView
                handoff={handoff}
                key={`${machine} ${path}`}
                machine={machine}
                path={path}
                git={false}
                face={face}
                reading
              />
            </div>
            {footer && <div className="flex flex-none items-center justify-between gap-row border-t py-in">{footer}</div>}
          </>
        )}
      </TaskOverlay>
    </Dialog>
  );
}

/** What a card says about a file besides its name: its size, and a
    picture's thumbnail. */
type Facts = { size: number | null; thumb: string | null };

/** Asked once a page per file (and per `stamp`, for a file the agent
    rewrote): a conversation repaints often and its files do not move. A
    question that could not be asked answers `null` and is forgotten, so
    the card asks again (`useFacts`). */
const asked = new Map<string, Promise<Facts | null>>();
const factBytes=new Map<string,number>();
function forgetFact(key:string){asked.delete(key);factBytes.delete(key);}
function trimFacts(){
 while(asked.size>64||[...factBytes.values()].reduce((sum,n)=>sum+n,0)>8*1024*1024){
  const oldest=asked.keys().next().value;if(oldest===undefined)break;forgetFact(oldest);
 }
}

function factsOf(machine: string, path: string, stamp: string): Promise<Facts | null> {
  const key = `${machine}\n${path}\n${stamp}`;
  const had = asked.get(key);
  if (had) {asked.delete(key);asked.set(key,had);return had;}
  const dir = path.slice(0, path.lastIndexOf("/")) || "/";
  const name = baseName(path);
  const p = fetchLs(machine, dir).then(
    async (l): Promise<Facts> => {
      const row = l.entries.find((e) => e.name === name && !e.dir);
      if (!row) return { size: null, thumb: null };
      if (kindOf(path) !== "image" || row.size > THUMB_MAX) return { size: row.size, thumb: null };
      const look = await fetchPreview(machine, path).catch(() => null);
      const thumb = look && look.data !== "" && !look.cut ? `data:${look.media};base64,${look.data}` : null;
      return { size: row.size, thumb };
    },
    () => {
      forgetFact(key);
      return null;
    },
  );
  asked.set(key, p);trimFacts();
  void p.then(facts=>{if(asked.get(key)!==p)return;factBytes.set(key,(facts?.thumb?.length??0)*2);trimFacts();},()=>{if(asked.get(key)===p)forgetFact(key);});
  return p;
}

const RETRY_MS = 5000;
const RETRIES = 6;

function useFacts(machine: string, path: string, stamp: string): Facts {
  const [facts, setFacts] = useState<Facts>({ size: null, thumb: null });
  useEffect(() => {
    let live = true;
    setFacts({size:null,thumb:null});
    let timer: ReturnType<typeof setTimeout> | undefined;
    // A machine that could not be reached is asked again a little later,
    // a few times: a card left without its size for the life of the page
    // because of one dropped answer is the failure this avoids.
    const ask = (left: number) =>
      void factsOf(machine, path, stamp).then((f) => {
        if (!live) return;
        if (f !== null) setFacts(f);
        else if (left > 0) timer = setTimeout(() => ask(left - 1), RETRY_MS);
      });
    ask(RETRIES);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [machine, path, stamp]);
  return facts;
}

/** **A file in the conversation, as a card** (round CHAT item 2): the
    picture's thumbnail or the kind's icon, the name as it was given, its
    size, and where it is — `machine · path`, cut at the start like every
    path in a line too short for it (`PathCut`). Pressed, it opens. */
export function FileCard({
  machine,
  shown,
  path,
  note = null,
  stamp = "",
  onOpen,
}: {
  /** The API's spelling of the machine the file is on. */
  machine: string;
  /** That machine's name as a person reads it; empty draws none. */
  shown: string;
  /** Absolute on that machine. */
  path: string;
  /** What happened to it, before the size (「已写入」). */
  note?: string | null;
  /** Changes when the file did, so its facts are asked again. */
  stamp?: string;
  onOpen: () => void;
}) {
  const { size, thumb } = useFacts(machine, path, stamp);
  const name = shownName(path);
  const first = [note, size === null ? null : sizeWord(size)].filter(Boolean).join(" · ");
  return (
    <button
      type="button"
      data-chat-file={path}
      onClick={onOpen}
      title={path}
      className="flex w-80 max-w-full min-w-0 items-center gap-row rounded-lg bg-fill p-2 text-left"
    >
      {thumb ? (
        <img src={thumb} alt="" className="size-10 flex-none rounded-md object-cover" />
      ) : (
        <FileKindIcon name={name} className="size-10 flex-none" />
      )}
      <span className="flex min-w-0 flex-1 flex-col">
        <span data-chat-file-name className="truncate text-body text-foreground">
          {name}
        </span>
        {first && <span className="text-caption text-muted-foreground tabular-nums">{first}</span>}
        <span className="flex min-w-0 text-caption text-muted-foreground">
          {shown && <span className="flex-none">{shown}&nbsp;·&nbsp;</span>}
          <PathCut text={path} className="min-w-0 flex-1" />
        </span>
      </span>
    </button>
  );
}
