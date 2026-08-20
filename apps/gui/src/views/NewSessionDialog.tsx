// The wizard (会话身份批B): opening a session is the first act, and it
// asks the four questions of the ruling — where (目录), which agent
// (claude or codex — 批8; codex is chat-only, its TUI cannot be named
// before it exists), which form (对话 or 终端 — the same two words the
// detail's face switch wears), and what to call it (optional; the
// directory names it otherwise). The row appears in the list like any
// other; the vendor's own uuid is its id on the chat form, so hooks
// and the disk sweep merge into it with no ceremony.
import { useEffect, useState } from "react";

import { openSession } from "@/api";
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

export function NewSessionDialog({
  open,
  onOpenChange,
  onOpened,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The freshly-minted session id — the caller may select its row. */
  onOpened: (id: string) => void;
}) {
  const [dir, setDir] = useState("~");
  const [title, setTitle] = useState("");
  const [form, setForm] = useState<"chat" | "term">("chat");
  const [agent, setAgent] = useState<"claude" | "codex">("claude");
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);

  // Reopening must not inherit a stale attempt (TellDialog's rule).
  useEffect(() => {
    if (!open) return;
    setDir("~");
    setTitle("");
    setForm("chat");
    setAgent("claude");
    setError(null);
    setOpening(false);
  }, [open]);

  const create = () => {
    if (opening) return;
    setOpening(true);
    setError(null);
    openSession(dir, title, form, agent)
      .then((id) => {
        onOpenChange(false);
        onOpened(id);
      })
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setOpening(false));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-new-session-dialog>
        <DialogHeader>
          <DialogTitle>{gui.new_session}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_dir}</span>
            <Input
              data-new-session-dir
              value={dir}
              onChange={(e) => setDir(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && create()}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_name}</span>
            <Input
              data-new-session-name
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && create()}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_agent}</span>
            <div className="flex gap-1">
              {/* Vendor names are proper nouns with no catalog entry
                  (words.ts's rule), so the buttons wear them as-is. */}
              <Button
                size="sm"
                variant={agent === "claude" ? "secondary" : "ghost"}
                data-new-session-claude
                data-on={agent === "claude"}
                onClick={() => setAgent("claude")}
              >
                claude
              </Button>
              <Button
                size="sm"
                variant={agent === "codex" ? "secondary" : "ghost"}
                data-new-session-codex
                data-on={agent === "codex"}
                onClick={() => {
                  setAgent("codex");
                  // codex is chat-only this batch (批8): the form
                  // follows, and the 终端 button below greys out.
                  setForm("chat");
                }}
              >
                codex
              </Button>
            </div>
          </label>
          <div className="flex gap-1">
            <Button
              size="sm"
              variant={form === "chat" ? "secondary" : "ghost"}
              data-new-session-chat
              data-on={form === "chat"}
              onClick={() => setForm("chat")}
            >
              {gui.view_chat}
            </Button>
            <Button
              size="sm"
              variant={form === "term" ? "secondary" : "ghost"}
              data-new-session-term
              data-on={form === "term"}
              disabled={agent === "codex"}
              onClick={() => setForm("term")}
            >
              {gui.view_terminal}
            </Button>
          </div>
          {error && (
            <p data-new-session-error className="text-sm text-destructive">
              {error}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button data-new-session-create disabled={opening} onClick={create}>
            {gui.create}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
