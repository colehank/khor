// 我的 khor (UX-16, the owner 2026-10-05): this machine, the network it
// is in, and who it is to that network — the first item of 更多. It
// replaces 这台机器 and takes in what 身份 meant on the other pages.
//
// Top to bottom:
//
//   the face and the name, **editable here** (the owner's reading of
//   UX-16 overturns 2026-09-30's 「『我』的里面不要有更改头像的设置」 for
//   this page): a press opens the one editor every machine has
//   (`MachineEditDialog`) — still one window for both facts, now with a
//   second door that is this machine's own;
//   网络: the topology map (`MandalaMap`, the machines overview's picture;
//   on the narrow face its one column, `TopologyColumn`);
//   身份: the machine id, the khor version, the faces this machine has
//   open; then the acts on the identity — 恢复词 (where the root is held),
//   离开网络, 重置身份 — each confirming in a window as before.
//
// 离开网络 came from 网络 and 重置身份 / 恢复词 from 安全; the flows are
// moved, not copied — one door each.
import { useEffect, useState } from "react";

import { fetchAbout, fetchWaiting, inApp, leaveNetwork, resetIdentity, type About, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { ActionRow } from "@/components/settings/ActionRow";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cli, gui, msg } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { cn } from "@/lib/utils";
import { CopyId } from "@/views/DetailPane";
import { MachineEditDialog } from "@/views/MachineEditDialog";
import { MandalaMap, TopologyColumn, waitingOn } from "@/views/MandalaMap";
import { MoreFact, MoreFrame, MoreSection } from "@/views/more/MorePage";
import { RootWordsDialog } from "@/views/PairDialogs";

export function MyKhorPage({
  devices,
  rows,
  onClose,
  onOpenMachine,
  onChanged,
  onReset,
}: {
  /** The shell's own polled table — **not fetched again here** (two
      readers of one answer is how two surfaces disagree). */
  devices: DeviceRow[];
  /** The session rows, for the map's 待批 on each seat. */
  rows: { home: string; word: string }[];
  onClose: () => void;
  /** A node on the map was pressed: that machine's page. */
  onOpenMachine: (row: DeviceRow) => void;
  /** The device table changed (a rename, a face, leaving the network). */
  onChanged: () => void;
  /** This machine's identity was reset. */
  onReset: () => void;
}) {
  const narrow = useNarrow();
  const [about, setAbout] = useState<About | null>(null);
  useEffect(() => {
    void fetchAbout().then(setAbout, () => {});
  }, []);
  /** This machine holds the network's root — the 恢复词 row is its alone,
      and only in the app (the web face is never given them). */
  const [holdsRoot, setHoldsRoot] = useState(false);
  useEffect(() => {
    if (!inApp) return;
    let live = true;
    fetchWaiting().then(
      (w) => live && setHoldsRoot(w.holds),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);
  const [editing, setEditing] = useState(false);
  const [showingWords, setShowingWords] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [typedName, setTypedName] = useState("");
  const [refused, setRefused] = useState<string | null>(null);
  // What khor said the last time 离开网络 was pressed, in the node's own
  // sentences (`msg::left`).
  const [said, setSaid] = useState<string[] | null>(null);

  const me = devices.find((d) => d.me) ?? null;
  const alone = devices.every((d) => d.me);
  const hereKey = inApp ? "app" : "web";
  const ask = (what: "leave" | "reset") => {
    setRefused(null);
    setSaid(null);
    setTypedName("");
    if (what === "leave") setLeaving(true);
    else setResetting(true);
  };

  return (
    <MoreFrame page="me" onClose={onClose}>
      {/* **The face and the name, and a press edits them** — the Apple
          Account head of System Settings: centred, the face large. */}
      {me && (
        <button
          type="button"
          data-me-head
          aria-label={`${me.shown} ${gui.edit_ask}`}
          onClick={() => setEditing(true)}
          className="flex min-w-0 flex-col items-center gap-in self-center rounded-lg px-pane py-in"
        >
          <MachineAvatar face={me.face} className="size-avatar-lg flex-none" />
          <span data-me-name className="max-w-full truncate text-title-md font-title">
            {me.shown}
          </span>
          <span className="text-callout text-muted-foreground">{gui.edit_ask}</span>
        </button>
      )}
      <MachineEditDialog row={editing ? me : null} onClose={() => setEditing(false)} onChanged={onChanged} />

      {said?.map((line) => (
        <p key={line} data-net-left className="text-callout text-muted-foreground">
          {line}
        </p>
      ))}

      {/* 网络: the topology map. A picture where there is room for a
          square, the same graph as one column where there is not. */}
      <MoreSection name="map" label={gui.network} mark={{ "data-me-map": "", "aria-label": gui.mesh_map }}>
        {narrow ? (
          <TopologyColumn rows={devices} onOpen={onOpenMachine} />
        ) : (
          <div className="aspect-square w-full">
            <MandalaMap rows={devices} onOpen={onOpenMachine} waiting={waitingOn(devices, rows)} subtitle={false} />
          </div>
        )}
      </MoreSection>

      <MoreSection name="identity" label={gui.identity_section}>
        {me && (
          <MoreFact name="id" label={gui.identity_id}>
            {/* Twelve characters to recognise; pressing copies all 64. */}
            <CopyId id={me.id} face="id" />
          </MoreFact>
        )}
        <MoreFact name="version" label={gui.identity_version}>
          <span className="tabular-nums">{about?.version ?? ""}</span>
        </MoreFact>
        {/* The faces this machine has open, and which one this is (a
            phone has neither, and no rows). */}
        {about?.faces.map((f) => (
          <div key={f.key} data-iface={f.key} data-on={f.up} data-more-row className="flex min-w-0 items-center gap-row text-body">
            <span className="flex min-w-0 flex-1 items-baseline gap-in">
              <span className="min-w-0 truncate">{f.name}</span>
              {f.key === hereKey && f.up && (
                <span data-iface-here className="flex-none text-muted-foreground">
                  {cli.face_current}
                </span>
              )}
            </span>
            <span data-iface-said data-setting-trail className={cn("flex-none text-right", f.up ? "" : "text-muted-foreground")}>
              {f.said}
            </span>
          </div>
        ))}
      </MoreSection>

      {/* The acts on the identity, each confirmed in a window. No group
          name: each row's title says what it does. */}
      {me && (
        <MoreSection name="identity-acts">
          {inApp && holdsRoot && (
            <ActionRow
              name="root-words"
              label={gui.root_words_title}
              actionLabel={gui.root_words_show_ask}
              onPress={() => setShowingWords(true)}
              mark={{ "data-root-words-open": "" }}
            />
          )}
          {/* Not on a machine in no network: `Node::leave` refuses there. */}
          {!alone && (
            <ActionRow
              name="leave"
              label={gui.identity_leave_do}
              hint={gui.net_leave_note}
              actionLabel={gui.net_leave_ask}
              danger
              onPress={() => ask("leave")}
              mark={{ "data-net-leave": "" }}
            />
          )}
          <ActionRow
            name="reset"
            label={gui.reset_section}
            hint={gui.reset_warn}
            actionLabel={gui.reset_ask}
            danger
            onPress={() => ask("reset")}
            mark={{ "data-reset-open": "" }}
          />
        </MoreSection>
      )}
      <RootWordsDialog open={showingWords} onClose={() => setShowingWords(false)} />

      <Dialog open={leaving} onOpenChange={setLeaving}>
        <DialogContent data-net-leave-dialog>
          <DialogHeader>
            <DialogTitle>{gui.identity_leave_do}</DialogTitle>
          </DialogHeader>
          {/* Not red like the reset's: leaving is undone by a ticket. */}
          <p data-net-leave-warn className="text-callout text-muted-foreground">
            {gui.identity_leave_warn}
          </p>
          <Input
            data-net-leave-name
            autoComplete="off"
            aria-label={gui.identity_leave_type_name}
            placeholder={gui.identity_leave_type_name}
            value={typedName}
            onChange={(e) => setTypedName(e.target.value)}
            className="min-h-touch w-full"
          />
          {refused && (
            <p data-net-leave-refused className="text-callout text-destructive">
              {refused}
            </p>
          )}
          <DialogFooter>
            <Button onClick={() => setLeaving(false)}>{gui.delete_keep}</Button>
            <Button
              variant="destructive"
              data-net-leave-do
              // Always pressable (`components/Why`); the node checks the
              // name too (`Node::leave`).
              onClick={() => {
                if (typedName.trim() !== (me?.shown ?? "")) {
                  setRefused(gui.identity_leave_type_name);
                  document.querySelector<HTMLElement>("[data-net-leave-name]")?.focus();
                  return;
                }
                setRefused(null);
                leaveNetwork(typedName).then(
                  (left) => {
                    setTypedName("");
                    setLeaving(false);
                    setSaid([
                      msg.left,
                      ...(left.told.length ? [msg.left_told(left.told.join(cli.name_separator))] : []),
                      ...(left.untold.length ? [msg.left_untold(left.untold.join(cli.name_separator), me?.shown ?? "")] : []),
                    ]);
                    onChanged();
                  },
                  (e: unknown) => setRefused(String(e instanceof Error ? e.message : e)),
                );
              }}
            >
              {gui.identity_leave_do}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={resetting} onOpenChange={setResetting}>
        <DialogContent data-reset-dialog>
          <DialogHeader>
            <DialogTitle>{gui.reset_title(me?.shown ?? "")}</DialogTitle>
          </DialogHeader>
          {/* Red: the one act here that cannot be undone. */}
          <p data-reset-warn className="text-callout text-destructive">
            {gui.reset_warn}
          </p>
          <Input
            data-reset-name
            autoComplete="off"
            aria-label={gui.reset_type_name}
            placeholder={gui.reset_type_name}
            value={typedName}
            onChange={(e) => setTypedName(e.target.value)}
            className="min-h-touch w-full"
          />
          {refused && (
            <p data-reset-refused className="text-callout text-destructive">
              {refused}
            </p>
          )}
          <DialogFooter>
            <Button onClick={() => setResetting(false)}>{gui.delete_keep}</Button>
            <Button
              variant="destructive"
              data-reset-do
              onClick={() => {
                if (typedName.trim() !== (me?.shown ?? "")) {
                  setRefused(gui.reset_type_name);
                  document.querySelector<HTMLElement>("[data-reset-name]")?.focus();
                  return;
                }
                setRefused(null);
                resetIdentity(typedName).then(
                  () => {
                    setResetting(false);
                    onReset();
                  },
                  (e: unknown) => setRefused(String(e instanceof Error ? e.message : e)),
                );
              }}
            >
              {gui.reset_do}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </MoreFrame>
  );
}
