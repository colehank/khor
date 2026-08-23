import { useCallback, useEffect, useRef, useState } from "react";

import {
  acceptTransfer,
  machineLog,
  tell,
  type ChatMsgRow,
  type MachineLog,
} from "@/api";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { word } from "@/words";

/** The rows' beat, borrowed: this pane is a face on the same CRDT the
    list reads, and two different clocks would show two conversations. */
const POLL_MS = 2000;

/** Bytes for a chip, in the reader's units. A number, not a word — the
    catalog owns words, and a size is data (the files pane's rule). */
function fmtSize(n: number): string {
  if (n < 1024) return `${n}B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)}KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)}MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(1)}GB`;
}

/**
 * A machine's conversation — the same stream `khor log <机器>` prints
 * (`Node::log_faced` behind both), painted as a chat.
 *
 * Text lands as bubbles, own words on the right in `ChatView`'s
 * clothes so one product speaks one bubble language. A file message is
 * a chip: name, size, and the transfer's own state word — the word is
 * the node's (docs/UX.md 状态呈现), this pane never derives one. The
 * waiting chip (待批, the receiver's seat) carries the one verb the
 * seat owns: 收下, `khor accept` behind a button. No send-file control
 * exists here — the upload pipe does not, and a dead button is worse
 * than an absent one (ledger: GUI 发文件).
 *
 * The composer at the foot is `tell`'s pipe — the same call the 留言
 * dialog makes, so the two ways of speaking cannot drift.
 */
export function MachineChat({ id }: { id: string }) {
  const [log, setLog] = useState<MachineLog | null>(null);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const scroller = useRef<HTMLDivElement | null>(null);
  const stickToEnd = useRef(true);

  const refresh = useCallback(() => {
    machineLog(id).then(
      (l) => setLog(l),
      // A poll that threw on a blip must not take the screen down;
      // the last read stays up (the sessions poll's rule).
      () => {},
    );
  }, [id]);

  useEffect(() => {
    setLog(null);
    refresh();
    const t = setInterval(refresh, POLL_MS);
    return () => clearInterval(t);
  }, [refresh]);

  // Follow the tail only while the reader is at it: pinning the scroll
  // on every poll would tear the pane out from under somebody reading
  // history.
  useEffect(() => {
    const el = scroller.current;
    if (el && stickToEnd.current) el.scrollTop = el.scrollHeight;
  }, [log]);

  const send = () => {
    const text = draft.trim();
    if (!text || sending) return;
    setSending(true);
    // The channel is the row's title-independent name; the id carries
    // it (`chat/<channel>`), and peeling it here mirrors the read side.
    tell(id.startsWith("chat/") ? id.slice(5) : id, text).then(
      () => {
        setDraft("");
        setSending(false);
        refresh();
      },
      () => setSending(false),
    );
  };

  return (
    <div data-machine-chat className="flex min-h-0 flex-1 flex-col">
      <div
        ref={scroller}
        onScroll={(e) => {
          const el = e.currentTarget;
          stickToEnd.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
        }}
        className="min-h-0 flex-1 space-y-2 overflow-y-auto px-4 py-3"
      >
        {log && log.broken > 0 && (
          <div data-chat-broken className="text-center text-xs text-muted-foreground">
            {cli.broken_blocks(log.broken)}
          </div>
        )}
        {log?.messages.map((m) => <MsgLine key={m.id} m={m} onAccepted={refresh} />)}
      </div>
      <div className="flex flex-none items-end gap-2 border-t px-3 py-2">
        <textarea
          data-chat-composer
          aria-label={gui.message_text}
          value={draft}
          rows={1}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
            }
          }}
          className="min-h-0 flex-1 resize-none rounded-md border bg-transparent px-3 py-1.5 text-sm outline-none"
        />
        <Button size="sm" data-chat-send disabled={sending || !draft.trim()} onClick={send}>
          {gui.send_it}
        </Button>
      </div>
    </div>
  );
}

function MsgLine({ m, onAccepted }: { m: ChatMsgRow; onAccepted: () => void }) {
  return (
    <div data-msg={m.id} className={cn("flex", m.from_me ? "justify-end" : "justify-start")}>
      <div className="max-w-[85%]">
        {/* The sender, over the bubble — the name as written, never
            re-resolved (`Sender`'s rule), and only where it informs:
            own messages sit on their own side already. */}
        {!m.from_me && (
          <div data-msg-from className="mb-0.5 text-xs text-muted-foreground">
            {m.from}
          </div>
        )}
        {m.retracted ? (
          <div className="text-xs text-muted-foreground">{cli.retracted}</div>
        ) : m.text !== null ? (
          <div
            data-msg-text
            className={cn(
              "whitespace-pre-wrap rounded-lg px-3 py-1.5 text-sm",
              m.from_me ? "bg-muted" : "border",
            )}
          >
            {m.text}
          </div>
        ) : m.files !== null ? (
          <FileChip m={m} onAccepted={onAccepted} />
        ) : (
          // A body kind this build cannot read — said out loud rather
          // than rendered blank (`MsgBody::Unknown`'s rule).
          <div className="text-xs text-muted-foreground">{cli.unknown_body(m.unknown ?? "")}</div>
        )}
      </div>
    </div>
  );
}

/**
 * A file message: the summary that travelled the conversation. The
 * bytes are a transfer behind it, and the chip wears that transfer's
 * state word. 收下 appears only on the waiting word — the seat that
 * can answer is the only one shown the verb.
 */
function FileChip({ m, onAccepted }: { m: ChatMsgRow; onAccepted: () => void }) {
  const [accepting, setAccepting] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!failed) return;
    const t = window.setTimeout(() => setFailed(false), 2_000);
    return () => window.clearTimeout(t);
  }, [failed]);
  return (
    <div
      data-msg-files
      data-word={m.word ?? undefined}
      className={cn("flex flex-col gap-1 rounded-lg px-3 py-1.5 text-sm", m.from_me ? "bg-muted" : "border")}
    >
      {(m.files ?? []).map((f) => (
        <div key={f.name} className="flex items-center gap-2">
          <span data-file-name className="min-w-0 truncate">
            {f.name}
          </span>
          <span className="flex-none text-xs text-muted-foreground">{fmtSize(f.size)}</span>
        </div>
      ))}
      {m.word && (
        <div className="flex items-center gap-2">
          {m.word && (
            <span data-file-word className="text-xs" style={{ color: `var(--state-${m.word})` }}>
              {word(m.word)}
            </span>
          )}
          {m.word === "blocked" && (
            <Button
              variant="outline"
              size="sm"
              data-accept-file
              data-failed={failed}
              disabled={accepting}
              className="data-[failed=true]:text-state-failed"
              onClick={() => {
                setAccepting(true);
                acceptTransfer(m.id).then(
                  () => {
                    setAccepting(false);
                    onAccepted();
                  },
                  () => {
                    setAccepting(false);
                    setFailed(true);
                  },
                );
              }}
            >
              {failed ? gui.accept_file_failed : gui.accept_file}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
