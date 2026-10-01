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
  invite,
  pair,
  rootWords,
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
import { YIELD_MS } from "@/lib/tempo";

/**
 * Issues a one-time ticket and shows it, ready to be carried across —
 * **one invite, two spellings** (2026-09-30, the owner: 「邀请设备显示两个
 * 内容：现在的 token，和一个二维码，后者为手机连接奠定基础」): the QR code
 * above, the token under it. The code carries the token itself, not a URL
 * around it, so a phone that scans it one day holds exactly what 加入网络
 * takes today; it is encoded by the node with the ticket
 * (`khor_gui_core::qr`) and only drawn here.
 */
export function InviteDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [ticket, setTicket] = useState<Ticket | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** What the last copy did, said on the button for a beat (`CopyId`'s
      rule: a refused clipboard write must not look like one that took). */
  const [copied, setCopied] = useState<"copied" | "failed" | null>(null);

  // The ticket is minted on opening, not on mounting: it burns on use,
  // and one issued per glance at the menu leaves a pile of live tokens.
  useEffect(() => {
    if (!open) return;
    setTicket(null);
    setError(null);
    setCopied(null);
    let live = true;
    invite()
      .then((t) => live && setTicket(t))
      .catch((e) => live && setError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
  }, [open]);
  useEffect(() => {
    if (!copied) return;
    const t = window.setTimeout(() => setCopied(null), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [copied]);
  const copy = () => {
    if (!ticket) return;
    navigator.clipboard.writeText(ticket.token).then(
      () => setCopied("copied"),
      () => setCopied("failed"),
    );
  };

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
            {/* **The code, drawn as the node answered it.** One path in
                module units; the `viewBox` is the whole picture with its
                four-module quiet zone already in the path's coordinates,
                so nothing here can crop it. Ink is `currentColor` (the
                `ink` token) on no ground of its own, and `crispEdges`
                keeps neighbouring modules from growing hairline seams
                when the picture is scaled. A square of the same size
                stands in while the ticket is on its way, so the dialog
                does not jump when it lands; a ticket too long for any QR
                version answers no code and the token alone is the
                invite. */}
            {ticket?.qr ? (
              <svg
                data-ticket-qr
                role="img"
                aria-label={gui.ticket_qr}
                viewBox={`0 0 ${ticket.qr.span} ${ticket.qr.span}`}
                shapeRendering="crispEdges"
                className="mx-auto size-60 flex-none text-foreground"
              >
                <path d={ticket.qr.path} fill="currentColor" />
              </svg>
            ) : (
              !ticket && <div aria-hidden className="mx-auto size-60 flex-none" />
            )}
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
        {/* **The one act is carrying the token across**; closing is the
            plain button beside it (components/Dialog). The ticket stays
            good after the dialog goes — it burns on use or on its window,
            not on this — so the way out is 关闭, not 取消. */}
        <DialogFooter>
          <Button data-dialog-cancel onClick={() => onOpenChange(false)}>
            {gui.close}
          </Button>
          {!error && (
            <Button
              variant="primary"
              data-copy-ticket
              data-said={copied ?? undefined}
              disabled={!ticket}
              onClick={copy}
            >
              {copied === "copied" ? gui.copied : copied === "failed" ? gui.copy_failed : gui.copy_ticket}
            </Button>
          )}
        </DialogFooter>
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
      <DialogContent form data-join-dialog>
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
          <Button data-dialog-cancel onClick={() => onOpenChange(false)}>
            {gui.delete_keep}
          </Button>
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
 * The root's words, **when somebody asks** (docs/CONNECT.md 身份): the
 * 24 words are the network's root key, and the only way back to it if
 * the machine holding it is lost.
 *
 * **On demand, not at first start** (2026-10-01, the owner: 「根钥匙是
 * 不是只在共享的时候出现更友好」). Since 0e4fcaa the machine that made
 * the root opened on this window by itself; now nothing opens it but the
 * 「显示…」 on 「…」 → 安全 (`SecurityPage`), and the sharing flow will
 * call it from there when it lands. That press is the deliberate act, so
 * the words are fetched as the window opens — the second press inside
 * the window existed only because nobody had chosen to open it. Looking
 * clears the node's 「not yet shown」 mark (`Node::root_words`).
 *
 * **The app only.** The words are not served to the web face
 * (`khor_gui_core` `tests/command_tables.rs`), and the row that opens
 * this is drawn in the app alone.
 */
export function RootWordsDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [words, setWords] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      // Off the screen means out of memory too: the words are the root.
      setWords(null);
      setError(null);
      return;
    }
    let live = true;
    rootWords()
      .then((w) => live && setWords(w.split(" ")))
      .catch((e) => live && setError(said(e)));
    return () => {
      live = false;
    };
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
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
          <Button data-root-words-done onClick={onClose}>
            {gui.close}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
