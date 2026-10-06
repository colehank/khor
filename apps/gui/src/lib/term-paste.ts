// **A paste into a terminal: its text, else its pictures** (item 60
// part 4, the audit's P12). Text on the clipboard pastes as it always
// did; a picture with no text goes down the drop's road (`dropFiles` in
// `TerminalPane`: staged where this machine's node serves it, pulled by
// the session's machine into `~/khor-drop`, its path pasted when it
// lands). A picture and text together paste the text, as iTerm2 does: a
// file copied in Finder carries its name as text and its icon as a
// picture, and the name is what a person means.

/** The picture types a paste takes, and the suffix each lands under. */
const SUFFIX: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/webp": "webp",
  "image/heic": "heic",
  "image/tiff": "tiff",
};

/** What a paste carries: text wins; pictures only when there is none. */
export type Pasted = { text: string } | { images: File[] };

const two = (n: number) => String(n).padStart(2, "0");

/** The name a pasted picture lands under, `paste-20261007-153012.png`:
    a clipboard's picture has no name of its own (WebKit calls every one
    `image.png`), and a collision is suffixed on the road. */
export function pastedName(type: string, now = new Date()): string {
  const day = `${now.getFullYear()}${two(now.getMonth() + 1)}${two(now.getDate())}`;
  const time = `${two(now.getHours())}${two(now.getMinutes())}${two(now.getSeconds())}`;
  return `paste-${day}-${time}.${SUFFIX[type] ?? "png"}`;
}

/** A picture's bytes as a file under its pasted name. */
export function pastedFile(blob: Blob, type = blob.type): File {
  return new File([blob], pastedName(type), { type });
}

/** What a paste event's clipboard carries (⌘V, iOS's own 粘贴). */
export function pastedFrom(data: DataTransfer | null): Pasted {
  const text = data?.getData("text/plain") ?? "";
  if (text || !data) return { text };
  const images: File[] = [];
  for (const item of Array.from(data.items)) {
    if (item.kind !== "file" || !(item.type in SUFFIX)) continue;
    const f = item.getAsFile();
    if (f) images.push(pastedFile(f, item.type));
  }
  return images.length ? { images } : { text };
}

/** The clipboard read for a paste a person pressed (the key row's 粘贴,
    the Mac's right-click 粘贴). It can refuse (no permission, WebKit's
    paste callout dismissed); then nothing is pasted, the true answer. */
export async function readPaste(): Promise<Pasted> {
  const board = navigator.clipboard;
  if (!board) return { text: "" };
  if (!board.read) return { text: await board.readText().catch(() => "") };
  const items = await board.read().catch(() => null);
  if (!items) return { text: "" };
  for (const item of items) {
    if (!item.types.includes("text/plain")) continue;
    const text = await item
      .getType("text/plain")
      .then((b) => b.text())
      .catch(() => "");
    if (text) return { text };
  }
  const images: File[] = [];
  for (const item of items) {
    const type = item.types.find((t) => t in SUFFIX);
    if (!type) continue;
    const blob = await item.getType(type).catch(() => null);
    if (blob) images.push(pastedFile(blob, type));
  }
  return images.length ? { images } : { text: "" };
}

/** A picture the native edit menu read off iOS's pasteboard
    (`src-tauri/src/edit_menu.rs`), base64 with its type. */
export function pastedImage(image: { type: string; data: string }): File {
  const bin = atob(image.data);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return pastedFile(new Blob([bytes], { type: image.type }), image.type);
}

/** A pressed 粘贴: the text into the terminal's own paste (bracketed
    when the program asked, the multi-line question when not), or the
    pictures to the drop. */
export function pasteInto(term: { paste: (text: string) => void }, images: (files: File[]) => void) {
  void readPaste().then((p) => {
    if ("images" in p) images(p.images);
    else if (p.text) term.paste(p.text);
  });
}
