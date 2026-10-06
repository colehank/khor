/**
 * **The files panel: 📎 in the key row** (UX-41; TERM-BRIEF item 60 part
 * 3, the owner 2026-10-06 18:20, asked for now 2026-10-07 02:50: 「可不
 * 可以现在做文件面板」). It stands where the keyboard stands, in the band
 * 「…」's keys use (`lib/key-panel`), and puts files into the terminal as
 * paths, the way a file dragged into iTerm becomes its path:
 *
 * - **照片 / 文件**: the system's own pickers (the photo library; Files
 *   and the rest). What is picked is copied to `~/khor-drop/` on the
 *   session's machine and its path goes in at the cursor.
 * - **网内机器**: a machine of the mesh, then its directories; a file
 *   pressed goes the same way — on the session's machine, the path as it
 *   is; on another, the session's machine pulls it (item 60's road).
 *
 * Nothing takes the focus: the terminal keeps it, as under the keys.
 *
 * ⚠️ **Lab draft**: the paths are typed as the road would answer them;
 * the lane that takes this wires `term_drop(id, paths, from)` (term-60.md)
 * and the phone as a source.
 */
import { useEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import type { Terminal } from "@xterm/xterm";

import { fetchDevices, fetchLs, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconBack, IconChevronRight, IconFile, IconFiles } from "@/components/icons";
import { FileKindIcon } from "@/components/FileKindIcon";
import { gui } from "@/gen/catalog";
import type { DirListing } from "@/gen/bindings/DirListing";

/** A path as a shell reads it: bare when it can be, else single-quoted. */
function shellWord(p: string): string {
  return /^[\w@%+=:,./~-]+$/.test(p) ? p : `'${p.replace(/'/g, `'\\''`)}'`;
}

/** The paths in at the cursor, space-separated, through the terminal's
    own paste (bracketed when the program asked). */
function putIn(term: Terminal | null, paths: string[]) {
  if (!term || paths.length === 0) return;
  term.paste(paths.map(shellWord).join(" ") + " ");
}

const ROW = "flex min-h-touch w-full items-center gap-row border-t text-left text-body";

export function TermFiles({ term }: { term: RefObject<Terminal | null> }) {
  const photos = useRef<HTMLInputElement>(null);
  const files = useRef<HTMLInputElement>(null);
  /** Lab: what the road answers for a file copied to the session's machine. */
  const dropped = (list: FileList | null) => {
    if (!list) return;
    putIn(
      term.current,
      [...list].map((f) => `~/khor-drop/${f.name}`),
    );
  };
  const [devices, setDevices] = useState<DeviceRow[] | null>(null);
  useEffect(() => {
    void fetchDevices()
      .then(setDevices)
      .catch(() => setDevices([]));
  }, []);
  const [at, setAt] = useState<{ machine: DeviceRow; listing: DirListing | null } | null>(null);
  const open = (machine: DeviceRow, path: string) => {
    setAt({ machine, listing: null });
    void fetchLs(machine.name, path)
      .then((listing) => setAt({ machine, listing }))
      .catch(() => setAt(null));
  };
  const up = (path: string) => path.replace(/\/[^/]+\/?$/, "") || "/";

  const pickers = (
    <div data-files-pickers className="grid grid-cols-2 gap-in">
      {(
        [
          ["photos", gui.files_photos, photos, "image/*,video/*"],
          ["files", gui.files_files, files, undefined],
        ] as const
      ).map(([key, label, ref, accept]) => (
        <button
          key={key}
          type="button"
          data-files-pick={key}
          onClick={() => ref.current?.click()}
          className="flex h-20 flex-col items-center justify-center gap-xxs rounded-lg bg-fill text-callout font-label"
        >
          {key === "photos" ? <FileKindIcon name="x.png" className="size-6" /> : <IconFile className="size-6" />}
          {label}
          <input ref={ref} type="file" multiple accept={accept} hidden onChange={(e) => dropped(e.target.files)} />
        </button>
      ))}
    </div>
  );

  let mesh: ReactNode;
  if (at === null) {
    mesh = (devices ?? []).map((d) => (
      <button key={d.id} type="button" data-files-machine={d.name} onClick={() => open(d, "")} className={ROW}>
        <MachineAvatar face={d.face} className="size-kind-mark flex-none" />
        <span className="min-w-0 flex-1 truncate">{d.shown}</span>
        <IconChevronRight className="size-4 text-tertiary" />
      </button>
    ));
  } else {
    const { machine, listing } = at;
    mesh = (
      <>
        <button
          type="button"
          data-files-up
          onClick={() => (listing && listing.path !== "/" ? open(machine, up(listing.path)) : setAt(null))}
          className={ROW}
        >
          <IconBack className="size-4 flex-none text-muted-foreground" />
          <MachineAvatar face={machine.face} className="size-kind-mark flex-none" />
          <span className="min-w-0 flex-1 truncate text-muted-foreground">{listing?.path ?? machine.shown}</span>
        </button>
        {listing?.entries.map((e) => {
          const path = `${listing.path.replace(/\/$/, "")}/${e.name}`;
          return (
            <button
              key={e.name}
              type="button"
              data-files-entry={e.dir ? "dir" : "file"}
              onClick={() =>
                e.dir
                  ? open(machine, path)
                  : // Lab: this machine's file goes in as itself; another
                    // machine's as the copy the session's machine pulls.
                    putIn(term.current, [machine.me ? path : `~/khor-drop/${e.name}`])
              }
              className={ROW}
            >
              {e.dir ? <IconFiles className="size-kind-mark flex-none text-muted-foreground" /> : <FileKindIcon name={e.name} className="size-kind-mark flex-none" />}
              <span className="min-w-0 flex-1 truncate">{e.name}</span>
              {e.dir && <IconChevronRight className="size-4 text-tertiary" />}
            </button>
          );
        })}
      </>
    );
  }

  return (
    <div data-key-panel-scroll data-files-panel className="flex min-h-0 flex-1 touch-pan-y flex-col gap-base overflow-y-auto overscroll-contain px-row pt-in">
      {pickers}
      <section data-files-mesh className="flex flex-col">
        <h3 className="pb-in text-group text-muted-foreground">{gui.devices_tab}</h3>
        {mesh}
      </section>
      <p data-files-land className="pb-in text-footnote text-muted-foreground">
        {gui.files_land}
      </p>
    </div>
  );
}
