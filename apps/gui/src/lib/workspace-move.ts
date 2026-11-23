import type { DraggedPath } from "./path-drag";
import { directoryPrefix, isWindowsPath, parentPath, pathLeaf } from "./path-input";

const inside = (root: string, path: string) => path !== root && path.startsWith(directoryPrefix(root));
const ambiguous = (path: string) => path.split(isWindowsPath(path) ? /[\\/]/ : '/').some(part => part === '.' || part === '..');

/** Only an internal drag in this workspace is a move. Remote imports and
 * OS paths keep their existing owners. The backend resolves filesystem aliases
 * and destination races; cached entries are only an early refusal. */
export function workspaceMoveTarget(source: DraggedPath | null, machine: string | null, root: string,
  directory: string | null, names: readonly string[] = []): string | null {
  if (!source?.workspace || source.machine !== machine || source.workspace.root !== root || !directory) return null;
  if (!inside(root, source.path) || (directory !== root && !inside(root, directory))) return null;
  if (ambiguous(source.path) || ambiguous(directory)) return null;
  if (parentPath(source.path) === directory) return null;
  if (source.workspace.dir && (directory === source.path || inside(source.path, directory))) return null;
  const name = pathLeaf(source.path);
  if (!name || names.includes(name)) return null;
  return directoryPrefix(directory) + name;
}
