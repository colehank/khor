// The wizard (会话身份批B): opening a session is the first act, and it
// asks the four questions of the ruling — where (目录), which agent
// (智能体), which form (对话 or 终端 — the same two words the detail's
// face switch wears), and what to call it (optional; the directory
// names it otherwise). The row appears in the list like any other; the
// vendor's own uuid is its id on the chat form, so hooks and the disk
// sweep merge into it with no ceremony.
//
// **智能体 is an open position (批⑥):** khor's own two, plus every ACP
// agent this person registered (`khor agents add`). The registered ones
// are fetched rather than listed here — the registry is a replicated
// document, so a name added on another machine is offerable on this one
// the moment it syncs, and a hard-coded list would be a second copy
// that disagrees.
//
// **Only claude keeps the 终端 form.** codex cannot be pre-named before
// its session exists (批8), and a registered agent has no terminal
// spelling khor knows at all — the node refuses both in their own
// words, and this face greys the button so nobody spends a click
// finding that out. Greying rather than hiding: a form that exists for
// one agent and not another is a fact about the agent, and a button
// that vanishes reads as khor having lost it.
import { useEffect, useState } from "react";

import { agents as fetchAgents, openSession, type AgentRow } from "@/api";
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
  const [agent, setAgent] = useState("claude");
  const [known, setKnown] = useState<AgentRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);

  // Reopening must not inherit a stale attempt (TellDialog's rule), and
  // the registry is re-read each time: it syncs from other machines, so
  // a list cached across openings would be a stale answer to the one
  // question this dialog cannot guess.
  useEffect(() => {
    if (!open) return;
    setDir("~");
    setTitle("");
    setForm("chat");
    setAgent("claude");
    setKnown([]);
    setError(null);
    setOpening(false);
    let live = true;
    fetchAgents()
      .then((rows) => live && setKnown(rows))
      // **A registry that could not be read is not an empty registry.**
      // Swallowing this would paint "you have registered nothing",
      // which is a different fact with a different next step — and the
      // one shape khor refuses to let a failure borrow.
      .catch((e) => live && setError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
  }, [open]);

  // Picking anything but claude settles the form too: only claude has a
  // terminal khor knows how to open, so leaving 终端 selected would send
  // a refusal the person could have been spared.
  const pick = (name: string) => {
    setAgent(name);
    if (name !== "claude") setForm("chat");
  };

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
            <div className="flex flex-wrap gap-1">
              {/* Vendor names are proper nouns with no catalog entry
                  (words.ts's rule), and a registered agent wears the
                  name its owner gave it — so every button here is a
                  name, never a translated word. khor's own two lead
                  because they need no registering; the rest follow in
                  the registry's order, which is alphabetical, so two
                  machines paint the same row of buttons. */}
              {["claude", "codex", ...known.map((a) => a.name)].map((name) => (
                <Button
                  key={name}
                  size="sm"
                  variant={agent === name ? "secondary" : "ghost"}
                  data-new-session-agent={name}
                  data-on={agent === name}
                  onClick={() => pick(name)}
                >
                  {name}
                </Button>
              ))}
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
              disabled={agent !== "claude"}
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
