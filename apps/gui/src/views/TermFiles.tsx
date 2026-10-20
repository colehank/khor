/** Attachment browser in the existing key-panel band. Browsing never
 * sends: files open read-only; selection is sent through the pane. */
import { useCallback, useEffect, useRef, useState } from "react";
import { fetchDevices, fetchLs, nativeFolders, said, termDropForget, termPickFolders, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu } from "@/components/RowMenu";
import { IconBack, IconCheck, IconClose, IconFile, IconFiles } from "@/components/icons";
import { FileKindIcon } from "@/components/FileKindIcon";
import { Button } from "@/components/ui/button";
import { PopupButton } from "@/components/ui/popup-button";
import { gui, cli } from "@/gen/catalog";
import type { DirListing } from "@/gen/bindings/DirListing";
import { deviceAttachments, includesAttachment, joinAttachmentPath, parentAttachmentPath, pathAttachment, type Attachment } from "@/lib/term-attachments";
import { cn } from "@/lib/utils";
import { isDown } from "@/lib/topology";
import { hopWord } from "@/words";
import { sizeWord } from "@/views/FilesPane";
import { FilePreview } from "@/views/chat-files";

export type FilesRoad = {
  machine: string | null; cwd: string | null;
  selected: Attachment[];
  add: (entries: Attachment[]) => void;
  remove: (entry: Attachment) => void;
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
type Look = { entry: Attachment; machine: DeviceRow; face?: "source" | "preview" };
export function TermFiles({ road }: { road: FilesRoad }) {
  const picker = useRef<HTMLInputElement>(null), folders = useRef<HTMLInputElement>(null);
  const [devices, setDevices] = useState<DeviceRow[]>([]);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const deviceGeneration = useRef(0);
  const [at, setAt] = useState<Place | null>(null);
  const [queue, setQueue] = useState(false), [query, setQuery] = useState("");
  const [into, setInto] = useState("receive");
  const [look, setLook] = useState<Look | null>(null);
  const [error, setError] = useState<string | null>(null), [result, setResult] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const generation = useRef(0), mounted = useRef(false);
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
      window.removeEventListener("focus", visible);
      document.removeEventListener("visibilitychange", visible);
    };
  }, [refreshDevices]);
  const sources = !at && !queue;
  useEffect(() => { if (sources) refreshDevices(); }, [sources, refreshDevices]);
  const open = (machine: DeviceRow, path: string, remember = true) => {
    if (remember) trail.current.push({ at, query });
    const mark = ++generation.current;
    if (remember) setQuery("");
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
  const destination = into === "cwd" && road.cwd ? road.cwd : null;
  const direct = (entries: Attachment[]) => entries.length > 0 && !destination && entries.every((e) => !e.device && e.from === road.machine);
  const send = async (entries: Attachment[]) => {
    if (road.busy || !entries.length) return;
    setError(null); setLook(null);
    const ok = await road.send(entries, destination);
    if (!mounted.current) return;
    setResult(ok ? gui.files_sent(entries.length) : gui.files_retry_ready);
    if (ok) setQueue(false);
  };
  const back = () => {
    if (queue) { setQueue(false); return; }
    generation.current++;
    const previous = trail.current.pop();
    setAt(previous?.at ?? null);
    setQuery(previous?.query ?? "");
  };
  const entries = at?.listing?.entries.filter((e) => e.name.toLocaleLowerCase().includes(query.toLocaleLowerCase())) ?? [];
  return <div data-files-panel className="flex min-h-0 flex-1 flex-col">
    {(at || queue) && <div data-files-head className="flex min-h-touch flex-none items-center gap-in border-b px-row">
      <button type="button" data-files-up aria-label={gui.back} onClick={back} className="grid min-h-touch min-w-touch place-items-center"><IconBack className="size-5" /></button>
      <div className="min-w-0 flex-1"><h3 className="truncate text-body font-label">{queue ? gui.files_selected : at?.machine.shown}</h3>{at && !queue && <p className="truncate font-mono text-caption text-muted-foreground" title={at.path}>{at.path}</p>}</div>
      {at && !queue && <input data-files-filter aria-label={gui.filter} placeholder={gui.filter} value={query} onChange={(e) => setQuery(e.target.value)} onMouseDown={(e) => e.stopPropagation()} className="h-ctl-md min-h-touch w-20 rounded-sm bg-fill px-in text-callout outline-none focus-visible:ring-2 focus-visible:ring-ring" />}
    </div>}
    <div data-key-panel-scroll className="min-h-0 flex-1 touch-pan-y overflow-y-auto overscroll-contain px-row">
      {queue ? road.selected.length ? road.selected.map((e) => <div data-files-selected={e.name} key={e.key} className="flex min-h-touch items-center gap-row border-b py-in">
        <Mark entry={e} /><div className="min-w-0 flex-1"><p className="truncate text-body">{e.name}</p><p className="truncate text-caption text-muted-foreground" title={e.path}>{e.shown} · {e.path}</p></div>
        {e.size !== null && <span className="flex-none text-caption tabular-nums text-muted-foreground">{sizeWord(e.size)}</span>}
        <button type="button" disabled={road.busy} aria-label={`${gui.files_deselect} ${e.name}`} onClick={() => road.remove(e)} className="grid min-h-touch min-w-touch place-items-center disabled:opacity-40"><IconClose className="size-4" /></button>
      </div>) : <p className="py-pane text-center text-callout text-muted-foreground">{gui.files_empty_selection}</p> : !at ? <>
        <h3 className="pb-in pt-in text-group text-muted-foreground">{gui.machine_here}</h3>
        <div className="flex gap-in pb-row"><Button data-files-pick="device" disabled={road.busy} className="min-h-touch flex-1" onClick={() => picker.current?.click()}><span data-files-pick-glyph className="grid size-affordance place-items-center [&>svg]:size-affordance"><IconFile /></span>{gui.files_local_file}</Button><Button data-files-pick="folder" disabled={road.busy || picking} className="min-h-touch flex-1" onClick={() => void pickFolders()}><span data-files-pick-glyph className="grid size-affordance place-items-center [&>svg]:size-affordance"><IconFiles /></span>{picking ? gui.files_reading_folder : gui.ws_kind_folder}</Button></div>
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
          const view = (face?: "source" | "preview") => setLook({ entry, machine: at.machine, face });
          return <RowMenu key={entry.key} items={[
            ...(entry.dir ? [] : [{ key: "preview", label: gui.file_face_preview, run: () => view("preview") }]),
            ...(!inherited && !road.busy ? [{ key: "select", label: own ? gui.files_deselect : gui.files_select, run: () => toggle(entry) }] : []),
            { key: "copy", label: gui.tab_copy_path, run: () => { void navigator.clipboard.writeText(entry.path).catch((e: unknown) => setError(said(e))); } },
          ]}>
            <div data-files-entry={entry.dir ? "dir" : "file"} data-files-name={entry.name} className={cn("flex min-h-touch items-center border-b", own && "bg-fill-strong")}>
              <button type="button" onClick={() => entry.dir ? open(at.machine, entry.path) : view()} className="flex min-h-touch min-w-0 flex-1 items-center gap-row py-in text-left"><Mark entry={entry} /><span className="min-w-0 flex-1 truncate text-body">{entry.name}</span>{entry.size !== null && <span className="text-caption tabular-nums text-muted-foreground">{sizeWord(entry.size)}</span>}</button>
              <button type="button" data-files-select={entry.name} aria-label={`${own ? gui.files_deselect : gui.files_select} ${entry.name}`} aria-pressed={own || inherited} disabled={inherited || road.busy} title={inherited ? gui.files_included : undefined} onClick={() => toggle(entry)} className="grid min-h-touch min-w-touch place-items-center disabled:opacity-40"><span className={cn("grid size-5 place-items-center rounded-full border", (own || inherited) && "bg-fill-strong")}>{(own || inherited) && <IconCheck className="size-3.5" />}</span></button>
            </div>
          </RowMenu>;
        })}
        {at.listing.truncated && <p role="status" className="py-in text-callout text-muted-foreground">{cli.dir_truncated(at.listing.entries.length)}</p>}
      </>}
    </div>
    <input ref={picker} type="file" multiple hidden onChange={(e) => { addDevice([...(e.target.files ?? [])], false); e.target.value = ""; }} />
    <input ref={folders} type="file" multiple hidden {...{ webkitdirectory: "" }} onChange={(e) => { addDevice([...(e.target.files ?? [])], true); e.target.value = ""; }} />
    <div className="flex flex-none flex-col gap-in border-t px-row py-in">
      {error && <p role="alert" className="text-callout text-destructive">{error}</p>}
      {road.busy ? <div data-files-transfer data-stopping={road.stopping || undefined} className="flex min-h-touch items-center gap-row"><span role="status" className="min-w-0 flex-1 truncate text-callout">{road.stopping ? gui.files_cancelling : gui.files_sending(road.selected.map((x) => x.name).join(", "))}</span>{!road.stopping && <span className="text-caption tabular-nums">{Math.round((road.progress ?? 0) * 100)}%</span>}{!road.stopping && <Button size="sm" onClick={road.cancel}>{gui.term_drop_stop}</Button>}</div> : <>
        {road.selected.length > 0 && road.cwd && <PopupButton value={into} onPick={setInto} label={gui.send_into} mark={{ "data-files-destination": true }} look="field" choices={[{ value: "receive", label: gui.files_receive }, { value: "cwd", label: gui.files_current_dir, note: road.cwd }]} />}
        <div className="flex items-center gap-row"><button type="button" data-files-review disabled={!road.selected.length} onClick={() => setQueue(!queue)} className="min-h-touch min-w-0 flex-1 truncate text-left text-callout disabled:text-muted-foreground">{road.selected.length ? gui.files_selected_count(road.selected.length) : result || gui.files_select_hint}</button><Button data-files-send disabled={!road.selected.length || picking} className="min-h-touch" onClick={() => void send(road.selected)}>{direct(road.selected) ? gui.files_insert_count(road.selected.length) : gui.files_send_count(road.selected.length)}</Button></div>
      </>}
    </div>
    <FilePreview machine={look?.machine.name ?? ""} shown={look?.machine.shown ?? ""} here={look?.machine.me ?? false} path={look?.entry.path ?? null} initialFace={look?.face} onClose={() => setLook(null)} footer={look && <><Button disabled={road.busy || included(look.entry)} onClick={() => toggle(look.entry)}>{selected(look.entry) ? gui.files_deselect : gui.files_select}</Button><Button disabled={road.busy} onClick={() => void send([look.entry])}>{direct([look.entry]) ? gui.files_insert_path : gui.files_send_session}</Button></>} />
  </div>;
}
function Mark({ entry }: { entry: Attachment }) {
  return entry.dir ? <IconFiles className="size-kind-mark flex-none text-muted-foreground" /> : <FileKindIcon name={entry.name} className="size-kind-mark flex-none" />;
}
