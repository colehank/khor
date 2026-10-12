// **A new tab's page** (UX-45, the owner 2026-10-07): every road into
// 新建会话 — the list's ＋, ⌘N / ⌘T, the strip's ＋, the machine card's
// and the machines row's 开会话, the workspace column's ＋, a terminal's
// 发给智能体 and 在这里新开会话 — opens a new tab whose page is this, with
// what the road knew filled in. ⚠️ 墓碑 (UX-45): the dialog this was
// (`NewSessionDialog`), its 取消 / 创建 foot, and 留言's line and 发送.
//
// The wizard (会话身份批B): opening a session is the first act, and it
// asks the four questions of the ruling — where (目录), which agent
// (智能体), which form (对话 or 终端 — the same two words the detail's
// face switch wears), and what to call it (optional; the directory
// names it otherwise). The row appears in the list like any other; the
// vendor's own uuid is its id on the chat form, so hooks and the disk
// sweep merge into it with no ceremony.
//
// # One door, four steps, three kinds (2026-09-06)
//
// 「新建 session 不分留言和会话。点击后就是选机器、选类型(这就包括留言、
// 智能体、shell)、起名字、定目录」. So the steps are 在哪台 → 类型 → 名字
// → 目录, and 类型 is one of three:
//
//   - **留言** — a line left in that machine's window, `khor tell`.
//     Was its own dialog behind the "+" menu's second item; it is the
//     first kind here now and that door is gone. A line has no name
//     and no directory, so those two steps are not drawn for it —
//     absent, not greyed: a greyed field is a question still being
//     asked.
//   - **智能体** — what this dialog always was, with the agent row
//     directly under the kind (it refines the kind) and, for claude
//     alone, the 对话/终端 switch (the ruling: three kinds and no
//     fourth, and claude in a terminal stays reachable — greyed for the
//     others rather than hidden, the rule two paragraphs down).
//   - **shell** — that machine's own shell, `open --on`, in the
//     directory and under the name given. The user's word, not 终端:
//     a terminal is the window a session appears in.
//
// **A machine that cannot host takes a line and nothing else**
// (docs/IOS.md 第二步). A phone says so on its own row (`DeviceRow.hosts`):
// it is offered for 留言 — its window is a window like any other — and
// 智能体 / shell are greyed while it is the one picked, the 终端 form's
// rule below (a kind that exists for one machine and not another is a
// fact about the machine). **The default is never such a machine**: on
// a phone the dialog opens on the first machine that can host, so the
// first press of 创建 is a session somewhere real rather than a refusal.
//
// **智能体 is an open position (批⑥):** khor's own two, plus every ACP
// agent this person registered (`khor agents add`). The registered ones
// are fetched rather than listed here — the registry is a replicated
// document, so a name added on another machine is offerable on this one
// the moment it syncs, and a hard-coded list would be a second copy
// that disagrees.
//
// **Only claude keeps the 终端 form.** codex cannot be pre-named before
// its session exists (批8), and a registered agent has no terminal
// spelling khor knows at all — the node refuses both in their own
// words, and this face greys the button so nobody spends a click
// finding that out. Greying rather than hiding: a form that exists for
// one agent and not another is a fact about the agent, and a button
// that vanishes reads as khor having lost it.
//
// # Two things that used to need a terminal
//
// **接一个** opens the shipped ACP registry snapshot (`khor agents
// known`, 40 entries, 22 of them one `npx` argv). Somebody who never
// opens a terminal has no other way to learn those exist — the same
// reason `new_session_more_agents` is said at all, one step further.
// Attaching goes by **id**: the argv is shown but never sent back, and
// the id doubles as the registration's name so two machines attaching
// the same agent arrive at the same 类.
//
// ⚠️ 墓碑 (2026-09-22, docs/SHELL.md 第八轮): **已有的对话** — what a
// registered agent said it already has (`session/list`), with its four
// sentences and its 问一次 — stood here. It moved to 导入 whole
// (`ImportDialog`): continuing a conversation that exists is bringing
// something in, not starting something, and one act has one place.
import { useEffect, useRef, useState, type KeyboardEvent } from "react";

import {
  agentAsk,
  agents as fetchAgents,
  agentsKnown,
  agentsKnownTaken,
  fetchDevices,
  openSession,
  openTermOn,
  registerKnownAgent,
  type AgentRow,
  type DeviceRow,
  type KnownAgent,
} from "@/api";
import { FaceMark, type KindMark } from "@/components/FaceMark";
import { PathField, settledPath } from "@/components/PathField";
import { sessionKind } from "@/components/SessionFace";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PopupButton } from "@/components/ui/popup-button";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { FormGrid, FormLabel } from "@/components/ui/form-grid";
import { gui } from "@/gen/catalog";
import { draftFor, dropHandOff, peekHandOff } from "@/lib/composer-handoff";
import { useHereHosts } from "@/lib/here";
import { useSetting } from "@/settings";
import { cn } from "@/lib/utils";
import { connectionWord } from "@/words";

/** 类型 — the wizard's second step. Three, and no fourth (the ruling):
    a line for the broadcast addressed to the machine, an agent session, a
    bare shell. */
type Kind = "tell" | "agent" | "shell";

/** **One create token per set of answers** (round NET-7, `khor_node`'s
    `once`, from ux's first-run walk): a machine that quit before its
    answer came back had started the session all the same, and 创建
    pressed again made a second. The token is made before the first ask
    and kept, by the answers, until one is answered; the far machine
    answers a token it has seen with the session it made. Module-level, so
    closing the dialog and opening it again with the same answers still
    finds it. */
const createTokens = new Map<string, string>();
const createToken = (key: string): string => {
  let token = createTokens.get(key);
  if (!token) {
    token = crypto.randomUUID();
    createTokens.set(key, token);
  }
  return token;
};

export function NewSessionPage({
  onCancel,
  onOpened,
  machine: picked = "",
  dir: at = "",
}: {
  /** 取消: the new tab goes, and the person is back where they asked
      from (UX-45). */
  onCancel: () => void;
  /** The freshly-minted session id — the tab becomes it. */
  onOpened: (id: string) => void;
  // ⚠️ 墓碑 (2026-09-30): `onImport` stood here — 导入… as a button in
  // this dialog's foot, offered because ＋ was one act then. ＋ is a menu
  // of 新会话… and 导入… now, and an act has one home (10-layout).
  /** 在哪台, answered before the dialog opens — the machine card's
      开会话 names the card's machine (2026-09-06). Empty is the "+"
      menu's opening and means this machine, as it always has. */
  machine?: string;
  /** 目录, answered before the dialog opens — the workspace column's
      「+」 names that project's directory (工作区批⑩). Empty is every
      other door: the box opens empty, and empty means that machine's
      home.
   *
   * **Answered, not decided**: it lands in the same box a person types
   * into, so it can be typed over. A prefill that could not be changed
   * would be a fifth question asked somewhere else. */
  dir?: string;
}) {
  /** 在哪台 — which machine the session is born on. Every paired
      machine is offered; the row is drawn only when there is a choice
      (one machine is a fact, not a question). `""` until the table
      answers, which `openSession` reads as this machine. */
  const [machines, setMachines] = useState<DeviceRow[]>([]);
  const [machine, setMachine] = useState("");
  /** 目录. **Empty until typed in** (2026-09-27:「涉及到目录的，都不要
      推荐任何路径，简单一个输入框，用户可以边写边自动推荐」) — and empty
      is that machine's home to every verb this sends it to. */
  const [dir, setDir] = useState("");
  const [title, setTitle] = useState("");
  /** 类型 — the second step. Opens on 智能体, the kind this dialog was
      born for; a person who came to leave a line is one press away. */
  const [kind, setKind] = useState<Kind>("agent");
  const [form, setForm] = useState<"chat" | "term">("chat");
  /** 新会话默认开成 (`khor config set session-form chat|term`). **Only an
      explicit 「term」 moves it**: `useSetting` answers `null` until khor
      has said, and spelling the other half out here would be a second
      copy of a default only `khor_node::config::KEYS` decides. */
  const wantsTerm = useSetting("session-form") === "term";
  const [agent, setAgent] = useState("claude");
  const [known, setKnown] = useState<AgentRow[]>([]);
  // **Whether the registry answered**, which is not the same as it
  // being empty. Without this bit the dialog paints "you have not
  // registered any" during the moment before the answer arrives —
  // an empty list and an unanswered question look identical, and only
  // one of them is a fact about this person.
  const [listed, setListed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);
  // The snapshot panel: shut until asked for, and fetched then rather
  // than on every opening of this dialog — it is a constant table, and
  // a person who came to start claude never needs it.
  const [browsing, setBrowsing] = useState(false);
  const [snapshot, setSnapshot] = useState<KnownAgent[] | null>(null);
  const [taken, setTaken] = useState<[string, number, number] | null>(null);
  const [query, setQuery] = useState("");
  const [attaching, setAttaching] = useState<string | null>(null);
  /** A terminal block handed over by 发给智能体 (roadmap 2.3): shown under
      智能体, attached to the conversation this opens, and let go if the
      dialog closes any other way. */
  const [handed, setHanded] = useState<string | null>(null);
  const startedAgent = useRef(false);

  // Reopening must not inherit a stale attempt (TellDialog's rule), and
  // the registry is re-read each time: it syncs from other machines, so
  // a list cached across openings would be a stale answer to the one
  // question this dialog cannot guess.
  useEffect(() => {
    setMachines([]);
    // The card's pick, or this machine. Read at opening: the prop only
    // ever changes together with `open` (`App.openNewSession`).
    setMachine(picked);
    // The caller's directory, or that machine's home. Read at opening
    // for `picked`'s reason: the prop only ever changes together with
    // `open` (`App.openNewSession`).
    setDir(at);
    setTitle("");
    setKind("agent");
    // **The reset is what decides**, not the `useState` above: this
    // dialog stays mounted and every opening comes back through here.
    //
    // ⚠️ 终端 is only reachable for the vendor khor can speak for, and
    // the two lines below are in that order for that reason: this opens
    // on claude, which is the one agent 终端 is offered for. The day the
    // agent below becomes a setting too, this line has to converge
    // against it the way `pick` does — a selected 终端 under an agent
    // that has no terminal is a button the 开 press refuses.
    setForm(wantsTerm ? "term" : "chat");
    setAgent("claude");
    setKnown([]);
    setListed(false);
    setError(null);
    setOpening(false);
    setBrowsing(false);
    setQuery("");
    setAttaching(null);
    setHanded(peekHandOff());
    startedAgent.current = false;
    let live = true;
    // The machine table, read once per opening like the registry: it
    // syncs from other machines, and a paired machine that arrived
    // since the last opening is exactly the one a person came to pick.
    fetchDevices()
      .then((rows) => live && setMachines(rows))
      .catch(() => {});
    fetchAgents()
      .then((rows) => {
        if (!live) return;
        setKnown(rows);
        setListed(true);
      })
      // **A registry that could not be read is not an empty registry.**
      // Swallowing this would paint "you have registered nothing",
      // which is a different fact with a different next step — and the
      // one shape khor refuses to let a failure borrow.
      .catch((e) => live && setError(String(e instanceof Error ? e.message : e)));
    return () => {
      live = false;
    };
  }, [picked, at]);

  // **Named by id or by name**: a terminal's 发给智能体 and 在这里新开会话
  // name the session's machine as its row does, by device id, and every
  // choice here is a name — read as one once the table is in, or the
  // picker shows no machine and the kinds are judged on nothing.
  useEffect(() => {
    const d = machines.find((m) => m.id === machine);
    if (d) setMachine(d.name);
  }, [machines, machine]);
  // A handed-over block is for the conversation this page starts and no
  // other: 取消 lets it go (`cancel`). Switching to another tab and back
  // keeps it — the page is drawn again with the same block.
  const cancel = () => {
    if (!startedAgent.current) dropHandOff();
    onCancel();
  };

  const registered = (name: string) => known.some((a) => a.name === name);
  /** Every agent this dialog offers, khor's own two first. */
  const agentNames = ["claude", "codex", ...known.map((a) => a.name)];

  const why = (e: unknown) => setError(String(e instanceof Error ? e.message : e));

  // Picking anything but claude settles the form too: only claude has a
  // terminal khor knows how to open, so leaving 终端 selected would send
  // a refusal the person could have been spared.
  const pick = (name: string) => {
    setAgent(name);
    if (name !== "claude") setForm("chat");
  };

  const browse = () => {
    setBrowsing(true);
    if (snapshot) return;
    Promise.all([agentsKnown(), agentsKnownTaken()])
      .then(([rows, when]) => {
        setSnapshot(rows);
        setTaken(when);
      })
      .catch(why);
  };

  /** Attaches a snapshot entry and then asks it once.
      **Two acts, and the second is the expensive one** — but this is
      the moment the person is already waiting on this agent, and it is
      what the command line does at the same point. It runs behind the
      dialog: the section says 「问着…」 rather than freezing the face
      for the seconds that takes. */
  const attach = (id: string) => {
    if (attaching) return;
    setAttaching(id);
    setError(null);
    registerKnownAgent(id)
      .then(fetchAgents)
      .then((rows) => {
        setKnown(rows);
        setListed(true);
        setAgent(id);
        setForm("chat");
        setBrowsing(false);
        // Asked once, the way `khor agents add` does at this point —
        // what it says it has lands among the import candidates
        // (docs/SHELL.md 第八轮: 已有的对话 moved to 导入). Not painted
        // here, and not cancelled when this dialog closes: the answer
        // is written to the agents doc and syncs. **Not from a phone**
        // (docs/IOS.md 第二步): asking starts the agent here, and a phone
        // starts nothing — the registration alone travels.
        if (here) agentAsk(id).catch(() => {});
      })
      .catch(why)
      .finally(() => setAttaching(null));
  };

  /** The machine the act lands on, by name. `machine` empty is this
      one — the table's own `me` row spells its name, and the two calls
      that leave this dialog for a shell or a line resolve a name, not
      a blank. */
  const here = useHereHosts();
  /** Where the dialog opens when nobody picked: this machine — unless
      it cannot host (a phone), and then the first machine that can and is
      not a satellite (a satellite takes no agent). `""` is this machine
      to every verb, so a computer's default sends exactly what it always
      sent. No machine at all that can host leaves this machine, and
      the kinds below leave it a line. */
  const fallback = here ? "" : (machines.find((d) => d.hosts && d.sponsor === null)?.name ?? "");
  /** The machine every verb below is sent to, `""` meaning this one. */
  const chosen = machine || fallback;
  const target = chosen || machines.find((d) => d.me)?.name || "";
  /** Whether the machine picked can host a session — its own row says
      (a phone does not). **Yes until the table has answered**: the
      dialog opens before it, and on a phone the target is not known yet
      (it becomes the first machine that can host) — reading the phone's
      own no then would fall the kind back to 留言 for a machine nobody
      picked. */
  const targetHosts =
    machines.length === 0 ? true : (machines.find((d) => d.name === target)?.hosts ?? true);
  // 智能体 and shell picked on a machine that cannot host fall back to
  // the one kind it takes — the 终端 form's `pick` rule, one step up.
  useEffect(() => {
    if (!targetHosts && kind !== "tell") setKind("tell");
  }, [targetHosts, kind]);
  // The terminal form is claude's; another agent takes the conversation.
  useEffect(() => {
    if (agent !== "claude" && form === "term") setForm("chat");
  }, [agent, form]);

  const create = () => {
    if (opening) return;
    setOpening(true);
    setError(null);
    // The same answers ask with the same token until one is answered —
    // pressed again after 「没有响应」, or after closing and reopening.
    const key = JSON.stringify([kind, target, title, settledPath(dir), form, agent]);
    const token = createToken(key);
    (kind === "shell"
      ? openTermOn(target, title, settledPath(dir), token)
      : openSession(settledPath(dir), title, form, agent, chosen, token)
    )
      .then((id) => {
        createTokens.delete(key);
        startedAgent.current = kind === "agent";
        onOpened(id);
      })
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setOpening(false));
  };

  /* ⚠️ 墓碑 (UX-45): `send` — 留言's line sent from here (`khor tell`).
     留言 is a kind now; 创建 opens the machine's conversation, and the
     line is written there. */

  /** **The kinds as a page's choices** (UX-45, the owner: the kinds 新建
      会话 offers — shell, claude, codex, …): one row of tiles, each the
      chosen machine's face wearing that kind's mark and its name under
      it, the picked one a `fill-strong` block. 类型 and 智能体 in one
      pick, since on a page they are one question: what runs here. */
  const tileFace = machines.find((d) => d.name === target)?.face ?? null;
  const tiles: { key: string; label: string; mark: KindMark; on: boolean; pick: () => void }[] = [
    ...(targetHosts
      ? [
          {
            key: "shell",
            label: gui.new_session_kind_shell,
            mark: sessionKind("shell", "shell"),
            on: kind === "shell",
            pick: () => setKind("shell"),
          },
          ...agentNames.map((name) => ({
            key: `agent:${name}`,
            label: name,
            mark: sessionKind("tui", name),
            on: kind === "agent" && agent === name,
            pick: () => {
              setKind("agent");
              pick(name);
            },
          })),
        ]
      : []),
    { key: "tell", label: gui.message_text, mark: sessionKind("broadcast", null), on: kind === "tell", pick: () => setKind("tell") },
  ];
  /** The arrows walk the tiles and pick as they go, one tab stop for
      the row — a radio group's keys (`SegmentedControl`'s). */
  const tileKey = (i: number) => (e: KeyboardEvent<HTMLButtonElement>) => {
    const by = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (by === 0) return;
    e.preventDefault();
    const next = (i + by + tiles.length) % tiles.length;
    tiles[next].pick();
    e.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>("[data-new-session-tile]")[next]?.focus();
  };
  const tileRow = (
    <div data-new-session-tiles role="radiogroup" aria-label={gui.new_session_kind} className="flex flex-wrap gap-in">
      {tiles.map((t, i) => (
        <button
          key={t.key}
          type="button"
          role="radio"
          aria-checked={t.on}
          tabIndex={t.on ? 0 : -1}
          data-new-session-tile={t.key}
          data-on={t.on}
          onClick={t.pick}
          onKeyDown={tileKey(i)}
          className={cn(
            "flex w-new-tile flex-col items-center gap-xxs rounded-lg py-in text-callout",
            t.on ? "bg-fill-strong text-foreground" : "text-muted-foreground hover:bg-fill",
          )}
        >
          <FaceMark face={{ machine: tileFace }} mark={t.mark} size="dial" />
          <span className="max-w-full truncate px-xxs">{t.label}</span>
        </button>
      ))}
    </div>
  );
  /** The page's one act is 创建 whatever the kind (UX-45): 留言 makes
      nothing on the node — the broadcast is already there — so 创建 turns
      the tab into it, with `@<机器> ` written in its box (STORE-BRIEF
      phase 5: one channel).
      **The brand's primary, the one page that may wear it** (the owner
      2026-10-07, after UX-45 made it a plain button): `primary` is a
      dialog's one act, and this page is the dialog it replaced — the one
      exception (components/Button, README 「一个品牌电压」; `check-face`
      round 17 b allows exactly this button, tabs b checks it). */
  const pageAct = (
    <Button
      variant="primary"
      data-new-session-create
      disabled={opening}
      aria-busy={opening || undefined}
      onClick={() => {
        if (kind !== "tell") return create();
        const m = machines.find((d) => d.name === target);
        if (m && !m.me) draftFor(`@${m.label && !/[\s:"]/.test(m.label) ? m.label : m.name} `);
        onOpened("wide/all");
      }}
    >
      {gui.create}
    </Button>
  );

  return (
      // **A page, top-down** (UX-45): the machine, what runs there (the
      // tiles), then the name and the directory, and 取消 · 创建 at the
      // end of the column — no title (the tab says 新标签页).
      <div data-new-session-page className="h-full min-h-0 overflow-y-auto">
        <div className="mx-auto flex w-full max-w-measure flex-col gap-group px-page pt-pane pb-nav-room">
          <FormGrid data-new-session-form>
          {machines.length > 1 && (
            <>
              <FormLabel>{gui.new_session_machine}</FormLabel>
              <div data-new-session-machines className="flex min-w-0">
                <PopupButton
                  look="field"
                  label={gui.new_session_machine}
                  value={target}
                  mark={{ "data-new-session-machine-select": "" }}
                  // A machine that cannot be reached directly is not
                  // listed (the audit fix plan F1.9: not greyed).
                  choices={machines
                    .filter((d) => d.sponsor === null)
                    .map((d) => ({
                      value: d.name,
                      label: connectionWord(d) ? `${d.name} ${connectionWord(d)}` : d.name,
                      mark: { "data-new-session-machine": d.name },
                    }))}
                  onPick={(name) => {
                    setMachine(name);
                    // A path typed for this disk means nothing on
                    // that one.
                    setDir("");
                  }}
                />
              </div>
            </>
          )}
          <FormLabel>{gui.new_session_kind}</FormLabel>
          {tileRow}
          {kind === "agent" && (
            <>
              <div className="col-start-2 flex">
                {/* **The door onto the snapshot, and not a fourth
                    segment.** It opens a list; it is not an agent one
                    could be using. As a segment it would be announced
                    as a radio, the arrow keys would land on it and
                    「choose」 opening a panel, and it would look like a
                    name. So it stands after the control as a plain
                    button with the ellipsis a door wears (components/
                    Button). Beside the names rather than behind a
                    settings page: this is the moment the question
                    「有别的吗」 is being asked. */}
                <Button
                  variant="ghost"
                  data-new-session-add-agent
                  data-on={browsing}
                  aria-expanded={browsing}
                  onClick={() => (browsing ? setBrowsing(false) : browse())}
                  // The finger's box is the button's `::after` (`app.css`),
                  // so the row is as tall as the segments beside it — a
                  // 44 floor here made 「Run with」's group the loose one.
                  className="flex-none"
                >
                  {gui.new_session_add_agent}
                </Button>
              </div>
              {browsing && (
                <div data-new-session-registry className="col-start-2 flex min-w-0 flex-col gap-1">
                  <div className="flex items-baseline gap-2">
                    <span className="text-caption">{gui.new_session_registry}</span>
                    {/* **When it was taken, on screen.** A pinned table
                        with no date reads as「现在有这些」, which is a
                        claim khor cannot make about a file from one day. */}
                    {taken && (
                      <span data-registry-taken className="text-caption text-muted-foreground">
                        {gui.new_session_registry_taken(taken[0], taken[1])}
                      </span>
                    )}
                  </div>
                  <Input
                    data-registry-search
                    placeholder={gui.new_session_search}
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                  />
                  <div className="flex max-h-56 min-w-0 flex-col gap-1 overflow-y-auto">
                    {(snapshot ?? [])
                      .filter((k) => {
                        const q = query.trim().toLowerCase();
                        return (
                          q === "" ||
                          `${k.id} ${k.name} ${k.about}`.toLowerCase().includes(q)
                        );
                      })
                      .map((k) => (
                        <div
                          key={k.id}
                          data-registry-row={k.id}
                          data-registry-startable={k.argv !== null}
                          className="flex min-w-0 flex-col text-caption"
                        >
                          <div className="flex min-w-0 items-baseline gap-2">
                            <span className="min-w-0 flex-1 truncate">{k.id}</span>
                            {registered(k.id) ? (
                              // Shown, not hidden: an entry that vanished
                              // once attached reads as the snapshot having
                              // shrunk.
                              <span data-registry-attached className="shrink-0 text-muted-foreground">
                                {gui.new_session_attached}
                              </span>
                            ) : k.argv === null ? (
                              // **The other 18.** Not「接不了」—— khor has no
                              // argv for a platform binary, and that is a
                              // different sentence with a different next
                              // step. Said rather than dropped from the
                              // list, or a person concludes khor cannot
                              // speak to these at all.
                              <span data-registry-byhand className="shrink-0 text-muted-foreground">
                                {gui.new_session_byhand}
                              </span>
                            ) : (
                              <Button
                                size="sm"
                                variant="ghost"
                                data-registry-attach={k.id}
                                disabled={attaching !== null}
                                aria-busy={attaching === k.id || undefined}
                                onClick={() => attach(k.id)}
                              >
                                {gui.new_session_attach}
                              </Button>
                            )}
                          </div>
                          {/* **What it is, and what khor would run.**
                              The blurb is the registry's own line, and
                              the argv is the one thing a person cannot
                              find out any other way — 「khor 起得来」
                              without it is a claim they take on trust
                              (`khor agents known` prints it for the same
                              reason). Read here, never sent back: the
                              attach goes out as the id. */}
                          <span className="min-w-0 truncate text-muted-foreground">
                            {k.about}
                          </span>
                          {k.argv !== null && (
                            <span
                              data-registry-argv={k.id}
                              className="min-w-0 truncate text-muted-foreground"
                            >
                              {k.argv}
                            </span>
                          )}
                        </div>
                      ))}
                  </div>
                </div>
              )}
              {listed && known.length === 0 && (
                // Only once the registry has answered that it is empty.
                // The open position is otherwise invisible to anyone who
                // never opens a terminal: two names look like the whole
                // world, and a third is the one thing they cannot guess.
                <span data-new-session-more-agents className="col-start-2 text-caption text-muted-foreground">
                  {gui.new_session_more_agents}
                </span>
              )}
            </>
          )}
          {/* ⚠️ 墓碑 (UX-45, the owner 2026-10-07): 留言's line and 发送
              on the page. 留言 is chosen like any kind and 创建 opens
              that machine's conversation, where the line is written. */}
            {kind === "agent" && (
              <>
              {handed && (
                <>
                  <FormLabel mark={{ "data-new-session-label": "handed" }}>{gui.new_session_handed}</FormLabel>
                  <span data-new-session-handed className="min-w-0 truncate text-body" style={{ fontFamily: "var(--font-term)" }}>
                    {handed}
                  </span>
                </>
              )}
              {/* **claude's own two forms**, right under its name and
                  only for it: the ruling keeps claude-in-a-terminal
                  reachable without making it a fourth kind. ⚠️ 墓碑 (the
                  owner's progressive disclosure, 2026-10-02; the audit
                  fix plan F1.9): greyed for the others — a control that
                  cannot be used is not drawn. */}
              {agent === "claude" && (
                <>
                  <FormLabel mark={{ "data-new-session-label": "form" }}>{gui.new_session_form}</FormLabel>
                  <div data-new-session-forms className="flex min-w-0">
                    <SegmentedControl<"chat" | "term">
                      label={gui.new_session_form}
                      value={form}
                      onChange={setForm}
                      options={[
                        { key: "chat", label: gui.view_chat, mark: { "data-new-session-chat": "" } },
                        { key: "term", label: gui.view_terminal, mark: { "data-new-session-term": "" } },
                      ]}
                    />
                  </div>
                </>
              )}
              </>
            )}
          {kind !== "tell" && (
            <>
              <FormLabel htmlFor="khor-new-session-name">{gui.new_session_name}</FormLabel>
              <Input
                id="khor-new-session-name"
                data-new-session-name
                placeholder={gui.new_session_name_hint}
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && create()}
              />
              {/* **One box, completing as it is typed in** (2026-09-27).
                  It had 常去的地方 under it and a 「挑…」 picker beside it;
                  the ruling took both — nothing is offered before a
                  character is typed, and what is offered after is the
                  chosen machine's own directories (`PathField`). **Not a
                  `label` around it**: the completions are buttons, and a
                  button inside a label is a control inside a control. */}
              <FormLabel>{gui.new_session_dir}</FormLabel>
              <div className="flex min-w-0 flex-col">
                <PathField
                  data-new-session-dir
                  aria-label={gui.new_session_dir}
                  machine={chosen}
                  value={dir}
                  onValueChange={setDir}
                  onSubmit={create}
                />
              </div>
            </>
          )}
          {error && (
            <p data-new-session-error className="col-span-2 text-callout text-destructive">
              {error}
            </p>
          )}
          </FormGrid>
          {/* 取消 beside 创建 (UX-45, the owner 2026-10-07: on the phone a
              tab has no × to close it by): the new tab goes, and the tab
              that was in front comes back. */}
          <div className="flex justify-end gap-in">
            <Button data-new-tab-cancel onClick={cancel}>
              {gui.delete_keep}
            </Button>
            {pageAct}
          </div>
        </div>
      </div>
    );
}
