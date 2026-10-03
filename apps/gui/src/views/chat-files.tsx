// **Files in an agent conversation** (round CHAT, 2026-10-03, the owner:
// 「gui 对话得做到像 WeChat 一样：发文件、预览文件、还得显示文件在哪个路径」).
//
// # Where a file goes, and what the agent is told
//
// A file from the device — a phone's photo, a Mac's desktop file, a drop —
// is put on the machine the conversation runs on, in that machine's chat
// files folder (`<khor home>/.khor/chat/<machine>/files/<digest8>-<name>`,
// `khor_gui_core::chat::chat_put`), the very folder a `khor send` to that
// machine lands in. A file already on that machine (「这台机器上的文件」) is
// not copied at all. Either way the agent is told the **path**, one line
// `@<path>` per file after the person's words (`lib/paths`): what Claude
// Code and Codex read. A picture also rides inline when the agent said it
// takes pictures, as it did before this round.
//
// # Composing
//
// Each file is put on the machine as soon as it is chosen, so sending
// waits on nothing that could have happened earlier; its chip says so
// while it travels (busy), and says why when it could not.
//
// # Looking at one
//
// A pressed file opens in `FilePreview`, which is the workspace's own
// `FileView` in a window — pictures, PDFs, markdown, code, and the same
// sentences for what it will not draw — read only, with the three things
// a person does next: open it where it lives, copy its path, and hand it
// on (Finder on the Mac, the system's share sheet on a phone).
import { useEffect, useRef, useState } from "react";

import { chatPut, chatPutPaths, fetchFileText, fetchPreview, fetchRevealHere, pullFile, reveal } from "@/api";
import { FileKindIcon } from "@/components/FileKindIcon";
import { PathCut } from "@/components/PathCut";
import { PathField } from "@/components/PathField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { baseName, shownName } from "@/lib/paths";
import { cn } from "@/lib/utils";
import { sizeWord } from "@/views/FilesPane";
import { FileView, kindOf, rendererOf } from "@/views/FileView";
import { useShowFile } from "@/views/linkHome";

/** One file going along with the line being written. */
export type Pending = {
  key: string;
  name: string;
  /** Bytes, where the device said; `null` for a file named by path. */
  size: number | null;
  /** A small picture for the chip: an object URL, images under 1 MiB. */
  thumb: string | null;
  /** The picture itself for agents that take pictures, `[mime, base64]`. */
  inline: [string, string] | null;
  /** Where it is on the conversation's machine, once it is there. */
  path: string | null;
  /** Why it could not be put there. */
  error: string | null;
};

/** How large a picture still rides inline beside its path (the frame's
    ceiling, `ChatView`'s old `SHOT_MAX`): base64 inside one frame. */
const INLINE_MAX = 5 * 1024 * 1024;
/** How large a picture still gets a thumbnail on its chip. */
const THUMB_MAX = 1024 * 1024;

const why = (e: unknown) => String((e as { message?: string })?.message ?? e);

async function base64Of(f: File): Promise<string> {
  const buf = new Uint8Array(await f.arrayBuffer());
  // Chunked: spreading a multi-megabyte array into one call overflows the
  // stack at a size a test image never reaches.
  let bin = "";
  for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
  return btoa(bin);
}

export function usePendingFiles(id: string) {
  const [items, setItems] = useState<Pending[]>([]);
  const seq = useRef(0);
  const mounted = useRef(true);
  // Set on every mount, not only in the ref's first value: React's
  // development double mount runs the cleanup once in between.
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const patch = (key: string, p: Partial<Pending>) =>
    mounted.current && setItems((now) => now.map((i) => (i.key === key ? { ...i, ...p } : i)));

  /** Files from the device: put on the machine at once. */
  const addFiles = (files: File[]) => {
    for (const f of files) {
      const key = `f${++seq.current}`;
      const picture = f.type.startsWith("image/");
      setItems((now) => [
        ...now,
        {
          key,
          name: f.name || gui.chat_image(String(seq.current)),
          size: f.size,
          thumb: picture && f.size <= THUMB_MAX ? URL.createObjectURL(f) : null,
          inline: null,
          path: null,
          error: null,
        },
      ]);
      // Read for every small picture; whether it rides is the send's
      // question, asked of the agent's handshake then.
      if (picture && f.size <= INLINE_MAX) {
        void base64Of(f).then((b) => patch(key, { inline: [f.type, b] }));
      }
      chatPut(id, f).then(
        (path) => patch(key, { path }),
        (e) => patch(key, { error: why(e) }),
      );
    }
  };

  /** Paths this device has (a drop in the app): put on the machine. */
  const addLocalPaths = (paths: string[]) => {
    const keys = paths.map(() => `p${++seq.current}`);
    setItems((now) => [
      ...now,
      ...paths.map((p, i) => ({ key: keys[i], name: baseName(p), size: null, thumb: null, inline: null, path: null, error: null })),
    ]);
    chatPutPaths(id, paths).then(
      (landed) => landed.forEach((path, i) => patch(keys[i], { path })),
      (e) => keys.forEach((k) => patch(k, { error: why(e) })),
    );
  };

  /** Paths already on the conversation's machine: nothing to move. */
  const addPaths = (paths: string[]) =>
    setItems((now) => [
      ...now,
      ...paths.map((p) => ({ key: `m${++seq.current}`, name: baseName(p), size: null, thumb: null, inline: null, path: p, error: null })),
    ]);

  const remove = (key: string) =>
    setItems((now) => {
      const gone = now.find((i) => i.key === key);
      if (gone?.thumb) URL.revokeObjectURL(gone.thumb);
      return now.filter((i) => i.key !== key);
    });
  const clear = () =>
    setItems((now) => {
      for (const i of now) if (i.thumb) URL.revokeObjectURL(i.thumb);
      return [];
    });
  /** A file still on its way: sending waits for it. */
  const busy = items.some((i) => i.path === null && i.error === null);
  return { items, addFiles, addLocalPaths, addPaths, remove, clear, busy };
}

/** The chips over the text: each file going along, with its × to take
    it back off. A thumbnail for a small picture, the kind's icon
    otherwise; the size where it is known; busy while it travels to the
    machine, the reason where it did not get there. */
export function PendingChips({ items, onRemove }: { items: Pending[]; onRemove: (key: string) => void }) {
  if (items.length === 0) return null;
  return (
    <div data-chat-pending className="flex flex-wrap items-center gap-in px-2 pb-in">
      {items.map((i) => (
        <span
          key={i.key}
          data-chat-pending-file={i.name}
          data-placed={i.path !== null || undefined}
          aria-busy={(i.path === null && i.error === null) || undefined}
          title={i.error ?? i.path ?? undefined}
          className={cn(
            "flex max-w-full min-w-0 items-center gap-in rounded-lg bg-fill py-1 ps-1 pe-0 text-callout",
            "aria-busy:animate-pulse motion-reduce:aria-busy:animate-none",
          )}
        >
          {i.thumb ? (
            <img src={i.thumb} alt="" className="size-7 flex-none rounded-md object-cover" />
          ) : (
            <FileKindIcon name={i.name} className="size-7 flex-none" />
          )}
          <span className="flex min-w-0 flex-col">
            <span className="max-w-48 min-w-0 truncate">{i.name}</span>
            {(i.error || i.size !== null) && (
              <span
                className={cn("text-caption tabular-nums", i.error ? "text-state-failed" : "text-muted-foreground")}
              >
                {i.error ? gui.chat_put_failed : sizeWord(i.size ?? 0)}
              </span>
            )}
          </span>
          <button
            type="button"
            data-chat-pending-drop
            aria-label={gui.remove}
            onClick={() => onRemove(i.key)}
            className="grid min-h-touch min-w-touch flex-none place-items-center text-muted-foreground pointer-fine:min-h-ctl-md pointer-fine:min-w-ctl-md"
          >
            ×
          </button>
        </span>
      ))}
    </div>
  );
}

/** 「<machine> 上的文件…」: a path on the conversation's machine, typed
    with the file completion every directory box has (`PathField`). Not
    copied anywhere — the agent is told where it already is. */
export function MachineFilePick({
  open,
  onOpenChange,
  machine,
  shown,
  onPick,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The API's spelling: `""` is this machine. */
  machine: string;
  /** The name a person reads. */
  shown: string;
  onPick: (path: string) => void;
}) {
  const [value, setValue] = useState("~/");
  useEffect(() => {
    if (open) setValue("~/");
  }, [open]);
  const take = () => {
    const v = value.trim();
    if (!v || v.endsWith("/")) return;
    onPick(v);
    onOpenChange(false);
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form data-chat-machine-pick>
        <DialogHeader>
          <DialogTitle>{gui.chat_attach_machine_title(shown)}</DialogTitle>
        </DialogHeader>
        <PathField machine={machine} value={value} onValueChange={setValue} onSubmit={take} files autoFocus />
        <DialogFooter>
          <Button data-dialog-cancel onClick={() => onOpenChange(false)}>
            {gui.delete_keep}
          </Button>
          <Button variant="primary" data-chat-machine-pick-add onClick={take}>
            {gui.watch_add}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** Whether Finder can show this machine's files, asked once a page. */
let revealing: Promise<boolean> | null = null;
const revealHere = () => (revealing ??= fetchRevealHere().catch(() => false));

/** The file's bytes for the share sheet, from the reads the preview
    already makes: a picture or a PDF as the preview carries it, text as
    text. `null` for anything else, which is not offered. */
async function bytesOf(machine: string, path: string): Promise<File | null> {
  const name = shownName(path);
  const kind = kindOf(path);
  if (kind === "text") {
    const f = await fetchFileText(machine, path);
    return f.binary ? null : new File([f.text], name, { type: "text/plain" });
  }
  if (kind === "doc") return null;
  const look = await fetchPreview(machine, path);
  if (look.data === "" || look.cut) return null;
  const raw = atob(look.data);
  const buf = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) buf[i] = raw.charCodeAt(i);
  return new File([buf], name, { type: look.media });
}

/** The system's share sheet takes files here (a phone's WebKit; a desktop
    browser may take only links). */
const shares = () =>
  typeof navigator.share === "function" &&
  typeof navigator.canShare === "function" &&
  navigator.canShare({ files: [new File([""], "x.txt", { type: "text/plain" })] });

/** One file of a conversation, looked at (round CHAT item 3). */
export function FilePreview({
  machine,
  shown,
  path,
  here,
  onClose,
}: {
  /** The API's spelling of the conversation's machine. */
  machine: string;
  /** That machine is this one: its files are already on this disk. */
  here: boolean;
  /** Its name as a person reads it; empty draws no name. */
  shown: string;
  /** Absolute on that machine; `null` is closed. */
  path: string | null;
  onClose: () => void;
}) {
  const narrow = useNarrow();
  const show = useShowFile();
  const [finder, setFinder] = useState(false);
  const [said, setSaid] = useState<"copied" | "failed" | string | null>(null);
  useEffect(() => {
    void revealHere().then(setFinder);
  }, []);
  useEffect(() => setSaid(null), [path]);
  const kind = path === null ? "text" : kindOf(path);
  const sharing = path !== null && kind !== "doc" && shares();
  const copy = () => {
    if (path === null) return;
    navigator.clipboard.writeText(path).then(
      () => setSaid("copied"),
      () => setSaid("failed"),
    );
  };
  // A far machine's file is taken to this Mac first (the downloads,
  // `pullFile`) and that copy is the one Finder shows.
  const toFinder = () => {
    if (path === null) return;
    const local = here ? Promise.resolve(path) : pullFile(machine, path);
    void local
      .then((p) => reveal(p)).catch((e: unknown) => setSaid(String((e as Error)?.message ?? e)));
  };
  const share = () => {
    if (path === null) return;
    void bytesOf(machine, path)
      .then((f) => (f ? navigator.share({ files: [f] }) : undefined))
      .catch(() => {
        // Dismissing the sheet rejects too; nothing to say about it.
      });
  };
  return (
    <Dialog open={path !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent
        data-file-preview={path ?? undefined}
        // The phone's sheet (grabber, pulled down to put away); a window
        // over the scrim elsewhere, closed by a press outside or Esc.
        form={narrow}
        // Important, all four: the dialog's own `p-pane`, `gap-row` and
        // `w-dialog` are tokens tailwind-merge cannot tell are a padding,
        // a gap and a width, so it keeps both and theirs wins.
        className={cn(
          "flex flex-col overflow-hidden !gap-0 !p-0",
          narrow ? "h-[calc(100dvh-var(--safe-top)-3rem)] !w-[calc(100%-1rem)]" : "h-[80vh] !w-[min(56rem,calc(100%-2rem))]",
        )}
      >
        {path !== null && (
          <>
            <div data-file-preview-head className="flex flex-none flex-col gap-in px-pane pt-pane pb-row">
              <div className="flex min-w-0 items-center gap-row">
                <DialogTitle className="min-w-0 flex-1 truncate">{shownName(path)}</DialogTitle>
                <Button size="sm" data-file-preview-close onClick={onClose}>
                  {gui.close}
                </Button>
              </div>
              <div className="flex min-w-0 text-caption text-muted-foreground">
                {shown && <span className="flex-none">{shown}&nbsp;·&nbsp;</span>}
                <PathCut text={path} className="min-w-0 flex-1" />
              </div>
              <div className="flex flex-wrap items-center gap-in">
                {show && (
                  <Button
                    size="sm"
                    data-file-preview-open
                    onClick={() => {
                      onClose();
                      show(machine, path);
                    }}
                  >
                    {gui.chat_file_open}
                  </Button>
                )}
                <Button size="sm" data-file-preview-copy onClick={copy}>
                  {gui.tab_copy_path}
                </Button>
                {finder && (
                  <Button size="sm" data-file-preview-reveal onClick={toFinder}>
                    {gui.chat_file_reveal}
                  </Button>
                )}
                {!finder && sharing && (
                  <Button size="sm" data-file-preview-share onClick={share}>
                    {gui.chat_file_share}
                  </Button>
                )}
                {said !== null && (
                  <span data-file-preview-said className="text-callout text-muted-foreground">
                    {said === "copied" ? gui.copied : said === "failed" ? gui.copy_failed : said}
                  </span>
                )}
              </div>
            </div>
            <div data-file-preview-body className="flex min-h-0 flex-1 flex-col border-t">
              {/* The workspace's own view of a file, read only: editing is
                  the file view's, one press away above. */}
              <FileView
                key={`${machine} ${path}`}
                machine={machine}
                path={path}
                git={false}
                face={rendererOf(path) ? "preview" : "source"}
                reading
              />
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
