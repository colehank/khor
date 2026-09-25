import { useCallback, useEffect, useRef, useState } from "react";

import {
  acceptTransfer,
  inApp,
  machineLog,
  pickAndSendFile,
  tell,
  uploadFile,
  type ChatMsgRow,
  type MachineLog,
} from "@/api";
import { IconAttach } from "@/components/icons";
import { FileKindIcon } from "@/components/FileKindIcon";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { YIELD_MS } from "@/lib/tempo";
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
 * seat owns: 收下, `khor accept` behind a button.
 *
 * The composer at the foot is `tell`'s pipe — the same call the 留言
 * dialog makes, so the two ways of speaking cannot drift. Beside it,
 * 发文件 (GUI 发文件批): in the app a native picker answers with a
 * real path (`pick_and_send`), in a browser the bytes upload to the
 * sending machine first (`uploadFile`) — one act, two ways of handing
 * the file to the same `send`. The upload's in-between is said out
 * loud (上传中…), and a refusal — the size cap included — lands as the
 * backend's own sentence where the act was started (the pin's rule).
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

  // The channel is the row's title-independent name; the id carries
  // it (`chat/<channel>`), and peeling it here mirrors the read side.
  const machine = id.startsWith("chat/") ? id.slice(5) : id;

  const send = () => {
    const text = draft.trim();
    if (!text || sending) return;
    setSending(true);
    tell(machine, text).then(
      () => {
        setDraft("");
        setSending(false);
        refresh();
      },
      () => setSending(false),
    );
  };

  // 发文件 — the file leg of the composer (module head). One in-flight
  // at a time; the failure keeps the backend's own sentence, because
  // "too big" and "machine unreachable" have different next steps and
  // the sentence is where they differ.
  const [sendingFile, setSendingFile] = useState(false);
  const [fileError, setFileError] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement | null>(null);
  const fileOutcome = (p: Promise<unknown>) =>
    p.then(
      () => {
        setSendingFile(false);
        refresh();
      },
      (e: unknown) => {
        setSendingFile(false);
        setFileError(String(e instanceof Error ? e.message : e));
      },
    );
  const attach = () => {
    if (sendingFile) return;
    setFileError(null);
    if (inApp) {
      // The native picker owns the visible in-between here; a cancel
      // resolves null and simply ends the flight.
      setSendingFile(true);
      fileOutcome(pickAndSendFile(machine));
    } else {
      fileInput.current?.click();
    }
  };
  const picked = (f: File | undefined) => {
    if (!f) return;
    setSendingFile(true);
    fileOutcome(uploadFile(machine, f));
  };

  return (
    <div data-machine-chat className="flex min-h-0 flex-1 flex-col">
      <div
        ref={scroller}
        onScroll={(e) => {
          const el = e.currentTarget;
          stickToEnd.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
        }}
        className="min-h-0 flex-1 overflow-y-auto px-4 py-3"
      >
        {/* The same reading column the agent conversation is set in
            (`ChatView`), and it is here because of 两端对齐的是事实
            (docs/UX.md): a message is read the same way whichever
            conversation it arrived in, so the two faces may not be two
            different sizes of type. What is *not* copied over is that
            face's controls — this one has its own.

            The measure is an em, so it and `text-read` ride together. */}
        <div data-chat-column className="mx-auto w-full max-w-measure space-y-2 text-read">
        {log && log.broken > 0 && (
          <div data-chat-broken className="text-center text-caption text-muted-foreground">
            {cli.broken_blocks(log.broken)}
          </div>
        )}
        {log?.messages.map((m, i) => (
          <MsgLine
            key={m.id}
            m={m}
            stamp={stampFor(m, i === 0 ? null : log.messages[i - 1])}
            onAccepted={refresh}
          />
        ))}
        </div>
      </div>
      {/* The file leg's own two lines, above the composer: the upload's
          in-between (browser forms only — the app's picker is its own
          visible state), and a refusal in the backend's sentence where
          the act was started. Neither renders over nothing. */}
      {sendingFile && !inApp && (
        <div data-uploading className="flex-none border-t px-3 py-1 text-caption text-muted-foreground">
          {gui.uploading}
        </div>
      )}
      {fileError && (
        <div data-file-error className="flex-none border-t px-3 py-1 text-caption text-state-failed">
          {gui.send_file_failed}: {fileError}
        </div>
      )}
      {/* The rule spans the pane, the controls stand in the column — the
          same split as the agent conversation's composer, and for the
          same reason: the line divides two regions, the column is where
          the writing is. */}
      <div className="flex-none border-t bg-card px-3 py-2">
      <div className="mx-auto flex w-full max-w-measure items-end gap-2 text-read">
        <Button
          variant="ghost"
          size="icon"
          data-attach-file
          aria-label={gui.send_file}
          disabled={sendingFile}
          onClick={attach}
          className="flex-none text-muted-foreground"
        >
          <IconAttach />
        </Button>
        {/* The browser forms' picker: a real <input type=file>, hidden —
            the button above is its face. Value cleared after each pick
            so the same file twice still fires onChange. */}
        {!inApp && (
          <input
            ref={fileInput}
            type="file"
            hidden
            data-file-input
            onChange={(e) => {
              picked(e.target.files?.[0]);
              e.target.value = "";
            }}
          />
        )}
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
          // No size of its own, for the reason the shared box gives up
          // its own (`ui/textarea`): the size belongs to the column this
          // sits in, and a person should write in the size they read in.
          className="min-h-0 flex-1 resize-none rounded-md border bg-background px-3 py-1.5 outline-none"
        />
        <Button size="sm" data-chat-send disabled={sending || !draft.trim()} onClick={send}>
          {gui.send_it}
        </Button>
      </div>
      </div>
    </div>
  );
}

/** WeChat's clustering distance: two messages further apart than this
    get a time over the later one. One constant, one home. */
const CLUSTER_GAP_MS = 5 * 60_000;

/** The viewer's civil date of a moment — the ledger is read where the
    reader sits, whatever clock the sender wrote on (`Message::at`). */
function civilDay(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function clock(ms: number): string {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/**
 * What stands over a message, if anything: a day line where the civil
 * day turns (今天 / 昨天 / the date itself — a ledger has dates or it
 * is not a ledger), a bare clock where the same day pauses longer than
 * one cluster. The words are the catalog's; the date and the clock are
 * data, not copy.
 */
function stampFor(m: ChatMsgRow, prev: ChatMsgRow | null): string | null {
  const day = civilDay(m.at_ms);
  if (prev === null || civilDay(prev.at_ms) !== day) {
    const now = new Date();
    const today = civilDay(now.getTime());
    const yesterday = civilDay(now.getTime() - 24 * 60 * 60_000);
    const name = day === today ? gui.today : day === yesterday ? gui.yesterday : day;
    return `${name} ${clock(m.at_ms)}`;
  }
  if (m.at_ms - prev.at_ms > CLUSTER_GAP_MS) return clock(m.at_ms);
  return null;
}

function MsgLine({
  m,
  stamp,
  onAccepted,
}: {
  m: ChatMsgRow;
  stamp: string | null;
  onAccepted: () => void;
}) {
  return (
    <>
      {stamp && (
        <div data-msg-stamp className="pt-1 text-center text-caption tabular-nums text-muted-foreground">
          {stamp}
        </div>
      )}
    <div data-msg={m.id} className={cn("flex", m.from_me ? "justify-end" : "justify-start")}>
      <div className="max-w-[85%]">
        {/* The sender, over the bubble — the name as written, never
            re-resolved (`Sender`'s rule), and only where it informs:
            own messages sit on their own side already. */}
        {!m.from_me && (
          <div data-msg-from className="mb-0.5 text-caption text-muted-foreground">
            {m.from}
          </div>
        )}
        {m.retracted ? (
          <div className="text-caption text-muted-foreground">{cli.retracted}</div>
        ) : m.text !== null ? (
          <div
            data-msg-text
            className={cn(
              "whitespace-pre-wrap rounded-lg px-3 py-1.5 text-read-ink",
              m.from_me ? "bg-accent" : "border bg-card",
            )}
          >
            {m.text}
          </div>
        ) : m.files !== null ? (
          <FileChip m={m} onAccepted={onAccepted} />
        ) : (
          // A body kind this build cannot read — said out loud rather
          // than rendered blank (`MsgBody::Unknown`'s rule).
          <div className="text-caption text-muted-foreground">{cli.unknown_body(m.unknown ?? "")}</div>
        )}
      </div>
    </div>
    </>
  );
}

/** The sender's neutral facts, by delivery key (发端中性词批) — an
    unknown key echoes itself, the catalog's own rule for words this
    build has not met. */
// ⚠️ **A function, not a `const`** (批 D3): catalog reads taken at
// module level would pin one language for the life of the tab.
const deliveryWords = (): Record<string, string> => ({
  waiting: gui.file_not_taken,
  fetching: gui.file_being_taken,
  served: gui.file_taken,
});

/**
 * A file message: the summary that travelled the conversation. The
 * bytes are a transfer behind it, and the chip wears that transfer's
 * state word — or, on the sender's seat, a neutral delivery fact
 * (还没收/对方在收/收下了): not one of the six, no state colour, no
 * pressure — 待批 stays the receiver's alone. 收下 appears only on the
 * waiting word — the seat that can answer is the only one shown the
 * verb.
 */
function FileChip({ m, onAccepted }: { m: ChatMsgRow; onAccepted: () => void }) {
  const [accepting, setAccepting] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!failed) return;
    const t = window.setTimeout(() => setFailed(false), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [failed]);
  return (
    <div
      data-msg-files
      data-word={m.word ?? undefined}
      className={cn(
        "flex flex-col gap-1 rounded-lg px-3 py-2 text-callout",
        m.from_me ? "bg-accent" : "border bg-card",
      )}
    >
      {/* The file leads with the family's own mark and its name at
          title weight: a document being handed over is the thing this
          message *is*, not a line in it — what makes the chip readable
          as "a file" before any word is. Size rides under the name at
          aux, the qualifying fact's place. */}
      {(m.files ?? []).map((f) => (
        <div key={f.name} className="flex items-center gap-2">
          <FileKindIcon name={f.name} className="size-4" />
          <div className="min-w-0">
            <div data-file-name className="truncate font-title">
              {f.name}
            </div>
            <div className="text-caption tabular-nums text-muted-foreground">{fmtSize(f.size)}</div>
          </div>
        </div>
      ))}
      {m.delivery && (
        <span data-file-delivery={m.delivery} className="text-caption text-muted-foreground">
          {deliveryWords()[m.delivery] ?? m.delivery}
        </span>
      )}
      {m.word && (
        <div className="flex items-center justify-between gap-2">
          {m.word && (
            <span data-file-word className="text-caption" style={{ color: `var(--state-${m.word})` }}>
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
              {failed
                ? gui.accept_file_failed
                : accepting
                  ? gui.accept_file_working
                  : gui.accept_file}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
