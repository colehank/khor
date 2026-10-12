import { useCallback, useEffect, useRef, useState } from "react";

import {
  compose,
  fetchDevices,
  inApp,
  machineLog,
  pickPaths,
  retryTransfer,
  said,
  stageFile,
  stopTransfer,
  type ChatMsgRow,
  type DeviceRow,
  type MachineLog,
} from "@/api";
import { PathCut } from "@/components/PathCut";
import { FilePreview, PendingChips, type Pending } from "@/views/chat-files";
import { takeDraft } from "@/lib/composer-handoff";
import { baseName } from "@/lib/paths";
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
 * **The broadcast** (`wide/all`, STORE-BRIEF phase 3 and 5): the khor's
 * one conversation for people — the same stream `khor log` prints,
 * painted as a chat. Text lands as bubbles, own words on the right in
 * `ChatView`'s clothes so one product speaks one bubble language.
 *
 * **A message names its targets** (phase 5, one channel): `@turing`
 * pushes it to turing, `@turing:~/data` names where there, and the files
 * land with nobody pressing anything; every other machine reads the file
 * as an offer (待接收 with 接收). `@` in the box opens the machine picker
 * above it — on every face, the phone too — and the typed form works as
 * typed. A file on another machine is typed as `hinton:/data/x.bin`;
 * ＋ adds this machine's files (in the app a native picker answers with
 * real paths, in a browser the bytes are staged here first). A target
 * that is not reachable refuses the whole message: the node's sentence
 * stands above the box, naming the machine, and the draft stays.
 *
 * Each bubble names its targets by today's names, each with its own
 * move. ⚠️ 墓碑 (phase 5): a machine's own `chat/<机器>` conversation was
 * painted here too, with 「存到」 above its box; the directory is typed
 * as `@机器:路径` now.
 */
export function MachineChat({ id }: { id: string }) {
  const [log, setLog] = useState<MachineLog | null>(null);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
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

  /** The machines, for the picker and for the landed paths' machine. */
  const [machines, setMachines] = useState<DeviceRow[]>([]);
  useEffect(() => {
    let live = true;
    fetchDevices().then(
      (rows) => live && setMachines(rows),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);
  /** The API's spelling of this machine: a chip shows its own landing. */
  const landsOn = machines.find((r) => r.me)?.name ?? "";
  /** **The `@机器` filter** (phase 5): only the messages pushed to that
      machine, by device id; `null` is all. How the messages for one
      machine are found — there is no view of them on its page. */
  const [only, setOnly] = useState<{ device: string; name: string } | null>(null);
  const shownMsgs = log ? (only ? log.messages.filter((m) => m.to.some((t) => t.device === only.device)) : log.messages) : [];

  // 新建会话's 留言 on a machine: the box opens with `@<machine> ` in it.
  useEffect(() => {
    const d = takeDraft();
    if (d === null) return;
    setDraft(d);
    requestAnimationFrame(() => {
      const el = box.current;
      if (!el) return;
      el.focus();
      el.setSelectionRange(d.length, d.length);
    });
  }, []);

  // ── the files going along with the line ──
  const [pending, setPending] = useState<Pending[]>([]);
  const seq = useRef(0);
  const placing = pending.some((p) => p.path === null && p.error === null);
  const fileInput = useRef<HTMLInputElement | null>(null);
  const attach = () => {
    setRefused(null);
    if (inApp) {
      void pickPaths().then(
        (ps) =>
          setPending((now) => [
            ...now,
            ...ps.map((p) => ({ key: `p${++seq.current}`, name: baseName(p), size: null, thumb: null, inline: null, path: p, error: null })),
          ]),
        (e: unknown) => setRefused(said(e)),
      );
    } else {
      fileInput.current?.click();
    }
  };
  const picked = (files: File[]) => {
    for (const f of files) {
      const key = `f${++seq.current}`;
      setPending((now) => [...now, { key, name: f.name, size: f.size, thumb: null, inline: null, path: null, error: null }]);
      stageFile(f).then(
        (path) => setPending((now) => now.map((p) => (p.key === key ? { ...p, path } : p))),
        (e: unknown) => setPending((now) => now.map((p) => (p.key === key ? { ...p, error: said(e) } : p))),
      );
    }
  };

  // ── the @ picker ──
  /** Where the caret is, so the picker reads the word being typed. */
  const [caret, setCaret] = useState(0);
  const [pick, setPick] = useState(0);
  const [pickerShut, setPickerShut] = useState(false);
  /** The word a choice just wrote: the picker stays shut on it. */
  const shut = useRef<string | null>(null);
  const typing = /(^|\s)@([^\s:"]*)$/.exec(draft.slice(0, caret));
  const asked = typing ? typing[2].toLowerCase() : null;
  const choices =
    asked === null || pickerShut
      ? []
      : machines.filter(
          (d) =>
            !d.me &&
            d.sponsor === null &&
            (d.shown.toLowerCase().startsWith(asked) || d.name.toLowerCase().startsWith(asked)),
        );
  useEffect(() => {
    setPick(0);
    setPickerShut(asked !== null && asked === shut.current);
    if (asked !== shut.current) shut.current = null;
  }, [asked]);
  const pickerOpen = asked !== null && !pickerShut && asked !== shut.current;
  /** A choice writes `@<machine>` and shuts the picker; what comes next
      is the person's — `:路径` for a directory there, a space for the
      next target or the words. */
  const choose = (d: DeviceRow) => {
    if (!typing) return;
    const start = caret - typing[2].length;
    const word = typedName(d);
    setDraft(`${draft.slice(0, start)}${word}${draft.slice(caret)}`);
    const at = start + word.length;
    setCaret(at);
    shut.current = word.toLowerCase();
    requestAnimationFrame(() => box.current?.setSelectionRange(at, at));
  };

  const send = () => {
    const text = draft.trim();
    const files = pending.flatMap((p) => (p.path === null ? [] : [p.path]));
    if ((!text && files.length === 0) || sending || placing) return;
    // What this send carries: a file added while it is on its way stays
    // for the next one.
    const went = new Set(pending.map((p) => p.key));
    setSending(true);
    setRefused(null);
    compose(text, files).then(
      () => {
        setDraft((now) => (now.trim() === text ? "" : now));
        setPending((now) => now.filter((p) => !went.has(p.key)));
        setSending(false);
        stickToEnd.current = true;
        refresh();
      },
      // **Refused, nothing written** (phase 5): the node's sentence names
      // the machine; the draft and its files stay for the next try.
      (e: unknown) => {
        setSending(false);
        setRefused(said(e));
      },
    );
  };
  const ready = (draft.trim() !== "" || pending.some((p) => p.path !== null)) && !placing;

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
        {only && (
          <div data-at-only={only.name} className="flex items-center justify-center gap-in text-caption text-muted-foreground">
            <span>{gui.at_only(only.name)}</span>
            <Button size="sm" variant="ghost" data-at-only-clear onClick={() => setOnly(null)}>
              {gui.at_only_clear}
            </Button>
          </div>
        )}
        {shownMsgs.map((m, i) => (
          <MsgLine
            key={m.id}
            m={m}
            stamp={stampFor(m, i === 0 ? null : shownMsgs[i - 1])}
            machine={landsOn}
            me={machines.find((r) => r.me)?.id ?? ""}
            onOnly={(device, name) => setOnly((now) => (now?.device === device ? null : { device, name }))}
            onChanged={refresh}
          />
        ))}
        </div>
      </div>
      {/* The node's refusal, where the act was started (the pin's rule):
          a target not reachable names the machine, and nothing was sent. */}
      {refused && (
        <div data-post-refused className="flex-none border-t px-3 py-1 text-caption text-state-failed">
          {refused}
        </div>
      )}
      {/* The rule spans the pane, the controls stand in the column — the
          same split as the agent conversation's composer, and for the
          same reason: the line divides two regions, the column is where
          the writing is. **On the narrow face it is the session bar's
          second row** (`components/SessionBar`, 2026-10-02), the agent
          conversation's arrangement. */}
      <BarSeat at="under">
      <div className="flex-none px-row pt-in pb-in">
      <div className="relative mx-auto flex w-full max-w-measure flex-col text-read">
        {/* **The machine picker** (phase 5): over the box, not pushing
            the conversation up — a list that moves the text being read
            is worse than none (the slash menu's rule). Keys ↑↓ walk it,
            Enter or Tab choose, Esc leaves the text as typed; a press
            chooses without taking the focus from the box, so the phone's
            keyboard stays up. */}
        {pickerOpen && (
          <div
            data-at-picker
            role="listbox"
            aria-label={gui.at_pick_title}
            className="absolute bottom-full left-0 mb-1 max-h-56 w-full overflow-y-auto rounded-lg border bg-popover p-1 shadow-(--shadow-float)"
          >
            <div className="px-2 py-1 text-caption text-muted-foreground">{gui.at_pick_title}</div>
            {choices.length === 0 ? (
              <div data-at-none className="px-2 py-1 text-callout text-muted-foreground">
                {gui.at_pick_none}
              </div>
            ) : (
              choices.map((d, n) => (
                <button
                  key={d.id}
                  type="button"
                  role="option"
                  aria-selected={n === pick}
                  data-at-item={d.name}
                  data-on={n === pick}
                  className={cn(
                    "flex min-h-touch w-full items-center gap-2 rounded-sm px-2 text-left text-callout pointer-fine:min-h-ctl-md",
                    n === pick && "bg-fill-strong",
                  )}
                  onPointerDown={(e) => e.preventDefault()}
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => choose(d)}
                >
                  <span className="min-w-0 truncate">@{typedName(d)}</span>
                  {typedName(d) !== d.shown && <span className="truncate text-caption text-muted-foreground">{d.shown}</span>}
                </button>
              ))
            )}
            <div className="px-2 py-1 text-caption text-muted-foreground">{gui.at_pick_hint}</div>
          </div>
        )}
        <PendingChips items={pending} onRemove={(key) => setPending((now) => now.filter((p) => p.key !== key))} />
        {/* **The conversation's capsule** (`Composer`, the redesign,
            2026-10-02): ＋ to add a file, the text, and send once there is
            something to send. No mode and no interrupt — a message has no
            agent on the other end. */}
        <Composer
          lead={
            <Tip label={gui.send_file}>
              <PlusSeat data-attach-file aria-label={gui.send_file} onClick={attach} />
            </Tip>
          }
          trail={
            // Drawn only once there is something to send (progressive
            // disclosure: never a dimmed control).
            // **The keyboard stays up after a note** (P7, 2026-10-03, the
            // owner's ruling): the press does not take the focus from the
            // text, so the next line is typed at once, as in Messages.
            ready ? <SendSeat onSend={send} disabled={sending} keepFocus /> : null
          }
        >
        {/* The browser forms' picker: a real <input type=file>, hidden —
            ＋ is its face. Value cleared after each pick so the same file
            twice still fires onChange. */}
        {!inApp && (
          <input
            ref={fileInput}
            type="file"
            multiple
            hidden
            data-file-input
            onChange={(e) => {
              picked([...(e.target.files ?? [])]);
              e.target.value = "";
            }}
          />
        )}
        <textarea
          ref={box}
          data-chat-composer
          aria-label={gui.message_text}
          aria-expanded={pickerOpen}
          // The same word the agent conversation's capsule wears: one
          // capsule, one invitation (the closing pass, 2026-10-03).
          placeholder={gui.chat_input}
          // Return sends, and the soft keyboard's key says so (Shift+Enter
          // on a hardware keyboard breaks the line).
          enterKeyHint="send"
          // `@machine:path` is not prose: no capital, no correction.
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          value={draft}
          rows={1}
          onChange={(e) => {
            setDraft(e.target.value);
            setCaret(e.target.selectionStart ?? e.target.value.length);
          }}
          onSelect={(e) => setCaret(e.currentTarget.selectionStart ?? 0)}
          onKeyDown={(e) => {
            if (imeKey(e)) return;
            if (pickerOpen && choices.length > 0) {
              // While the picker is open the keys belong to it: Enter
              // chooses rather than sends.
              if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                e.preventDefault();
                const step = e.key === "ArrowDown" ? 1 : choices.length - 1;
                setPick((p) => (p + step) % choices.length);
                return;
              }
              if (e.key === "Enter" || e.key === "Tab") {
                e.preventDefault();
                choose(choices[Math.min(pick, choices.length - 1)]);
                return;
              }
            }
            if (pickerOpen && e.key === "Escape") {
              e.preventDefault();
              setPickerShut(true);
              return;
            }
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

/** How a machine is written in the typed form: its shown name when that
    is one token, its name otherwise — `khor_node::post::typed_name`. */
function typedName(d: DeviceRow): string {
  return d.shown && !/[\s:"]/.test(d.shown) ? d.shown : d.name;
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
  me,
  onOnly,
  onChanged,
}: {
  m: ChatMsgRow;
  stamp: string | null;
  machine: string;
  /** This machine's device id: its own move is the chip's to say. */
  me: string;
  /** A target pressed: the conversation narrows to that machine. */
  onOnly: (device: string, name: string) => void;
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
        {!m.retracted && m.to.length > 0 && <Targets m={m} me={me} onOnly={onOnly} />}
        {m.retracted ? (
          <div className="text-caption text-muted-foreground">{cli.retracted}</div>
        ) : m.text !== null ? (
          // **A word longer than the bubble breaks inside the bubble**
          // (afix-a; the audit's S2: a long URL ran past the bubble and
          // off a phone's screen, its tail unreadable). `anywhere`, so a
          // URL with no break of its own still wraps.
          <div
            data-msg-text
            className={cn(
              "whitespace-pre-wrap [overflow-wrap:anywhere] rounded-lg px-3 py-1.5 text-read-ink",
              m.from_me ? "bg-accent" : "border bg-card",
            )}
          >
            {m.text}
          </div>
        ) : m.files !== null ? (
          <>
            {m.note && (
              <div
                data-msg-note
                className={cn(
                  "mb-1 whitespace-pre-wrap [overflow-wrap:anywhere] rounded-lg px-3 py-1.5 text-read-ink",
                  m.from_me ? "bg-accent" : "border bg-card",
                )}
              >
                {m.note}
              </div>
            )}
            <FileChip m={m} machine={machine} onChanged={onChanged} />
          </>
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
  onChanged,
}: {
  m: ChatMsgRow;
  machine: string;
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
  // An offer waits for this machine's 接收; a file pushed here is on its
  // way by itself (not started yet included); the machine that sent it
  // has nothing to move.
  const sent = m.from_me;
  const waiting = !sent && !m.pushed && !m.retracted && m.step === null;
  const moving = !sent && !m.retracted && (m.step === "moving" || (m.pushed && m.step === null));
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
      {sent && (
        // The sender's seat: its own offer is nothing to receive — no
        // 接收, no failure — only the count beside.
        <span data-bcast-own className="text-caption text-muted-foreground">
          {gui.bcast_own}
        </span>
      )}
      {sent && m.got !== null && m.of !== null && (
        // The sender's chip counts the machines that have it, read off
        // the records the receivers wrote.
        <span data-bcast-count={`${m.got}/${m.of}`} className="text-caption tabular-nums text-muted-foreground">
          {gui.bcast_count(m.got, m.of)}
        </span>
      )}
      {waiting && (
        <div className="flex items-center justify-between gap-2">
          <span data-bcast-waiting className="text-caption text-muted-foreground">
            {gui.bcast_waiting}
          </span>
          <Button size="sm" data-bcast-take disabled={busy} aria-busy={busy || undefined} onClick={() => act(retryTransfer(m.id))}>
            {gui.bcast_take}
          </Button>
        </div>
      )}
      {moving && !m.pushed && (
        <span data-bcast-taking className="text-caption text-muted-foreground">
          {gui.bcast_taking}
        </span>
      )}
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
      {m.step === "landed" && (
        // Pulled from a machine that had it rather than the sender: which.
        <span data-bcast-landed data-bcast-via={m.via ?? undefined} className="text-caption text-muted-foreground">
          {m.via ? gui.bcast_landed_via(m.via) : gui.bcast_landed}
        </span>
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
            {/* A cancelled offer is received again; anything else that
                broke is tried again. */}
            {m.step === "stopped" && !m.pushed ? gui.bcast_take : gui.region_retry}
          </Button>
        </div>
      )}
      {failed !== null && (
        <span data-file-act-failed className="text-caption text-state-failed">
          {failed}
        </span>
      )}
      <FilePreview machine={machine} here shown="" path={looking} onClose={() => setLooking(null)} />
    </div>
  );
}

/**
 * **The machines a message was pushed to** (phase 5), over its bubble:
 * `@名字` by today's name (a renamed machine shows its new name), the
 * directory named there, and on a file, how that machine's move stands —
 * its own record, the same on every device. This machine's own move is
 * the chip's to say (the bar, the path, the sentence), not said twice.
 */
function Targets({ m, me, onOnly }: { m: ChatMsgRow; me: string; onOnly: (device: string, name: string) => void }) {
  return (
    <div data-msg-to className={cn("mb-0.5 flex flex-wrap gap-x-row gap-y-xxs text-caption", m.from_me && "justify-end")}>
      {m.to.map((t) => (
        <span key={t.device} data-msg-target={t.name} data-target-step={t.step ?? undefined} className="flex min-w-0 items-baseline gap-in">
          {/* Pressed, the conversation narrows to this machine (the
              `@机器` filter); pressed again, it opens out. */}
          <button
            type="button"
            data-msg-target-only={t.name}
            className="truncate text-left text-muted-foreground hover:text-foreground"
            onClick={() => onOnly(t.device, t.name)}
          >
            @{t.name}
            {t.path ? `:${t.path}` : ""}
          </button>
          {m.files !== null && t.step !== null && t.device !== me && (
            <span className={cn("flex-none", t.step === "failed" ? "text-state-failed" : "text-muted-foreground")}>
              {t.step === "landed"
                ? gui.at_pushed
                : t.step === "moving"
                  ? gui.at_pushing
                  : t.step === "failed"
                    ? gui.at_push_failed(t.why ?? "")
                    : gui.at_push_stopped}
            </span>
          )}
        </span>
      ))}
    </div>
  );
}
