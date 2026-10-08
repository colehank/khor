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
// - **Tool lines carry status now, and the note they replace was right
//   when it was written.** It said: `tool_call_update` is not painted,
//   so a line shows a title and no status, because "no status is honest
//   where a stale *running* would be a lie shaped like a fact." The
//   premise was that nothing ever sent a status — khor's own claude
//   垫片 built every `ToolCall` with the default and never followed it
//   up, so a painted status could only have been invented here. The
//   垫片 now reports both ends (`cagent`: `InProgress` at `tool_use`,
//   `Completed`/`Failed` at the `tool_result` riding a user line), so
//   the status is a fact off the wire and the old note's own reasoning
//   is what says to paint it.
//   **`pending` is still never seen from this agent** and is drawn like
//   `in_progress` if it ever arrives: a third agent may use it, and the
//   two readings differ by "has not started" versus "is running",
//   which is not a difference a reader of this line can act on.
// - **Both sides are markdown now, and the second half of this note is
//   a tombstone.** It read: the agent's text is markdown and the user's
//   is verbatim, because「they typed those characters, and an asterisk
//   they meant as an asterisk must not come back as emphasis」.
//
//   That reasoning is still true and it is still the cost. It was
//   overruled deliberately, by the person reading this app, after two
//   things showed up together: a user line is where a **message from
//   another agent session** arrives (see `utterances_of` — `user` is
//   the CLI's word for「went in through the prompt」, not for「a person
//   wrote this」), and the people writing into this box write markdown.
//   A wall of raw source on one side of a conversation and rendered
//   text on the other reads as a bug, and it was one half of the time.
//
//   **What was traded away, said plainly:** a literal asterisk, or
//   underscore, or leading `#`, typed as itself, now comes back as
//   emphasis or a heading. Nothing warns about it. If that turns out to
//   cost more than the walls of source did, the way back is this one
//   element — the fold and the frames are unchanged, which is why the
//   change is reversible and this note can be short.
import { offerTabAct } from "@/lib/tab-acts";
import { takeHandOff } from "@/lib/composer-handoff";
import { Fragment, useEffect, useMemo, useRef, useState } from "react";

import {
  launchOn,
  chatAnswer,
  chatLeave,
  chatOpen,
  chatPoll,
  chatReplay,
  chatSay,
  chatSetMode,
  chatSetOption,
  chatStop,
  chatWait,
  followHistory,
  inApp,
  type ChatFrame,
} from "@/api";
import { IconBack } from "@/components/icons";
import { Markdown } from "@/components/Markdown";
import { Waiting } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { gui } from "@/gen/catalog";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useKeysTo } from "@/hooks/use-keys-to";
import { useCoarse } from "@/hooks/use-coarse";
import { BarSeat } from "@/components/SessionBar";
import { sessionMark } from "@/components/BrandIcons";
import { FileCard, FilePreview, MachineFilePick, PendingChips, SendInto, usePendingFiles } from "@/views/chat-files";
import { LinkHomeProvider, useKnownFiles, useLinkHome, type LinkHome } from "@/views/linkHome";
import { filesFolderIn, splitAttachments, withAttachments } from "@/lib/paths";
import { AddSeat, Composer, COMPOSER_TEXT, ModeSeat, SendSeat, StopSeat, WhoSeat, useGrowingText, type AddOption, type Who } from "@/views/Composer";
import { cn } from "@/lib/utils";
import { lasting, word } from "@/words";
import { imeKey } from "@/lib/ime";

/**
 * How often to ask for frames, while a turn runs and while none does.
 *
 * **The cadence is the whole of the latency.** The registry's reader
 * thread already holds a frame the instant it arrives and `chat_poll`
 * answers with whatever is there (`khor_gui_core::chat` module head),
 * so nothing is ever lost between polls — the wait only decides how
 * long a frame sits in Rust before a screen asks for it. Measured
 * before this change, at 400ms: median 195–248 ms from the agent
 * emitting a chunk to it being painted, which is half the gap, the
 * signature of arrivals landing uniformly inside it.
 *
 * 50 is not a guess: `TerminalPane` has polled at 50 on both transports
 * for as long as terminals have worked here, and a poll with nothing to
 * say costs a lock and an empty slice.
 *
 * The idle pace is *slower* than the 400 it replaces. A conversation
 * with no turn running produces nothing to see, and the pane that is
 * paying for the fast pace is the one with something happening in it.
 */
const LIVE_MS = 50;
const IDLE_MS = 500;
/** How long one pushed wait is held open with nothing new (`chatWait`):
    in the app, a visible conversation holds one wait at a time instead
    of either beat above, and a frame shows the moment it lands (round
    NET-2, piece 6). */
const WAIT_MS = 1000;
/**
 * How often a *discovered* conversation's transcript is re-read.
 *
 * **Slower than the live pane on purpose, and the reason is what is
 * being watched.** A hosted conversation polls at 50ms during a turn
 * because khor is the thing streaming it and the latency is khor's own.
 * A discovered one is a file somebody else is appending to: the writer
 * flushes in bursts, so a faster read finds the same bytes again and
 * pays for the finding. 700ms is under the eye's tolerance for「it is
 * happening」and an order of magnitude off the wasteful end.
 */
const STILL_MS = 700;

/** One painted item, folded from the protocol's chunks. */
type Item =
  | { who: "user"; text: string }
  | { who: "agent"; text: string }
  | { who: "thought"; text: string }
  | {
      who: "tool";
      /** The protocol's `toolCallId`. Carried because a later
          `tool_call_update` names the call it is about, and matching by
          title would fold two calls to the same tool into one. */
      id: string;
      title: string;
      /** The protocol's `ToolKind`, lowercase off the wire. Unused by
          this face today and carried anyway: it is the field that lets
          tools from *other* agents group by what they do rather than by
          what their author called them, which is what the next batch
          needs and what a title can never give. */
      kind: string;
      /** `pending` | `in_progress` | `completed` | `failed`. */
      status: string;
      /** What the tool was handed, verbatim. The title is a summary and
          summaries lose things; this is the thing itself, and it is
          what a line opens onto. */
      raw: unknown;
      /** Files the call names, for a face that wants to offer them. */
      where: string[];
      /** The diff an edit carries, if it is an edit. */
      diff: { path: string; old: string | null; next: string } | null;
      /** What the tool said when it finished. */
      out: string | null;
    }
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

/**
 * The painting, kept as items rather than as frames.
 *
 * **Nothing here is recomputed from the whole conversation.** Each poll
 * folds only the frames it carried into what is already painted, which
 * is what lets the cadence be fast: refolding thousands of frames is
 * affordable eight times a minute and not twenty times a second.
 *
 * The two halves are not a tidiness: history is a *state* and the live
 * stream is a *stream*. A replay arriving twice (a reconnect, a second
 * asker on this attachment) must replace the past, never stack it —
 * so the past is rebuilt whole from `arriving` when its end marker
 * lands, and `live` is never touched by that. One still arriving keeps
 * the previous picture until then.
 *
 * They are also two arrays rather than one because text merges only
 * within an array: the old fold pushed a `seal` between them to stop a
 * replayed line and a live one running together, and two arrays say
 * the same thing without an item that exists to be filtered out.
 */
type Painting = {
  past: Item[];
  arriving: ChatFrame[];
  live: Item[];
};

const BLANK: Painting = { past: [], arriving: [], live: [] };

/** One `ToolCallContent`, the protocol's three-tag union. */
type ToolContent = {
  type?: string;
  content?: { type?: string; text?: string };
  path?: string;
  oldText?: string | null;
  newText?: string;
};

type Update = {
  sessionUpdate?: string;
  /** **The protocol overloads this key.** On a message chunk it is one
      content block; on a tool call it is a *list* of `ToolCallContent`.
      One field with both shapes, narrowed at each reader, rather than
      two names for one wire key — two names would let a reader take the
      wrong one and get `undefined` rather than a type error. */
  content?: { type?: string; text?: string } | ToolContent[];
  title?: string | null;
  /** `tool_call` and `tool_call_update` both name their call with it. */
  toolCallId?: string;
  kind?: string;
  status?: string;
  /** `tool_call` / `tool_call_update`: what the tool was given and what
      it produced. `content` is the protocol's tagged union — `content`,
      `diff` or `terminal`. */
  rawInput?: unknown;
  locations?: { path?: string; line?: number }[];
  /** `current_mode_update`: which mode the agent is in. */
  currentModeId?: string;
  /** `plan`: the agent's own list of what it is doing. */
  entries?: { content?: string; status?: string; priority?: string }[];
  /** `usage_update`: how much of the window this conversation holds. */
  used?: number;
  size?: number;
  cost?: { amount?: number; currency?: string } | null;
  /** `available_commands_update`'s payload. The spelling is pinned by a
      test on the Rust side (`cagent`'s `wire_tests`) — this face reads
      the protocol's JSON by hand, so a rename there would leave the "/"
      menu quietly empty rather than break anything. */
  availableCommands?: { name?: string; description?: string }[];
};

/** The agent's own command list, if this frame carries one.
    A list, not a merge: the agent republishes the whole set (claude's
    垫片 turns each init frame's `slash_commands` into one of these), so
    the last frame to carry one is the truth and this simply replaces.

    Read per frame rather than by scanning the conversation: the scan
    was a second full pass over every frame ever received, run on every
    render — it cost as much as the fold and answered a question only
    one frame in a thousand changes. */
function commandsIn(frame: ChatFrame): { name: string; description: string }[] | null {
  const u = updateOf(frame);
  if (u?.sessionUpdate !== "available_commands_update") return null;
  return (u.availableCommands ?? [])
    .map((c) => ({ name: c.name ?? "", description: c.description ?? "" }))
    .filter((c) => c.name);
}

/** This conversation's context reading, if this frame carries one.
    Replaces rather than accumulates: `used` is the whole window's
    occupancy at that moment, so the newest frame is simply the truth
    and adding them up would be a number with no meaning. */
function usageIn(frame: ChatFrame): { used: number; size: number; cost: Cost | null } | null {
  const u = updateOf(frame);
  if (u?.sessionUpdate !== "usage_update") return null;
  // A size of zero is not a reading. The 垫片 will not send one (it
  // reports nothing until the agent tells it the window), but a
  // third-party agent might, and dividing by it paints Infinity%.
  if (typeof u.used !== "number" || typeof u.size !== "number" || u.size <= 0) return null;
  const c = u.cost;
  return {
    used: u.used,
    size: u.size,
    cost:
      c && typeof c.amount === "number"
        ? { amount: c.amount, currency: c.currency ?? "USD" }
        : null,
  };
}

/** The agent's plan, if this frame carries one.
    **Replaces, never appends.** The protocol re-sends the whole list
    every time any entry moves, so keeping the newest is the truth and
    accumulating would paint the same three steps five times over.

    **claude sends none of these**, measured 2026-08-30: its tool list
    on this machine has 128 entries and not one of them is todo-shaped
    (`Task`/`TaskOutput`/`TaskStop` are how it runs subagents, not how
    it plans). An earlier note in this repo said the plan would come
    from a `TodoWrite` tool; there is no such tool here. So this reads a
    frame khor's own 垫片 will never send, for the same reason the title
    does. */
function planIn(frame: ChatFrame): PlanEntry[] | null {
  const u = updateOf(frame);
  if (u?.sessionUpdate !== "plan") return null;
  return (u.entries ?? [])
    .map((e) => ({ content: e.content ?? "", status: e.status ?? "pending" }))
    .filter((e) => e.content);
}

type PlanEntry = { content: string; status: string };

/** One of the agent's own modes (`SessionMode`). */
type ModeListed = { id: string; name: string; description?: string | null };
/** One value a select offers (`SessionConfigSelectOption`). */
type ValueListed = { value: string; name: string; description?: string | null };
/** Values under a heading (`SessionConfigSelectGroup`). */
type GroupListed = { group: string; name: string; options: ValueListed[] };
/** One option (`SessionConfigOption`): the protocol tags the kind on
    `type` and flattens the kind's fields beside it, so a select has
    `currentValue: string` and `options`, a switch `currentValue: boolean`. */
type OptionListed = {
  id: string;
  name: string;
  description?: string | null;
  category?: string | null;
  type?: string;
  currentValue?: string | boolean;
  options?: (ValueListed | GroupListed)[];
};
/** What the session can be set to and is set to — `ChatFrame::Settings`,
    which the host sends whole on attach and on every change. */
type Settings = {
  modes: { currentModeId: string; availableModes: ModeListed[] } | null;
  options: OptionListed[];
  /** The model id, when the agent said it (khor's shims, in `_meta`;
      the host holds it — `khor_acp::MODEL_META`). */
  model: string | null;
};

/** The settings this frame carries, if it is one. Read loosely — the
    body is the protocol's JSON, and a field this face does not know is
    left where it is rather than refused. */
function settingsIn(frame: ChatFrame): Settings | null {
  if (frame.kind !== "settings") return null;
  const carried = frame.settings;
  if (typeof carried !== "object" || carried === null) return null;
  const s = carried as { modes?: unknown; options?: unknown; model?: unknown };
  const modes =
    typeof s.modes === "object" && s.modes !== null
      ? (s.modes as { currentModeId?: unknown; availableModes?: unknown })
      : null;
  return {
    modes:
      modes && typeof modes.currentModeId === "string"
        ? {
            currentModeId: modes.currentModeId,
            availableModes: Array.isArray(modes.availableModes)
              ? (modes.availableModes as ModeListed[])
              : [],
          }
        : null,
    options: Array.isArray(s.options) ? (s.options as OptionListed[]) : [],
    model: typeof s.model === "string" && s.model.trim() ? s.model.trim() : null,
  };
}

/**
 * The model a conversation runs on, where it can be known: what the host
 * holds (`settings.model`: codex's shim says it when the thread starts,
 * claude's on its first turn), or else the agent's own `model` option —
 * how an agent off the registry names it, when it does — by the value's
 * id, which is the provider's spelling.
 */
function modelOf(settings: Settings | null): string | null {
  if (!settings) return null;
  if (settings.model) return settings.model;
  const o = settings.options.find((x) => x.category === "model" && typeof x.currentValue === "string");
  return typeof o?.currentValue === "string" && o.currentValue ? o.currentValue : null;
}

/** The agent's name as a person knows it: the vendor's product for the
    three khor knows, else the agent's own name for itself. */
function agentName(category: string | null, said: string | null): string | null {
  switch (category) {
    case "claude":
      return gui.vendor_claude;
    case "codex":
      return gui.accounts_provider_codex;
    case "cursor":
      return gui.vendor_cursor;
    default:
      return said;
  }
}

/** A select's values, groups opened out, each with the heading it sat
    under (`null` for the ungrouped). */
function valuesOf(o: OptionListed): { heading: string | null; value: ValueListed }[] {
  const out: { heading: string | null; value: ValueListed }[] = [];
  for (const item of o.options ?? []) {
    if ("options" in item && Array.isArray(item.options)) {
      for (const value of item.options) out.push({ heading: item.name, value });
    } else if ("value" in item) {
      out.push({ heading: null, value: item });
    }
  }
  return out;
}

/**
 * A mode's name **inside the menu**, where every listed mode needs one
 * to be picked by — so unlike `modeSaid` the ordinary mode has a word
 * here. Known ids get this face's words (claude's four, codex's three —
 * `codexagent::PRESETS`, whose read-only and full access mean what
 * claude's 只读不改 and 全部自动批准 mean to a person); anything else is
 * the agent's own name for it, and its id when it gave none.
 */
function modeWord(id: string, listed: ModeListed[]): string {
  switch (id) {
    case "default":
      return gui.chat_mode_default;
    case "plan":
    case "read-only":
      return gui.chat_mode_plan;
    case "acceptEdits":
      return gui.chat_mode_accept_edits;
    case "bypassPermissions":
    case "full-access":
      return gui.chat_mode_bypass;
    case "workspace":
      return gui.chat_mode_codex_auto;
    case "auto":
      return gui.chat_mode_claude_auto;
    default:
      return listed.find((m) => m.id === id)?.name ?? id;
  }
}

/** The menu's second line: exactly what the mode asks about and what it
    does not — this face's words for the ids it knows, else the agent's
    own description. */
function modeNote(id: string, listed: ModeListed[]): string | null {
  switch (id) {
    case "default":
      return gui.chat_mode_default_note;
    case "plan":
      return gui.chat_mode_plan_note;
    case "acceptEdits":
      return gui.chat_mode_accept_edits_note;
    case "bypassPermissions":
      return gui.chat_mode_bypass_note;
    case "read-only":
      return gui.chat_mode_codex_read_only_note;
    case "workspace":
      return gui.chat_mode_codex_auto_note;
    case "full-access":
      return gui.chat_mode_codex_full_access_note;
    case "auto":
      return gui.chat_mode_claude_auto_note;
    default:
      return listed.find((m) => m.id === id)?.description ?? null;
  }
}

/**
 * What to say about the mode the agent is in — **and `null` for the
 * ordinary one**, which is the whole design.
 *
 * `default` is what nearly every session is, and a marker that is
 * always there is not a marker (docs/UX.md 角标可归零). The three that
 * do show are the ones where a reader's expectation would otherwise be
 * wrong: one that will not touch a file, and two that will touch one
 * without asking. A mode this face has never heard of is shown as the
 * agent's own word rather than hidden — an unknown mode is still not
 * the default, and「something is different here」is the fact.
 */
function modeSaid(id: string): string | null {
  switch (id) {
    case "default":
    case "":
      return null;
    case "plan":
      return gui.chat_mode_plan;
    case "acceptEdits":
      return gui.chat_mode_accept_edits;
    case "bypassPermissions":
      return gui.chat_mode_bypass;
    case "auto":
      return gui.chat_mode_claude_auto;
    default:
      return id;
  }
}

/** The agent's own name for this conversation, if this frame carries
    one. An empty string counts as "cleared": the protocol distinguishes
    a field that was not sent from one sent as null, and this face's
    `null` means "no title", which is what both of those come to here. */
function titleIn(frame: ChatFrame): string | null | undefined {
  const u = updateOf(frame);
  if (u?.sessionUpdate !== "session_info_update") return undefined;
  const t = u.title;
  return typeof t === "string" && t.trim() ? t : null;
}

type Cost = { amount: number; currency: string };

function updateOf(frame: ChatFrame): Update | null {
  if (frame.kind !== "note" && frame.kind !== "history") return null;
  const carried = frame.update;
  if (typeof carried !== "object" || carried === null) return null;
  const inner = (carried as { update?: unknown }).update;
  return typeof inner === "object" && inner !== null ? (inner as Update) : null;
}

function textOf(u: Update): string {
  const c = u.content;
  if (!c || Array.isArray(c)) return "";
  return c.type === "text" ? (c.text ?? "") : "";
}

/** The tool half of the overloaded `content` key. */
function toolContent(u: Update): ToolContent[] {
  return Array.isArray(u.content) ? u.content : [];
}

/**
 * Folds one frame into `out`, in place. Answers whether it painted
 * anything — a `Turning` or a `Gone` carries no picture, and a poll
 * that only brought those must not hand React a new array.
 *
 * Items are replaced rather than edited (`{ ...last, text: … }`): the
 * array is what React is given, and an object mutated inside it is a
 * change React has no way to see.
 */
function eat(out: Item[], frame: ChatFrame, paintUser: boolean): boolean {
  const chunk = (who: "user" | "agent" | "thought", text: string): boolean => {
    if (!text) return false;
    const last = out[out.length - 1];
    if (last && last.who === who && "text" in last) {
      out[out.length - 1] = { ...last, text: last.text + text };
    } else {
      out.push({ who, text });
    }
    return true;
  };
  if (frame.kind === "ask") {
    out.push({
      who: "ask",
      ask: frame.ask,
      title: frame.title,
      options: frame.options,
      answer: null,
    });
    return true;
  }
  if (frame.kind === "answered") {
    // The answer lands on the ask it settles, wherever that sits — the
    // host may have re-sent the ask to this face long after it was
    // raised (`gui_host` — a face that arrives mid-ask). Searching only
    // the live half is not a shortcut: a replay carries no asks, so an
    // ask is never in the past.
    const at = out.findIndex((i) => i.who === "ask" && i.ask === frame.ask);
    const seat = at < 0 ? null : out[at];
    if (seat && seat.who === "ask") {
      out[at] = { ...seat, answer: frame.option ?? "" };
      return true;
    }
    return false;
  }
  if (frame.kind === "refused") {
    // A line of the conversation's own, because that is where the
    // person is looking when it happens — and it says which of the two
    // it was: one means wait, the other means the turn is already over.
    const said =
      frame.why === "turn-in-flight"
        ? gui.chat_refused_turn_in_flight
        : frame.why === "setting-refused"
          ? gui.chat_refused_setting
          : gui.chat_refused_no_turn;
    out.push({ who: "sys", text: said });
    return true;
  }
  if (frame.kind === "turn") {
    // 中断 gets a line; the calm endings are the list row's to say.
    if (frame.stop !== "EndTurn" && frame.stop !== "Cancelled") {
      out.push({ who: "sys", text: word("errored") });
    }
    // The seal stops the next turn's first words running into this
    // turn's last ones. It is filtered out before painting.
    out.push({ who: "seal" });
    return true;
  }
  const u = updateOf(frame);
  if (!u) return false;
  switch (u.sessionUpdate) {
    case "agent_message_chunk":
      return chunk("agent", textOf(u));
    case "agent_thought_chunk":
      return chunk("thought", textOf(u));
    case "user_message_chunk":
      return paintUser ? chunk("user", textOf(u)) : false;
    case "tool_call":
      out.push({
        who: "tool",
        id: u.toolCallId ?? "",
        title: u.title ?? "",
        kind: u.kind ?? "other",
        status: u.status ?? "pending",
        raw: u.rawInput ?? null,
        where: (u.locations ?? []).map((l) => l.path ?? "").filter(Boolean),
        diff: diffIn(u),
        out: contentIn(u),
      });
      return true;
    case "tool_call_update": {
      // **Backwards, and only within this half.** A call and its result
      // land in the same array (both halves are folded whole: history
      // is rebuilt from its own frames, live grows from live ones), and
      // the newest matching line is the one this update is about — an
      // agent that calls the same tool twice gets two ids, but an id
      // reused across a replay boundary must not reach back into the
      // past and edit a line the record already settled.
      const id = u.toolCallId;
      if (!id) return false;
      for (let i = out.length - 1; i >= 0; i--) {
        const at = out[i];
        if (at.who !== "tool" || at.id !== id) continue;
        // Every field is optional on an update frame, and **absent is
        // not empty**: the protocol sends only what changed, so a frame
        // that carries a status and no title must not blank the title.
        out[i] = {
          ...at,
          status: u.status ?? at.status,
          title: u.title ?? at.title,
          kind: u.kind ?? at.kind,
          raw: u.rawInput ?? at.raw,
          diff: diffIn(u) ?? at.diff,
          out: contentIn(u) ?? at.out,
        };
        return true;
      }
      // An update naming a call this face never painted. Dropped rather
      // than pushed as a new line: a status with no call to attach to
      // would appear as a tool that ran with no name.
      return false;
    }
    // **Everything named here was once in this line and is not any
    // more** — plan, usage, session info, the mode and the tool
    // payloads all have readers above or beside this fold. What is
    // still unpainted, named so the next reader does not have to
    // re-derive it from the protocol:
    //
    // - `config_option_update` — a session's own settings changing.
    //   Nothing offers those settings yet, so a notice that one moved
    //   would be about a control this face does not have.
    // - the unstable plan operations (`plan_update`, `plan_removed`),
    //   which are behind a feature flag in the protocol crate and may
    //   change or go.
    //
    // **`ToolCallContent::Terminal` left this list, and not by being
    // painted.** It sat here reading like a missing view — a tool whose
    // output is a live terminal, and khor draws no terminal for it —
    // and read that way the next step is obviously to go paint one.
    //
    // It cannot arrive. `terminal/*` are agent→client requests: the
    // agent asks *khor* to start a process on this machine and is
    // handed a `TerminalId` back, and the v1 schema grants them only to
    // a client that set `terminal: true`. khor sets it false, so no
    // conforming agent has an id to put in a tool call. **What the line
    // described as a gap in this face is a door khor is holding shut**,
    // and the thing worth guarding is the door — which is now a test
    // beside the handshake rather than a sentence here
    // (`khor_acp::what_khor_grants`).
    //
    // A list like this rots, and this one already has twice: it used to
    // name three things that are painted now, and one thing that was
    // never going to happen.
    default:
      return false;
  }
}

/** The diff a tool frame carries, if it carries one.
    The protocol's `content` is a tagged union and `diff` is one of its
    three tags; the others (`content`, `terminal`) are read elsewhere or
    not at all, so this picks rather than assumes. */
function diffIn(u: Update): { path: string; old: string | null; next: string } | null {
  for (const c of toolContent(u)) {
    if (c.type === "diff" && typeof c.newText === "string") {
      return { path: c.path ?? "", old: c.oldText ?? null, next: c.newText };
    }
  }
  return null;
}

/** The text a tool frame carries as plain content. */
function contentIn(u: Update): string | null {
  for (const c of toolContent(u)) {
    if (c.type === "content" && c.content?.type === "text" && c.content.text) {
      return c.content.text;
    }
  }
  return null;
}

/**
 * The mark that says whether something is **running or settled**, on a
 * tool line and on the turn itself.
 *
 * **Three shapes, not three colours.** Running breathes and is filled;
 * settled is a hollow ring; failed is filled in the failure colour. The
 * obvious design — one filled dot in three hues — has a hole this repo
 * has already paid for once: 忙碌's indigo was placed at grey's exact
 * loudness on purpose (`toolInk` has the measurement, ΔL* 1.03), so
 * with the breath off it and a completed mark would be **the same dot**.
 * And the breath is off for anyone who asked for less motion, which is
 * exactly the reader least able to spare a distinction.
 *
 * So the shape carries it and the motion is the second signal, never
 * the only one.
 *
 * **A fixed box, always.** A mark that changes size when the status
 * changes moves the title beside it — the same bug the terminal grid
 * was just rebuilt to kill, one surface over.
 */
function ToolDot({ status }: { status: string }) {
  const live = running(status);
  const failed = status === "failed";
  return (
    <span
      data-tool-dot
      data-dot-state={live ? "running" : failed ? "failed" : "done"}
      // `data-halo` rather than a selector of its own: that is already
      // the breath's name for a *mark* (as opposed to a word), and it
      // already carries the reduced-motion guard (app.css).
      data-halo={live ? "busy" : undefined}
      aria-hidden
      // The ring is the third shape — a hollow dot — and it is a
      // utility rather than an inline `box-shadow` because the width
      // is a pixel, and a pixel written here is a number this app's
      // tokens do not own (`check-px`). `ring-1 ring-inset` compiles to
      // exactly the inset shadow it replaced.
      className={cn(
        "inline-block size-1.5 shrink-0 rounded-full",
        !live && !failed && "ring-1 ring-inset ring-muted-foreground",
      )}
      style={{
        background: live
          ? "var(--state-busy)"
          : failed
            ? "var(--state-failed)"
            : "transparent",
      }}
    />
  );
}

/**
 * How long the running turn has been running, ticking once a second.
 *
 * **Its own component because a clock must not repaint a
 * conversation.** The first version held `now` in `ChatView` and
 * re-rendered the whole item list every second — on top of the live
 * poll, and for a number four characters wide. Here the tick is local:
 * one text node changes, and the stream above it is not touched.
 *
 * The interval exists only while this is mounted, which is only while a
 * turn runs — an idle conversation has no timer at all.
 */
function BusyFor({ since }: { since: number }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    setNow(Date.now());
    const t = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(t);
  }, [since]);
  return (
    <span data-chat-busy-for style={{ color: "var(--state-busy)" }}>
      {gui.chat_busy_for(lasting(Math.max(0, now - since)))}
    </span>
  );
}

/** Whether a tool call is still open.
    `pending` counts: this agent never sends it (`cagent` opens at
    `in_progress`), but a third-party agent may, and of the two possible
    wrong pictures — "running" for something not yet started, versus
    "finished" for something that has not run — only the second one
    tells the reader the turn is further along than it is. */
function running(status: string): boolean {
  return status === "in_progress" || status === "pending";
}

/**
 * The ink a **plan entry** is written in — a different function from
 * `toolInk`, and the difference is the word `pending`.
 *
 * **The same word means opposite things in the two places.** On a tool
 * call `pending` is a state khor's own 垫片 never sends and a
 * third-party one rarely does, and of the two possible wrong pictures —
 * "running" for something not yet started versus "finished" for
 * something still going — only the second misleads about where the turn
 * is; so `toolInk` draws it as running. In a plan `pending` is the
 * normal state of most entries and means exactly "not started", and
 * drawing those like the step being worked erases the one thing a plan
 * is read for: **which line is now**.
 *
 * So here the present is full ink and everything else — done behind,
 * not started ahead — is muted. Two states sharing a weight is correct:
 * neither is the present, and three weights for three statuses would
 * spend contrast on a distinction the order already carries.
 *
 * Caught by `check-acp` printing `in_progress L*90.8  pending L*90.8`
 * on a run whose assertion covered only the done/doing pair — and the
 * comment beside that assertion described this split while the code
 * still shared `toolInk`.
 */
function planInk(status: string): string {
  return status === "in_progress" ? "var(--foreground)" : "var(--muted-foreground)";
}

/**
 * The ink a tool line is written in, by what happened to it.
 *
 * **Running is full ink and settled is muted — the pen's own pressures,
 * not a state hue.** The first version painted a running tool in
 * 忙碌's colour, on the reasoning that reusing a state word beats
 * inventing one. `check-acp` failed it before it ever shipped: in the
 * light theme 忙碌 and `--muted-foreground` are **ΔL* 1.03** apart.
 * That is not a mistake in the colour, it is what the colour is for —
 * 忙碌's indigo was placed at grey's exact loudness on purpose, so that
 * a session row could gain a hue without gaining weight. A hue built to
 * have no lightness difference cannot then be asked to carry one.
 *
 * It matters most with motion off: the breath stops (app.css), and if
 * hue were the only carrier there would be nothing left. Ink weight
 * survives both that and a reader who does not separate those hues.
 *
 * 失败 keeps its own colour, because there the word and the ink agree:
 * a tool that failed is that word's subject, and the alternative was
 * the state before this batch, where a tool that failed and a tool that
 * worked were the same grey line.
 */
function toolInk(status: string): string {
  if (status === "failed") return "var(--state-failed)";
  if (running(status)) return "var(--foreground)";
  return "var(--muted-foreground)";
}

/** One poll's worth of frames, folded into the picture. Answers the
    same object when nothing was painted, so React stops there. */
function absorb(paint: Painting, frames: ChatFrame[], readOnly = false): Painting {
  let { past, arriving, live } = paint;
  let moved = false;
  for (const f of frames) {
    if (f.kind === "history") {
      if (arriving === paint.arriving) arriving = arriving.slice();
      arriving.push(f);
      continue;
    }
    if (f.kind === "history_end") {
      // The replacement. A replay that arrived empty replaces with an
      // empty past, which is the honest answer for a conversation with
      // no record — not a reason to keep painting the old one.
      const rebuilt: Item[] = [];
      for (const h of arriving) eat(rebuilt, h, true);
      past = rebuilt;
      arriving = [];
      moved = true;
      continue;
    }
    if (live === paint.live) live = live.slice();
    // **Whether a live user line is painted depends on whether this
    // face is the one that said it.** A hosted conversation paints its
    // own words first-hand as they are sent, so the protocol's echo of
    // them is skipped — painting both doubles every message. A
    // *followed* conversation says nothing: it is a record of somebody
    // else's terminal, and every user line in it is one this face has
    // never seen.
    //
    // Found by the smoke, which watches a recorded conversation whose
    // whole content is one user line: the follower sent it down the
    // live path and the fold dropped it, so a page that had been
    // painting fine went blank — silently, because agent text still
    // came through.
    if (eat(live, f, readOnly)) moved = true;
  }
  if (!moved && arriving === paint.arriving) return paint;
  return { past, arriving, live };
}

/**
 * `still` paints a conversation khor is not holding: a discovered
 * session's recorded past, fetched once as replay-shaped frames — same
 * fold, same painting, no attachment, no polling, and no input, since
 * there is no protocol channel to say anything into.
 */
export function ChatView({
  id,
  still: recorded = false,
  ended = false,
  category = null,
  machine = null,
  home = null,
}: {
  id: string;
  still?: boolean;
  /** **The conversation is over: its host is gone** (UX-4, decided; the
      row's 失败). Not opened — there is nobody to attach to — and read
      off the vendor's record where there is one (claude), as a
      discovered session is; 「对话已结束」 in the input's place, no 重试. */
  ended?: boolean;
  /** The machine this conversation runs on, by the name a person reads
      (`SessionRow::home`) — what a file's caption says it is on. */
  home?: string | null;
  /** The far machine this conversation lives on — `null` for one held
      here. What a long wait names. */
  machine?: string | null;
  /** The row's vendor (`SessionRow::category`): which mark and which
      name the composer's who seat wears. */
  category?: string | null;
}) {
  /** Read from a record rather than attached: a discovered session, or an
      ended claude conversation, whose transcript outlives its host. */
  const still = recorded || (ended && category === "claude");
  const [paint, setPaint] = useState<Painting>(BLANK);
  /** The agent's own commands, as of the last frame that carried a set. */
  const [commands, setCommands] = useState<{ name: string; description: string }[]>([]);
  const [gone, setGone] = useState(false);
  // **Why it never opened, in the node's own words, and a way to try
  // again** (round NET-FIX, audit/net.md F9): a machine that cannot be
  // reached used to read 「没有找到这段对话的记录」, as though the
  // conversation did not exist. And while it is being opened, the wait
  // is said (`Waiting`) rather than an empty page.
  const [openWhy, setOpenWhy] = useState<string | null>(null);
  const [openedAt, setOpenedAt] = useState<number | null>(null);
  const [attempt, setAttempt] = useState(0);
  // **The tab pill's third seat is this conversation's re-attach**
  // (UX-33): the same attempt its 重试 makes.
  useEffect(() => offerTabAct(id, { kind: "reload", run: () => setAttempt((n) => n + 1) }), [id]);
  const [inTurn, setInTurn] = useState(false);
  /** When the running turn started. Cleared with the turn, so a
      finished conversation counts nothing — and see `waiting` below for
      why a duration is the one thing worth adding to a screen that
      already shows the stream. */
  const [turnSince, setTurnSince] = useState<number | null>(null);
  /** This conversation's own context window, as the agent last reported
      it. `null` until an agent says — never a default: the whole reason
      the 垫片 learns the size instead of tabulating it is that a made-up
      denominator draws a gauge nobody measured. */
  const [usage, setUsage] = useState<{ used: number; size: number; cost: Cost | null } | null>(null);
  /** The agent's own name for this conversation, if it has one.
      **claude never sends this** — measured 2026-08-30 off a real
      `--output-format stream-json` run: it writes an `ai-title` record
      into its transcript on disk and puts nothing on the stream. It is
      read here because other ACP agents do send it, and the face
      learning to read a field is what makes attaching one of them a
      matter of configuration rather than of code. */
  const [title, setTitle] = useState<string | null>(null);
  /** The agent's plan, as of the last frame that carried one. */
  const [plan, setPlan] = useState<PlanEntry[]>([]);
  /** The mode the agent is in, when it has said. */
  const [mode, setMode] = useState<string | null>(null);
  /** What the session can be set to, as the host last told it —
      `null` until a `settings` frame, which most agents never send. */
  const [settings, setSettings] = useState<Settings | null>(null);
  /** The directory on the conversation's machine its files go to, picked
      above the box (「存到」, `SendInto`); `null` is its receive location.
      Read as each file is added: a file is put there when it is chosen. */
  const [into, setInto] = useState<string | null>(null);
  /** The files going along with the line being written (round CHAT,
      `chat-files`): each put on the conversation's machine as it is
      chosen, cleared by sending like the text beside them. ⚠️ 墓碑:
      `shots` carried pictures as base64 inside the prompt, gated on the
      agent's capabilities; every file is now a path on the machine, and
      a picture also rides inline where the agent takes pictures. */
  const files = usePendingFiles(id, into);
  const [pickOpen, setPickOpen] = useState(false);
  /** A terminal block handed to the next conversation (roadmap 2.3), as
      `[name, text]`: it rides as the prompt's own text, there being no
      file on any machine to point at. Cleared by sending. */
  const [docs, setDocs] = useState<[string, string][]>(() => {
    const handed = takeHandOff();
    return handed ? [handed] : [];
  });
  /** A turn with attachments on its way: bytes sent and in all, from the
      poll (`ChatBatch.sending`); `null` when nothing is crossing. */
  const [sending, setSending] = useState<[number, number] | null>(null);
  /** Which tool lines are open. **Closed by default and per line**: a
      conversation is read for what was said, and twenty shell commands
      unfolded is a transcript of the machine rather than of the work.
      Keyed by the protocol's own id, so a line keeps its state while
      new frames land above and below it. */
  const [open, setOpen] = useState<Set<string>>(new Set());
  /** What the agent on the other end said it can do, from the handshake.
      **Nothing read this frame until now** — `replays` had crossed the
      wire since the day it was added, with `gui_host`'s own note saying
      a face must act on it, and no face did. That was survivable while
      both agents khor spoke to were ones khor wrote (both replay); it
      is not survivable now that khor ships a registry snapshot, because
      3 of the registry's 32 probed agents do not replay and 19 refuse
      to open a session before a login. */
  const [facts, setFacts] = useState<{
    /** The agent's own name for itself (ACP `agentInfo`). */
    name: string | null;
    replays: boolean;
    needsLogin: boolean;
    /** The gate on attaching a picture at all. */
    images: boolean;
    /** The gate on attaching a file's contents. */
    embeds: boolean;
  } | null>(null);
  const [text, setText] = useState("");
  /** Which line of the "/" menu is under the keyboard. */
  const [pick, setPick] = useState(0);
  // Asks this face has answered: the buttons go, the line stays — a
  // record that permission was asked outlives the asking. Local state
  // on purpose: the host sends no receipt for an answer, and the turn
  // moving on is the only confirmation the protocol has.
  const [answered, setAnswered] = useState<Set<number>>(new Set());
  // Where the reader is standing, and whether anything landed while they
  // were away. Two facts, not one: leaving the tail is the reader's own
  // doing and needs no announcement, while text arriving behind their
  // back does — and a single flag would have to pick one of those to
  // lie about. Both are cleared by arriving at the bottom, which is the
  // only way this pair can go to zero (docs/UX.md 角标可归零).
  const [away, setAway] = useState(false);
  const [fresh, setFresh] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  const nearBottom = useRef(true);
  const box = useRef<HTMLTextAreaElement>(null);
  const coarse = useCoarse();
  // The box takes the keys the moment this pane opens, and any key
  // typed at a button afterwards (`useKeysTo`). Off while there is no
  // box to take them: a recorded conversation has nowhere to speak
  // into, and an ended one has nobody listening.
  useKeysTo(box, !still && !gone);
  /** How long to wait before asking again, and the way to stop waiting. */
  const hidden = useHidden();
  const pace = useRef(IDLE_MS);
  /** The last line handed to the host, until it says what became of it. */
  const sent = useRef("");
  const pollNow = useRef<() => void>(() => {});

  useEffect(() => {
    let stopped = false;
    let cursor = 0;
    if (ended && !still) {
      // Over, and no record to read: nothing to open and nothing to retry.
      setOpenWhy(null);
      setGone(true);
      return;
    }
    if (still) {
      // **Read-only, but not read-once.** This used to fetch the
      // transcript a single time, on the reasoning that a discovered
      // session has「no attachment, no polling, and no input, since
      // there is no protocol channel to say anything into」. That is
      // true of *input* — there is still no way to speak here, and the
      // composer stays a read-only line with 接管 beside it. It was
      // never true of output: the agent's transcript is growing on disk
      // the whole time, and khor already reads its tail every few
      // seconds for the row's preview line. So a session running in its
      // own terminal sat frozen on screen while the row beside it
      // updated.
      //
      // Followed by byte offset, so the cost is what was *said* rather
      // than the size of the file — the biggest transcript on this
      // machine is 280 MB, and re-reading it to find one new paragraph
      // is what made this read-once in the first place.
      let from = 0;
      let timer: number | null = null;
      const tick = () => {
        followHistory(id, from)
          .then((batch) => {
            if (stopped) return;
            from = batch.at;
            if (batch.frames.length > 0) {
              setPaint((prev) => absorb(prev, batch.frames, true));
            }
            // **Some records cannot be followed**, and khor says which
            // — a codex rollout, or a leaf nothing resolves. There the
            // answer is the whole file and asking again returns the
            // same file, so this stops: the read-once behaviour it
            // replaced, kept where following is not on offer. Without
            // it the pane re-read and repainted the entire transcript
            // every 700 ms, which the smoke found as a flake.
            if (!batch.following) stopped = true;
          })
          .catch(() => {
            // A transcript that cannot be read at all is a conversation
            // with no record — the same fact `fetchHistory` reported,
            // and the same answer. Only on the *first* read: a follow
            // that fails later is a moment, not an ending, and calling
            // it one would blank a conversation that is on screen.
            if (!stopped && from === 0) setGone(true);
          })
          .finally(() => {
            if (!stopped) timer = window.setTimeout(tick, STILL_MS);
          });
      };
      tick();
      return () => {
        stopped = true;
        if (timer !== null) window.clearTimeout(timer);
      };
    }
    // A self-scheduling wait rather than an interval, because the wait
    // changes length: an interval would have to be torn down and rebuilt
    // to change pace, and two of them briefly overlapping is two polls
    // racing on one cursor.
    let timer: number | null = null;
    let asking = false;
    const tick = () => {
      if (stopped) return;
      if (timer !== null) {
        window.clearTimeout(timer);
        timer = null;
      }
      // A poll already in the air will schedule the next one itself;
      // calling in on top of it would fork the loop in two.
      if (asking) return;
      asking = true;
      const pushed = inApp && pace.current !== HIDDEN_MS;
      (pushed ? chatWait(id, cursor, WAIT_MS) : chatPoll(id, cursor))
        .then((batch) => {
          if (stopped) return;
          cursor = batch.next;
          setSending(batch.sending_of > 0 ? [batch.sending, batch.sending_of] : null);
          // **The past this face was holding is no longer backed by
          // anything.** The attachment's frame list is capped
          // (`khor_gui_core::chat::FRAME_CAP`), and this says the trim
          // reached past where this reader had got to — so the paint,
          // which was built by absorbing those frames one at a time, is
          // now a picture with a hole in it that nothing will fill.
          //
          // Rebuilt the way a face that never saw them would: clear, and
          // ask for history. The frames in *this* batch are still
          // absorbed below, because they are the newest and the replay
          // lands in `past` regardless of when it arrives.
          if (batch.lost) {
            setPaint(BLANK);
            chatReplay(id).catch(() => {});
          }
          if (batch.frames.length) {
            if (batch.frames.some((f) => f.kind === "turn" || f.kind === "gone")) {
              setInTurn(false);
            }
            // A turn that was already running when this face attached.
            // Without it, a second face opens on an inviting empty box
            // and what it types is dropped (one turn at a time is the
            // host's rule — `gui_host` `GuiNote::Turning`).
            if (batch.frames.some((f) => f.kind === "turning")) setInTurn(true);
            for (const f of batch.frames) {
              if (f.kind === "agent") {
                setFacts({
                  name: f.name,
                  replays: f.replays,
                  needsLogin: f.needs_login,
                  images: f.images,
                  embeds: f.embeds,
                });
              }
            }
            // A refused line never reached the agent, so this face is
            // not in a turn — and the words are given back rather than
            // retyped. Only into an empty box: whatever was typed in
            // the meantime is newer than this and outranks it.
            if (batch.frames.some((f) => f.kind === "refused")) {
              setInTurn(false);
              const back = sent.current;
              sent.current = "";
              if (back) {
                // **Taken back off the screen.** It was painted the
                // moment it was typed, as though it had gone; it did
                // not go, and leaving it there is the transcript
                // claiming this person said something they did not.
                // Only the tail, and only if it is still that line —
                // anything after it belongs to somebody else's turn.
                setPaint((prev) => {
                  const last = prev.live[prev.live.length - 1];
                  if (!last || last.who !== "user" || last.text !== back) return prev;
                  return { ...prev, live: prev.live.slice(0, -1) };
                });
                // And handed back, so it need not be retyped. Only into
                // an empty box: whatever was typed since is newer than
                // this and outranks it.
                setText((now) => (now.trim() ? now : back));
              }
            }
            for (const f of batch.frames) {
              const list = commandsIn(f);
              if (list) setCommands(list);
              const u = usageIn(f);
              if (u) setUsage(u);
              const t = titleIn(f);
              if (t !== undefined) setTitle(t);
              const pl = planIn(f);
              if (pl) setPlan(pl);
              const u2 = updateOf(f);
              if (u2?.sessionUpdate === "current_mode_update" && u2.currentModeId) {
                setMode(u2.currentModeId);
              }
              const st = settingsIn(f);
              if (st) setSettings(st);
            }
            setPaint((prev) => absorb(prev, batch.frames));
          }
          if (batch.gone) setGone(true);
        })
        .catch(() => {
          // A poll that misses answers on the next tick; a chat that
          // never opened is `gone` below.
        })
        .finally(() => {
          asking = false;
          // A held wait has already waited: the next is asked at once.
          if (!stopped) timer = window.setTimeout(tick, pushed ? 0 : pace.current);
        });
    };
    pollNow.current = tick;
    // An unreachable host is an over conversation, not an error of this
    // pane's own: the host holds the agent, so no host means no one to
    // talk to (`gui_host` — the host file's pids are one process).
    setGone(false);
    setOpenWhy(null);
    setOpenedAt(Date.now());
    chatOpen(id)
      .then((fresh) => {
        if (stopped) return;
        setOpenedAt(null);
        // Only a fresh attachment asks for history: the host answers
        // every replay it is asked for, and a second mount of a living
        // chat (dev double-mount) already holds the replayed frames.
        if (fresh) chatReplay(id).catch(() => {});
        tick();
      })
      .catch((e: unknown) => {
        if (stopped) return;
        setOpenedAt(null);
        setOpenWhy(String((e as { message?: string })?.message ?? e));
        setGone(true);
      });
    return () => {
      stopped = true;
      if (timer !== null) window.clearTimeout(timer);
      pollNow.current = () => {};
      chatLeave(id).catch(() => {});
    };
  }, [id, still, ended, attempt]);

  // The paces, and one rule covering all of them: **a shorter pace has
  // to take effect at once, a longer one never does.** Waiting out a
  // whole gap to notice a turn started hands back the latency the fast
  // pace exists to remove, and the same is true of coming back to a
  // window that has been dawdling at ten seconds. Slowing down needs no
  // kick — the wait already in flight is the last fast one, and the next
  // is scheduled at the new length.
  //
  // **That is what makes this able to go back to zero**: nothing
  // remembers a pace, it is read fresh at each scheduling, so a state
  // that has gone away simply stops being read.
  useEffect(() => {
    const next = hidden ? HIDDEN_MS : inTurn ? LIVE_MS : IDLE_MS;
    const shorter = next < pace.current;
    pace.current = next;
    if (shorter) pollNow.current();
  }, [inTurn, hidden]);

  /**
   * The running turn's clock: started when a turn starts, stopped when
   * it stops, ticking once a second in between.
   *
   * **Derived from `inTurn` rather than set beside it.** A turn begins
   * and ends in four places (a `turning` frame, a `Turn` frame, a send
   * that threw, and the send itself), and a clock started at three of
   * them is a clock that is sometimes null while a turn runs — the kind
   * of drift nothing goes red for. One effect, one source.
   *
   * The ticking lives in `BusyFor`, which is mounted only while a turn
   * runs — so an idle conversation carries no timer, and a running one
   * repaints one text node rather than its whole stream.
   */
  useEffect(() => {
    setTurnSince(inTurn ? Date.now() : null);
  }, [inTurn]);

  // Follow the stream only while the reader is at the tail — new frames
  // must not yank someone who scrolled up into the past. What they get
  // instead is the control below saying something landed.
  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    if (nearBottom.current) el.scrollTop = el.scrollHeight;
    else setFresh(true);
  }, [paint]);

  // The box is as tall as what has been typed into it (`useGrowingText`).
  useGrowingText(box, text, still, gone);

  const toBottom = () => {
    const el = scroller.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
    nearBottom.current = true;
    setAway(false);
    setFresh(false);
  };

  /** A paste: the files on the clipboard go along with the line. */
  const takePaste = (e: React.ClipboardEvent) => {
    const wanted = [...e.clipboardData.items]
      .filter((i) => i.kind === "file")
      .map((i) => i.getAsFile())
      .filter((f): f is File => f !== null);
    if (wanted.length === 0) return;
    e.preventDefault();
    files.addFiles(wanted);
  };
  /** ＋'s picker: any file, several at once — on a phone the system's own
      sheet (photos, the camera, Files), on a Mac the open panel. */
  const picker = useRef<HTMLInputElement>(null);
  /** Files can go along while there is somebody to say them to. */
  const canAttach = !still && !gone;
  /** The file a press asked to look at (round CHAT): a file card, or a
      path in the agent's words. */
  const [preview, setPreview] = useState<string | null>(null);
  // **A path in the prose opens the preview first**, outside a workspace;
  // its 「在文件视图中打开」 is where the press used to go. Beside a tree
  // the press keeps going straight to the middle column (`beside`).
  const outer = useLinkHome();
  // **The conversation's files folder is painted too** (round CHAT item
  // 2): what the agent says about a file it was handed names it there,
  // outside any project. Found from the paths already said.
  const folder = useMemo(() => {
    for (const i of [...paint.past, ...paint.live]) {
      const f = "text" in i ? filesFolderIn(i.text) : null;
      if (f) return f;
    }
    return null;
  }, [paint]);
  const inFolder = useKnownFiles(home ?? "", folder ?? "");
  const chatHome = useMemo<LinkHome | null>(() => {
    // No project around this conversation: only its folder is painted.
    const base: LinkHome | null =
      outer ??
      (home === null
        ? null
        : {
            machine: home,
            root: "",
            known: () => false,
            see: () => {},
            open: () => {},
            launch: (abs) => void launchOn(home, abs).catch(() => {}),
          });
    if (base === null) return null;
    return {
      ...base,
      // Beside a tree a project file opens there; anything else, and
      // everything outside a workspace, opens the preview.
      open: (abs, line) =>
        base.beside && base.root !== "" && abs.startsWith(`${base.root}/`) ? base.open(abs, line) : setPreview(abs),
      more: folder === null ? undefined : { root: folder, known: inFolder.known, see: inFolder.see },
    };
  }, [outer, home, folder, inFolder.known, inFolder.see]);
  /** A file card pressed: the same road as a path in the prose. */
  const openFile = (abs: string) => (chatHome ? chatHome.open(abs, null) : setPreview(abs));
  const dropBox = useRef<HTMLDivElement>(null);
  const [dropping, setDropping] = useState(false);
  // **In the app a drop is tauri's**, which hands over the real paths
  // (the HTML5 drop never fires with anything useful there; the terminal
  // pane's note): files this device has, put on the conversation's
  // machine like any other (`chat_put_paths`). The position is checked
  // against this pane's box, the event being the window's.
  useEffect(() => {
    if (!inApp || !canAttach) return;
    let undo: (() => void) | undefined;
    let stopped = false;
    void (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const un = await getCurrentWebview().onDragDropEvent((e) => {
          const box = dropBox.current?.getBoundingClientRect();
          if (e.payload.type === "leave" || !box) {
            setDropping(false);
            return;
          }
          // Physical pixels, the page's box is CSS ones.
          const x = e.payload.position.x / devicePixelRatio;
          const y = e.payload.position.y / devicePixelRatio;
          const over = x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
          if (e.payload.type !== "drop") {
            setDropping(over);
            return;
          }
          setDropping(false);
          if (over && e.payload.paths.length > 0) files.addLocalPaths(e.payload.paths);
        });
        if (stopped) un();
        else undo = un;
      } catch {
        // No tauri here: the HTML5 drop above is the whole of it.
      }
    })();
    return () => {
      stopped = true;
      undo?.();
    };
    // `files`' adders are fresh closures each render; the subscription
    // is per conversation, and the adders read only setters and `id`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, canAttach]);
  /** The agent's own settings, in its own words, for ＋'s menu. The
      change is asked of the host and the control follows the answer
      (`settings` frame), never its own press. */
  const addOptions: AddOption[] = (settings?.options ?? []).map((o) =>
    o.type === "boolean"
      ? {
          kind: "switch",
          id: o.id,
          name: o.name,
          note: o.description ?? null,
          on: o.currentValue === true,
          onToggle: () => void chatSetOption(id, o.id, o.currentValue !== true).catch(() => {}),
        }
      : {
          kind: "values",
          id: o.id,
          name: o.name,
          note: o.description ?? null,
          current: String(o.currentValue ?? ""),
          values: valuesOf(o).map((v) => ({
            value: v.value.value,
            name: v.value.name,
            note: v.value.description ?? null,
            heading: v.heading,
          })),
          onPick: (v) => void chatSetOption(id, o.id, v).catch(() => {}),
        },
  );
  /** Who is answering, for the capsule's who seat. */
  const who: Who = {
    Mark: sessionMark("gui", category),
    name: agentName(category, facts?.name ?? null),
    model: modelOf(settings),
    usage: usage ? { used: usage.used, size: usage.size } : null,
  };
  /** The mode: a menu where the agent offers modes; the word alone where
      it only reports one that is not the ordinary one; nothing else. */
  const modeSeat = settings?.modes ? (
    <ModeSeat
      current={settings.modes.currentModeId}
      word={modeWord(settings.modes.currentModeId, settings.modes.availableModes)}
      // **The menu always holds the mode the session is in** (V09). One
      // the agent went into by itself and does not list (claude's own
      // `auto`) heads the menu, chosen, and is not pressable.
      choices={[
        ...(settings.modes.availableModes.some((m) => m.id === settings.modes?.currentModeId)
          ? []
          : [
              {
                id: settings.modes.currentModeId,
                name: modeWord(settings.modes.currentModeId, settings.modes.availableModes),
                note: modeNote(settings.modes.currentModeId, settings.modes.availableModes),
                fixed: true,
              },
            ]),
        ...settings.modes.availableModes.map((m) => ({
          id: m.id,
          name: modeWord(m.id, settings.modes?.availableModes ?? []),
          note: modeNote(m.id, settings.modes?.availableModes ?? []),
        })),
      ]}
      onPick={(v) => void chatSetMode(id, v).catch(() => {})}
    />
  ) : mode !== null && modeSaid(mode) !== null ? (
    <ModeSeat current={mode} word={modeSaid(mode) ?? mode} />
  ) : null;

  const say = () => {
    const line = text.trim();
    // A line, or files without one; never while a file is still on its
    // way to the machine (the send seat says busy meanwhile).
    if ((!line && files.items.length === 0 && docs.length === 0) || inTurn || gone || files.busy) return;
    const placed = files.items.filter((f) => f.path !== null);
    const full = withAttachments(line, placed.map((f) => f.path as string));
    const carried = facts?.images ? placed.flatMap((f) => (f.inline ? [f.inline] : [])) : [];
    // A handed-off terminal block rides as text (roadmap 2.3).
    const carriedDocs = docs;
    setText("");
    setInTurn(true);
    files.clear();
    setDocs([]);
    // Painted first-hand, at the end of the live half (module head).
    // It used to be pinned to a count of live frames and slotted back
    // in by the fold; appended here it lands in the same place for the
    // same reason — saying happens now, so it goes after everything
    // that has arrived and before everything that has not.
    setPaint((prev) => ({ ...prev, live: [...prev.live, { who: "user", text: full }] }));
    // Kept until the host either takes it or refuses it: a refused line
    // was painted here a moment ago as though it had gone, and handing
    // it back is the difference between "it did not go in" and "it did
    // not go in and you get to type it again".
    sent.current = line;
    chatSay(id, full, carried, carriedDocs).catch(() => setInTurn(false));
  };

  // The "/" menu: the agent's own commands, filtered by what has been
  // typed so far. Open only while the box holds a single slash-word —
  // a space means the command is chosen and what follows is its
  // argument, which is the agent's business and not this menu's.
  const slash = /^\/\S*$/.test(text) ? text.slice(1).toLowerCase() : null;
  const menu =
    slash === null ? [] : commands.filter((c) => c.name.toLowerCase().startsWith(slash));
  const complete = (name: string) => {
    // A trailing space: it closes the menu and leaves room for an
    // argument, and `say` trims it back off for a command that takes
    // none.
    setText(`/${name} `);
    setPick(0);
  };

  // The past then the live half, with the seals dropped. Recomputed
  // only when the picture actually moved — `absorb` answers the same
  // object otherwise, so a poll that brought nothing paintable stops
  // here rather than walking the list again.
  const items = useMemo(
    () =>
      [...paint.past, ...paint.live].filter(
        (i): i is Exclude<Item, { who: "seal" }> => i.who !== "seal",
      ),
    [paint],
  );
  // **A turn is running — for the whole of it, and for how long.**
  //
  // 忙碌 in the conversation, wearing the state's own clothes rather
  // than restating them: no hue of its own, the mark beside it carries
  // the breath keyframe and the reduced-motion guard (`ToolDot`,
  // app.css). Reusing the word is also why no seventh one gets invented
  // here — this is not a new thing called "thinking".
  //
  // **The rule this replaces**: it used to appear only until the first
  // chunk landed, on the reasoning that "the stream itself is then the
  // indicator and a second one would be noise". True of a turn that is
  // *printing*; false of the case a person actually asks about. A tool
  // call two minutes deep prints nothing, and **a stream that has
  // stopped moving looks exactly like a stream that has finished** —
  // the reader is left with a static screen and no way to tell which.
  //
  // So it stands for the whole turn, and it carries the one fact the
  // stream cannot: how long. That is also what keeps it from being
  // noise — it is not restating what is already on screen.
  const waiting = inTurn;
  return (
    <LinkHomeProvider value={chatHome}>
    <div
      ref={dropBox}
      data-chat-drop={dropping || undefined}
      className="flex min-h-0 flex-1 flex-col"
      // **Files dropped on the conversation go along with the next line**
      // (round CHAT): in a browser the drop carries `File`s; in the app
      // tauri takes the drop first and hands over paths (below).
      onDragOver={(e) => {
        if (!canAttach || !e.dataTransfer.types.includes("Files")) return;
        e.preventDefault();
        setDropping(true);
      }}
      onDragLeave={(e) => {
        if (e.currentTarget.contains(e.relatedTarget as Node | null)) return;
        setDropping(false);
      }}
      onDrop={(e) => {
        setDropping(false);
        if (!canAttach || e.dataTransfer.files.length === 0) return;
        e.preventDefault();
        files.addFiles([...e.dataTransfer.files]);
      }}
    >
      {home !== null && (
        <MachineFilePick
          open={pickOpen}
          onOpenChange={setPickOpen}
          machine={home}
          shown={home}
          onPick={(p) => files.addPaths([p])}
        />
      )}
      <div className="relative flex min-h-0 flex-1 flex-col">
      <div
        ref={scroller}
        data-chat-scroll
        onScroll={(e) => {
          const el = e.currentTarget;
          const near = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
          nearBottom.current = near;
          setAway(!near);
          if (near) setFresh(false);
        }}
        className="min-h-0 flex-1 overflow-y-auto px-4 py-3"
      >
        {/* **The reading column, and the one place it is set.** The pane
            is as wide as the window lets it be; a line of prose is not.
            Unbounded, this face measured 88 Chinese characters to a
            line at a 1482 window — two and a half times the top of the
            band a paragraph is read in, and the width at which the eye
            stops finding the start of the next line.

            It caps the *whole* conversation rather than each turn, so
            the person's own bubble hangs off the same right edge the
            agent's text ends at. Centred, because in the wide posture
            this pane sits beside a list and a column pinned left would
            leave the reading against the divider.

            `text-read` is on this element and not further in **because
            the cap is an em** (`--measure`): both classes have to ride
            together or the column comes out narrower than it says.

            The size lives here and the ink lives on the children for a
            second reason, found while proving the gate fires: handing
            `Markdown` both at once (`text-read text-read-ink`) loses
            the size. It goes through `cn()`, and tailwind-merge reads
            `text-read-ink` as another font-size — it has no way to know
            the tail of that name is a colour — so it drops the class it
            thinks was superseded. Measured: the paragraph came back at
            the shell's base with the theme's own leading.

            `break-words`: **a word longer than the column breaks inside
            it** (afix-a; the audit's S2 found a URL running off 留言's
            bubble, and this column's own lines — an error, a request's
            title — had no wrap either). `break-word`, not `anywhere`: it
            leaves a table's widths alone, which scrolls in its frame. */}
        <div data-chat-column data-selectable className="mx-auto w-full max-w-measure space-y-2 break-words text-read">
        {/* **The agent's own name for this conversation.** Scrolls with
            the conversation rather than sitting in a fixed header: it
            names the thing below it, and a title pinned over a
            conversation somebody has scrolled two screens into is
            labelling something no longer on screen.

            khor's own row already has a name (`token-63`), which is the
            *session's* — where it lives and what it is. This is the
            agent's, which is what it is *about*, and the two are not
            competing for one slot.

            **Nothing draws it today from claude**, which sends no such
            frame (it writes `ai-title` to its transcript and puts it on
            no stream). It is here for the agents in the registry that
            do, and because a face that cannot read a field is the
            reason attaching one of them would otherwise need code. */}
        {title && (
          <div data-chat-title className="pb-1 text-title-md font-title text-foreground">
            {title}
          </div>
        )}
        {/* **Why this conversation starts empty, when it does.** An
            empty column reads as "nobody has said anything yet", and
            for an agent that cannot replay its own past that reading is
            wrong in the one direction that matters: there may be a
            great deal above, and this window simply cannot see it.

            Only while the column is actually empty. Once anything has
            been said here the line has nothing left to explain, and a
            standing notice about a limitation nobody is hitting is the
            kind of chrome this app's copy rules exist to keep out. */}
        {/* **The agent's plan, above the conversation it is working
            through.** Not inside the stream: the plan is a *state* and
            the conversation is a record — folding it in as items would
            paint the same three steps again every time one of them
            moved, which is what the protocol's re-send would produce.

            The present is full ink and everything else is muted —
            see `planInk`, which is deliberately *not* `toolInk`: the
            word `pending` means "not started" here and "probably
            running" there, and one function for both drew the
            not-yet-started steps exactly like the current one.

            No count, no progress bar. The list is short by
            construction, and「3/7」is a number nobody acts on beside
            the steps themselves. */}
        {plan.length > 0 && (
          <div data-chat-plan className="pb-1 text-caption">
            <div className="pb-0.5 text-muted-foreground">{gui.chat_plan}</div>
            {plan.map((e, i) => (
              <div
                key={i}
                data-plan-entry
                data-plan-status={e.status}
                style={{ color: planInk(e.status) }}
              >
                {e.content}
              </div>
            ))}
          </div>
        )}
        {openedAt !== null && items.length === 0 && (
          <Waiting
            since={openedAt}
            machine={machine}
            onCancel={() => {
              setOpenedAt(null);
              setOpenWhy(machine ? gui.waiting_stopped(machine) : gui.waiting_stopped_here);
              setGone(true);
            }}
            onRetry={() => setAttempt((n) => n + 1)}
          />
        )}
        {facts && items.length === 0 && (facts.needsLogin || !facts.replays) && (
          <div data-chat-facts className="pb-1 text-caption text-muted-foreground">
            {facts.needsLogin ? gui.chat_needs_login : gui.chat_no_replay}
          </div>
        )}
        {/* An empty conversation says so, once and quietly (V18) — never
            a blank page; not while a turn is coming, and not over the
            line above when that one explains the emptiness better. */}
        {facts && items.length === 0 && !inTurn && !facts.needsLogin && facts.replays && (
          <div data-chat-empty className="pt-group text-center text-callout text-muted-foreground">
            {gui.chat_empty}
          </div>
        )}
        {items.map((item, i) =>
          item.who === "user" ? (
            // **The files a line carried are cards under it** (round CHAT
            // item 2), not the `@<path>` lines the agent was told them
            // in (`lib/paths`).
            (() => {
              const { words, paths } = splitAttachments(item.text);
              return (
                <div key={i} className="flex flex-col items-end gap-in">
                  {words && (
                    <div
                      data-said
                      className="max-w-[85%] rounded-lg bg-muted px-3 py-1.5 text-read-ink"
                    >
                      <Markdown text={words} />
                    </div>
                  )}
                  {paths.map((p, j) => (
                    <FileCard
                      key={`${j} ${p}`}
                      machine={home ?? ""}
                      shown={home ?? ""}
                      path={p}
                      onOpen={() => openFile(p)}
                    />
                  ))}
                </div>
              );
            })()
          ) : item.who === "agent" ? (
            <Markdown key={i} text={item.text} className="text-read-ink" />
          ) : item.who === "thought" ? (
            <Markdown key={i} text={item.text} className="text-muted-foreground" />
          ) : item.who === "tool" ? (
            // **A tool line gives up the left rule, and that is the
            // whole of the fix.** It wore `border-l-2 pl-2` and so did
            // a blockquote (`Markdown`), down to the same border colour
            // and the same gutter — one shape saying two things, which
            // is the reading it may not have: a rule down the left is
            // quotation's own mark, and this is not somebody's words,
            // it is a record that the agent did something.
            //
            // What tells them apart now is shape, not colour (the six
            // words own the colours): no rule, and mono — the content
            // is a verb and a path, which is code and was never prose.
            // **The hue carries the fact and the breath does not.**
            // A running tool wears 忙碌's colour and 忙碌's markup
            // (`data-word`/`data-word-text`, app.css), so it inherits
            // the breath and the reduced-motion guard that stops it —
            // and because the colour is still there when the breath is
            // not, a reader who has motion turned off loses nothing.
            // That is the same split the waiting indicator below runs
            // on, and reusing it is why no seventh state word gets
            // invented for tools.
            //
            // A failed tool wears 失败's colour for the same reason the
            // word exists: **before this, a tool that failed and a tool
            // that worked were the same grey line**, and the whole of
            // what the person could see about a broken turn was that
            // the text after it read oddly.
            //
            // Settled-and-fine stays muted on purpose. A conversation
            // holds many finished tools and one running one; if the
            // finished ones were coloured too, the colour would stop
            // meaning anything — the same reason a badge that is never
            // zero is not a badge.
            (() => {
              // What there is to see if this line is opened. A line
              // with nothing behind it is **not** made to look
              // openable: a control that opens onto nothing is worse
              // than no control, and until this batch every tool line
              // had nothing behind it.
              const more =
                item.diff !== null || item.out !== null || item.raw !== null;
              const shown = open.has(item.id);
              // **A file the agent wrote is a card under the line** (round
              // CHAT item 4), once the write is done: the file as it is
              // now, one press from its preview. The line keeps the
              // before and after it carried.
              const wrote =
                item.kind === "edit" && item.status === "completed"
                  ? [item.diff?.path, ...item.where].find((p): p is string => !!p && p.startsWith("/")) ?? null
                  : null;
              return (
                <Fragment key={i}>
                <div
                  data-chat-tool
                  data-tool-status={item.status}
                  data-tool-kind={item.kind}
                  data-tool-open={more ? (shown ? "yes" : "no") : undefined}
                  data-word={running(item.status) ? "busy" : undefined}
                  className="font-mono text-mono"
                >
                  {more ? (
                    <button
                      data-tool-toggle
                      className="flex w-full min-h-touch items-center gap-1.5 text-left pointer-fine:min-h-0"
                      onClick={() =>
                        setOpen((now) => {
                          const next = new Set(now);
                          if (!next.delete(item.id)) next.add(item.id);
                          return next;
                        })
                      }
                    >
                      <ToolDot status={item.status} />
                      {/* A flex item: `min-w-0` and `anywhere`, or a long
                          path in the title holds the row past the column. */}
                      <span data-word-text className="min-w-0 [overflow-wrap:anywhere]" style={{ color: toolInk(item.status) }}>
                        {item.title}
                      </span>
                    </button>
                  ) : (
                    <span className="flex items-center gap-1.5">
                      <ToolDot status={item.status} />
                      <span data-word-text className="min-w-0 [overflow-wrap:anywhere]" style={{ color: toolInk(item.status) }}>
                        {item.title}
                      </span>
                    </span>
                  )}
                  {shown && (
                    <div data-tool-body className="mt-1 flex flex-col gap-1 pl-2">
                      {item.diff && (
                        <div data-tool-diff className="flex flex-col gap-1">
                          {item.diff.old !== null && (
                            <pre
                              data-diff-before
                              className="overflow-x-auto rounded-md bg-muted px-2 py-1 text-muted-foreground"
                            >
                              {gui.chat_diff_before}
                              {"\n"}
                              {item.diff.old}
                            </pre>
                          )}
                          <pre
                            data-diff-after
                            className="overflow-x-auto rounded-md bg-muted px-2 py-1"
                          >
                            {gui.chat_diff_after}
                            {"\n"}
                            {item.diff.next}
                          </pre>
                        </div>
                      )}
                      {item.out !== null && (
                        <pre
                          data-tool-out
                          className="max-h-64 overflow-auto rounded-md bg-muted px-2 py-1 text-muted-foreground"
                        >
                          {item.out}
                        </pre>
                      )}
                      {/* The raw input, last and only when nothing
                          better stood in for it: for an edit the diff
                          *is* the input said properly, and printing the
                          JSON underneath it would be the same fact
                          twice, the second time unreadably. */}
                      {!item.diff && item.raw !== null && (
                        <pre
                          data-tool-raw
                          className="max-h-64 overflow-auto rounded-md bg-muted px-2 py-1 text-muted-foreground"
                        >
                          {JSON.stringify(item.raw, null, 1)}
                        </pre>
                      )}
                    </div>
                  )}
                </div>
                {wrote !== null && (
                  <FileCard
                    machine={home ?? ""}
                    shown={home ?? ""}
                    path={wrote}
                    note={gui.chat_file_wrote}
                    stamp={item.id}
                    onOpen={() => openFile(wrote)}
                  />
                )}
                </Fragment>
              );
            })()
          ) : item.who === "ask" ? (
            <div
              key={i}
              data-chat-ask
              className="flex flex-col gap-1.5 border-l-2 pl-2 text-caption"
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
            <div key={i} className="text-caption" style={{ color: "var(--state-errored)" }}>
              {item.text}
            </div>
          ),
        )}
        {waiting && (
          <div data-chat-thinking className="flex items-center gap-1.5 text-caption">
            <ToolDot status="in_progress" />
            {turnSince === null ? (
              <span data-chat-busy-for style={{ color: "var(--state-busy)" }}>
                {word("busy")}
              </span>
            ) : (
              <BusyFor since={turnSince} />
            )}
          </div>
        )}
        </div>
      </div>
      {/* The way back to the tail. It hangs over the conversation rather
          than inside it: a strip that took a row of its own would move
          the text under it every time somebody scrolled. Absent at the
          bottom — that is what makes it able to go to zero, and it is
          why the fresh flag is cleared in the same breath. */}
      {away && (
        <div className="pointer-events-none absolute inset-x-0 bottom-0 flex justify-center pb-2">
          <Button
            size="sm"
            variant="secondary"
            data-chat-bottom
            data-fresh={fresh}
            className="pointer-events-auto shadow"
            onClick={toBottom}
          >
            {/* The back chevron, turned to point the way this goes.
                Turned rather than drawn again: a second glyph would be
                the same shape at a different stroke ratio the first time
                somebody redrew it by eye (icons.tsx — one family). */}
            <IconBack className="-rotate-90" />
            {fresh ? gui.new_below : gui.to_bottom}
          </Button>
        </div>
      )}
      </div>
      {/* **On the narrow face the footer is the session bar's second
          row** (`components/SessionBar`, 2026-10-02: 「where a terminal
          has its row of accessory keys, a chat has its input box」) —
          under the tabs, on the keyboard when it is up. Everywhere else
          it stands here, as it always did. */}
      <BarSeat at="under">
      {/* A recorded past has no channel to speak into: no input, and
          the only thing its footer can say is that the record is
          missing. */}
      {still ? (
        // The read-only face says so where the input would be — without
        // this line, "no input" reads as a bug, not as the 接管 ruling
        // (a live TUI is the one mouth).
        //
        // **The 接管 offer used to stand here and no longer does.** It
        // changes what the session *is*, and the acts that do that now
        // live together on the row's own menu (`SessionsList`, 转成对话)
        // rather than one of them growing a button inside the pane that
        // merely looks at it.
        <div className="flex-none px-row pt-xxs pb-in">
          <div className="flex items-center gap-2 px-2 py-1.5 text-callout">
            <span data-chat-readonly className="min-w-0 flex-1 text-muted-foreground">
              {ended ? gui.chat_over : gone ? (openWhy ?? gui.chat_no_record) : gui.chat_readonly}
            </span>
            {gone && openWhy && !ended && (
              <Button data-chat-retry size="sm" onClick={() => setAttempt((n) => n + 1)}>
                {gui.region_retry}
              </Button>
            )}
          </div>
        </div>
      ) : (
      <div className="flex-none px-row pt-xxs pb-in">
        {/* The composer stands in the column the conversation is read in.
            At the pane's full width it would be a control several times
            wider than the text it writes into, and the eye would have to
            cross that gap to find what it just said.

            The rule stays on the outer element so the conversation is
            still divided from the composer by a line across the pane —
            it separates two regions, and half a line would say the
            column ends rather than that the reading does. The measure
            and `text-read` ride together here for the reason they do
            above. */}
        <div className="mx-auto w-full max-w-measure text-read">
        {/* **The conversation's own context, above the box it fills.**
            Placed here and not in the header because it is consulted
            *before typing* — the decision it feeds is "keep going or
            start a new conversation", and that decision is made with a
            hand already on the box.

            A share, not a token count: `32126 / 1000000` is two
            six-digit numbers nobody acts on, and the one thing a person
            does act on is how full it is. The count stays in the DOM
            (`data-chat-usage`) for anything that wants to assert on it.

            Absent until an agent reports a window — see `usageIn`. A
            face that showed "0%" while it had no denominator would be
            claiming an empty context, which for a conversation with
            history behind it is the opposite of true. */}
        {/* **What is different about this session, above the box.**
            Beside the context reading and for the same reason: both are
            things a person checks before typing, and neither is worth a
            row of its own. */}
        {/* ⚠️ 墓碑 (the redesign, 2026-10-02): a row above the box held the
            mode's popup button and the agent's other settings, and a
            marker said the mode when it could not be changed. The mode
            is a word inside the capsule now (`ModeSeat`), and the other
            settings are in ＋'s menu (`AddSeat`). */}
        {/* ⚠️ 墓碑 (the redesign, 2026-10-02): the context share and the
            cost were a line above the box (「上下文 41%」, 「$0.38」). The
            share is the dial in the capsule's who seat now (`WhoSeat`),
            and the cost is gone from the composer (the owner: 「右边不写
            美元」). */}
        {gone ? (
          // Never opened is not the same as over: the node's sentence
          // (a machine that cannot be reached, say) and 重试 — only a
          // conversation that really ended reads 对话已结束.
          openWhy ? (
            <div className="flex items-center gap-2 px-2 py-1.5 text-callout">
              <span data-chat-unopened className="min-w-0 flex-1 text-muted-foreground">
                {openWhy}
              </span>
              <Button data-chat-retry size="sm" onClick={() => setAttempt((n) => n + 1)}>
                {gui.region_retry}
              </Button>
            </div>
          ) : (
            <div className="px-2 py-1.5 text-callout text-muted-foreground">{gui.chat_over}</div>
          )
        ) : (
          // While a turn runs the send is refused and the way out sits
          // next to it: a turn that never ends used to leave 关闭 the
          // session as the only exit, which throws the conversation
          // away to get rid of one stuck line (found live — an agent's
          // request to its own API never came back, and the row read
          // 忙碌 for as long as anyone waited).
          //
          // Bottom-aligned, not centred: the box is the only thing in
          // this row that changes height, and centring would walk the
          // two buttons up the screen line by line as somebody typed.
          <>
          {/* **What is going to be sent, before it is sent.** A paste
              that vanished into state with nothing on screen is a
              person pressing 发送 without knowing what goes along.

              Numbered, not named: a pasted screenshot has no filename,
              and「图 1」is the only honest identity it has right now.
              Each one can be taken back off — the picture is still on
              the clipboard, so this undoes the attaching and not the
              having. */}
          {/* Where a file put from here lands on the conversation's
              machine, and the way to choose another directory there
              (STORE-BRIEF phase 2) — the 留言's line, the same component. */}
          {home !== null && (
            <SendInto machine={machine ?? ""} name={home} into={into} onInto={setInto} className="pb-xxs" />
          )}
          {sending && (
            <div data-chat-sending className="px-2 pb-1 text-caption tabular-nums text-muted-foreground">
              {gui.chat_sending((sending[0] / 1e6).toFixed(1), (sending[1] / 1e6).toFixed(1))}
            </div>
          )}
          <PendingChips items={files.items} onRemove={files.remove} />
          {docs.length > 0 && (
            <div data-chat-shots className="flex flex-wrap items-center gap-1.5 px-2 pb-1 text-caption">
              {docs.map(([name], i) => (
                <span
                  key={`d${i}`}
                  data-chat-doc
                  className="flex items-center gap-1 rounded-md bg-muted px-2 py-0.5 text-muted-foreground"
                >
                  {gui.chat_file(name)}
                  <button
                    data-chat-doc-drop
                    aria-label={gui.remove}
                    className="min-h-touch min-w-touch px-1 pointer-fine:min-h-0 pointer-fine:min-w-0"
                    onClick={() => setDocs((now) => now.filter((_, k) => k !== i))}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
          )}
          <div className="relative flex items-end">
            {/* The agent's own commands, above the box that will carry
                them. Absolute so opening it does not push the
                conversation up — a list that moves the text you are
                reading is worse than no list. */}
            {menu.length > 0 && (
              <div
                data-slash-menu
                // One floating language (components/Menu, 2026-09-28):
                // `popover`, `float`, `radius-lg` — it wore `shadow-md`
                // and `rounded-md`, a menu of its own drawing.
                className="absolute bottom-full left-0 mb-1 max-h-40 w-full overflow-y-auto rounded-lg border bg-popover p-1 shadow-(--shadow-float)"
              >
                {menu.map((c, n) => (
                  <button
                    key={c.name}
                    type="button"
                    data-slash-item={c.name}
                    data-on={n === pick}
                    className={cn(
                      "flex w-full items-baseline gap-2 rounded-sm px-2 py-1 text-left text-callout",
                      // The picked command is a chosen row — `fill-strong`,
                      // like a selected list row; the brand wash is for
                      // the row being located to (README 颜色, 4K).
                      n === pick && "bg-fill-strong",
                    )}
                    // The box must not lose focus to the press, or the
                    // completion would land in a box nobody is typing in.
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => complete(c.name)}
                  >
                    <span>/{c.name}</span>
                    {c.description && (
                      <span className="truncate text-caption text-muted-foreground">
                        {c.description}
                      </span>
                    )}
                  </button>
                ))}
              </div>
            )}
            {/* ＋'s picker: any file, into the same path a paste takes. */}
            {canAttach && (
              <input
                ref={picker}
                type="file"
                multiple
                hidden
                data-chat-file-input
                onChange={(e) => {
                  const list = [...(e.target.files ?? [])];
                  e.target.value = "";
                  files.addFiles(list);
                }}
              />
            )}
            <Composer
              lead={
                <AddSeat
                  onAttach={canAttach ? () => picker.current?.click() : undefined}
                  onAttachMachine={
                    canAttach && home ? { label: gui.chat_attach_machine(home), pick: () => setPickOpen(true) } : undefined
                  }
                  options={addOptions}
                />
              }
              // The seat gives way while there is text to send, where the
              // text would otherwise keep too little (a phone with the
              // longest mode word); it is back when the box is empty or a
              // turn runs.
              seatYields={!inTurn && !!text.trim()}
              trail={
                <>
                  {modeSeat}
                  <WhoSeat who={who} tip={!coarse} />
                  {/* Send only while there is something to send and the
                      agent can take it; during a turn the interrupt
                      stands in its seat. At rest with an empty box,
                      neither: the mode is the capsule's last thing. */}
                  {!inTurn && (text.trim() || files.items.length > 0) && (
                    <SendSeat onSend={say} disabled={files.busy} />
                  )}
                  {inTurn && (
                    <StopSeat
                      tip={!coarse}
                      onStop={() => {
                        // The turn's own ending is what clears the box:
                        // this asks, the host answers with a `Turn` frame
                        // like any other ending. Painting it stopped here
                        // would be a guess about somebody else's process.
                        chatStop(id).catch(() => {});
                      }}
                    />
                  )}
                </>
              }
            >
            {/* The box is never closed while the conversation is alive.
                It used to be `disabled` for the length of a turn, which
                threw away the one thing there is to do while waiting:
                write the next line. What a running turn takes away is
                the *sending*. */}
            <Textarea
              ref={box}
              data-chat-input
              onPaste={takePaste}
              rows={1}
              value={text}
              placeholder={gui.chat_input}
              // The soft keyboard's return key says what Enter does here:
              // it sends (Shift+Enter, on a keyboard that has one, breaks
              // the line).
              enterKeyHint="send"
              onChange={(e) => {
                setText(e.target.value);
                setPick(0);
              }}
              onKeyDown={(e) => {
                // Composition first, and before every branch below: mid-
                // 组词 the Enter that picks a candidate is the IME's, and
                // a face that read it would send a half-typed word and
                // eat the choice. `imeKey`: `isComposing` **or** keyCode
                // 229, because WebKit hands the input method's keys over
                // with 229 while `isComposing` is still false (`lib/ime`).
                if (imeKey(e)) return;
                if (menu.length > 0) {
                  // While the menu is open the keys belong to it: Enter
                  // chooses rather than sends, because a half-typed
                  // command name is not a thing to say to an agent.
                  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                    e.preventDefault();
                    const step = e.key === "ArrowDown" ? 1 : menu.length - 1;
                    setPick((p) => (p + step) % menu.length);
                    return;
                  }
                  if (e.key === "Enter" || e.key === "Tab") {
                    e.preventDefault();
                    complete(menu[Math.min(pick, menu.length - 1)].name);
                    return;
                  }
                  if (e.key === "Escape") {
                    // Escape leaves the text alone and closes the menu
                    // the only way it can be closed: by no longer being
                    // a bare slash-word.
                    e.preventDefault();
                    setText(`${text} `);
                    return;
                  }
                }
                // Enter sends, Shift+Enter breaks the line. Prevented
                // either way once it is Enter-without-shift: when the
                // turn refuses the send, the alternative is a newline
                // appearing as the answer to a press that meant "go",
                // which reads as the box having eaten the line.
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  say();
                }
              }}
              data-composer-text
              className={COMPOSER_TEXT}
            />
            </Composer>
          </div>
          </>
        )}
        </div>
      </div>
      )}
      </BarSeat>
      {/* ⚠️ 墓碑 (round UX-3): the keyboard switch in the bar. The box raises
          the keyboard and a drag down over the content lowers it
          (`SessionScreen`); a third way was a second door. */}
      <FilePreview
        machine={home ?? ""}
        shown={home ?? ""}
        path={preview}
        here={machine === null}
        onClose={() => setPreview(null)}
      />
    </div>
    </LinkHomeProvider>
  );
}
