import { useState } from "react";

import type { SessionRow } from "@/api";
import { IconBack } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { ChatView } from "@/views/ChatView";
import { TerminalPane } from "@/views/TerminalPane";
import { word } from "@/words";

/**
 * An agent session has two honest faces: the conversation its vendor
 * recorded (the transcript, `Node::transcript_of`) and the live terminal
 * it runs in. The conversation is the default face (会话身份批 ruling —
 * it is the one that reads the same on every device); the choice is
 * remembered per row, which is the cheapest form of "user preference"
 * and adds no settings entry. The chat face is read-only — speaking into
 * a TUI from a chat box means injecting bytes into whatever state its
 * menus are in; speaking through ACP is its own batch on the ledger.
 */
const faceKey = (id: string) => `khor.session.face.${id}`;

function HostedAgentDetail({ id }: { id: string }) {
  const [view, setViewState] = useState<"term" | "chat">(() =>
    window.localStorage.getItem(faceKey(id)) === "term" ? "term" : "chat",
  );
  const setView = (v: "term" | "chat") => {
    setViewState(v);
    window.localStorage.setItem(faceKey(id), v);
  };
  return (
    <>
      <div className="flex flex-none gap-1 border-b p-1">
        <Button
          size="sm"
          variant={view === "term" ? "secondary" : "ghost"}
          data-view-term
          data-on={view === "term"}
          onClick={() => setView("term")}
        >
          {gui.view_terminal}
        </Button>
        <Button
          size="sm"
          variant={view === "chat" ? "secondary" : "ghost"}
          data-view-chat
          data-on={view === "chat"}
          onClick={() => setView("chat")}
        >
          {gui.view_chat}
        </Button>
      </div>
      {view === "term" ? <TerminalPane id={id} /> : <ChatView id={id} still />}
    </>
  );
}

// Per-kind faces: a GUI session gets the conversation (ChatView); a
// session khor hosts here gets its live terminal (TerminalPane); the
// rest (transfer card) land with their own batches, and until then the
// pane states the row's facts and nothing else — no invented copy
// (docs/UX.md 文案: 装饰性说明归零). Keyed by the row's **kind**, never
// its id: a GUI session's id is spelled `tui/…` on purpose (the
// vendor-session agreement — `gui_host` module head), so the id is an
// address and the kind is the behaviour.
export function DetailPane({
  row,
  narrow,
  onBack,
}: {
  row: SessionRow | null;
  narrow: boolean;
  onBack: () => void;
}) {
  return (
    <section className="flex h-full min-w-0 flex-col">
      <header data-detail-header className="flex h-ctl-lg flex-none items-center gap-2 border-b px-3">
        {/* The back button exists only on the narrow face: there the
            list is genuinely off-screen, so "back" has somewhere to go.
            Wide keeps the list on screen — nothing to go back from. */}
        {narrow && (
          <Button variant="ghost" size="icon" aria-label={gui.back} data-back onClick={onBack}>
            <IconBack />
          </Button>
        )}
        <span className="truncate font-semibold">{row ? row.title || row.id : ""}</span>
      </header>
      {row && row.kind === "gui" ? (
        // Keyed so switching sessions remounts the chat: its cursor,
        // frames and attachment all belong to one conversation.
        <ChatView key={row.id} id={row.id} />
      ) : row && row.attachable && row.category === "claude" ? (
        // A hosted claude gets both faces with a switch; keyed so the
        // choice resets with the session it was made about.
        <HostedAgentDetail key={row.id} id={row.id} />
      ) : row && row.attachable ? (
        // A session with a terminal to be had: hosted here already, or a
        // discovered tmux session the bridge stands a host up for on
        // first open (gui-core `term_open`). `attachable` is the signal,
        // not the kind — a `khor run` shares the kind but has no host
        // and no bridge. Remote hosted sessions are on the ledger (their
        // host is elsewhere), and fall through to the facts below.
        <TerminalPane key={row.id} id={row.id} />
      ) : row && row.category === "claude" ? (
        // A discovered claude session: its recorded past, read from
        // the vendor's own transcript. Claude only, this batch — the
        // other vendors' records are on the ledger, and their rows
        // keep the facts pane below rather than an empty chat.
        <ChatView key={row.id} id={row.id} still />
      ) : (
        <div className="grid flex-1 place-items-center text-center text-sm text-muted-foreground">
          {row ? (
            <div>
              <div style={{ color: `var(--state-${row.word})` }}>{word(row.word)}</div>
              <div>{row.kind}</div>
            </div>
          ) : (
            <div>{gui.pick_a_session}</div>
          )}
        </div>
      )}
    </section>
  );
}
