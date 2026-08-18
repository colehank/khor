// The conversation face of a GUI session: history above, the live
// stream below, one input. The frames come through the chat registry
// (`khor_gui_core::chat`) by polling — the cursor means nothing is lost
// between polls, so the cadence only shapes latency, not truth.
//
// What gets painted is this face's judgment (the host relays, it does
// not curate — `gui_host` module head): message and thought chunks
// merge into running text, tool calls take a line each, everything
// else in the protocol is left unpainted this batch. Two deliberate
// judgments to know about:
//
// - **The user's own words are painted first-hand.** Saying happens
//   here, so the bubble goes up when the op goes out; the protocol's
//   *live* echo of user text is skipped (painting both would double
//   every message), while *replayed* user text is painted — in history
//   this face said nothing, the record is all there is. The cost: a
//   line said from another attached face shows up only via replay.
// - **Tool lines never change.** `tool_call_update` is not painted, so
//   a line shows the call's title and no status — no status is honest
//   where a stale "running" would be a lie shaped like a fact.
// - **The agent's text is markdown, the user's is verbatim.** Agents
//   write markdown and their TUI renders it, so a chat face that showed
//   the source would be the worse reading of the same words
//   (`components/Markdown`). The user's own bubble is not rendered: they
//   typed those characters, and an asterisk they meant as an asterisk
//   must not come back as emphasis.
import { takeover as takeoverCall } from "@/api";
import { useEffect, useRef, useState } from "react";

import {
  chatAnswer,
  chatLeave,
  chatOpen,
  chatPoll,
  chatReplay,
  chatSay,
  chatStop,
  fetchHistory,
  type ChatFrame,
} from "@/api";
import { Markdown } from "@/components/Markdown";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { word } from "@/words";

/** One painted item, folded from the protocol's chunks. */
type Item =
  | { who: "user"; text: string }
  | { who: "agent"; text: string }
  | { who: "thought"; text: string }
  | { who: "tool"; title: string }
  | {
      who: "ask";
      ask: number;
      title: string;
      options: [string, string][];
      /** The option id this ask was answered with, once the host says
          so. `null` while it is still open, `""` for a refusal — the
          three states are three pictures, and "answered with nothing"
          is not the same as "still asking". */
      answer: string | null;
    }
  | { who: "sys"; text: string }
  | { who: "seal" };

/** A line said from this face, pinned to where the live stream stood. */
type Said = { afterLive: number; text: string };

type Update = {
  sessionUpdate?: string;
  content?: { type?: string; text?: string };
  title?: string | null;
};

function updateOf(frame: ChatFrame): Update | null {
  if (frame.kind !== "note" && frame.kind !== "history") return null;
  const carried = frame.update;
  if (typeof carried !== "object" || carried === null) return null;
  const inner = (carried as { update?: unknown }).update;
  return typeof inner === "object" && inner !== null ? (inner as Update) : null;
}

function textOf(u: Update): string {
  return u.content?.type === "text" ? (u.content.text ?? "") : "";
}

function fold(frames: ChatFrame[], says: Said[]): Exclude<Item, { who: "seal" }>[] {
  const out: Item[] = [];
  const chunk = (who: "user" | "agent" | "thought", text: string) => {
    if (!text) return;
    const last = out[out.length - 1];
    if (last && last.who === who && "text" in last) last.text += text;
    else out.push({ who, text });
  };
  const eat = (frame: ChatFrame, paintUser: boolean) => {
    if (frame.kind === "ask") {
      out.push({
        who: "ask",
        ask: frame.ask,
        title: frame.title,
        options: frame.options,
        answer: null,
      });
      return;
    }
    if (frame.kind === "answered") {
      // The answer lands on the ask it settles, wherever that sits —
      // the host may have re-sent the ask to this face long after it
      // was raised (`gui_host` — a face that arrives mid-ask).
      const seat = out.find((i) => i.who === "ask" && i.ask === frame.ask);
      if (seat && seat.who === "ask") seat.answer = frame.option ?? "";
      return;
    }
    if (frame.kind === "turn") {
      // 中断 gets a line; the calm endings are the list row's to say.
      if (frame.stop !== "EndTurn" && frame.stop !== "Cancelled") {
        out.push({ who: "sys", text: word("errored") });
      }
      out.push({ who: "seal" });
      return;
    }
    const u = updateOf(frame);
    if (!u) return;
    switch (u.sessionUpdate) {
      case "agent_message_chunk":
        chunk("agent", textOf(u));
        break;
      case "agent_thought_chunk":
        chunk("thought", textOf(u));
        break;
      case "user_message_chunk":
        if (paintUser) chunk("user", textOf(u));
        break;
      case "tool_call":
        out.push({ who: "tool", title: u.title ?? "" });
        break;
      // tool_call_update, plan, and the rest: unpainted (module head).
    }
  };

  // History is a state, not a stream: replayed twice (a reconnect, a
  // second asker on this attachment) it must replace itself, never
  // stack. Only the last *complete* replay paints — one still arriving
  // keeps the previous picture until its end marker lands.
  const past: ChatFrame[][] = [[]];
  for (const f of frames) {
    if (f.kind === "history") past[past.length - 1].push(f);
    else if (f.kind === "history_end") past.push([]);
  }
  const whole = past.length > 1 ? past[past.length - 2] : [];
  for (const f of whole) eat(f, true);
  out.push({ who: "seal" });
  const live = frames.filter((f) => f.kind !== "history" && f.kind !== "history_end");
  let at = 0;
  for (const f of live) {
    for (const s of says) if (s.afterLive === at) out.push({ who: "user", text: s.text });
    eat(f, false);
    at += 1;
  }
  for (const s of says) if (s.afterLive >= at) out.push({ who: "user", text: s.text });
  return out.filter((i): i is Exclude<Item, { who: "seal" }> => i.who !== "seal");
}

/**
 * `still` paints a conversation khor is not holding: a discovered
 * session's recorded past, fetched once as replay-shaped frames — same
 * fold, same painting, no attachment, no polling, and no input, since
 * there is no protocol channel to say anything into.
 */
export function ChatView({
  id,
  still = false,
  takeover,
}: {
  id: string;
  still?: boolean;
  /** Offered on a read-only face whose session could speak from here
      (批C 接管): "busy" warns the confirm that a turn will be cut. */
  takeover?: "busy" | "calm";
}) {
  const [frames, setFrames] = useState<ChatFrame[]>([]);
  const [says, setSays] = useState<Said[]>([]);
  const [gone, setGone] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [taking, setTaking] = useState(false);
  const [takeErr, setTakeErr] = useState<string | null>(null);
  const [inTurn, setInTurn] = useState(false);
  const [text, setText] = useState("");
  // Asks this face has answered: the buttons go, the line stays — a
  // record that permission was asked outlives the asking. Local state
  // on purpose: the host sends no receipt for an answer, and the turn
  // moving on is the only confirmation the protocol has.
  const [answered, setAnswered] = useState<Set<number>>(new Set());
  const scroller = useRef<HTMLDivElement>(null);
  const nearBottom = useRef(true);
  const liveCount = useRef(0);

  useEffect(() => {
    let stopped = false;
    let cursor = 0;
    if (still) {
      fetchHistory(id)
        .then((frames) => {
          if (!stopped) setFrames(frames);
        })
        .catch(() => {
          if (!stopped) setGone(true);
        });
      return () => {
        stopped = true;
      };
    }
    const tick = () =>
      chatPoll(id, cursor)
        .then((batch) => {
          if (stopped) return;
          cursor = batch.next;
          if (batch.frames.length) {
            liveCount.current += batch.frames.filter(
              (f) => f.kind !== "history" && f.kind !== "history_end",
            ).length;
            if (batch.frames.some((f) => f.kind === "turn" || f.kind === "gone")) {
              setInTurn(false);
            }
            // A turn that was already running when this face attached.
            // Without it, a second face opens on an inviting empty box
            // and what it types is dropped (one turn at a time is the
            // host's rule — `gui_host` `GuiNote::Turning`).
            if (batch.frames.some((f) => f.kind === "turning")) setInTurn(true);
            setFrames((prev) => prev.concat(batch.frames));
          }
          if (batch.gone) setGone(true);
        })
        .catch(() => {
          // A poll that misses answers on the next tick; a chat that
          // never opened is `gone` below.
        });
    // An unreachable host is an over conversation, not an error of this
    // pane's own: the host holds the agent, so no host means no one to
    // talk to (`gui_host` — the host file's pids are one process).
    chatOpen(id)
      .then((fresh) => {
        // Only a fresh attachment asks for history: the host answers
        // every replay it is asked for, and a second mount of a living
        // chat (dev double-mount) already holds the replayed frames.
        if (fresh) chatReplay(id).catch(() => {});
        tick();
      })
      .catch(() => {
        if (!stopped) setGone(true);
      });
    const timer = setInterval(tick, 400);
    return () => {
      stopped = true;
      clearInterval(timer);
      chatLeave(id).catch(() => {});
    };
  }, [id, still]);

  // Follow the stream only while the reader is at the tail — new frames
  // must not yank someone who scrolled up into the past.
  useEffect(() => {
    const el = scroller.current;
    if (el && nearBottom.current) el.scrollTop = el.scrollHeight;
  }, [frames, says]);

  const say = () => {
    const line = text.trim();
    if (!line || inTurn || gone) return;
    setText("");
    setInTurn(true);
    setSays((prev) => [...prev, { afterLive: liveCount.current, text: line }]);
    chatSay(id, line).catch(() => setInTurn(false));
  };

  const items = fold(frames, says);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div
        ref={scroller}
        onScroll={(e) => {
          const el = e.currentTarget;
          nearBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
        }}
        className="min-h-0 flex-1 space-y-2 overflow-y-auto px-4 py-3"
      >
        {items.map((item, i) =>
          item.who === "user" ? (
            <div key={i} className="flex justify-end">
              <div
                data-said
                className="max-w-[85%] whitespace-pre-wrap rounded-lg bg-muted px-3 py-1.5 text-sm"
              >
                {item.text}
              </div>
            </div>
          ) : item.who === "agent" ? (
            <Markdown key={i} text={item.text} className="text-sm" />
          ) : item.who === "thought" ? (
            <Markdown key={i} text={item.text} className="text-sm text-muted-foreground" />
          ) : item.who === "tool" ? (
            <div key={i} className="border-l-2 pl-2 text-xs text-muted-foreground">
              {item.title}
            </div>
          ) : item.who === "ask" ? (
            <div
              key={i}
              data-chat-ask
              className="flex flex-col gap-1.5 border-l-2 pl-2 text-xs"
              style={{ borderColor: "var(--state-blocked)" }}
            >
              <span style={{ color: "var(--state-blocked)" }}>
                {word("blocked")}
                {item.title ? ` · ${item.title}` : ""}
              </span>
              {/* Once answered, the decision replaces the buttons: the
                  line stays as the record that permission was asked,
                  and it says what was decided in the agent's own word
                  for it. `item.answer` is the host's fact and outlives
                  this mount; `answered` is this face's own click,
                  painted at once so the buttons do not sit there for a
                  poll's length after being pressed. */}
              {item.answer !== null && (
                <span data-ask-answer className="text-muted-foreground">
                  {item.options.find(([oid]) => oid === item.answer)?.[1] ?? ""}
                </span>
              )}
              {/* The options are the agent's own, in its order and its
                  words — including its refusals, so no extra dismiss
                  control exists to answer differently than offered. */}
              {!still && !gone && item.answer === null && !answered.has(item.ask) && (
                <span className="flex gap-1.5">
                  {item.options.map(([oid, label]) => (
                    <Button
                      key={oid}
                      size="sm"
                      variant="outline"
                      data-ask-option={oid}
                      onClick={() => {
                        setAnswered((prev) => new Set(prev).add(item.ask));
                        chatAnswer(id, item.ask, oid).catch(() => {});
                      }}
                    >
                      {label}
                    </Button>
                  ))}
                </span>
              )}
            </div>
          ) : (
            <div key={i} className="text-xs" style={{ color: "var(--state-errored)" }}>
              {item.text}
            </div>
          ),
        )}
      </div>
      {/* A recorded past has no channel to speak into: no input, and
          the only thing its footer can say is that the record is
          missing. */}
      {still ? (
        // The read-only face says so where the input would be — without
        // this line, "no input" reads as a bug, not as the 接管 ruling
        // (a live TUI is the one mouth). Where a takeover is possible it
        // is offered right here, with a second look before the cut: the
        // click ends a process somewhere else, and mid-turn it cuts a
        // turn — the confirm's words say which (词按行的状态分).
        <div className="flex-none border-t p-2">
          {takeover && confirming ? (
            <div className="flex items-center gap-2 px-2 py-1.5 text-sm">
              <span data-takeover-warn className="min-w-0 flex-1 text-muted-foreground">
                {takeover === "busy" ? gui.takeover_busy : gui.takeover_calm}
              </span>
              <Button
                size="sm"
                data-takeover-go
                disabled={taking}
                onClick={() => {
                  setTaking(true);
                  setTakeErr(null);
                  takeoverCall(id).catch((e) => {
                    setTakeErr(String(e instanceof Error ? e.message : e));
                    setTaking(false);
                    setConfirming(false);
                  });
                  // On success the row turns gui and the poll swaps this
                  // face for the live one — nothing to do here.
                }}
              >
                {gui.takeover}
              </Button>
              <Button size="sm" variant="ghost" data-takeover-back onClick={() => setConfirming(false)}>
                {gui.back}
              </Button>
            </div>
          ) : (
            <div className="flex items-center gap-2 px-2 py-1.5 text-sm">
              <span data-chat-readonly className="min-w-0 flex-1 text-muted-foreground">
                {gone ? gui.chat_no_record : gui.chat_readonly}
              </span>
              {takeErr && (
                <span data-takeover-error className="text-destructive">
                  {takeErr}
                </span>
              )}
              {takeover && (
                <Button size="sm" variant="secondary" data-takeover onClick={() => setConfirming(true)}>
                  {gui.takeover}
                </Button>
              )}
            </div>
          )}
        </div>
      ) : (
      <div className="flex-none border-t p-2">
        {gone ? (
          <div className="px-2 py-1.5 text-sm text-muted-foreground">{gui.chat_over}</div>
        ) : (
          // While a turn runs the box is closed and the way out sits
          // next to it: a turn that never ends used to leave 关闭 the
          // session as the only exit, which throws the conversation
          // away to get rid of one stuck line (found live — an agent's
          // request to its own API never came back, and the row read
          // 忙碌 for as long as anyone waited).
          <div className="flex items-center gap-2">
            <Input
              data-chat-input
              value={text}
              placeholder={gui.chat_input}
              disabled={inTurn}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.nativeEvent.isComposing) say();
              }}
              className={cn("min-w-0 flex-1", inTurn && "opacity-60")}
            />
            {inTurn && (
              <Button
                size="sm"
                variant="secondary"
                data-chat-stop
                onClick={() => {
                  // The turn's own ending is what clears the box: this
                  // asks, the host answers with a `Turn` frame like any
                  // other ending. Painting it stopped here would be a
                  // guess about somebody else's process.
                  chatStop(id).catch(() => {});
                }}
              >
                {gui.chat_stop}
              </Button>
            )}
          </div>
        )}
      </div>
      )}
    </div>
  );
}
