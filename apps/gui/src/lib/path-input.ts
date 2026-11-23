/** Path syntax belongs to the target, not to the operating system displaying it. */
export function pathSeparator(path: string): '/' | '\\' {
  return /^[a-z]:\\/i.test(path) || (path.startsWith('\\\\') || path.startsWith('~\\')) ? '\\' : '/';
}

export const isWindowsPath = (path: string) => /^[a-z]:[\\/]|^[\\/]{2}|^~\\/i.test(path);

export function directoryPrefix(path: string): string {
  const separator = pathSeparator(path);
  return path.endsWith(separator) || separator === '\\' && path.endsWith('/') ? path : path + separator;
}

export function pathLeaf(path: string): string {
  const windows = isWindowsPath(path);
  const cut = path.replace(windows ? /[\\/]+$/ : /\/+$/, '');
  const at = windows ? Math.max(cut.lastIndexOf('/'), cut.lastIndexOf('\\')) : cut.lastIndexOf('/');
  return cut.slice(at + 1) || cut;
}
export function settledPath(typed: string): string {
  const value = typed.trim();
  if (value === '/' || /^[a-z]:[\\/]$/i.test(value)) return value;
  return value.replace(/[\\/]+$/, '');
}
export function splitPath(typed: string): {
  parent: string;
  leaf: string;
} | null {
  const cut = Math.max(typed.lastIndexOf('/'), typed.lastIndexOf('\\'));
  if (cut < 0) return null;
  const parent = typed.slice(0, cut + 1);
  if (!parent.startsWith('/') && !/^~[\\/]/.test(parent) && !/^[a-z]:[\\/]/i.test(parent) && !parent.startsWith('\\\\')) return null;
  return {
    parent,
    leaf: typed.slice(cut + 1)
  };
}
export function homeRelative(home: string, relative: string): string {
  return directoryPrefix(home) + relative.replace(/[\\/]/g, pathSeparator(home));
}

export function parentPath(path: string): string | null {
  const drive = /^(?:\\\\\?\\)?[a-z]:[\\/]/i.exec(path)?.[0];
  if (path === '/' || path === drive || /^\\\\(?:\?\\UNC\\)?[^\\]+\\[^\\]+\\?$/i.test(path)) return null;
  const windows = isWindowsPath(path);
  const cut = path.replace(windows ? /[\\/]+$/ : /\/+$/, '');
  const at = windows ? Math.max(cut.lastIndexOf('/'), cut.lastIndexOf('\\')) : cut.lastIndexOf('/');
  if (at < 0) return null;
  return drive && at === drive.length - 1 ? drive : cut.slice(0, at) || '/';
}
