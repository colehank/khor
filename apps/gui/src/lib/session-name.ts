import { sessionCwd, type SessionDirectory } from "./session-cwd";

/** User names stand; unnamed terminals follow their directory. OSC titles
 * are a program's transient UI and must never replace this identity. */
export function sessionName(row: SessionDirectory & { title: string }): string {
  if (!/^(-?(zsh|bash|fish|sh|pwsh|cmd)|shell)?$/.test(row.title.trim())) return row.title;
  const cwd = sessionCwd(row);
  if (!cwd) return row.title;
  return cwd.split(/[\\/]/).filter(Boolean).at(-1) ?? cwd;
}
