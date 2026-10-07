import { useCallback, useEffect, useRef, useState } from "react";

import {
  fetchDevices,
  inApp,
  machineLog,
  pickAndSendFile,
  retryTransfer,
  stopTransfer,
  tell,
  uploadFile,
  type ChatMsgRow,
  type MachineLog,
} from "@/api";
import { PathCut } from "@/components/PathCut";
import { FilePreview, SendInto } from "@/views/chat-files";
import { StatusWord } from "@/components/StatusWord";
import { FileKindIcon } from "@/components/FileKindIcon";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { YIELD_MS } from "@/lib/tempo";
import { BarSeat } from "@/components/SessionBar";
import { Composer, COMPOSER_TEXT, PlusSeat, SendSeat, Tip, useGrowingText } from "@/views/Composer";
import { imeKey } from "@/lib/ime";

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
 * a chip: name, size, and how its move stands — **the file goes
 * directly** (STORE-BRIEF phase 2: 直接传，不用等接收): the receiver
 * pulls it at once, and every device reads the same record of it (its
 * progress with a ×, where it landed, or why it failed with 重试).
 * ⚠️ 墓碑: the receiver's 收下 on a 待批 chip.
 *
 * Where a file lands is said above the box: the receiver's receive
 * location by default, or a directory picked on that machine (更改…);
 * ＋ sends there in one press.
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
  /** The box, for the keyboard's switch (`TermKeys`). */
  const box = useRef<HTMLTextAreaElement | null>(null);
  // As tall as its lines, up to the capsule's ceiling (V05).
  useGrowingText(box, draft);

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
  /** The directory picked on the receiver; `null` is its receive
      location. Kept while this conversation is open. */
  const [into, setInto] = useState<string | null>(null);
  /** Whether the receiver is this machine (its files are already on
      this disk). */
  const [here, setHere] = useState(false);
  useEffect(() => {
    let live = true;
    setInto(null);
    fetchDevices().then(
      (rows) => live && setHere(!!rows.find((r) => r.name === machine)?.me),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [machine]);
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
      fileOutcome(pickAndSendFile(machine, into));
    } else {
      fileInput.current?.click();
    }
  };
  const picked = (f: File | undefined) => {
    if (!f) return;
    setSendingFile(true);
    fileOutcome(uploadFile(machine, f, into));
  };

  return (
    <div data-machine-chat className="flex min-h-0 flex-1 flex-col">
      <div
        ref={scroller}
        // The deck folds while the messages are read (UX-13, `lib/deck-fold`).
        data-machine-chat-scroll
        onScroll={(e) => {
          const el = e.currentTarget;
          stickToEnd.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
        }}
        // The messages are read and copied (app.css 选字与手).
        data-selectable
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
            machine={machine}
            here={here}
            onChanged={refresh}
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
          the writing is. **On the narrow face it is the session bar's
          second row** (`components/SessionBar`, 2026-10-02), the agent
          conversation's arrangement. */}
      <BarSeat at="under">
      {/* Where a file sent from here lands: the receiver's receive
          location, or a directory picked on it. ＋ sends there at once. */}
      <div className="flex-none px-row pt-in">
        <SendInto machine={machine} name={machine} into={into} onInto={setInto} className="mx-auto w-full max-w-measure" />
      </div>
      {/* **The conversation's capsule** (`Composer`, the redesign,
          2026-10-02): ＋ to send a file, the text, and send once there is
          something to send. No mode and no interrupt — a note has no
          agent on the other end. */}
      <div className="flex-none px-row pt-xxs pb-in">
      <div className="mx-auto flex w-full max-w-measure text-read">
        <Composer
          lead={
            <Tip label={gui.send_file}>
              <PlusSeat data-attach-file aria-label={gui.send_file} disabled={sendingFile} aria-busy={sendingFile || undefined} onClick={attach} />
            </Tip>
          }
          trail={
            // Drawn only once there is something to send (progressive
            // disclosure: never a dimmed control).
            // **The keyboard stays up after a note** (P7, 2026-10-03, the
            // owner's ruling): the press does not take the focus from the
            // text, so the next line is typed at once, as in Messages.
            draft.trim() ? <SendSeat onSend={send} disabled={sending} keepFocus /> : null
          }
        >
        {/* The browser forms' picker: a real <input type=file>, hidden —
            ＋ is its face. Value cleared after each pick so the same file
            twice still fires onChange. */}
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
          ref={box}
          data-chat-composer
          aria-label={gui.message_text}
          // The same word the agent conversation's capsule wears: one
          // capsule, one invitation (the closing pass, 2026-10-03).
          placeholder={gui.chat_input}
          // Return sends, and the soft keyboard's key says so (Shift+Enter
          // on a hardware keyboard breaks the line).
          enterKeyHint="send"
          value={draft}
          rows={1}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (imeKey(e)) return;
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
            }
          }}
          // No size of its own, for the reason the shared box gives up
          // its own (`ui/textarea`): the size belongs to the column this
          // sits in, and a person should write in the size they read in.
          data-composer-text
          className={COMPOSER_TEXT}
        />
        </Composer>
      </div>
      </div>
      </BarSeat>
      {/* ⚠️ 墓碑 (round UX-3): the keyboard switch in the bar. The box raises
          the keyboard and a drag down over the content lowers it
          (`SessionScreen`); a third way was a second door. */}
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
  machine,
  here,
  onChanged,
}: {
  m: ChatMsgRow;
  stamp: string | null;
  machine: string;
  here: boolean;
  onChanged: () => void;
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
          <FileChip m={m} machine={machine} here={here} onChanged={onChanged} />
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

/**
 * A file message: the summary that travelled the conversation, and how
 * its move stands as the receiver wrote it — the same on every device.
 * While it moves: how far, and × to cancel (nothing partial stays).
 * Landed: where, pressed to open it (copy the path and 在文件视图中打开
 * are there). Failed: the receiver's sentence and 重试; cancelled: said,
 * with 重试. The receiver's own seat also wears the state word.
 */
function FileChip({
  m,
  machine,
  here,
  onChanged,
}: {
  m: ChatMsgRow;
  machine: string;
  here: boolean;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const [looking, setLooking] = useState<string | null>(null);
  useEffect(() => {
    if (failed === null) return;
    const t = window.setTimeout(() => setFailed(null), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [failed]);
  const size = (m.files ?? []).reduce((n, f) => n + f.size, 0);
  // Not started yet is on its way too: the receiver starts it by itself.
  const moving = !m.retracted && (m.step === null || m.step === "moving");
  const pct = size > 0 ? Math.min(100, Math.floor((m.moved * 100) / size)) : 0;
  const act = (p: Promise<unknown>) => {
    setBusy(true);
    p.then(
      () => {
        setBusy(false);
        onChanged();
      },
      (e: unknown) => {
        setBusy(false);
        setFailed(String(e instanceof Error ? e.message : e));
      },
    );
  };
  return (
    <div
      data-msg-files
      data-word={m.word ?? undefined}
      data-file-step={m.step ?? "waiting"}
      className={cn(
        "flex flex-col gap-1 rounded-lg px-3 py-2 text-callout",
        m.from_me ? "bg-accent" : "border bg-card",
      )}
    >
      {/* The file leads with the family's own mark and its name at
          title weight: a document being handed over is the thing this
          message *is*, not a line in it. Size rides under the name. */}
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
      {moving && (
        <div className="flex items-center gap-2">
          <div
            data-file-progress={pct}
            role="progressbar"
            aria-valuenow={pct}
            aria-valuemin={0}
            aria-valuemax={100}
            className="h-1 min-w-0 flex-1 overflow-hidden rounded-full bg-fill"
          >
            <div className="h-full bg-foreground/40" style={{ width: `${pct}%` }} />
          </div>
          <span className="flex-none text-caption tabular-nums text-muted-foreground">
            {fmtSize(m.moved)} / {fmtSize(size)}
          </span>
          <Button
            size="sm"
            variant="ghost"
            data-file-stop
            aria-label={gui.file_stop}
            disabled={busy}
            onClick={() => act(stopTransfer(m.id))}
          >
            ×
          </Button>
        </div>
      )}
      {m.step === "landed" &&
        m.landed.map((p) => (
          <button
            key={p}
            type="button"
            data-file-landed={p}
            className="min-w-0 text-left text-caption text-muted-foreground"
            onClick={() => setLooking(p)}
          >
            <PathCut text={p} className="min-w-0" />
          </button>
        ))}
      {(m.step === "failed" || m.step === "stopped") && (
        <div className="flex items-center justify-between gap-2">
          <span
            data-file-why
            className={cn("min-w-0 text-caption", m.step === "failed" ? "text-state-failed" : "text-muted-foreground")}
          >
            {m.step === "failed" ? gui.file_failed(m.why ?? "") : gui.file_stopped}
          </span>
          <Button size="sm" data-file-retry disabled={busy} aria-busy={busy || undefined} onClick={() => act(retryTransfer(m.id))}>
            {gui.region_retry}
          </Button>
        </div>
      )}
      {failed !== null && (
        <span data-file-act-failed className="text-caption text-state-failed">
          {failed}
        </span>
      )}
      {m.word && <StatusWord data-file-word word={m.word} />}
      <FilePreview machine={machine} here={here} shown="" path={looking} onClose={() => setLooking(null)} />
    </div>
  );
}
