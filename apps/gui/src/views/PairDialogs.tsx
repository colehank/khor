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

import {
  approveMachine,
  fetchWaiting,
  inApp,
  invite,
  pair,
  rootUnshown,
  rootWords,
  rootWordsSeen,
  said,
  type Ticket,
  type Waiting,
} from "@/api";
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

/** Issues a one-time ticket and shows it, ready to be carried across. */
export function InviteDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [ticket, setTicket] = useState<Ticket | null>(null);
  const [error, setError] = useState<string | null>(null);

  // The ticket is minted on opening, not on mounting: it burns on use,
  // and one issued per glance at the menu leaves a pile of live tokens.
  useEffect(() => {
    if (!open) return;
    setTicket(null);
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
          <div data-dialog-error className="text-callout text-destructive">
            {error}
          </div>
        ) : (
          <>
            <Input
              data-ticket
              aria-label={gui.ticket}
              readOnly
              value={ticket?.token ?? ""}
              onFocus={(e) => e.currentTarget.select()}
            />
            {/* **How long it lasts, in the same words the terminal
                uses** — one catalog key rendered twice, so the two faces
                cannot come to say different things about one rule. The
                number comes from the answer, never from here: a `15`
                written into this file would be a second copy of
                something only `link::INVITE_WINDOW_MS` decides, and the
                day it moves this screen would go on promising the old
                one with nothing anywhere disagreeing.
                Drawn only once there is a ticket, because until then
                there is nothing for it to be true of. */}
            {ticket && (
              <div data-ticket-window className="text-callout text-muted-foreground">
                {cli.invite_window(ticket.minutes)}
              </div>
            )}
          </>
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
      <DialogContent sheet data-join-dialog>
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
          <div data-dialog-error className="text-callout text-destructive">
            {error}
          </div>
        )}
        <DialogFooter>
          <Button variant="primary" data-join disabled={!ticket.trim() || joining} onClick={join}>
            {gui.join}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** The word for what a waiting machine waits for — the terminal's own
    (`khor pending`), so the two faces say one thing. */
function waitsFor(kind: string): string {
  if (kind === "satellite") return cli.pending_kind_satellite;
  if (kind === "leave") return cli.pending_kind_leave;
  return cli.pending_kind_device;
}

/**
 * 待批 (docs/CONNECT.md 根不在场), over the machines list: who is waiting
 * for the machine that holds the root. **Drawn only while something
 * waits** — an empty strip would be a row the list does not have.
 *
 * The holder gets one 批准 per row; every other machine gets the one
 * sentence that says where that button is, because pressing it here
 * could only be refused (`Node::approve_here`). Which of the two is the
 * node's answer (`Waiting.holds`), not a guess made here.
 *
 * Asked on its own beat, the way the list asks for its rows: a machine
 * joins through somebody else's ticket, and this is where it shows up.
 */
export function PendingStrip() {
  const [waiting, setWaiting] = useState<Waiting | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    const read = () =>
      fetchWaiting()
        .then((w) => live && setWaiting(w))
        // A store that cannot say leaves the strip as it was; the list
        // under it has its own way of saying the store is unreadable.
        .catch(() => {});
    read();
    const beat = window.setInterval(read, 5000);
    return () => {
      live = false;
      window.clearInterval(beat);
    };
  }, []);

  if (!waiting || waiting.rows.length === 0) return null;

  const approve = (id: string) => {
    setBusy(true);
    setError(null);
    approveMachine(id)
      .then(setWaiting)
      .catch((e) => setError(said(e)))
      .finally(() => setBusy(false));
  };

  return (
    <div data-pending className="px-in pb-in">
      <div className="px-in pt-in pb-xxs text-group text-tertiary">{gui.pending_title}</div>
      {waiting.rows.map((r) => (
        <div key={r.id} data-pending-row={r.id} className="flex min-h-row-1 min-w-0 items-center gap-row px-in">
          <span className="min-w-0 flex-1 truncate text-body">{r.name}</span>
          <span className="text-caption text-tertiary">{waitsFor(r.kind)}</span>
          {waiting.holds && (
            <Button
              size="sm"
              data-approve={r.id}
              className="pointer-coarse:min-h-touch"
              disabled={busy}
              onClick={() => approve(r.id)}
            >
              {gui.approve}
            </Button>
          )}
        </div>
      ))}
      {!waiting.holds && (
        <div data-pending-elsewhere className="px-in text-callout text-muted-foreground">
          {gui.pending_elsewhere}
        </div>
      )}
      {error && (
        <div data-pending-error className="px-in text-callout text-destructive">
          {error}
        </div>
      )}
    </div>
  );
}

/**
 * The root's words, asked for once (docs/CONNECT.md 身份): this machine
 * made the network's root when it first opened, and the 24 words are
 * the only way back if it is lost.
 *
 * **Shown only after a press** — the words are the root, and a window
 * that opened with them on it would put them on a screen nobody chose
 * to look at. **Asked once**: answering or closing marks them seen, and
 * after that the way to them is `khor root words` on this machine,
 * which the sentence says.
 *
 * **The app only.** The words are not served to the web face
 * (`khor_gui_core` `tests/command_tables.rs`), and a browser opening
 * this page is a different screen from the machine's own.
 */
export function RootWordsDialog() {
  const [open, setOpen] = useState(false);
  const [words, setWords] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!inApp) return;
    let live = true;
    rootUnshown()
      .then((waiting) => live && waiting && setOpen(true))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, []);

  const close = () => {
    setOpen(false);
    setWords(null);
    rootWordsSeen().catch(() => {});
  };
  const show = () => {
    setError(null);
    rootWords()
      .then((w) => setWords(w.split(" ")))
      .catch((e) => setError(said(e)));
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent data-root-words-dialog>
        <DialogHeader>
          <DialogTitle>{gui.root_words_title}</DialogTitle>
        </DialogHeader>
        <div className="text-body text-muted-foreground">{gui.root_words_body}</div>
        {words && (
          <ol data-root-words className="grid grid-cols-3 gap-in font-mono text-body tabular-nums">
            {words.map((w, i) => (
              <li key={i} className="flex min-w-0 gap-xxs">
                <span className="text-tertiary">{i + 1}</span>
                <span className="truncate">{w}</span>
              </li>
            ))}
          </ol>
        )}
        {error && (
          <div data-dialog-error className="text-callout text-destructive">
            {error}
          </div>
        )}
        <DialogFooter>
          {words ? (
            <Button variant="primary" size="lg" data-root-words-done onClick={close}>
              {gui.root_words_done}
            </Button>
          ) : (
            <Button variant="primary" size="lg" data-root-words-show onClick={show}>
              {gui.root_words_show}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
