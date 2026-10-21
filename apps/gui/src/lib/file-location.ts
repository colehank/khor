/** A deliberate “show in workspace” request survives the file page leaving and
 * is consumed by the first visible tree that can actually locate its row. */
export type FileLocation = { id: number; machine: string; path: string };
const pending = new Map<string, FileLocation>();
const listeners = new Set<() => void>();
let nextId = 0;

export function requestFileLocation(machine: string, path: string): void {
  pending.set(machine, { id: ++nextId, machine, path });
  for (const listener of listeners) listener();
}

export function pendingFileLocation(machine: string): FileLocation | undefined {
  return pending.get(machine);
}

export function consumeFileLocation(request: FileLocation): boolean {
  if (pending.get(request.machine) !== request) return false;
  pending.delete(request.machine);
  return true;
}

export function onFileLocation(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

const windowsPath = (path: string) => /^[a-z]:[\\/]|^[\\/]{2}/i.test(path);
const normalized = (path: string) => windowsPath(path) ? path.replaceAll('\\', '/').replace(/\/+/g, '/') : path;
export function sameFileLocation(a: string, b: string): boolean {
  return windowsPath(a) && windowsPath(b)
    ? normalized(a).toLowerCase() === normalized(b).toLowerCase()
    : a === b;
}

/** Ancestors only, never the target directory itself. Names retain the tree's
 * own root spelling; containment has a boundary, so /app never claims /apple. */
export function fileLocationAncestors(root: string, path: string): string[] | null {
  const base = normalized(root).replace(/\/+$/, '') || '/';
  const full = normalized(path);
  const insensitive = windowsPath(root);
  const prefix = base === '/' ? '/' : `${base}/`;
  const compare = (value: string) => insensitive ? value.toLowerCase() : value;
  if (compare(full) === compare(base)) return [];
  if (!compare(full).startsWith(compare(prefix))) return null;
  const parts = full.slice(prefix.length).split('/');
  if (parts.some(part => part === '.' || part === '..' || part === '')) return null;
  const original = root.replace(/\/+$/, '') || '/';
  return parts.slice(0, -1).map((_, index) =>
    `${original === '/' ? '' : original}/${parts.slice(0, index + 1).join('/')}`,
  );
}
