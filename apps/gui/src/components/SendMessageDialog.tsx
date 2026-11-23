import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, TaskActions, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { gui } from "@/gen/catalog";

/** 发一条消息 into a conversation the vendor keeps.
 *
 * The answer is the **vendor's own receipt** (`Queued message … for thread
 * …`), shown rather than summarised: whether the thread took it is Codex's
 * sentence, and a khor sentence claiming success would be a claim about
 * another program's queue. A refusal is the vendor's stderr for the same
 * reason — a wrong id should read like Codex saying the id is wrong.
 */
export function SendMessageDialog({
  open,
  onClose,
  onSend,
}: {
  open: boolean;
  onClose: () => void;
  onSend: (text: string) => Promise<string>;
}) {
  const [busy, setBusy] = useState(false);
  const [value, setValue] = useState("");
  const [receipt, setReceipt] = useState<string | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  useEffect(() => {
    if (open) {
      setBusy(false);
      setValue("");
      setReceipt(null);
      setRefused(null);
    }
  }, [open]);
  const go = () => {
    const text = value;
    if (busy || receipt !== null || !text.trim()) return;
    setBusy(true);
    onSend(text)
      .then((said) => setReceipt(said), (e: unknown) => setRefused(String((e as Error)?.message ?? e)))
      .finally(() => setBusy(false));
  };
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <TaskOverlay purpose="edit" data-send-message>
        <DialogHeader>
          <DialogTitle>{gui.send_message_title}</DialogTitle>
        </DialogHeader>
        <Textarea
          autoFocus
          rows={4}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          aria-label={gui.send_message}
        />
        <span className="text-callout text-muted-foreground">{gui.send_message_hint}</span>
        {refused && <span className="sr-only text-state-failed" data-send-message-failed>{refused}</span>}
        {refused && <span className="text-callout text-state-failed">{refused}</span>}
        {receipt !== null && (
          <span className="text-callout text-muted-foreground" data-send-message-receipt>
            {receipt}
          </span>
        )}
        <TaskActions
          busy={busy}
          dirty={receipt === null && value.trim().length > 0}
          cancel={
            <Button data-dialog-cancel onClick={onClose}>
              {gui.delete_keep}
            </Button>
          }
          primary={
            <Button variant="primary" data-send-message-go onClick={go} disabled={receipt !== null}>
              {gui.send_message}
            </Button>
          }
        />
      </TaskOverlay>
    </Dialog>
  );
}
