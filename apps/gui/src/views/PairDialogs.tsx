// The devices pane's "+": the two halves of joining a mesh. Both are
// real here — the app embeds `serve`, so it holds this key's endpoint and
// can issue a ticket carrying live addresses, and joining hands off to
// that same resident serve. Neither half needs a terminal.
//
// Pairing has no direction (docs/NET.md): whichever machine issues the
// ticket, when the join returns *both* device tables hold both machines.
// So these two are one act seen from its two ends, not a client and a
// server.
import { useEffect, useState } from "react";

import { invite, pair } from "@/api";
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

/** Issues a one-time ticket and shows it, ready to be carried across. */
export function InviteDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [ticket, setTicket] = useState("");
  const [error, setError] = useState<string | null>(null);

  // The ticket is minted on opening, not on mounting: it burns on use,
  // and one issued per glance at the menu leaves a pile of live tokens.
  useEffect(() => {
    if (!open) return;
    setTicket("");
    setError(null);
    let live = true;
    invite()
      .then((t) => live && setTicket(t))
      .catch((e) => live && setError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-invite-dialog>
        <DialogHeader>
          <DialogTitle>{gui.make_a_ticket}</DialogTitle>
        </DialogHeader>
        {error ? (
          <div data-dialog-error className="text-sm text-destructive">
            {error}
          </div>
        ) : (
          <Input
            data-ticket
            aria-label={gui.ticket}
            readOnly
            value={ticket}
            onFocus={(e) => e.currentTarget.select()}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

/** Takes someone's ticket and joins. */
export function JoinDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [ticket, setTicket] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [joining, setJoining] = useState(false);

  useEffect(() => {
    if (!open) return;
    setTicket("");
    setError(null);
    setJoining(false);
  }, [open]);

  const join = () => {
    if (!ticket.trim() || joining) return;
    setJoining(true);
    setError(null);
    pair(ticket.trim())
      // Nothing to announce: the new machine's row arrives on the next
      // poll wearing its own face, which is the report.
      .then(() => onOpenChange(false))
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setJoining(false));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-join-dialog>
        <DialogHeader>
          <DialogTitle>{gui.join_with_a_ticket}</DialogTitle>
        </DialogHeader>
        <Input
          data-ticket-input
          aria-label={gui.paste_a_ticket}
          placeholder={gui.paste_a_ticket}
          value={ticket}
          onChange={(e) => setTicket(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && join()}
        />
        {error && (
          <div data-dialog-error className="text-sm text-destructive">
            {error}
          </div>
        )}
        <DialogFooter>
          <Button data-join disabled={!ticket.trim() || joining} onClick={join}>
            {gui.join}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
