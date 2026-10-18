// What is left of the devices pane's pairing pieces once 添加机器
// (`AddMachine.tsx`, round join-d) took over both halves of joining:
// the camera layer its khor 主机 page scans through, the alone note,
// 待批, and the root's words.
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";

import {
  approveMachine,
  fetchWaiting,
  rootWords,
  said,
  stopScan,
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
import { cli, gui } from "@/gen/catalog";

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
export function ScanLayer() {
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
 * what to do next and the way to do it. A brand-new install landed on a
 * list holding one row, its own, with the way to add another two presses
 * away under ＋ and nothing saying so. It goes the moment a second
 * machine is in the table. One act since round join-d: 添加机器, whose
 * khor 主机 page holds both ends of a pairing.
 */
export function AloneNote({ onAdd }: { onAdd: () => void }) {
  return (
    // On the rows' own left line: the list's inset plus the row's, or the
    // narrow list's one inset (`SideList`).
    <div data-alone className="px-in py-base [[data-narrow]_&]:px-0">
      <div className="flex flex-col items-start gap-in px-row text-callout text-muted-foreground [[data-narrow]_&]:px-group">
        <span>{gui.alone_how}</span>
        <Button size="sm" data-alone-add className="pointer-coarse:min-h-touch" onClick={onAdd}>
          {gui.add_machine}
        </Button>
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
