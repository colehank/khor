// 编辑… — everything a person can change about a machine, in one window
// (2026-09-30, the owner: 「更改头像的弹窗 UI 有点丑，能不能右键→编辑，然后
// 这里可以改名称、头像等诸多信息」).
//
// # One door, one window, every machine
//
// It opens from a machine row's menu (`DevicesList`), on **any** machine
// — a name and a face are 全网同一份, and anyone may set anyone's — and it
// is the only place either is edited. 这台机器 used to carry a second
// copy of both (a 改名… row and the whole face editor); it states them now
// and nothing more (「不要冗余」). The list row's double-click stays as the
// quick road to the name alone — the same writer (`renameMachine`), so the
// two cannot come to mean different things.
//
// # What the device table lets a person edit, and what is here
//
// A machine's row (`khor_sync::devices::DeviceInfo`) holds its name
// (`label`, the rename over the name it reports), its face (`style`),
// its pin, the directories it is asked to watch, and for a satellite the
// ssh host it is reached by. Here: **the name and the face** — what the
// machine *is called and looks like*. Not here, on purpose:
//
//   - the pin (and 暂停) are acts on the row, one press each, and stay the
//     row menu's items beside this one;
//   - the watched directories are edited where their readings are, in the
//     disk drawer on the card: a path appears once, with its answer
//     beside it (2026-09-06), and a copy here would be a second list that
//     disagrees with the first the moment one is saved;
//   - a satellite's ssh host is `khor add`'s, set when it joined.
//
// # 完成, and no 取消
//
// The face writes on every pick (the node paints; see `FaceSettings`), so
// there is nothing a 取消 could take back — a 取消 beside picks that have
// already landed would be a promise the window cannot keep. The name is
// typed, and half a name is not a name, so it waits: 完成 (or Enter)
// writes it, Esc leaves it as it was. One primary button, the dialog's
// one (components/Dialog).
import { useEffect, useState } from "react";

import { renameMachine, type DeviceRow } from "@/api";
import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, DialogBody, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { FormLabel } from "@/components/ui/form-grid";
import { gui } from "@/gen/catalog";
import { FaceSettings } from "@/views/FaceSettings";

export function MachineEditDialog({
  row,
  onClose,
  onChanged,
}: {
  /** The machine being edited, re-read off the table by the caller so the
      title follows a rename and the window goes if the machine does.
      `null` is shut. */
  row: DeviceRow | null;
  onClose: () => void;
  /** Something was written — the shell re-reads the table, and every face
      and name on screen moves in the same frame. */
  onChanged: () => void;
}) {
  const [typed, setTyped] = useState("");
  const [going, setGoing] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const id = row?.id ?? null;
  // **The box opens on the rename, not on the name.** A machine wearing
  // the name it reports opens on an empty box with that name greyed in it
  // — what an emptied box gives back (`DeviceRow::label`). Seeded once per
  // opening, on the machine's id: a poll re-reading the table must not
  // reset what is being typed.
  useEffect(() => {
    if (!row) return;
    setTyped(row.label ?? "");
    setRefused(null);
    setGoing(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  const done = () => {
    if (!row || going) return;
    const name = typed.trim();
    // Nothing typed differs from what is stored: nothing to write.
    if (name === (row.label ?? "").trim()) {
      onClose();
      return;
    }
    setGoing(true);
    setRefused(null);
    renameMachine(row.id, name).then(
      () => {
        setGoing(false);
        onChanged();
        onClose();
      },
      // **The node's own sentence, under the box**, and the window stays:
      // closing it would paint the picture a rename that took paints.
      (e: unknown) => {
        setGoing(false);
        setRefused(String(e instanceof Error ? e.message : e) || gui.rename_failed);
      },
    );
  };

  return (
    <Dialog open={row !== null} onOpenChange={(o) => !o && onClose()}>
      <TaskOverlay purpose="edit" scrollable data-machine-edit={row?.id}>
        <DialogHeader>
          <DialogTitle>{row ? gui.machine_edit_title(row.shown) : ""}</DialogTitle>
        </DialogHeader>
        <DialogBody>
        {row && (
          <FaceSettings machine={row.id} onChanged={onChanged}>
            <FormLabel htmlFor="khor-machine-edit-name">{gui.machine_name}</FormLabel>
            {/* Its own form, so Enter in the box is 完成 — and only the
                box: the face's buttons are not inside it, and a button
                with no type inside a form would submit it. */}
            <form
              className="flex min-w-0 flex-col gap-xxs"
              onSubmit={(e) => {
                e.preventDefault();
                done();
              }}
            >
              <Input
                id="khor-machine-edit-name"
                data-machine-edit-name
                autoComplete="off"
                spellCheck={false}
                value={typed}
                placeholder={row.name}
                onChange={(e) => {
                  setTyped(e.target.value);
                  setRefused(null);
                }}
                className="w-full"
              />
              {refused !== null && (
                <p data-machine-edit-refused className="text-callout text-destructive">
                  {refused}
                </p>
              )}
            </form>
          </FaceSettings>
        )}
        </DialogBody>
        <DialogFooter>
          <Button variant="primary" data-machine-edit-done data-footer-trailing className="min-h-touch" disabled={going} aria-busy={going || undefined} onClick={done}>
            {gui.edit_done}
          </Button>
        </DialogFooter>
      </TaskOverlay>
    </Dialog>
  );
}
