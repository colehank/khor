/**
 * **存储** (STORE-BRIEF phase 1, the owner 2026-10-07: 「工作区不显示"放进
 * 来的"，这些上传的文件，统一到更多里，有个存储吧，这里可以查看上传文件，
 * 管理上传文件，以及对话session接收文件的存储默认位置」): one machine's
 * received files and where they land.
 *
 * - **Any machine, this one first** (the owner: 「存储 can switch to
 *   another machine」): a popup of the machines beside the find box,
 *   drawn only when there is more than one (progressive disclosure).
 * - **Where its files land**: the setting on that machine's row
 *   (`receive_dir`), changed through the directory picker (选择目录) or
 *   taken back to the default; under it, where the machine itself
 *   resolved it (`receive_at`), why when it fell back (`receive_why`, its
 *   own sentence), and that it has not seen a change yet
 *   (`receive_for` ≠ the setting).
 * - **What landed there**: the files in `receive_at`, newest first, as
 *   `SideList` rows (a file's kind on the machine's face, no ›). A press
 *   opens the file here, as the file page with its deck (‹ back to the
 *   list); the row's menu has 打开, 复制路径, 在文件视图中打开 (the
 *   workspace holding it, else that machine's files) and 删除… — the
 *   file trees' own ask (`useRemoveFlow`). Deleting a received file is
 *   done here or in a tree: closing a conversation's file row no longer
 *   deletes it (the lead, 2026-10-07).
 *
 * **Read again only when it changed** (store-c): no beat while nothing
 * happens. A far machine watches the receive location and answers a long
 * wait (`dirsWait`, the file tree's watch, NET-4); khor's own acts on
 * files — a delete, a rename, a drop landing there — say the level they
 * changed (`lib/tree-changed`). Where the watch cannot hear everything —
 * this machine and any other it cannot watch (`unavailable`), or a
 * location on a network mount another host writes into too
 * (`unreported`: turing and hinton share an NFS home) — the page also
 * reads again when the window comes back to the front and, slowly, while
 * it stays open. A new receive location is a new watch.
 *
 * The same page is the 附栏's 更多 → 存储 seat (`bare`): no head of its
 * own there — the 附栏's one head says who and which seat — and where it
 * stands (the machine, the file open) is said as it moves (`onSpot`) and
 * handed back when the 附栏 opens again (`spot`, UX-47).
 *
 * ⚠️ 墓碑 (STORE-BRIEF phase 1): 「放进来的」, a row above the 工作区
 * seat's tree opening `~/khor-drop` as a tree of its own.
 */
import { useEffect, useRef, useState } from "react";

import { dirsWait, fetchLs, setReceiveDir, type DeviceRow } from "@/api";
import { DirPicker } from "@/components/DirPicker";
import { GlassCircle } from "@/components/Glass";
import { IconBack } from "@/components/icons";
import { PathCut } from "@/components/PathCut";
import { RemoveDialog } from "@/components/RemoveDialog";
import { fileKind } from "@/components/SessionFace";
import { SideFind } from "@/components/SideFind";
import { NarrowHead } from "@/components/SideHead";
import { SideList, type SideRow } from "@/components/SideList";
import { Button } from "@/components/ui/button";
import { PopupButton } from "@/components/ui/popup-button";
import type { DirRow } from "@/gen/bindings/DirRow";
import { gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import type { StoreSpot } from "@/hooks/use-session-side";
import { onTreeChanged, sameMachine } from "@/lib/tree-changed";
import { FilePage } from "@/views/FilePage";
import { sizeWord } from "@/views/FilesPane";
import { useRemoveFlow } from "@/views/fileRow";
import { ago } from "@/words";

/** How long one watch question is held when nothing changes (the
    file tree's `WATCH_HOLD_MS`). */
const WATCH_HOLD_MS = 20_000;
/** Where the watch cannot hear every change: how often the listing is
    asked again while the page stays open. Slow — the window coming back
    to the front asks at once. */
const UNHEARD_MS = 60_000;

/** A directory as one spelling: no trailing slash. */
const level = (d: string) => d.replace(/\/+$/, "") || "/";

export function StorePage({
  devices,
  spot,
  onSpot,
  onChanged,
  onReveal,
}: {
  devices: DeviceRow[];
  /** 在文件视图中打开: the main area's road on 更多's page (the workspace
      holding the file, else that machine's files), the 工作区 seat's tree
      in the 附栏 — the column never sends the person away. */
  onReveal: (machine: string, path: string) => void;
  /** Where it was left (UX-47), read once at mount; absent starts on
      this machine's list. */
  spot?: StoreSpot | null;
  onSpot?: (spot: StoreSpot) => void;
  /** A setting written: the device table is worth reading again. */
  onChanged?: () => void;
}) {
  const here = devices.find((d) => d.me) ?? null;
  const [machine, setMachine] = useState<string>(spot?.machine ?? here?.name ?? "");
  const row = devices.find((d) => d.name === machine) ?? here;
  const name = row?.name ?? machine;
  const isHere = row?.me ?? true;
  const [file, setFile] = useState<string | null>(spot?.file ?? null);
  useEffect(() => {
    onSpot?.({ machine: name, file });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [name, file]);

  const dir = row?.receive_at ?? null;
  const [listed, setListed] = useState<{ dir: string; rows: DirRow[] } | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [again, setAgain] = useState(0);
  // One read per ask: the mount, a new machine or location, and every
  // `again` below.
  useEffect(() => {
    if (dir === null) return;
    let live = true;
    fetchLs(name, dir).then(
      (l) => {
        if (!live) return;
        setListed({ dir, rows: l.entries });
        setFailed(null);
      },
      (e: unknown) => live && setFailed(String(e instanceof Error ? e.message : e)),
    );
    return () => {
      live = false;
    };
  }, [name, dir, again]);

  /** The watch hears every change to this location: khor's own acts are
      left to it then, as a watched tree leaves them (`FileTree`). */
  const heard = useRef(false);
  const hidden = useHidden();
  /** Hidden since the last arm: the location may have changed unseen. */
  const wasHidden = useRef(false);
  useEffect(() => {
    heard.current = false;
    if (dir === null) return;
    if (hidden) {
      wasHidden.current = true;
      return;
    }
    let stopped = false;
    const reread = () => {
      if (!stopped && !document.hidden) setAgain((n) => n + 1);
    };
    if (wasHidden.current) {
      wasHidden.current = false;
      reread();
    }
    // What the watch cannot hear is asked again: on the window's return
    // and on a slow beat.
    let slow: number | null = null;
    const unheard = () => {
      heard.current = false;
      if (slow !== null) return;
      slow = window.setInterval(reread, UNHEARD_MS);
      window.addEventListener("focus", reread);
    };
    const at = level(dir);
    void (async () => {
      let seq = 0;
      let first = true;
      while (!stopped) {
        try {
          const w = await dirsWait(name, [dir], seq, first ? 0 : WATCH_HOLD_MS);
          if (stopped) return;
          if (w.unavailable) {
            unheard();
            return;
          }
          if (w.unreported.some((d) => level(d) === at)) unheard();
          else if (!first) heard.current = slow === null;
          // The first answer only arms the watch: the listing was just
          // read. Every later one that names the location is a change.
          if (!first && w.dirs.some((d) => level(d) === at)) reread();
          first = false;
          seq = w.seq;
        } catch {
          // Not reached just now: the next question will say.
          await new Promise((r) => setTimeout(r, 5_000));
        }
      }
    })();
    return () => {
      stopped = true;
      if (slow !== null) window.clearInterval(slow);
      window.removeEventListener("focus", reread);
    };
  }, [name, dir, hidden]);

  // A delete, a rename, a save or a drop landing anywhere in khor in
  // this directory — at once, unless the watch is about to say so.
  useEffect(
    () =>
      onTreeChanged((c) => {
        if (dir === null || heard.current) return;
        if (sameMachine(c, { machine: name, here: isHere }) && level(c.dir) === level(dir)) setAgain((n) => n + 1);
      }),
    [name, isHere, dir],
  );

  const [query, setQuery] = useState("");
  const [picking, setPicking] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const [copyRefused, setCopyRefused] = useState<string | null>(null);
  const remove = useRemoveFlow({
    machine: name,
    here: isHere,
    onGone: (path) => {
      if (file === path) setFile(null);
      setAgain((n) => n + 1);
    },
  });
  const writeSetting = (to: string) =>
    setReceiveDir(name, to).then(() => {
      setRefused(null);
      onChanged?.();
    });

  if (file !== null) {
    return (
      <div data-store-file={file} className="relative flex min-h-0 flex-1 flex-col bg-background">
        <FilePage
          key={`${name} ${file}`}
          machine={name}
          path={file}
          here={isHere}
          onBack={() => setFile(null)}
          onGone={() => {
            setFile(null);
            setAgain((n) => n + 1);
          }}
          onMoved={(_from, to) => {
            setFile(to);
            setAgain((n) => n + 1);
          }}
        />
      </div>
    );
  }

  const shown = listed !== null && listed.dir === dir ? listed.rows : null;
  const files = (shown ?? [])
    // Files only, and none of khor's own (a pull's partial, a probe).
    .filter((e) => !e.dir && !e.name.startsWith("."))
    .sort((a, b) => b.at_ms - a.at_ms);
  const full = (n: string) => `${(dir ?? "").replace(/\/$/, "")}/${n}`;
  const rows: SideRow[] = files.map((e) => {
    const path = full(e.name);
    return {
      id: path,
      group: "",
      pinned: false,
      face: { machine: row?.face ?? null },
      mark: fileKind(e.name),
      title: e.name,
      name: e.name,
      sub: <span className="tabular-nums">{sizeWord(e.size)}</span>,
      corners: { bottom: <span data-store-at={e.at_ms}>{ago(e.at_ms)}</span> },
      find: e.name,
      onOpen: () => setFile(path),
      menu: [
        { key: "open", label: gui.web_open, run: () => setFile(path) },
        {
          key: "copy-path",
          label: copyRefused === path ? gui.copy_failed : gui.tab_copy_path,
          failed: copyRefused === path || undefined,
          run: () =>
            navigator.clipboard.writeText(path).then(
              () => setCopyRefused(null),
              (err: unknown) => {
                setCopyRefused(path);
                throw err;
              },
            ),
        },
        { key: "reveal", label: gui.chat_file_open, run: () => onReveal(name, path) },
        ...remove.actions(path, false),
      ],
      rowData: { "data-store-row": e.name },
    };
  });

  const setting = row?.receive_dir ?? null;
  // Not seen yet: the setting differs from the one the machine last
  // resolved in full (both `null` for the default).
  const pending = row !== null && (row.receive_for ?? null) !== setting;
  const note = pending
    ? gui.store_place_pending(row?.shown ?? name)
    : (row?.receive_why ?? (setting === null ? gui.store_place_default : null));
  const machines = devices;

  return (
    <div data-store data-store-machine={name} className="flex min-h-0 flex-1 flex-col">
      <SideFind
        label={gui.store_search}
        query={query}
        onQuery={setQuery}
        plus={
          machines.length > 1 ? (
            <PopupButton
              label={gui.devices_tab}
              value={name}
              mark={{ "data-store-machine-pick": "" }}
              choices={[...machines.filter((d) => d.me), ...machines.filter((d) => !d.me)].map((d) => ({
                value: d.name,
                label: d.shown,
                mark: { "data-store-machine-choice": d.name },
              }))}
              onPick={(m) => {
                setMachine(m);
                setQuery("");
                setRefused(null);
              }}
            />
          ) : undefined
        }
      />
      <div data-nav-clear className="min-h-0 flex-1 overflow-y-auto">
        <div data-store-place data-store-pending={pending || undefined} className="flex flex-col gap-xxs border-b px-(--page-inset) py-in">
          <div className="flex min-w-0 items-center gap-row">
            <div className="flex min-w-0 flex-1 flex-col">
              <span className="text-callout text-muted-foreground">{gui.store_place}</span>
              {dir !== null ? (
                <PathCut text={dir} className="text-body" data-store-dir={dir} />
              ) : (
                <span data-store-unknown className="text-body">
                  {gui.store_place_unknown(row?.shown ?? name)}
                </span>
              )}
            </div>
            <Button size="sm" data-store-change onClick={() => setPicking(true)}>
              {gui.store_place_change}
            </Button>
            {setting !== null && (
              <Button
                size="sm"
                data-store-reset
                onClick={() => {
                  writeSetting("").catch((e: unknown) => setRefused(String(e instanceof Error ? e.message : e)));
                }}
              >
                {gui.store_place_reset}
              </Button>
            )}
          </div>
          {note !== null && (
            <p data-store-note className="text-callout text-muted-foreground">
              {note}
            </p>
          )}
          {refused !== null && (
            <p data-store-refused className="text-callout text-destructive">
              {refused}
            </p>
          )}
        </div>
        {dir === null ? null : failed !== null && shown === null ? (
          <SideList rows={[]} label={() => ""} empty={failed} mark={{ "data-list": "store", "data-store-failed": "" }} />
        ) : (
          <SideList
            rows={rows}
            query={query}
            loading={shown === null}
            label={() => ""}
            empty={gui.store_empty}
            none={gui.no_matches}
            mark={{ "data-list": "store" }}
          />
        )}
      </div>
      <DirPicker open={picking} onOpenChange={setPicking} machine={name} fixed onPick={(p) => writeSetting(p.path)} />
      <RemoveDialog {...remove.dialog} />
    </div>
  );
}

/**
 * **存储 as a page of 更多** (`data-more-page="store"`): the page, under
 * the narrow face's head — ‹ and the name in the window's first row, as
 * every 更多 page has (`MoreFrame`'s) — and under R0 on the wide face,
 * which the shell draws (the name, ×). Not `MoreFrame` itself: that is a
 * column of settings groups at a reading measure, and this is a list.
 * While a file is open its own deck carries ‹, so the head stands aside.
 */
export function StoreScreen({
  devices,
  onClose,
  onChanged,
  onReveal,
}: {
  devices: DeviceRow[];
  onClose: () => void;
  onChanged?: () => void;
  onReveal: (machine: string, path: string) => void;
}) {
  const [reading, setReading] = useState(false);
  return (
    <section data-more-page="store" aria-label={gui.settings_store} className="flex h-full min-h-0 flex-col bg-background">
      {!reading && (
        <NarrowHead>
          {(seated) =>
            seated ? (
              <header data-more-head className="flex min-w-0 flex-1 items-center gap-in self-stretch">
                <GlassCircle aria-label={gui.back} data-more-back onClick={onClose}>
                  <IconBack />
                </GlassCircle>
                <h1 className="min-w-0 flex-1 truncate pr-glass-btn text-center text-title-sm font-title">{gui.settings_store}</h1>
              </header>
            ) : null
          }
        </NarrowHead>
      )}
      <StorePage devices={devices} onSpot={(s) => setReading(s.file !== null)} onChanged={onChanged} onReveal={onReveal} />
    </section>
  );
}
