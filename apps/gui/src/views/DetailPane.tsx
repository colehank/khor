import { useEffect, useState, type ReactNode } from "react";

import {
  closeSession,
  takeoverTerm as takeoverTermCall,
  type SessionRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { SessionTitle } from "@/components/SessionTitle";
import { IconBack } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { YIELD_MS } from "@/lib/tempo";
import { ChatView } from "@/views/ChatView";
import { MachineChat } from "@/views/MachineChat";
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

/** Whether — and how — the read-only chat face offers 接管 (批C): only
    local claude rows that are not already conversations; "busy" makes
    the confirm warn that a turn will be cut. */
function takeoverOffer(row: SessionRow): "busy" | "calm" | undefined {
  if (row.category !== "claude" || row.kind === "gui" || row.source) return undefined;
  return row.word === "busy" ? "busy" : "calm";
}

/**
 * The terminal face of a session khor is holding **as a conversation**:
 * there is no terminal to show, and the way to get one is the 接管
 * ruling read backwards — the conversation ends here and the same
 * session comes back up inside a terminal, under the same vendor uuid.
 *
 * The confirm is the forward takeover's, with its own two sentences:
 * which form is ending is the one thing a person must not have to
 * guess from the button.
 */
function ChatToTerm({ row }: { row: SessionRow }) {
  const [confirming, setConfirming] = useState(false);
  const [taking, setTaking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <div className="grid flex-1 place-items-center p-4 text-center text-sm">
      <div className="flex max-w-sm flex-col items-center gap-2">
        <p data-term-is-a-chat className="text-muted-foreground">
          {gui.term_is_a_chat}
        </p>
        {confirming ? (
          <>
            <p data-takeover-term-warn className="text-muted-foreground">
              {row.word === "busy" ? gui.takeover_term_busy : gui.takeover_term}
            </p>
            <div className="flex gap-2">
              <Button
                size="sm"
                data-takeover-term-go
                className="min-h-touch"
                disabled={taking}
                onClick={() => {
                  setTaking(true);
                  setError(null);
                  takeoverTermCall(row.id).catch((e) => {
                    setError(String(e instanceof Error ? e.message : e));
                    setTaking(false);
                    setConfirming(false);
                  });
                  // On success the row turns tui and the list's poll
                  // swaps this pane for a terminal.
                }}
              >
                {gui.takeover}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                data-takeover-term-back
                className="min-h-touch"
                onClick={() => setConfirming(false)}
              >
                {gui.back}
              </Button>
            </div>
          </>
        ) : (
          <Button
            size="sm"
            variant="secondary"
            data-takeover-term
            className="min-h-touch"
            onClick={() => setConfirming(true)}
          >
            {gui.takeover}
          </Button>
        )}
        {error && (
          <span data-takeover-term-error className="text-destructive">
            {error}
          </span>
        )}
      </div>
    </div>
  );
}

/**
 * The row's id, as a thing to hand to somebody: `khor` prints this
 * string and people paste it between machines, so the pane that shows
 * the session offers it rather than making them read it off a list.
 *
 * The button reports its own outcome, the pin's rule: a clipboard write
 * can be refused (an unfocused document, a webview without permission),
 * and a copy that silently did nothing looks exactly like one that
 * worked — until somebody pastes. The word goes back to the offer after
 * a beat, because "复制好了" is about the press and not about the row.
 */
export function CopyId({ id, variant = "ghost" }: { id: string; variant?: "ghost" | "outline" }) {
  const [said, setSaid] = useState<"copied" | "failed" | null>(null);
  useEffect(() => {
    if (!said) return;
    const t = window.setTimeout(() => setSaid(null), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [said]);
  return (
    <Button
      size="sm"
      // The dress follows the row, not the word (动作排语法批): in the
      // session header every control is quiet text (ghost), on the
      // machine card's verb row every verb wears the outline — one
      // component, two rows, each row speaking one language. What must
      // not differ is the behaviour: the copy, the cadence, the words.
      variant={variant}
      data-copy-id
      data-said={said ?? undefined}
      // `min-h-touch` — the finger floor (tokens.css `--touch`). The
      // `sm` look stays; the box it is pressed in grows to 44, and the
      // ghost wash paints that box.
      className="min-h-touch flex-none data-[said=failed]:text-state-failed"
      onClick={() => {
        navigator.clipboard.writeText(id).then(
          () => setSaid("copied"),
          () => setSaid("failed"),
        );
      }}
    >
      {said === "copied" ? gui.copied : said === "failed" ? gui.copy_failed : gui.copy_id}
    </Button>
  );
}

/**
 * What 关闭 will actually do to this row, in that row's own terms.
 *
 * The three sentences are the three `KindSurface::close` bodies: a live
 * session's process group is killed, a device chat's received files are
 * deleted, a transfer's payloads are deleted **and its row stays**. A
 * single sentence covering all three would be true of each and useful
 * for none — what a person is deciding is what they are about to lose.
 *
 * The default is not a fallback for a mistake. Kinds are an open set by
 * design (`khor_core::kind`: "new kinds must appear without touching
 * this crate"), so a kind with no sentence here is one that arrived
 * later, and the honest thing to say about it is only the part that is
 * true of every close.
 */
function closeWarning(kind: string): string {
  if (kind === "chat") return gui.close_warn_chat;
  if (kind === "transfer") return gui.close_warn_transfer;
  if (kind === "shell" || kind === "tui" || kind === "gui" || kind === "borrow") {
    return gui.close_warn_live;
  }
  return gui.close_warn;
}

/**
 * Ending the session this pane is showing.
 *
 * **The button does not decide whether the close is possible.** A
 * discovered session is not khor's to end, and a remote one routes to
 * the machine it lives on — both are `Node::close`'s judgment, and a
 * refusal comes back as a whole sentence written for a person ("不是
 * khor 起的,关不了它;去它自己的窗口里退"). Guessing the same rule here
 * would be this face re-deriving a backend judgment, and it would go
 * stale the day one of those paths changes. So it asks, and prints what
 * comes back.
 *
 * The confirm takes a strip of its own under the header rather than
 * squeezing into it: the sentence is the point, and a warning truncated
 * to fit beside a title is a warning nobody reads.
 */
function useCloseSession(row: SessionRow | null) {
  const [confirming, setConfirming] = useState(false);
  const [closing, setClosing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The row is the subject: moving to another session must not leave
  // this one's confirm — or its refusal — standing over a row it was
  // never about.
  useEffect(() => {
    setConfirming(false);
    setClosing(false);
    setError(null);
  }, [row?.id]);
  // Two pieces because they sit in two places: the control belongs on
  // the header's line, the sentence needs a line of its own. A hook
  // rather than two components so there is still exactly one piece of
  // state behind them.
  const button = !row ? null : (
    <Button
        size="sm"
        variant="ghost"
        data-close-session
        // The finger floor, same as its neighbour on this line.
        className="min-h-touch flex-none"
        onClick={() => {
          setError(null);
          setConfirming(true);
        }}
    >
      {gui.close}
    </Button>
  );
  const strip = !row ? null : (
    <>
      {confirming && (
        <div
          data-close-confirm
          className="flex flex-none items-center gap-2 border-b px-3 py-1.5 text-sm"
        >
          <span data-close-warn className="min-w-0 flex-1 text-muted-foreground">
            {closeWarning(row.kind)}
          </span>
          <Button
            size="sm"
            data-close-go
            className="min-h-touch"
            disabled={closing}
            onClick={() => {
              setClosing(true);
              closeSession(row.id)
                .then(() => {
                  // Nothing to paint on success: the row leaves the
                  // list on its next poll and this pane goes with it.
                  setConfirming(false);
                })
                .catch((e) => {
                  setError(String(e instanceof Error ? e.message : e));
                  setClosing(false);
                  setConfirming(false);
                });
            }}
          >
            {gui.close}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            data-close-back
            className="min-h-touch"
            onClick={() => setConfirming(false)}
          >
            {gui.back}
          </Button>
        </div>
      )}
      {error && (
        <div data-close-error className="flex-none border-b px-3 py-1.5 text-sm text-destructive">
          {error}
        </div>
      )}
    </>
  );
  return { button, strip };
}

/** The face switch, for a session with two faces to switch between.
    Which two they are depends on the row: a hosted agent has a live
    terminal and a recorded conversation; a conversation khor holds has
    the conversation and the offer to move it into a terminal. */
function Faces({
  id,
  term,
  chat,
}: {
  id: string;
  term: ReactNode;
  chat: ReactNode;
}) {
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
          className="min-h-touch"
          onClick={() => setView("term")}
        >
          {gui.view_terminal}
        </Button>
        <Button
          size="sm"
          variant={view === "chat" ? "secondary" : "ghost"}
          data-view-chat
          data-on={view === "chat"}
          className="min-h-touch"
          onClick={() => setView("chat")}
        >
          {gui.view_chat}
        </Button>
      </div>
      {view === "term" ? term : chat}
    </>
  );
}

function HostedAgentDetail({ row }: { row: SessionRow }) {
  return (
    <Faces
      id={row.id}
      term={<TerminalPane id={row.id} />}
      chat={<ChatView id={row.id} still takeover={takeoverOffer(row)} />}
    />
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
  onNew,
  empty,
}: {
  row: SessionRow | null;
  narrow: boolean;
  onBack: () => void;
  /** Opens the new-session dialog — the same act the pane bar's "+"
      offers. Handed in rather than owned: the dialog is `App`'s, and
      this pane only points at it from its empty state. */
  onNew?: () => void;
  /** What stands where nothing is picked (the network's now, `MeshNow`)
      — handed in because it is painted from `App`'s own polled state,
      and this pane must not grow a second copy of those polls. Absent,
      the plain sentence-and-button empty state stands. */
  empty?: ReactNode;
}) {
  const close = useCloseSession(row);
  return (
    <section className="flex h-full min-w-0 flex-col">
      {/* **`min-h`, not `h`.** The bar's look is still `--ctl-lg`; what
          changed is that it may no longer clip. Everything in it that can
          be pressed now carries the finger floor (tokens.css `--touch`),
          and a fixed 40 would have drawn a 44 target overflowing a bar
          that says it is 40 — the hit area would be real and the band it
          lights up would be cut off, which is the one thing the floor's
          own rule forbids ("按下的底色要画在扩后的盒上"). So the bar is
          the height of the tallest thing a finger must land on. */}
      <header data-detail-header className="flex min-h-ctl-lg flex-none items-center gap-2 border-b px-3">
        {/* The back button exists only on the narrow face: there the
            list is genuinely off-screen, so "back" has somewhere to go.
            Wide keeps the list on screen — nothing to go back from.
            **The narrow face's only exit, so it is the one control in
            this app that must not be under the floor** — it was an
            icon-only 30, and there is no second way out of a detail
            screen on a phone. */}
        {narrow && (
          <Button
            variant="ghost"
            size="icon"
            aria-label={gui.back}
            data-back
            onClick={onBack}
            className="min-h-touch min-w-touch"
          >
            <IconBack />
          </Button>
        )}
        {/* `min-w-0` (inside SessionTitle) is what lets it actually
            truncate: a flex child will not shrink below its content
            without it, so on the narrow face the title would push the
            facts and the two controls off the end of the header instead
            of clipping itself. Keyed by the row so switching sessions
            drops any half-typed edit — a draft belongs to the row it
            was typed over. */}
        {row ? (
          <SessionTitle
            key={row.id}
            id={row.id}
            title={row.title}
            className="min-w-0 font-title"
            inputClassName="flex-1 font-title"
          />
        ) : (
          <span className="min-w-0 truncate font-title" />
        )}
        {row && (
          <>
            {/* The state, in the same clothes the row wears it in — the
                same `data-word` pair, so the colour, the breath and the
                reduced-motion guard are the doctrine's and not a second
                copy of it (app.css). Two places, one fact, one word:
                what must never differ between a list and a detail is
                what they say about the same thing (docs/UX.md). */}
            {/* `text-fact`, the tier the word wears in the list — "a
                reading, a state word, a count" is the tier's own
                definition, and this is the same word about the same
                session. It was `text-sm`, which is the raw size `aux`
                resolves to, so the header said in aux clothes what the
                list is now saying in fact clothes: one fact, two
                sizes. */}
            <span data-word={row.word} className="flex-none text-fact">
              <span data-word-text style={{ color: `var(--state-${row.word})` }}>
                {word(row.word)}
              </span>
            </span>
            {/* Which machine this session is on. **The face is always
                there, the name only on a reported row** — and that is
                one rule, not two: the face is the machine's identity and
                every row carries one, while the *name* is the offline
                axis and only a row another device told us about has it.
                It is exactly what the list shows, which is the point —
                one machine, one face, wherever it appears. */}
            <span data-detail-machine className="flex flex-none items-center gap-1.5">
              <MachineAvatar face={row.face} className="size-kind-mark" />
              {row.source && (
                <span data-detail-device className="truncate text-sm text-muted-foreground">
                  {gui.from_device(row.source.device)}
                </span>
              )}
            </span>
            <span className="min-w-0 flex-1" />
            <CopyId id={row.id} />
            {close.button}
          </>
        )}
      </header>
      {close.strip}
      {row && row.kind === "chat" ? (
        // A machine's conversation — the stream `khor log` prints,
        // with the composer `tell` fills. Keyed so switching machines
        // drops the draft and the scroll with the conversation they
        // belonged to.
        <MachineChat key={row.id} id={row.id} />
      ) : row && row.kind === "gui" ? (
        // Keyed so switching sessions remounts the chat: its cursor,
        // frames and attachment all belong to one conversation. The
        // terminal face is not a terminal — this session has none — it
        // is where the conversation can be moved into one (接管 read
        // backwards).
        <Faces
          key={row.id}
          id={row.id}
          term={<ChatToTerm row={row} />}
          chat={<ChatView id={row.id} />}
        />
      ) : row && row.attachable && row.category === "claude" ? (
        // A hosted claude gets both faces with a switch; keyed so the
        // choice resets with the session it was made about.
        <HostedAgentDetail key={row.id} row={row} />
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
        <ChatView key={row.id} id={row.id} still takeover={takeoverOffer(row)} />
      ) : !row && empty ? (
        // The network's now: the biggest canvas in the app stops
        // reading "pick something" and starts showing what there is
        // to pick for. Handed in by `App` (see the prop).
        empty
      ) : (
        <div className="grid flex-1 place-items-center text-center text-sm text-muted-foreground">
          {row ? (
            // The same two tiers the list row draws, on the pane that
            // has nothing but them: the word is what this pane exists to
            // report (`fact`), the kind qualifies it (`aux`). Flat, they
            // read as two lines of equal news about a session nobody can
            // see.
            <div>
              <div className="text-fact" style={{ color: `var(--state-${row.word})` }}>
                {word(row.word)}
              </div>
              <div className="text-aux">{row.kind}</div>
            </div>
          ) : (
            // Not a dead end: the sentence says what this pane is
            // waiting for, and the button underneath is the other way
            // to satisfy it — the same act the pane bar's "+" offers,
            // where a first-time screen has no session to pick yet.
            <div className="flex flex-col items-center gap-2">
              <div>{gui.pick_a_session}</div>
              {onNew && (
                <Button
                  size="sm"
                  variant="secondary"
                  data-empty-new
                  className="min-h-touch"
                  onClick={onNew}
                >
                  {gui.new_session}
                </Button>
              )}
            </div>
          )}
        </div>
      )}
    </section>
  );
}
