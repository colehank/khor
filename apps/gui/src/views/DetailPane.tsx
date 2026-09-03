import { useEffect, useState, type ReactNode } from "react";

import {
  type DayRun,
  type SessionRow,
} from "@/api";
import { SessionFace } from "@/components/SessionFace";
import { SessionTitle } from "@/components/SessionTitle";
import { IconBack } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { YIELD_MS } from "@/lib/tempo";
import { ChatView } from "@/views/ChatView";
import { DayStrip } from "@/views/DayStrip";
import { MachineChat } from "@/views/MachineChat";
import { TerminalPane } from "@/views/TerminalPane";
import { groupLabel, word } from "@/words";

/**
 * **One session, one face.** A session is held in exactly one form —
 * a terminal khor owns, or a conversation khor speaks through — and
 * this pane shows that form. There is no switch, because there was
 * never anything to switch between: the pair of tabs this replaces had
 * a 终端 face that, on a conversation, was a card reading "there is no
 * terminal here, press this to make one". **That is a conversion
 * wearing a view's clothes**, and it cost the pane a control strip on
 * every session to offer an act that changes what the session *is*.
 *
 * The conversion itself did not go away — it moved to the row's own
 * menu (`SessionsList`, 转成对话 / 转成终端), beside the other things
 * that change a session rather than look at one. The form is also
 * chosen when a session is made (`NewSessionDialog`), which is the
 * other half of the same answer: **a face khor picked for you is the
 * face its host is actually running.**
 *
 * A recorded conversation with no host here stays read-only — speaking
 * into a TUI from a chat box means injecting bytes into whatever state
 * its menus are in.
 */

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
export function CopyId({
  id,
  variant = "ghost",
  face = "verb",
}: {
  id: string;
  variant?: "ghost" | "outline";
  /** **What the control looks like, never what it does.**
      `verb` is a button in a row of buttons and reads 复制 id.
      `id` is the id itself, standing where a machine's name needs it
      anyway, and copying is what pressing it does (2026-09-03:「复制 id
      就是点小字 id 自动复制」) — a separate button for a string already
      on screen was one control too many.

      The prefix keeps its own element so that what it says never
      changes: the report of the press rides beside it, because smoke
      reads that prefix immediately after clicking to copy, and a
      control that reports by *replacing* the fact would hand back
      「复制好了」 where an id was asked for. */
  face?: "verb" | "id";
}) {
  const [said, setSaid] = useState<"copied" | "failed" | null>(null);
  useEffect(() => {
    if (!said) return;
    const t = window.setTimeout(() => setSaid(null), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [said]);
  const copy = () =>
    navigator.clipboard.writeText(id).then(
      () => setSaid("copied"),
      () => setSaid("failed"),
    );
  if (face === "id") {
    return (
      <button
        type="button"
        data-copy-id
        data-said={said ?? undefined}
        // The word, for anyone who cannot see that this is an id: the
        // visible text is twelve characters of hex, which reads out as
        // nothing at all.
        aria-label={gui.copy_id}
        onClick={copy}
        // The finger floor again (tokens.css `--touch`), on a target
        // whose ink is twelve small characters — the one on this card
        // most likely to be missed.
        className="flex min-h-touch min-w-0 items-center gap-in rounded-md text-sm text-muted-foreground hover:text-foreground data-[said=failed]:text-state-failed"
      >
        <span data-card-id className="truncate">
          {id.slice(0, 12)}
        </span>
        {said && <span className="flex-none text-aux">{said === "copied" ? gui.copied : gui.copy_failed}</span>}
      </button>
    );
  }
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
      onClick={copy}
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
export function closeWarning(kind: string): string {
  if (kind === "chat") return gui.close_warn_chat;
  if (kind === "transfer") return gui.close_warn_transfer;
  if (kind === "shell" || kind === "tui" || kind === "gui" || kind === "borrow") {
    return gui.close_warn_live;
  }
  return gui.close_warn;
}

// (The header's 关闭 — its confirm strip, and the refusal line under
// it — left on 2026-09-02 with the header diet:「标签本来就自带关闭」.
// What a tab's × closes is the *view*; ending the session lives in the
// row's menu (`SessionsList`), whose confirm reads `closeWarning`
// above and whose refusal lands as the item's own note — one act, one
// place, same sentences.)

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
  days,
  onNew,
  empty,
  tabs,
  placeWord,
}: {
  row: SessionRow | null;
  narrow: boolean;
  onBack: () => void;
  /** This session's last day (`DayStrip`), drawn as a band under the
      header — the one place it is now shown (2026-09-02:「session的状态
      bar不要在session列表里显示…在session打开后的上方显示」). Fetched by
      the shell for every row on one beat and handed in, so the band
      costs the pane nothing of its own. */
  days?: DayRun[];
  /** Opens the new-session dialog — the same act the pane bar's "+"
      offers. Handed in rather than owned: the dialog is `App`'s, and
      this pane only points at it from its empty state. */
  onNew?: () => void;
  /** What stands where nothing is picked (the network's now, `MeshNow`)
      — handed in because it is painted from `App`'s own polled state,
      and this pane must not grow a second copy of those polls. Absent,
      the plain sentence-and-button empty state stands. */
  empty?: ReactNode;
  /** The way to the other open tabs, on the face that has no strip —
      the narrow shell hands in a `TabCount` here (Chrome's own answer:
      the number of tabs is the button). At the end of the header, where
      a browser keeps it; absent on wide, where the strip above this
      pane already shows every tab. */
  tabs?: ReactNode;
  /** The row's workspace, in the words the list's chips use — resolved
      by the shell (`placeLabels` needs every row) and only printed
      here. Null for a row with no place. */
  placeWord?: string | null;
}) {
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
        {/* The row's identity assembly, only where the strip is not
            there to carry it: on wide the front tab wears this exact
            face — halo for the state, corner mark for the kind — a few
            pixels up, and repeating it here was the header's old
            crowding (2026-09-02:「都和标签内容重复了」— the state word,
            the machine face and name, 复制 id and 关闭 all left
            together; the copy and the close live in the row's menu
            now). Narrow has no strip, so the face stays, or an open
            session's state would be on no screen at all. */}
        {narrow && row && (
          <SessionFace face={row.face} word={row.word} kind={row.kind} category={row.category} />
        )}
        {/* `min-w-0` (inside SessionTitle) is what lets it actually
            truncate: a flex child will not shrink below its content
            without it, so on the narrow face the title would push the
            facts off the end of the header instead of clipping itself.
            Keyed by the row so switching sessions drops any half-typed
            edit — a draft belongs to the row it was typed over. */}
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
        {/* What the title is *of* — the workspace and the kind, which
            the user named as the header's whole remaining content
            (「标题应该就是session名,所属工作区,session类型」). Both in
            the list's own words: the place label the shell resolved
            (`placeLabels`, handed in as `placeWord`) and the
            category's group word — so a heading here and a chip there
            cannot spell one fact two ways. */}
        {row && (
          <span data-detail-facts className="min-w-0 shrink truncate text-sm text-muted-foreground">
            {[placeWord, groupLabel(`cat:${row.category ?? ""}`)].filter(Boolean).join(" · ")}
          </span>
        )}
        <span className="min-w-0 flex-1" />
        {tabs}
      </header>
      {/* The session's day, as a band the width of the pane, directly
          under the header: it is a fact about *this* session and reads
          beside its name and word. Full-bleed and in its own box, so
          the runs' reach-up hit boxes (`DayStrip`) land on the header's
          bottom edge rather than on nothing. Absent with no row — the
          empty state has no day. */}
      {row && (
        <div data-detail-day className="relative h-1 flex-none">
          <DayStrip runs={days} />
        </div>
      )}
      {row && row.kind === "chat" ? (
        // A machine's conversation — the stream `khor log` prints,
        // with the composer `tell` fills. Keyed so switching machines
        // drops the draft and the scroll with the conversation they
        // belonged to.
        <MachineChat key={row.id} id={row.id} />
      ) : row && row.mux ? (
        // **The face follows the host** (2026-09-01: 「什么宿主跑的就用
        // 什么显示」). The process sits in the person's own tmux, so
        // the terminal is where they are — even for a `gui` row, which
        // is what a conversation khor started becomes once the person
        // resumes it in a terminal (`claude -r`): one session, two
        // hosts, and the one they are typing into wins. The row menu's
        // 转为 GUI 显示 is the other way round, on purpose.
        <TerminalPane key={row.id} id={row.id} />
      ) : row && row.kind === "gui" ? (
        // A conversation khor holds, and the only thing it has: there
        // is no terminal under it to show. Keyed so switching sessions
        // remounts the chat — its cursor, frames and attachment all
        // belong to one conversation.
        <ChatView key={row.id} id={row.id} />
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
