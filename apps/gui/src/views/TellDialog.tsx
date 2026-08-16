// What the sessions pane's "+" opens today: leaving a line in a
// machine's window. It is the whole of `khor tell` and nothing more —
// the menu holds only what works, so this is the only item in it until
// the next kind lands.
import { useEffect, useState } from "react";

import { tell, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export function TellDialog({
  open,
  onOpenChange,
  devices,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  devices: DeviceRow[];
}) {
  const [machine, setMachine] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

  // A dialog that reopens holding the last attempt's leftovers is a
  // dialog that sends the wrong line to the wrong machine one day.
  useEffect(() => {
    if (!open) return;
    setMachine(null);
    setText("");
    setError(null);
    setSending(false);
  }, [open]);

  const send = () => {
    if (!machine || !text.trim() || sending) return;
    setSending(true);
    setError(null);
    tell(machine, text)
      .then(() => onOpenChange(false))
      // The message is the node's, in the catalog's words — this layer
      // has none of its own to add.
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setSending(false));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-tell-dialog>
        <DialogHeader>
          <DialogTitle>{gui.tell_machine}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-wrap gap-2" role="group" aria-label={gui.pick_a_machine}>
          {devices.map((d) => (
            <button
              key={d.id}
              type="button"
              data-machine={d.name}
              aria-pressed={machine === d.name}
              onClick={() => setMachine(d.name)}
              className={cn(
                "flex h-ctl-lg items-center gap-2 rounded-md border px-3 hover:bg-secondary",
                machine === d.name && "border-primary bg-accent",
              )}
            >
              <MachineAvatar face={d.face} className="size-icon-rail" />
              <span className="truncate">{d.name}</span>
            </button>
          ))}
        </div>
        <Input
          data-tell-text
          aria-label={gui.message_text}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && send()}
        />
        {error && (
          <div data-dialog-error className="text-sm text-destructive">
            {error}
          </div>
        )}
        <DialogFooter>
          <Button data-tell-send disabled={!machine || !text.trim() || sending} onClick={send}>
            {gui.send_it}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
