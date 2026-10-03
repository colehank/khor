// **Paths in prose, and the files a line carries** (round CHAT,
// 2026-10-03). One detector for every place khor turns written text into
// files: the agent's replies (`components/PathLinks`), the attachments a
// line was sent with, and whatever reads a terminal's text next.

/** What a path looks like in prose. One or more segments with at least
    one `/`, optionally `./`-, `~/`- or `/`-led, optionally `:12`-tailed.

    **Deliberately conservative about what a segment may hold**: letters,
    digits and the four punctuation marks that appear in real file names.
    A wider class starts eating the sentence around the path — a comma, a
    closing bracket, the full stop at the end of the line. */
export const PATH = /(?:\.\/|~\/|\/)?(?:[\w.@+-]+\/)+[\w.@+-]+(?::\d+)?/g;

/** Every path in a run of text, where it starts and how it is written.
    **An `@` right before a `/` or `~/` is the attachment mark and not
    the path's** (`withAttachments`): `PATH` lets a segment start with
    `@` for `node_modules/@scope/pkg`, which read `@/abs/file` as a
    relative path under whatever directory the text was resolved in. */
export function pathsIn(text: string): { index: number; written: string }[] {
  const out: { index: number; written: string }[] = [];
  const find = new RegExp(PATH.source, "g");
  let hit: RegExpExecArray | null;
  while ((hit = find.exec(text)) !== null) {
    const marked = /^@(?:\/|~\/)/.test(hit[0]);
    out.push({ index: hit.index + (marked ? 1 : 0), written: marked ? hit[0].slice(1) : hit[0] });
  }
  return out;
}

/** An attachment line: `@` then a path on the machine, the rest of the
    line (a name may hold spaces). The spelling Claude Code reads as a
    file mention; to any other agent it is a path in plain text. */
const ATTACHED = /^@((?:\/|~\/).+)$/;

/** The lines a message's files ride on, after the person's own words. */
export function withAttachments(text: string, paths: string[]): string {
  if (paths.length === 0) return text;
  const tail = paths.map((p) => `@${p}`).join("\n");
  return text.trim() === "" ? tail : `${text}\n\n${tail}`;
}

/** A sent line, split back into the words and the files it carried:
    the trailing run of attachment lines, in order. Text with none is
    all words. */
export function splitAttachments(text: string): { words: string; paths: string[] } {
  const lines = text.replace(/\s+$/, "").split("\n");
  const paths: string[] = [];
  while (lines.length > 0) {
    const hit = ATTACHED.exec(lines[lines.length - 1].trim());
    if (!hit) break;
    paths.unshift(hit[1]);
    lines.pop();
  }
  return { words: lines.join("\n").replace(/\s+$/, ""), paths };
}

/** The last segment of a path. */
export function baseName(path: string): string {
  const cut = path.replace(/\/+$/, "");
  return cut.slice(cut.lastIndexOf("/") + 1) || cut;
}

/** A name as the person gave it, from a path this machine landed: the
    landing folder prefixes the first 8 of the content's digest
    (`<digest8>-<name>`, `khor_node::transfer::payload_path`), which is
    the folder's business and not the reader's. */
export function shownName(path: string): string {
  const b = baseName(path);
  return /^[0-9a-f]{8}-./.test(b) && path.includes("/.khor/chat/") ? b.slice(9) : b;
}

/** The folder a machine keeps a conversation's files in, as some path in
    this text spells it (`<khor home>/.khor/chat/<machine>/files`,
    `khor_node::transfer::files_dir`), or `null` where none does. Read
    off the paths themselves, because only the machine knows its own
    khor home. */
export function filesFolderIn(text: string): string | null {
  const hit = /(\/[^\s@]*?\/\.khor\/chat\/[^/\s]+\/files)\/[^/\s]/.exec(text);
  return hit ? hit[1] : null;
}
