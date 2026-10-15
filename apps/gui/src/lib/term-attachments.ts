/** Selection and staging shared by the key panel, paste and browser drops.
 * Selection owns native picker tokens; each send owns its browser tokens.
 * Cancellation releases only the latter, so selected folders can be retried. */
import { termDropFolder, termDropForget, termDropStage, termDropStageInto } from "@/api";
import type { DropItem } from "@/gen/bindings/DropItem";
import { msg } from "@/gen/catalog";

export const ATTACHMENT_CAP = 200 * 1024 * 1024;
const ENTRY_CAP = 20_000;
type DeviceTree = { files: { file: File; relative: string }[]; dirs: string[] };
export type Attachment = {
  key: string; name: string; dir: boolean; size: number | null;
  shown: string; path: string; from: string | null;
  /** Native picker paths can be app-owned copies on iOS. */
  native?: boolean;
  device?: DeviceTree;
};
export type PreparedDrop = { items: DropItem[]; staged: string[] };
let serial = 0;
export const pathName = (p: string) => p.replace(/[\\/]+$/, "").split(/[\\/]/).at(-1) || p;
const pathKey = (p: string) => /^[a-z]:[\\/]|^\\\\/i.test(p) ? p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase() : p.replace(/\/+$/, "");
export function pathAttachment(path: string, from: string | null, shown: string, dir = false, size: number | null = null, native = false): Attachment {
  return { key: `${from ?? ""}\n${pathKey(path)}`, path, from, shown, name: pathName(path), dir, size, native };
}
export function includesAttachment(parent: Attachment, child: Attachment): boolean {
  return !parent.device && !child.device && parent.dir && parent.from === child.from && pathKey(child.path).startsWith(pathKey(parent.path) + "/");
}
export function addAttachments(old: Attachment[], entries: Attachment[]): Attachment[] {
  let now = [...old];
  for (const e of entries) {
    if (now.some((x) => x.key === e.key || includesAttachment(x, e))) continue;
    now = [...now.filter((x) => !includesAttachment(e, x)), e];
  }
  return now;
}
export function joinAttachmentPath(dir: string, name: string): string {
  const separator = dir.includes("\\") ? "\\" : "/";
  return dir.replace(/[\\/]+$/, "") + separator + name;
}
export function parentAttachmentPath(path: string): string | null {
  if (/^(?:\/|[a-z]:[\\/]|\\\\[^\\]+\\[^\\]+\\?)$/i.test(path)) return null;
  const cut = path.replace(/[\\/]+$/, "");
  const at = Math.max(cut.lastIndexOf("/"), cut.lastIndexOf("\\"));
  if (at < 0) return null;
  return at === 2 && /^[a-z]:/i.test(cut) ? cut.slice(0, 3) : cut.slice(0, at) || "/";
}
function safeRelative(path: string): string {
  if (!path || /[\\:\0]/.test(path) || path.split("/").some((p) => !p || p === "." || p === "..") || path.split("/").length > 64) throw new Error(msg.tree_bad_path(path));
  return path;
}
function deviceAttachment(name: string, dir: boolean, tree: DeviceTree, shown: string): Attachment {
  const size = tree.files.reduce((n, f) => n + f.file.size, 0);
  if (size > ATTACHMENT_CAP) throw new Error(msg.tree_too_big(ATTACHMENT_CAP >> 20));
  if (tree.files.length + tree.dirs.length > ENTRY_CAP) throw new Error(msg.tree_too_many(ENTRY_CAP));
  return { key: `device-${++serial}`, name, dir, size, path: name, from: null, shown, device: tree };
}
/** webkitdirectory's FileList omits empty folders; native selection and
 * DataTransfer entries retain them. All file names stay relative. */
export function deviceAttachments(files: File[], folders: boolean, shown: string): Attachment[] {
  if (!folders) return files.map((file) => deviceAttachment(file.name, false, { files: [{ file, relative: file.name }], dirs: [] }, shown));
  const groups = new Map<string, DeviceTree>();
  for (const file of files) {
    const [name, ...parts] = safeRelative(file.webkitRelativePath).split("/");
    if (!parts.length) throw new Error(msg.tree_bad_path(file.webkitRelativePath));
    const tree = groups.get(name) ?? { files: [], dirs: [] };
    const relative = parts.join("/");
    tree.files.push({ file, relative });
    for (let i = 1; i < parts.length; i++) tree.dirs.push(parts.slice(0, i).join("/"));
    groups.set(name, tree);
  }
  return [...groups].map(([name, tree]) => deviceAttachment(name, true, { ...tree, dirs: [...new Set(tree.dirs)] }, shown));
}
/** Capture handles during drop; enumerate asynchronously after the DOM event. */
export async function droppedAttachments(data: DataTransfer, shown: string): Promise<Attachment[]> {
  const roots = [...data.items].filter((i) => i.kind === "file").map((i) => i.webkitGetAsEntry());
  const files = [...data.files];
  if (!roots.length || roots.some((e) => !e)) return deviceAttachments(files, false, shown);
  const out: Attachment[] = [];
  for (const root of roots) {
    const tree: DeviceTree = { files: [], dirs: [] };
    let bytes = 0, entries = 0;
    const walk = async (entry: FileSystemEntry, relative: string, depth: number): Promise<void> => {
      if (++entries > ENTRY_CAP || depth > 64) throw new Error(msg.tree_too_many(ENTRY_CAP));
      if (relative) safeRelative(relative);
      if (entry.isFile) {
        const file = await new Promise<File>((ok, fail) => (entry as FileSystemFileEntry).file(ok, fail));
        bytes += file.size;
        if (bytes > ATTACHMENT_CAP) throw new Error(msg.tree_too_big(ATTACHMENT_CAP >> 20));
        tree.files.push({ file, relative: relative || file.name });
      } else if (entry.isDirectory) {
        if (relative) tree.dirs.push(relative);
        const reader = (entry as FileSystemDirectoryEntry).createReader();
        for (;;) {
          const batch = await new Promise<FileSystemEntry[]>((ok, fail) => reader.readEntries(ok, fail));
          if (!batch.length) break;
          for (const child of batch) await walk(child, relative ? `${relative}/${child.name}` : child.name, depth + 1);
        }
      } else throw new Error(msg.tree_entry_unsupported(entry.fullPath));
    };
    await walk(root!, "", 0);
    out.push(deviceAttachment(root!.name, root!.isDirectory, tree, shown));
  }
  return out;
}
export async function prepareAttachments(entries: Attachment[], gone: () => boolean): Promise<PreparedDrop | null> {
  const prepared: PreparedDrop = { items: [], staged: [] };
  try {
    for (const entry of entries) {
      if (gone()) break;
      if (!entry.device) {
        prepared.items.push({ path: entry.path, from: entry.from, keep: !!entry.native });
        continue;
      }
      let path: string;
      if (entry.dir) {
        path = await termDropFolder(entry.name, entry.device.dirs);
        prepared.staged.push(path);
        for (const { file, relative } of entry.device.files) {
          if (gone()) break;
          await termDropStageInto(path, relative, file);
        }
      } else {
        path = await termDropStage(entry.device.files[0].file);
        prepared.staged.push(path);
      }
      prepared.items.push({ path, from: null, keep: false });
    }
    if (!gone()) return prepared;
  } catch (e) {
    await termDropForget(prepared.staged).catch(() => {});
    throw e;
  }
  await termDropForget(prepared.staged).catch(() => {});
  return null;
}
