import { useEffect, useState, type ReactNode } from "react";

import { type SessionRow } from "@/api";
import { StatusWord } from "@/components/StatusWord";
import { SessionFace } from "@/components/SessionFace";
import { EditableName, useSinglePress } from "@/components/EditableName";
import { IconBack, IconChevronDown } from "@/components/icons";
import { NarrowHead } from "@/components/SideHead";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { setDayShown, useDayShown } from "@/hooks/use-day-shown";
import { YIELD_MS } from "@/lib/tempo";
import { cn } from "@/lib/utils";
import { ChatView } from "@/views/ChatView";
import { DAY_ROW_ID, DayStrip, daySwitch, dayHasRecord, type Day } from "@/views/DayStrip";
import { MachineChat } from "@/views/MachineChat";
import { TerminalPane } from "@/views/TerminalPane";

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
  variant?: "ghost" | "default";
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
      「复制好了」 where an id was asked for.

      `fact` is `id` as a value on a contact card (the machine's, from
      2026-09-30): the same twelve characters and the same press, in the
      card's value ink — `body` `ink`, mono, because an id is read one
      character at a time — rather than a quiet `callout` note. */
  face?: "verb" | "id" | "fact";
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
  if (face === "id" || face === "fact") {
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
        className={
          face === "fact"
            ? "flex min-h-touch min-w-0 items-center gap-in rounded-md text-body text-foreground data-[said=failed]:text-state-failed"
            : "flex min-h-touch min-w-0 items-center gap-in rounded-md text-callout text-muted-foreground hover:text-foreground data-[said=failed]:text-state-failed"
        }
      >
        <span data-card-id className={face === "fact" ? "truncate font-mono" : "truncate"}>
          {id.slice(0, 12)}
        </span>
        {said && <span className="flex-none text-callout">{said === "copied" ? gui.copied : gui.copy_failed}</span>}
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
export function closeWarning(
  kind: string,
  imported = false,
  host: { hosted?: boolean; mux?: boolean } = {},
): string {
  // An imported row leaves the list and nothing else (docs/SHELL.md
  // 第八轮 第 3 条) — the window says so, whatever its kind.
  if (imported) return gui.close_warn_imported;
  if (kind === "chat") return gui.close_warn_chat;
  if (kind === "transfer") return gui.close_warn_transfer;
  // An ACP conversation outlives the row: it is the agent's, and 导入
  // finds it again (docs/SHELL.md 第八轮).
  if (kind === "gui") return gui.close_warn_gui;
  // **A row wearing a terminal is asked about the terminal** (2026-10-01,
  // the owner's bug lane). Its kind is the form it wears now (docs/
  // SESSION.md 形态随进程变), and by its host deleting it does opposite
  // things: khor's own pty closes with what runs in it, while the
  // person's tmux session is never ended by khor — only khor's terminal
  // on it goes. Both were asked the agent's sentence below.
  if (kind === "shell" && host.mux) return gui.close_warn_tmux;
  if (kind === "shell" && host.hosted) return gui.close_warn_terminal;
  if (kind === "shell" || kind === "tui" || kind === "borrow") {
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
  day,
  onNew,
  empty,
  tabs,
}: {
  row: SessionRow | null;
  narrow: boolean;
  onBack: () => void;
  /** This session's last day (`DayStrip`), drawn as a band under the
      tabs — the one place it is shown (2026-09-02:「session的状态bar不要
      在session列表里显示…在session打开后的上方显示」), and only once it
      is asked for (2026-09-29, `use-day-shown`). Fetched by the shell for
      every row on one beat and handed in, so the band costs the pane
      nothing of its own: `null` while that beat has not answered yet.
      **Absent where nobody fetches days** — a workspace's session column
      — and then there is no band and no switch for one. */
  day?: Day;
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
}) {
  const shown = useDayShown();
  /** The day's switch, where this pane has a day at all: on the wide
      face it is the front tab's (`App`), and on the narrow one the head
      below is that tab's stand-in and wears it. One answer for both. */
  const sw = day === undefined ? null : daySwitch(day, shown, () => setDayShown(!shown));
  /** Whether the band's row stands under the tabs: asked for, and either
      on its way (the skeleton) or a day with a record. Never an empty
      row — that was the blank band (`DayStrip`'s 「Hidden until asked
      for」). */
  const drawn = row !== null && day !== undefined && shown && (day === null || dayHasRecord(day));
  /** A tap on the narrow head's face or name — the name's double-click
      still renames (`useSinglePress`). */
  const pressHead = useSinglePress(() => sw?.toggle?.());
  /** F2 on the narrow head: the name sits inside the head's switch now,
      so it is not a control of its own, and this is its keyboard door
      (`EditableName`'s controlled `open`). */
  const [renaming, setRenaming] = useState(false);
  useEffect(() => setRenaming(false), [row?.id]);
  return (
    // **The pane's own anchor** (第十四轮 第 1 条). Until this round
    // every gate and every smoke step reached the open session through
    // `[data-detail-header]` — a row that exists on one face only now,
    // so「the pane」 needed a name of its own rather than a landmark
    // inside it.
    // `--day-under` is the surface the day floats over, so its fade
    // melts into that and not into a neighbour's. **One value since
    // 2026-09-30** (板是纸): a terminal used to paint the page's own
    // `background` and everything else the board's `card`; the terminal
    // is the paper now like every other session, so the day fades out of
    // the paper wherever it floats. `--day-over` is how far the day reaches down into the
    // session — its row, while it is up, and nothing while it is not —
    // which the session's own top-corner floats (the terminal's find bar
    // and title) step under, so the two never overprint.
    <section
      data-detail-pane
      className={cn(
        "flex h-full min-w-0 flex-col [--day-under:var(--card)]",
        drawn ? "[--day-over:calc(2*var(--gap-xxs)+var(--spacing))]" : "[--day-over:calc(0*var(--spacing))]",
      )}
    >
      {/* **`min-h`, not `h`.** The bar's look is still `--ctl-lg`; what
          changed is that it may no longer clip. Everything in it that can
          be pressed now carries the finger floor (tokens.css `--touch`),
          and a fixed 40 would have drawn a 44 target overflowing a bar
          that says it is 40 — the hit area would be real and the band it
          lights up would be cut off, which is the one thing the floor's
          own rule forbids ("按下的底色要画在扩后的盒上"). So the bar is
          the height of the tallest thing a finger must land on. */}
      {/* ⚠️ **On the wide face there is no header at all** (第十四轮 第
          1 条, 2026-09-24:「session标签页下还有个标题, 不要了, 标签页下
          就是状态bar然后就是内容」). What stood here — the name in an
          editable box, and 「工作区 · 类型」 beside it — was the tab's
          own content said a second time, one row lower: the strip above
          already carries this session's face, its state halo and its
          name, and renaming is done in the tab itself (`TabStrip`'s
          `EditableName`). The user's rule is wider than this pane:
          **no tab anywhere gets a subtitle under it**.

          So the wide order is: the strip, the day's band (once the
          front tab has been pressed for it, 2026-09-29), the session.

          The narrow face has no strip, so it keeps a header — and only
          what the strip would have carried: the way back, the face, the
          name. The facts do not come back with it; they were the half
          the user struck out. */}
      {/* **Seated in the window's first row on the narrow face**
          (`NarrowHead`, 2026-09-28): it was a second bar under L0's
          magnifier. Where there is no narrow shell — a workspace's
          session column on the wide face — it stays a bar in place. */}
      {narrow && (
        <NarrowHead>
        {(seated) => (
        <header
          data-detail-header
          className={seated ? "flex min-w-0 flex-1 items-center gap-2 self-stretch" : "flex min-h-ctl-lg flex-none items-center gap-2 border-b px-3"}
        >
          {/* The back button exists only on the narrow face: there the
              list is genuinely off-screen, so "back" has somewhere to
              go. Wide keeps the list on screen — nothing to go back
              from. **The narrow face's only exit, so it is the one
              control in this app that must not be under the floor** —
              it was an icon-only 30, and there is no second way out of
              a detail screen on a phone. */}
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
          {/* The row's identity assembly, only where the strip is not
              there to carry it: on wide the front tab wears this exact
              face — halo for the state, corner mark for the kind — and
              repeating it here was the header's old crowding
              (2026-09-02:「都和标签内容重复了」). Narrow has no strip,
              so the face stays, or an open session's state would be on
              no screen at all. **At a tab's size**, because it is the
              tab's face standing in for the strip: the row's 32 and its
              halo made 40 in a 32-tall header and stood out of it at both
              edges (2026-09-28 audit 第 6 条). */}
          {/* **The face and the name are the front tab's stand-in, and so
              its switch** (2026-09-29): the wide face shows and hides the
              day from the tab in front, and the narrow face has no tabs
              — the head is where that tab would be. One button round
              both, the tab's own arrangement (the name `nested` inside
              the control that opens, a double-click on it still renames,
              F2 as its keyboard door). **Pressable the way a title menu
              on a phone is**: a small chevron after the name, turned up
              while the day is open — no fill, no border, nothing else
              on a head that is otherwise just a name. No chevron, and
              `aria-disabled`, on a day with no record: there is nothing
              to open. Where nobody fetches days (a workspace's session
              column) the head is the plain name it always was. */}
          {row && sw ? (
            <button
              type="button"
              data-day-toggle
              aria-expanded={sw.expanded}
              aria-controls={sw.expanded ? DAY_ROW_ID : undefined}
              aria-disabled={sw.toggle ? undefined : true}
              aria-keyshortcuts="F2"
              onClick={pressHead}
              onKeyDown={(e) => {
                if (e.key !== "F2" || e.target !== e.currentTarget) return;
                e.preventDefault();
                setRenaming(true);
              }}
              // `min-w-touch` is also what lets the name truncate: a flex
              // item's floor is its content until something lowers it.
              className="flex min-h-touch min-w-touch items-center gap-2 rounded-sm text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              <SessionFace face={row.face} word={row.word} kind={row.kind} category={row.category} size="tab" />
              <EditableName
                key={row.id}
                id={row.id}
                title={row.title}
                nested
                open={renaming || undefined}
                onClose={() => setRenaming(false)}
                className={seated ? "min-w-0 text-title-sm font-title" : "min-w-0 font-title"}
                inputClassName={seated ? "flex-1 text-title-sm font-title" : "flex-1 font-title"}
              />
              {sw.toggle && (
                <IconChevronDown
                  className={cn("size-3 flex-none text-tertiary", sw.expanded && "rotate-180")}
                />
              )}
            </button>
          ) : (
            <>
              {row && (
                <SessionFace face={row.face} word={row.word} kind={row.kind} category={row.category} size="tab" />
              )}
              {/* `min-w-0` (inside EditableName) is what lets it actually
                  truncate: a flex child will not shrink below its content
                  without it. Keyed by the row so switching sessions drops
                  any half-typed edit — a draft belongs to the row it was
                  typed over. */}
              {row ? (
                <EditableName
                  key={row.id}
                  id={row.id}
                  title={row.title}
                  className={seated ? "min-w-0 text-title-sm font-title" : "min-w-0 font-title"}
                  inputClassName={seated ? "flex-1 text-title-sm font-title" : "flex-1 font-title"}
                />
              ) : (
                <span className="min-w-0 truncate font-title" />
              )}
            </>
          )}
          <span className="min-w-0 flex-1" />
          {tabs}
        </header>
        )}
        </NarrowHead>
      )}
      {/* The session's day, as a band the width of the pane, and on the
          wide face **the first thing under the tab strip** — 「标签页下
          就是状态bar然后就是内容」. It is a fact about *this* session
          and reads beside its name and word. The width of the pane, in
          **a row of its own geometry** (`DayStrip`: the 4 px band with
          `gap-xxs` over and under it): the stretches' hit boxes fill that
          row and nothing past it. The row is also what `use-day-strips`
          measures the axis by. Absent with no row — the empty state has
          no day.

          **Absent, too, until it is asked for** (2026-09-29,
          `use-day-shown`: the front tab, or the narrow head, is the
          switch), **and for a day with no record**. While the day is on
          its way the row holds a skeleton of the band (`DayStrip`).

          **It floats over the session and takes none of its room**
          (2026-09-29, the owner: 「下方多了一个空间…挤占了session的内容
          呀。悬浮内容悬浮在session内容上即可」). Laid out as a row, the
          day pushed the session down 12 px every time it was asked for —
          and under a terminal a push is a resize, so tmux reflowed the
          screen under the person's eyes, once to open and once to close.
          So the row hangs from **a seat of no height**, which is where a
          row would have stood (under the tabs on the wide face, under the
          head on the narrow one), and the session below does not move by
          a pixel. Over the session it fades out of `--day-under`, the
          surface beneath it — the edge a toolbar gives the content that
          scrolls under it — so the band reads over a terminal's first
          line without a hard cut. Only the stretches take a press: a
          gap between them, and the fade, are the session's. */}
      {drawn && (
        <div className="relative z-30 h-0 flex-none">
          <div
            data-detail-day
            id={DAY_ROW_ID}
            className="pointer-events-none absolute inset-x-0 top-0 bg-linear-to-b from-(--day-under) from-65% to-transparent"
          >
            <DayStrip runs={day ?? null} />
          </div>
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
        <div className="grid flex-1 place-items-center text-center text-callout text-muted-foreground">
          {row ? (
            // The same two tiers the list row draws, on the pane that
            // has nothing but them: the word is what this pane exists to
            // report (`fact`), the kind qualifies it (`aux`). Flat, they
            // read as two lines of equal news about a session nobody can
            // see.
            <div>
              <StatusWord word={row.word} />
              <div className="text-callout">{row.kind}</div>
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
                  {gui.new_session_ask}
                </Button>
              )}
            </div>
          )}
        </div>
      )}
    </section>
  );
}
