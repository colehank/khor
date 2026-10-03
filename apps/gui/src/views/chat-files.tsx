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
import { useEffect, useRef, useState } from "react";

import { chatPut, chatPutPaths } from "@/api";
import { FileKindIcon } from "@/components/FileKindIcon";
import { PathField } from "@/components/PathField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { baseName } from "@/lib/paths";
import { cn } from "@/lib/utils";
import { sizeWord } from "@/views/FilesPane";

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
