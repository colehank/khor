/**
 * **The files panel: 📎 in the key row** (UX-41; TERM-BRIEF item 60 part
 * 3, the owner 2026-10-06 18:20, asked for now 2026-10-07 02:50: 「可不
 * 可以现在做文件面板」). It stands where the keyboard stands, in the band
 * 「…」's keys use (`lib/key-panel`), and puts files into the terminal as
 * paths, the way a file dragged into iTerm becomes its path:
 *
 * - **照片 / 文件**: the system's own pickers (the photo library; Files
 *   and the rest). What is picked arrives as bytes, is staged where this
 *   device's node serves it by path (`term_drop_stage`), and the session's
 *   machine pulls it into its `~/khor-drop/` (`FilesRoad.dropFiles`): the
 *   phone answers `FetchPath` like any machine.
 * - **网内机器**: a machine of the mesh, then its directories; a file
 *   pressed goes the same way — on the session's machine, the path as it
 *   is; on another, the session's machine pulls it (item 60's road).
 *   **The session's machine comes first, marked as the session's** (the
 *   owner, 2026-10-07 03:10): it is where a file goes in as itself. A
 *   machine that hosts nothing (a phone) is not listed: its files are
 *   the pickers' above.
 *
 * Nothing takes the focus: the terminal keeps it, as under the keys.
 *
 * A machine's file rides the pane's drop (`FilesRoad.drop`, item 60's
 * road): on the session's machine the path goes in as it is; on another,
 * the session's machine pulls it into its `~/khor-drop/` and the path
 * there goes in when the copy is done.
 */
import { useEffect, useRef, useState, type ReactNode } from "react";

import { fetchDevices, fetchLs, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu } from "@/components/RowMenu";
import { IconBack, IconFile, IconFiles } from "@/components/icons";
import { FileKindIcon } from "@/components/FileKindIcon";
import { gui } from "@/gen/catalog";
import type { DirListing } from "@/gen/bindings/DirListing";

const ROW = "flex min-h-touch w-full items-center gap-row border-t text-left text-body";

/** What the panel needs of the terminal it puts files into. */
export type FilesRoad = {
  /** The session's machine by its table name (`DeviceRow.name`);
      `null` for a session hosted on this one. */
  machine: string | null;
  /** Paths on `from` (a machine's table name; `null`: this one) put
      where the session's machine reads them, and pasted when they are
      there: the pane's own drop (item 60's road, `term_drop`), its
      progress on the pill's line and a failure in its corner. */
  drop: (paths: string[], from: string | null, into?: string | null) => void;
  /** The shell's current directory on the session's machine (its `cwd`
      mark), where 「放到当前目录」 lands a file; `null` while unknown,
      and then the menu does not offer it. */
  cwd: string | null;
  /** Files picked on this device (照片与文件): staged where its node
      serves them, then the same drop from here. */
  dropFiles: (files: File[]) => void;
};

/** The mesh's machines for the panel: the session's first, then the
    rest as the table has them; a machine that hosts nothing left out. */
export function machinesFor(devices: DeviceRow[], machine: string | null): DeviceRow[] {
  const ours = (d: DeviceRow) => (machine === null ? d.me : d.name === machine);
  const hosting = devices.filter((d) => d.hosts);
  return [...hosting.filter(ours), ...hosting.filter((d) => !ours(d))];
}

export function TermFiles({ road }: { road: FilesRoad }) {
  const picker = useRef<HTMLInputElement>(null);
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

  // **One picker** (the owner 2026-10-07, on the iPhone: 照片 and 文件
  // opened the same system menu — choose a photo, take one, choose a
  // file): every kind, several at once. ⚠️ 墓碑: two buttons, 照片 with
  // an image filter and 文件 without.
  const pickers = (
    <button
      type="button"
      data-files-pick="device"
      onClick={() => picker.current?.click()}
      className="flex h-16 w-full flex-none items-center justify-center gap-in rounded-lg bg-fill text-callout font-label"
    >
      <FileKindIcon name="x.png" className="size-6" />
      <IconFile className="size-6" />
      {gui.files_device}
      <input
        ref={picker}
        type="file"
        multiple
        hidden
        onChange={(e) => {
          road.dropFiles([...(e.target.files ?? [])]);
          // The same file picked again is a change again.
          e.target.value = "";
        }}
      />
    </button>
  );

  // **No › at a row's end** (item 72, the owner 2026-10-07: 「list 不要有
  // 右侧的 >」): a machine or a directory still opens on a tap; its mark
  // (a face, the folder) says it is one to open. ⚠️ 墓碑: a tertiary
  // chevron at the end of the machine and directory rows.
  let mesh: ReactNode;
  if (at === null) {
    mesh = machinesFor(devices ?? [], road.machine).map((d, i) => {
      const session = i === 0 && (road.machine === null ? d.me : d.name === road.machine);
      return (
        <button
          key={d.id}
          type="button"
          data-files-machine={d.name}
          data-files-session={session || undefined}
          onClick={() => open(d, "")}
          className={ROW}
        >
          <MachineAvatar face={d.face} className="size-kind-mark flex-none" />
          <span className="min-w-0 flex-1 truncate">{d.shown}</span>
          {session && <span className="flex-none text-footnote text-muted-foreground">{gui.files_session}</span>}
        </button>
      );
    });
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
          const from = machine.me ? null : machine.name;
          const row = (
            <button
              key={e.name}
              type="button"
              data-files-entry={e.dir ? "dir" : "file"}
              onClick={() => (e.dir ? open(machine, path) : road.drop([path], from))}
              className={ROW}
            >
              {e.dir ? <IconFiles className="size-kind-mark flex-none text-muted-foreground" /> : <FileKindIcon name={e.name} className="size-kind-mark flex-none" />}
              <span className="min-w-0 flex-1 truncate">{e.name}</span>
            </button>
          );
          // A file's menu (long press, or a secondary click): where it
          // lands — the press's own ~/khor-drop/, or the shell's current
          // directory, offered only while that is known (part 5).
          if (e.dir) return row;
          const acts = [{ key: "to-term", label: gui.files_to_term, run: () => road.drop([path], from) }];
          const cwd = road.cwd;
          if (cwd) acts.push({ key: "to-cwd", label: gui.files_to_cwd, run: () => road.drop([path], from, cwd) });
          return (
            <RowMenu key={e.name} items={acts}>
              {row}
            </RowMenu>
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
