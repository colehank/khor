import type { DraggedPath } from "./path-drag";

const inside = (root: string, path: string) => root === "/" || path.startsWith(`${root}/`);
const parent = (path: string) => path.slice(0, path.lastIndexOf("/")) || "/";

/** Only an internal drag in this workspace is a move. Remote imports and
 * OS paths keep their existing owners. The backend resolves filesystem aliases
 * and destination races; cached entries are only an early refusal. */
export function workspaceMoveTarget(source: DraggedPath | null, machine: string | null, root: string,
  directory: string | null, names: readonly string[] = []): string | null {
  if (!source?.workspace || source.machine !== machine || source.workspace.root !== root || !directory) return null;
  if (!inside(root, source.path) || (directory !== root && !inside(root, directory))) return null;
  if (source.path.includes("/../") || source.path.includes("/./") || directory.includes("/../") || directory.includes("/./")) return null;
  if (parent(source.path) === directory) return null;
  if (source.workspace.dir && (directory === source.path || directory.startsWith(`${source.path}/`))) return null;
  const name = source.path.slice(source.path.lastIndexOf("/") + 1);
  if (!name || names.includes(name)) return null;
  return `${directory === "/" ? "" : directory}/${name}`;
}
