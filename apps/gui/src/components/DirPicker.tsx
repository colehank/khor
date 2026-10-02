// 跨机器目录 (工作区批②): one dialog that answers one question — 哪台机器上
// 的哪个目录 — and hands back the answer. The workspace 「+」 opens it.
//
// **One box, and nothing offered before it is typed in** (2026-09-27:
// 「"+" 弹出的窗口，涉及到目录的，都不要推荐任何路径，简单一个输入框，用户
// 可以边写边自动推荐」). It was a tree with a breadcrumb, a list of
// 常去的地方 and a box under both; the box is all that is left, and it
// completes from the chosen machine's own listing as it is typed in
// (`PathField`).
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
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PopupButton } from "@/components/ui/popup-button";
import { FormGrid, FormLabel } from "@/components/ui/form-grid";
import { gui } from "@/gen/catalog";
import { connectionWord } from "@/words";

/** What the picker answers with: the machine as the name every verb
    takes (`""` is this machine, the spelling the whole API uses), and
    the directory. */
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
  const [typed, setTyped] = useState("");
  /** What the caller said when it refused the pick. */
  const [refused, setRefused] = useState<string | null>(null);
  const [taking, setTaking] = useState(false);

  // Reopening must not inherit a stale path or a stale refusal: the
  // dialog is a question being asked now, and last time's answer is not
  // an answer to it.
  useEffect(() => {
    if (!open) return;
    setMachine(start);
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

  const take = () => {
    const path = settledPath(typed);
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

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form data-dir-picker>
        <DialogHeader>
          <DialogTitle>{gui.dir_picker}</DialogTitle>
        </DialogHeader>
        <FormGrid>
          {/* 在哪台, first — the directory is a fact about the machine
              (the wizard's order, and its heading). Satellites are
              listed: `Ls` routes through the sponsor, so completion
              reaches them too. **A popup, the wizard's control**: the
              machines are a list that grows (components/PopupButton). */}
          {machines.length > 1 && (
            <>
              <FormLabel>{gui.new_session_machine}</FormLabel>
              <div data-dir-picker-machines className="flex min-w-0">
                <PopupButton
                  look="field"
                  label={gui.new_session_machine}
                  value={machine === "" ? (machines.find((d) => d.me)?.name ?? "") : machine}
                  mark={{ "data-dir-picker-machine-select": "" }}
                  choices={machines.map((d) => ({
                    value: d.name,
                    label: connectionWord(d) ? `${d.shown} ${connectionWord(d)}` : d.shown,
                    mark: { "data-dir-picker-machine": d.name },
                  }))}
                  onPick={(name) => {
                    // A path typed for this disk means nothing on
                    // that one.
                    setMachine(name);
                    setTyped("");
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
              data-dir-picker-path
              aria-label={gui.new_session_dir}
              machine={machine}
              value={typed}
              onValueChange={(v) => {
                setTyped(v);
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
        <DialogFooter>
          <Button data-dialog-cancel onClick={() => onOpenChange(false)} className="min-h-touch">
            {gui.delete_keep}
          </Button>
          {/* Drawn once there is a path to take (F1.9: not greyed). */}
          {settledPath(typed) !== "" && (
            <Button
              variant="primary"
              data-dir-picker-take
              disabled={taking}
              aria-busy={taking || undefined}
              onClick={take}
              className="min-h-touch"
            >
              {gui.dir_picker_take}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
