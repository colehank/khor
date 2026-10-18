// 跨机器目录 (工作区批②): one dialog that answers one question — 哪台机器上
// 的哪个目录 — and hands back the answer. The workspace 「+」 opens it.
//
// The same searchable machine field and live path input as NewSessionPage.
// With no caller directory, the target's home is filled in; each target keeps
// its own draft while this picker is open.
//
// **It validates nothing.** Whether that directory is really there is
// the caller's question, asked by the caller's own verb — `ws_add`
// probes it with `Ls` — and it already has its own sentence for the
// answer. What this does instead is show what came back: `onPick` may
// reject, and a rejection keeps the dialog open with the node's own
// sentence under the box (`RowMenu`'s rule, one surface up).
import { useEffect, useState } from "react";

import { fetchDevices, type DeviceRow } from "@/api";
import { PathField, settledPath } from "@/components/PathField";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogBody,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { MachineField } from "@/components/MachineField";
import { FormGrid, FormLabel } from "@/components/ui/form-grid";
import { gui } from "@/gen/catalog";

/** What the picker answers with: the machine as the name every verb
    takes (`""` is this machine, the spelling the whole API uses), and
    the directory. */
export type DirPick = { machine: string; path: string };

export function DirPicker({
  open,
  onOpenChange,
  machine: start = "",
  fixed = false,
  onPick,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Which machine to open on; `""` is this one. */
  machine?: string;
  /** The machine is the question's, not the person's to change (存储's
      receive location: a directory on the machine on screen). */
  fixed?: boolean;
  /** Answered with the pick. **Rejecting is how the caller says it did
      not take**: the dialog stays open and shows what came back, which
      is where the person pressing is looking (`RowMenu`). Resolving
      closes it. */
  onPick: (pick: DirPick) => Promise<void> | void;
}) {
  const [machines, setMachines] = useState<DeviceRow[]>([]);
  const [machine, setMachine] = useState(start);
  const [paths, setPaths] = useState<Record<string, string>>({});
  const [pending, setPending] = useState(false);
  const target = machines.find(d => d.name === machine || d.id === machine || (machine === "" && d.me));
  const channel = target?.name ?? machine;
  const typed = paths[channel] ?? "";
  /** What the caller said when it refused the pick. */
  const [refused, setRefused] = useState<string | null>(null);
  const [taking, setTaking] = useState(false);

  // Reopening must not inherit a stale path or a stale refusal: the
  // dialog is a question being asked now, and last time's answer is not
  // an answer to it.
  useEffect(() => {
    if (!open) return;
    setMachine(start);
    setPaths({});
    setPending(false);
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

  const take = () => {
    const path = settledPath(typed);
    if (taking || pending) return;
    // **Always pressable** (the owner, 2026-10-02; `components/Why`): with
    // no path yet it says so and puts the caret in the box.
    if (path === "") {
      setRefused(gui.need_path);
      requestAnimationFrame(() =>
        document.querySelector<HTMLElement>("input[data-dir-picker-path], [data-dir-picker-path] input")?.focus(),
      );
      return;
    }
    setTaking(true);
    setRefused(null);
    Promise.resolve()
      .then(() => onPick({ machine: channel, path }))
      .then(
        () => onOpenChange(false),
        (e: unknown) => {
          setRefused(String(e instanceof Error ? e.message : e));
          setTaking(false);
        },
      );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form scrollable data-dir-picker>
        <DialogHeader>
          <DialogTitle>{gui.dir_picker}</DialogTitle>
        </DialogHeader>
        <DialogBody>
        <FormGrid>
          {!fixed && (
            <>
              <FormLabel>{gui.new_session_machine}</FormLabel>
              <div data-dir-picker-machines data-machine={channel}>
                <MachineField
                  devices={machines}
                  value={channel}
                  disabled={taking}
                  onPendingChange={setPending}
                  onPick={name => {
                    setMachine(name);
                    setRefused(null);
                  }}
                />
              </div>
            </>
          )}

          {/* **Not a `label`**: the completions under the box are
              buttons, and a button inside a label is a control inside
              a control. */}
          <FormLabel>{gui.new_session_dir}</FormLabel>
          <div className="flex min-w-0 flex-col">
            <PathField
              key={channel}
              data-dir-picker-path
              aria-label={gui.new_session_dir}
              machine={channel}
              disabled={taking}
              value={typed}
              onValueChange={(v) => {
                setPaths(was => ({ ...was, [channel]: v }));
                setRefused(null);
              }}
              onSubmit={take}
              // `ws_add` takes absolute paths only.
              keepHome={false}
            />
          </div>

          {/* What the caller said when it refused. Its sentence, not
              one invented here. */}
          {refused !== null && (
            <div data-dir-picker-refused className="col-span-2 text-callout text-destructive">
              {refused}
            </div>
          )}
        </FormGrid>
        </DialogBody>
        <DialogFooter>
          <Button data-dialog-cancel onClick={() => onOpenChange(false)} className="min-h-touch">
            {gui.delete_keep}
          </Button>
          <Button
            variant="primary"
            data-dir-picker-take
            disabled={taking || pending}
            onMouseDown={e=>e.preventDefault()}
            aria-busy={taking || undefined}
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
