/** Path syntax belongs to the target, not to the operating system displaying it. */
export function pathSeparator(path: string): '/' | '\\' {
  return /^[a-z]:\\/i.test(path) || (path.startsWith('\\\\') || path.startsWith('~\\')) ? '\\' : '/';
}
export function directoryPrefix(path: string): string {
  return /[\\/]$/.test(path) ? path : path + pathSeparator(path);
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
