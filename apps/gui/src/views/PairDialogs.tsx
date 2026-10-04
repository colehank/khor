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
import { createPortal } from "react-dom";

import {
  approveMachine,
  fetchScannerHere,
  fetchWaiting,
  invite,
  openCameraSettings,
  pair,
  rootWords,
  said,
  scanTicket,
  stopScan,
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
  machines,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The machine table as the lists read it, so a join with this ticket
      is said here (UX-8). */
  machines: { id: string; shown: string }[];
}) {
  const [ticket, setTicket] = useState<Ticket | null>(null);
  /** Who was in the table when this ticket was minted. */
  const [before, setBefore] = useState<Set<string> | null>(null);
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
    setBefore(new Set(machines.map((m) => m.id)));
    let live = true;
    invite()
      .then((t) => live && setTicket(t))
      .catch((e) => live && setError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
    // The table at the moment of minting, not as it moves: read once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);
  // **Somebody used it** (UX-8, the first run): a machine that was not in
  // the table when this ticket was minted is the one that joined with it,
  // and the dialog says so instead of going on offering a spent ticket.
  const joined = before && before.size > 0 && ticket ? machines.find((m) => !before.has(m.id)) : undefined;
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
            {/* A spent ticket is not offered again: once somebody joined
                with it, the code and the token go and the line under
                them says who (UX-8). */}
            {joined ? null : ticket?.qr ? (
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
            {!joined && (
              <Input
                data-ticket
                aria-label={gui.ticket}
                readOnly
                value={ticket?.token ?? ""}
                onFocus={(e) => e.currentTarget.select()}
              />
            )}
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
            {joined ? (
              <div data-ticket-used role="status" className="text-callout text-foreground">
                {gui.ticket_used_by(joined.shown)}
              </div>
            ) : (
              ticket && (
                <div data-ticket-window className="text-callout text-muted-foreground">
                  {cli.invite_window(ticket.minutes)}
                </div>
              )
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
          {!error && !joined && (
            <Button
              variant="primary"
              data-copy-ticket
              // The ticket is still being minted: busy, not greyed.
              disabled={!ticket}
              aria-busy={!ticket || undefined}
              data-said={copied ?? undefined}
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

/**
 * Takes someone's ticket and joins — **one join, two ways to enter the
 * ticket** (docs/IOS.md 第三步). It is typed or pasted into the field, or,
 * where this face has a camera (`fetchScannerHere`: the phone builds),
 * read off the other machine's 邀请设备 QR code: the code is the ticket
 * itself, so what the camera reads lands in this same field and goes
 * down this same `join`. A face without a camera draws no 扫码 at all.
 */
export function JoinDialog({
  open,
  onOpenChange,
  onJoined,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The join took: the lists read the table again now (UX-8). */
  onJoined?: () => void;
}) {
  const [ticket, setTicket] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [joining, setJoining] = useState(false);
  const [scanner, setScanner] = useState(false);
  /** The camera is up: the page steps aside for it (`ScanLayer`). */
  const [scanning, setScanning] = useState(false);
  /** Why the camera gave nothing — said, never a dead button. */
  const [camera, setCamera] = useState<"denied" | "failed" | null>(null);

  useEffect(() => {
    if (!open) return;
    setTicket("");
    setError(null);
    setJoining(false);
    setCamera(null);
    let live = true;
    fetchScannerHere().then((here) => live && setScanner(here));
    return () => {
      live = false;
    };
  }, [open]);

  const joinWith = (text: string) => {
    const t = text.trim();
    if (joining) return;
    // Always pressable (`components/Why`): with no ticket, say so and put
    // the caret in the box.
    if (!t) {
      setError(gui.need_ticket);
      requestAnimationFrame(() => document.querySelector<HTMLElement>("[data-ticket-input]")?.focus());
      return;
    }
    setJoining(true);
    setError(null);
    pair(t)
      // Nothing to announce: the new machine's row arrives wearing its
      // own face, which is the report — read now rather than on the next
      // poll, which left 「这里只有这台机器」 under the new row for a beat.
      .then(() => {
        onOpenChange(false);
        onJoined?.();
      })
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setJoining(false));
  };
  const join = () => joinWith(ticket);

  // The scan fills the field and presses 加入 — the person already chose
  // this ticket by pointing the camera at it, and the field shows what
  // was read in case the join says no.
  const scan = () => {
    setCamera(null);
    setError(null);
    void scanTicket(() => setScanning(true)).then((read) => {
      setScanning(false);
      if ("ticket" in read) {
        setTicket(read.ticket);
        joinWith(read.ticket);
      } else if ("refused" in read) {
        setCamera(read.refused);
      }
    });
  };

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent form data-join-dialog>
          <DialogHeader>
            <DialogTitle>{gui.join_with_a_ticket}</DialogTitle>
          </DialogHeader>
          <div className="flex items-center gap-in">
            <Input
              data-ticket-input
              aria-label={gui.paste_a_ticket}
              placeholder={gui.paste_a_ticket}
              value={ticket}
              onChange={(e) => setTicket(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && join()}
            />
            {scanner && (
              <Button
                data-scan-ticket
                className="flex-none pointer-coarse:min-h-touch"
                disabled={joining || scanning}
                aria-busy={scanning || undefined}
                onClick={scan}
              >
                {gui.scan_a_ticket}
              </Button>
            )}
          </div>
          {camera === "denied" && (
            <div data-scan-denied className="flex flex-col items-start gap-in text-callout text-destructive">
              <span>{gui.scan_camera_denied}</span>
              <Button
                size="sm"
                data-scan-settings
                className="pointer-coarse:min-h-touch"
                onClick={() => void openCameraSettings()}
              >
                {gui.scan_open_settings}
              </Button>
            </div>
          )}
          {camera === "failed" && (
            <div data-scan-failed className="text-callout text-destructive">
              {gui.scan_no_camera}
            </div>
          )}
          {/* **The wait, said** (UX-8): a dial can sit on its timeout, and a
              busy button alone was ten seconds with no word. */}
          {joining && (
            <div data-join-dialing role="status" className="text-callout text-muted-foreground">
              {gui.join_dialing}
            </div>
          )}
          {error && (
            <div data-dialog-error className="text-callout text-destructive">
              {error}
            </div>
          )}
          <DialogFooter>
            <Button data-dialog-cancel onClick={() => onOpenChange(false)}>
              {gui.delete_keep}
            </Button>
            <Button variant="primary" data-join disabled={joining} aria-busy={joining || undefined} onClick={join}>
              {gui.join}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {scanning && <ScanLayer />}
    </>
  );
}

/**
 * What the page is while the camera reads (docs/IOS.md 第三步).
 *
 * The scan is **windowed**: the plugin puts the camera *behind* the
 * webview and makes the webview clear, so the picture shows only where
 * the page paints nothing. While this is up the document says so
 * (`html[data-scanning]`, `app.css`): the page's grounds go clear and
 * everything but this layer is hidden, dialog and scrim included. What
 * is left is a sentence saying what to point at, a frame to point it
 * with, and 取消 — the way out the plugin's full-screen mode does not
 * have. Its ink is the one that does not change with the theme
 * (`mark-ink-light` over `mark-veil-dark`), because what is under it is
 * a camera, not the paper.
 *
 * In `body` rather than in the dialog: the dialog is centred with a
 * transform, which would make a fixed layer inside it the dialog's size.
 */
function ScanLayer() {
  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-scanning", "");
    return () => root.removeAttribute("data-scanning");
  }, []);
  return createPortal(
    <div
      data-scanner
      className="pointer-events-auto fixed inset-0 z-50 flex flex-col items-center justify-between text-mark-ink-light"
    >
      <div className="w-full bg-mark-veil-dark/50 px-pane pt-[max(env(safe-area-inset-top),var(--gap-pane))] pb-base text-center text-body">
        {gui.scan_aim}
      </div>
      <div aria-hidden className="size-60 flex-none rounded-lg border-2 border-mark-ink-light" />
      <div className="flex w-full justify-center bg-mark-veil-dark/50 px-pane pt-base pb-[max(env(safe-area-inset-bottom),var(--gap-pane))]">
        <Button data-scan-cancel className="pointer-coarse:min-h-touch" onClick={() => void stopScan()}>
          {gui.delete_keep}
        </Button>
      </div>
    </div>,
    document.body,
  );
}

/**
 * **Only this machine is here** (UX-8, the first run): under the list,
 * what to do next and the two ways to do it. A brand-new install landed
 * on a list holding one row, its own, with the way to add another two
 * presses away under 机器's ＋ and nothing saying so. Pairing has no
 * direction, so both halves are offered; it goes the moment a second
 * machine is in the table.
 */
export function AloneNote({ onInvite, onJoin }: { onInvite: () => void; onJoin: () => void }) {
  return (
    // On the rows' own left line: the list's inset plus the row's, or the
    // narrow list's one inset (`SideList`).
    <div data-alone className="px-in py-base [[data-narrow]_&]:px-0">
      <div className="flex flex-col items-start gap-in px-row text-callout text-muted-foreground [[data-narrow]_&]:px-group">
        <span>{gui.alone_how}</span>
        <div className="flex flex-wrap gap-in">
          <Button size="sm" data-alone-join className="pointer-coarse:min-h-touch" onClick={onJoin}>
            {gui.join_with_a_ticket_ask}
          </Button>
          <Button size="sm" data-alone-invite className="pointer-coarse:min-h-touch" onClick={onInvite}>
            {gui.make_a_ticket_ask}
          </Button>
        </div>
      </div>
    </div>
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
      <div className="px-in pt-in pb-xxs text-group text-muted-foreground">{gui.pending_title}</div>
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
              aria-busy={busy || undefined}
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
