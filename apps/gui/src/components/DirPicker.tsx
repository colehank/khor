// 跨机器目录选择器 (工作区批②): one dialog that answers one question —
// 哪台机器上的哪个目录 — and hands back the answer.
//
// **Two callers, one dialog, on purpose.** Declaring a workspace and
// starting a session both begin by pointing at a directory on some
// machine, and that is the same act however differently it ends. Two
// pickers would be two screens to learn, and the second one would be
// the one that forgets satellites.
//
// **It validates nothing.** Whether that directory is really there is
// the caller's question, asked by the caller's own verb — `ws_add`
// probes it with `Ls`, `openSession` finds out when it starts something
// — and each already has its own sentence for the answer. A check here
// would be a third opinion, one round trip earlier, in words neither
// caller wrote. What this does instead is show what came back: `onPick`
// may reject, and a rejection keeps the dialog open with the node's own
// sentence under the box (`RowMenu`'s rule, one surface up).
//
// **The tree is the shortcut and the box is the floor.** Every
// directory has to be reachable — a path pasted from somewhere else, a
// disk not under anybody's home — so the box takes anything and the
// confirm sends what the box holds. Clicking through the tree fills the
// box; typing in it does not move the tree until Enter. Two values, not
// one, because a listing on every keystroke is a machine asked a
// question per character.
import { useEffect, useState } from "react";

import {
  fetchDevices,
  fetchLs,
  fetchPlaces,
  type DeviceRow,
  type DirListing,
  type PlaceRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { PathCrumbs } from "@/components/PathCrumbs";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { connectionWord } from "@/words";

/** What the picker answers with: the machine as the name every verb
    takes (`""` is this machine, the spelling the whole API uses), and
    the directory.

    ⚠️ **It carried the machine's id too, and that field is gone**
    (2026-09-15). It was there so a caller keying the workspace table by
    id would not look it up again — and across four batches no caller
    ever read it: `ws_add` names a machine because adding one has to
    reach it, and the wizard writes a path into a box. A field nobody
    reads is a field that can be wrong without anybody noticing. */
export type DirPick = { machine: string; path: string };

export function DirPicker({
  open,
  onOpenChange,
  machine: start = "",
  onPick,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Which machine to open on; `""` is this one. */
  machine?: string;
  /** Answered with the pick. **Rejecting is how the caller says it did
      not take**: the dialog stays open and shows what came back, which
      is where the person pressing is looking (`RowMenu`). Resolving
      closes it. */
  onPick: (pick: DirPick) => Promise<void> | void;
}) {
  const [machines, setMachines] = useState<DeviceRow[]>([]);
  const [machine, setMachine] = useState(start);
  /** Where the listing is. `""` asks for that machine's home, and the
      answer carries the real path — whose home that is, is a fact about
      the far machine and not something this screen can spell. */
  const [at, setAt] = useState("");
  const [listing, setListing] = useState<DirListing | null>(null);
  /** Why that machine could not be listed, in its own sentence. Kept
      apart from an empty listing: 「读不了」 and 「这里没有目录」 are
      different next moves, and only one of them means keep walking. */
  const [cannot, setCannot] = useState<string | null>(null);
  const [places, setPlaces] = useState<PlaceRow[]>([]);
  /** The box. Filled by every step through the tree; typing in it moves
      nothing until Enter (file head). */
  const [typed, setTyped] = useState("");
  /** What the caller said when it refused the pick. */
  const [refused, setRefused] = useState<string | null>(null);
  const [taking, setTaking] = useState(false);

  // Reopening must not inherit a stale walk or a stale refusal — the
  // wizard's rule for the same reason: the dialog is a question being
  // asked now, and last time's answer is not an answer to it.
  useEffect(() => {
    if (!open) return;
    setMachine(start);
    setAt("");
    setListing(null);
    setCannot(null);
    setTyped("");
    setRefused(null);
    setTaking(false);
    let live = true;
    fetchDevices()
      .then((rows) => live && setMachines(rows))
      // The row is drawn from this; without it there is one machine to
      // pick from and it is the one already chosen, which is the honest
      // state of a table that did not answer.
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [open, start]);

  // **The listing follows both.** A path typed for this disk means
  // nothing on that one, so changing machine walks back to its home —
  // the same reset the wizard does to its 目录 box.
  useEffect(() => {
    if (!open) return;
    let live = true;
    setCannot(null);
    fetchLs(machine, at).then(
      (l) => {
        if (!live) return;
        setListing(l);
        // The machine's own spelling of where this landed, which for
        // `""` is the only way to learn what its home is called.
        setTyped(l.path);
      },
      (e) => {
        if (!live) return;
        setListing(null);
        setCannot(String(e instanceof Error ? e.message : e));
      },
    );
    return () => {
      live = false;
    };
  }, [open, machine, at]);

  useEffect(() => {
    if (!open) return;
    let live = true;
    setPlaces([]);
    // Silent on failure, the wizard's judgment for the same list: the
    // box below is still there and still takes anything, so a shortcut
    // that could not be built is not worth a red line.
    fetchPlaces(machine)
      .then((rows) => live && setPlaces(rows))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [open, machine]);

  const go = (path: string) => {
    setAt(path);
    setTyped(path);
    setRefused(null);
  };

  const take = () => {
    const path = typed.trim();
    if (path === "" || taking) return;
    setTaking(true);
    setRefused(null);
    Promise.resolve()
      .then(() => onPick({ machine, path }))
      .then(
        () => onOpenChange(false),
        (e: unknown) => {
          setRefused(String(e instanceof Error ? e.message : e));
          setTaking(false);
        },
      );
  };

  // Only the directories: this dialog answers 「which directory」, and a
  // file is not one. The count the truncation note prints is the node's
  // own (`FilesPane`), so it counts what came rather than what is drawn
  // — a listing cut at 4096 entries was cut at 4096 entries whether or
  // not they were directories.
  const dirs = listing?.entries.filter((e) => e.dir) ?? [];
  const here = listing?.path ?? "";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {/* `DialogContent` is the narrow face's whole answer already —
          `w-full max-w-[calc(100%-2rem)]` with `sm:max-w-lg` above it —
          so what is left here is keeping the body from growing past one
          screen: the list is the only part that can, and it is the part
          that scrolls. */}
      <DialogContent data-dir-picker className="max-h-[90vh]">
        <DialogHeader>
          <DialogTitle>{gui.dir_picker}</DialogTitle>
        </DialogHeader>
        <div className="flex min-h-0 min-w-0 flex-col gap-3">
          {/* 在哪台, first — the directory is a fact about the machine
              (the wizard's order, and its heading).

              **Drawn like the wizard's row and not shared with it**,
              which is a judgment and not laziness. Three of the four
              things that row does differ here: it greys satellites
              (nothing over there can host an agent), this lists them
              (`Ls` routes through the sponsor, which `FilesPane`
              already relies on); its anchor is `data-new-session-machine`
              and smoke presses that by name; and picking there resets a
              text box rather than a walk. A component whose every
              interesting part is a parameter is not one component, it is
              two with a lookup table in between. What *is* shared is the
              word — one heading, one catalog key. */}
          {machines.length > 1 && (
            <div data-dir-picker-machines className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.new_session_machine}</span>
              <div className="flex flex-wrap gap-1">
                {machines.map((d) => {
                  const on = machine === "" ? d.me : machine === d.name;
                  return (
                    <Button
                      key={d.id}
                      size="sm"
                      variant={on ? "secondary" : "ghost"}
                      data-dir-picker-machine={d.name}
                      data-on={on}
                      className="min-h-touch"
                      onClick={() => {
                        setMachine(d.name);
                        go("");
                      }}
                    >
                      <MachineAvatar face={d.face} className="size-kind-mark flex-none" />
                      {d.shown}
                      {connectionWord(d) && (
                        <span className="text-muted-foreground">{connectionWord(d)}</span>
                      )}
                    </Button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Where the walk is. The step you are standing on focuses the
              box instead of going nowhere — `PathCrumbs` reserves that
              press for whoever draws the bar, and here the box is what
              it is for. */}
          <PathCrumbs
            at={here}
            onGo={go}
            onEdit={() => document.querySelector<HTMLInputElement>("[data-dir-picker-path]")?.focus()}
          />

          {/* 常去的地方 as a jump and not a pick: pressing one lands the
              walk there so it can be looked at, the same as pressing a
              row. **Not a confirm** — a shortcut that finished the
              dialog would make the one list a person did not read the
              one that decides. */}
          {places.length > 0 && (
            <div data-dir-picker-places className="flex min-w-0 flex-col gap-1">
              <span className="text-xs text-muted-foreground">{gui.new_session_places}</span>
              <div className="flex max-h-24 min-w-0 flex-col gap-1 overflow-y-auto">
                {places.map((p) => (
                  <button
                    key={p.path}
                    type="button"
                    data-dir-picker-place={p.path}
                    onClick={() => go(p.path)}
                    className="flex min-h-touch w-full min-w-0 items-center rounded px-1 text-left text-xs hover:bg-accent"
                  >
                    <span className="truncate">{p.shown}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          <div className="min-h-0 flex-1 overflow-y-auto rounded border">
            {cannot !== null ? (
              // That machine's own sentence, at the size of something
              // meant to be read — `FilesPane`'s rule: a listing that
              // did not happen is the whole of what this half has to
              // say, never a footnote under an empty box.
              <div data-dir-picker-cannot className="p-3 text-fact">
                {cannot}
              </div>
            ) : listing === null ? null : dirs.length === 0 ? (
              <div data-dir-picker-none className="p-3 text-sm text-muted-foreground">
                {gui.dir_picker_none}
              </div>
            ) : (
              dirs.map((e) => {
                const full = `${here === "/" ? "" : here}/${e.name}`;
                return (
                  <button
                    key={e.name}
                    type="button"
                    data-dir-picker-row={e.name}
                    onClick={() => go(full)}
                    // Dot-named directories are sunk by the node, never
                    // hidden (`khor_node::files`); faded here for the
                    // same reason `FilesPane` fades them — they are
                    // rarely what somebody came for and must stay
                    // reachable when they are.
                    className={cn(
                      "flex min-h-touch w-full items-center gap-2 border-b px-3 text-left text-sm last:border-b-0 hover:bg-muted/50",
                      e.name.startsWith(".") && "text-muted-foreground",
                    )}
                  >
                    <span className="truncate">{e.name}</span>
                  </button>
                );
              })
            )}
            {listing?.truncated && (
              // The cap is however many rows actually came — the number
              // belongs to the node, and a copy here would drift.
              <div className="p-3 text-xs text-muted-foreground">
                {cli.dir_truncated(listing.entries.length)}
              </div>
            )}
          </div>

          <label className="flex min-w-0 flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.dir_picker_typed}</span>
            <Input
              data-dir-picker-path
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              // Enter moves the walk rather than confirming: a path
              // half-typed is a path, and a dialog that finished on the
              // first Enter would take it. Confirming is the button.
              onKeyDown={(e) => e.key === "Enter" && go(typed.trim())}
            />
          </label>

          {/* What the caller said when it refused. Its sentence, not
              one invented here — the two callers refuse for different
              reasons and both already have words for them. */}
          {refused !== null && (
            <div data-dir-picker-refused className="text-sm text-destructive">
              {refused}
            </div>
          )}
        </div>
        <DialogFooter>
          <Button
            data-dir-picker-take
            disabled={typed.trim() === "" || taking}
            onClick={take}
            className="min-h-touch"
          >
            {gui.dir_picker_take}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
