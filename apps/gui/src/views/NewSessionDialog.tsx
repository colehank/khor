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
// **已有的对话** is what one registered agent said it already has
// (`session/list` → the agents doc → every machine on the mesh). Two
// judgments hold this section up and both are load-bearing:
//
//  1. **Four answers, four sentences.** Never asked / it cannot list /
//     it has none / here they are. An empty list collapses the first
//     three into one shape, and the person's next move differs in all
//     of them — go ask, stop asking, open a new one, pick one. The
//     wire type (`AgentSaid`) carries `rows` on one variant only, so
//     this face cannot flatten them even by accident.
//  2. **Asking starts an agent process — measured 1.5s against a real
//     `@claude-acp` with its package cached, 5.64s without, against
//     0.017s to read what was recorded.** So the read runs when a person
//     picks an agent, and the ask is a button they press (or the one
//     beat right after they attached something, which is the same
//     press). Nothing on a timer may ask.
import { useEffect, useState } from "react";

import {
  agentAsk,
  agentSeen,
  agents as fetchAgents,
  agentsKnown,
  agentsKnownTaken,
  fetchDevices,
  fetchPlaces,
  openSession,
  openTermOn,
  registerKnownAgent,
  resumeSession,
  tell,
  type AgentRow,
  type AgentSaid,
  type DeviceRow,
  type KnownAgent,
  type PlaceRow,
} from "@/api";
import { DirPicker } from "@/components/DirPicker";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { connectionWord } from "@/words";
import { cn } from "@/lib/utils";

/** 类型 — the wizard's second step. Three, and no fourth (the ruling):
    a line left in the machine's window, an agent session, a bare shell. */
type Kind = "tell" | "agent" | "shell";

export function NewSessionDialog({
  open,
  onOpenChange,
  onOpened,
  machine: picked = "",
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The freshly-minted session id — the caller may select its row. */
  onOpened: (id: string) => void;
  /** 在哪台, answered before the dialog opens — the machine card's
      开会话 names the card's machine (2026-09-06). Empty is the "+"
      menu's opening and means this machine, as it always has. */
  machine?: string;
}) {
  /** 在哪台 — which machine the session is born on. Every paired
      machine is offered; the row is drawn only when there is a choice
      (one machine is a fact, not a question). `""` until the table
      answers, which `openSession` reads as this machine. */
  const [machines, setMachines] = useState<DeviceRow[]>([]);
  const [machine, setMachine] = useState("");
  const [dir, setDir] = useState("~");
  /** The directory picker is up (工作区批②). A second dialog over this
      one rather than a step inside it: the wizard asks four questions
      and this is a detour from one of them, so it has to be able to end
      without answering — and the answers already given must still be
      here when it does. */
  const [picking, setPicking] = useState(false);
  /** 常去的地方. Empty until the read lands, and empty is also the
      honest answer on a machine with no recorded conversations — there
      is nothing to say about it either way, so nothing is said. */
  const [places, setPlaces] = useState<PlaceRow[]>([]);
  const [title, setTitle] = useState("");
  /** 类型 — the second step. Opens on 智能体, the kind this dialog was
      born for; a person who came to leave a line is one press away. */
  const [kind, setKind] = useState<Kind>("agent");
  /** The line to leave, for the 留言 kind. */
  const [text, setText] = useState("");
  const [form, setForm] = useState<"chat" | "term">("chat");
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
  // **What this agent said, and which agent said it.** Kept as a pair
  // because the answer arrives after the click that asked for it: a
  // person switching agents faster than the read returns would
  // otherwise see one agent's conversations under another's name,
  // which is the kind of wrong that looks entirely plausible.
  const [said, setSaid] = useState<{ of: string; answer: AgentSaid } | null>(null);
  const [asking, setAsking] = useState<string | null>(null);

  // Reopening must not inherit a stale attempt (TellDialog's rule), and
  // the registry is re-read each time: it syncs from other machines, so
  // a list cached across openings would be a stale answer to the one
  // question this dialog cannot guess.
  useEffect(() => {
    if (!open) return;
    setMachines([]);
    // The card's pick, or this machine. Read at opening: the prop only
    // ever changes together with `open` (`App.openNewSession`).
    setMachine(picked);
    setDir("~");
    setPicking(false);
    setPlaces([]);
    setTitle("");
    setKind("agent");
    setText("");
    setForm("chat");
    setAgent("claude");
    setKnown([]);
    setListed(false);
    setError(null);
    setOpening(false);
    setBrowsing(false);
    setQuery("");
    setAttaching(null);
    setSaid(null);
    setAsking(null);
    let live = true;
    // **Read on opening, like the registry above it.** A place list
    // cached across openings would be answering with where this person
    // worked before the conversation they just finished — and this
    // whole surface exists because the freshest place is usually the
    // one they want. A failure here is left silent on purpose: the
    // 目录 box is still right there and still takes anything, so a
    // shortcut that could not be built is not a fact worth a red line
    // (the registry's failure *is*, because「你没接过 agent」and「我读
    // 不到」are different next steps).
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
  }, [open, picked]);

  // **The places follow the machine.** 常去的地方 are read from where
  // the session is about to be born, and a machine change resets the
  // box to that machine's home — a path typed for this disk means
  // nothing on that one. A failure is left silent on purpose: the 目录
  // box is still right there and still takes anything, so a shortcut
  // that could not be built is not a fact worth a red line (the
  // registry's failure *is*, because「你没接过 agent」and「我读不到」
  // are different next steps).
  useEffect(() => {
    if (!open) return;
    let live = true;
    setPlaces([]);
    fetchPlaces(machine)
      .then((rows) => live && setPlaces(rows))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [open, machine]);

  const registered = (name: string) => known.some((a) => a.name === name);

  const why = (e: unknown) => setError(String(e instanceof Error ? e.message : e));

  /** Reads what was recorded. **No process** — this is the 0.017s half,
      which is why it may run on a pick. */
  const readSeen = (name: string) => {
    setSaid(null);
    agentSeen(name)
      .then((answer) => setSaid({ of: name, answer }))
      .catch(why);
  };

  /** Puts the question to the agent. **Starts it.** Only ever from a
      press, or from the beat right after an attach — which is the same
      press, one step on, and is what keeps this face and `khor agents
      add` telling a person the same thing. */
  const ask = (name: string) => {
    setAsking(name);
    // Not cancelled when this dialog closes: the answer is written to
    // the agents doc and syncs, so a question already paid for is
    // worth keeping even if nobody is left watching. Only the painting
    // is guarded, by the name it belongs to.
    agentAsk(name)
      .then((answer) => setSaid({ of: name, answer }))
      .catch(why)
      .finally(() => setAsking((n) => (n === name ? null : n)));
  };

  // Picking anything but claude settles the form too: only claude has a
  // terminal khor knows how to open, so leaving 终端 selected would send
  // a refusal the person could have been spared.
  const pick = (name: string) => {
    setAgent(name);
    if (name !== "claude") setForm("chat");
    // khor's own two have no recorded answer and never will — khor
    // reads their conversations off disk. Asking the store about them
    // would paint 「还没问过」 about a question that does not apply.
    if (registered(name)) readSeen(name);
    else setSaid(null);
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
        // Nothing recorded yet, and that is a real state rather than a
        // gap: it is what shows if the question below fails.
        setSaid({ of: id, answer: { said: "unasked" } });
        ask(id);
      })
      .catch(why)
      .finally(() => setAttaching(null));
  };

  /** Opens a conversation that already exists. It brings its own
      directory and its own name, so 目录 and 名字 are not sent — the
      row prints where it lands for exactly that reason. */
  const resume = (session: string) => {
    if (opening) return;
    setOpening(true);
    setError(null);
    resumeSession(agent, session)
      .then((id) => {
        onOpenChange(false);
        onOpened(id);
      })
      .catch(why)
      .finally(() => setOpening(false));
  };

  /** The machine the act lands on, by name. `machine` empty is this
      one — the table's own `me` row spells its name, and the two calls
      that leave this dialog for a shell or a line resolve a name, not
      a blank. */
  const target = machine || machines.find((d) => d.me)?.name || "";

  const create = () => {
    if (opening) return;
    setOpening(true);
    setError(null);
    (kind === "shell" ? openTermOn(target, title, dir) : openSession(dir, title, form, agent, machine))
      .then((id) => {
        onOpenChange(false);
        onOpened(id);
      })
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setOpening(false));
  };

  /** 留言: the whole of `khor tell`. The message on refusal is the
      node's, in the catalog's words — this layer has none to add. */
  const send = () => {
    if (!text.trim() || opening) return;
    setOpening(true);
    setError(null);
    tell(target, text)
      .then(() => onOpenChange(false))
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setOpening(false));
  };

  // **The four answers, kept four.** `answer` is null only while the
  // read is still out — which is its own case again, for the reason
  // `listed` exists: a question not yet answered is not a fact.
  //
  // Written as one chain so the four are visibly exhaustive. Note that
  // `rows` is reachable only inside the last branch — that is the wire
  // type doing the work, and it is why 「它说它没有」 cannot quietly
  // become 「还没问过」 here.
  const answer = said !== null && said.of === agent ? said.answer : null;
  const mark =
    answer === null
      ? "reading"
      : answer.said === "unasked"
        ? "unasked"
        : answer.said === "cannot"
          ? "cannot"
          : answer.rows.length === 0
            ? "none"
            : "sessions";
  const sentence =
    mark === "unasked"
      ? gui.new_session_seen_unasked
      : mark === "cannot"
        ? gui.new_session_seen_cannot
        : mark === "none"
          ? gui.new_session_seen_none
          : // A listing says itself; a sentence over it would be a
            // count nobody asked for.
            null;
  const rows = answer !== null && answer.said === "sessions" ? answer.rows : [];

  return (
    <>
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-new-session-dialog>
        <DialogHeader>
          <DialogTitle>{gui.new_session}</DialogTitle>
        </DialogHeader>
        {/* **`min-w-0` on every one of these, and it is load-bearing.**
            `DialogContent` is a grid and this is its item; a grid (or
            flex) item defaults to `min-width: auto`, so one long cwd
            in the lists below pushes the whole dialog past its
            `max-w-lg` — inputs, buttons and all — and `truncate`
            never fires because nothing above it agreed to shrink. */}
        <div className="flex min-w-0 flex-col gap-3">
          {/* **在哪台, first — before the directory, because the
              directory is a fact about the machine.** The whole net's
              machines, this one marked; picking one makes the places
              below, the box and the agent all that machine's. Drawn
              only when there is a choice to make. A satellite is
              offered greyed: it has no khor of its own to host an
              agent (`open --on` reaches it over ssh for a shell), and
              a button that cannot be pressed says so more plainly
              than a refusal after the fact. */}
          {machines.length > 1 && (
            <div data-new-session-machines className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.new_session_machine}</span>
              <div className="flex flex-wrap gap-1">
                {machines.map((d) => (
                  <Button
                    key={d.id}
                    size="sm"
                    variant={(machine === "" ? d.me : machine === d.name) ? "secondary" : "ghost"}
                    data-new-session-machine={d.name}
                    data-on={machine === "" ? d.me : machine === d.name}
                    disabled={d.sponsor !== null}
                    onClick={() => {
                      setMachine(d.name);
                      setDir("~");
                    }}
                  >
                    {d.name}
                    {connectionWord(d) && (
                      <span data-new-session-connection className="text-muted-foreground">
                        {connectionWord(d)}
                      </span>
                    )}
                  </Button>
                ))}
              </div>
            </div>
          )}
          {/* **类型, second** (2026-09-06:「选机器、选类型、起名字、定目录」).
              Three kinds; which of the steps below are drawn depends on
              it, and the ones a kind does not ask are absent rather
              than greyed — a greyed field is a question still being
              asked, and a line has no name and no directory. */}
          <div data-new-session-kinds className="flex min-w-0 flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_kind}</span>
            <div className="flex flex-wrap gap-1">
              {(
                [
                  ["tell", gui.message_text],
                  ["agent", gui.new_session_agent],
                  ["shell", gui.new_session_kind_shell],
                ] as const
              ).map(([k, word]) => (
                <Button
                  key={k}
                  size="sm"
                  variant={kind === k ? "secondary" : "ghost"}
                  data-new-session-kind={k}
                  data-on={kind === k}
                  onClick={() => setKind(k)}
                >
                  {word}
                </Button>
              ))}
            </div>
          </div>
          {kind === "tell" && (
            // 留言's third step is the line itself — the whole of
            // `khor tell`, which used to be a dialog of its own.
            <Input
              data-tell-text
              aria-label={gui.message_text}
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && send()}
            />
          )}
          {kind === "agent" && (
            <>
          {/* The agent row, directly under the kind that chose it — no
              label of its own: the pressed 智能体 above is the label,
              and a second 智能体 under it would be the same word twice
              on one screen (the catalog refuses the spelling; this
              refuses the layout). `div`, not `label`: nothing here is
              a field. */}
          <div data-new-session-agents className="flex flex-col gap-1 text-sm">
            <div className="flex flex-wrap gap-1">
              {/* Vendor names are proper nouns with no catalog entry
                  (words.ts's rule), and a registered agent wears the
                  name its owner gave it — so every button here is a
                  name, never a translated word. khor's own two lead
                  because they need no registering; the rest follow in
                  the registry's order, which is alphabetical, so two
                  machines paint the same row of buttons. */}
              {["claude", "codex", ...known.map((a) => a.name)].map((name) => (
                <Button
                  key={name}
                  size="sm"
                  variant={agent === name ? "secondary" : "ghost"}
                  data-new-session-agent={name}
                  data-on={agent === name}
                  onClick={() => pick(name)}
                >
                  {name}
                </Button>
              ))}
              {/* The door onto the snapshot. Beside the names rather
                  than behind a settings page: this is the moment the
                  question「有别的吗」 is being asked. */}
              <Button
                size="sm"
                variant="ghost"
                data-new-session-add-agent
                data-on={browsing}
                onClick={() => (browsing ? setBrowsing(false) : browse())}
              >
                {gui.new_session_add_agent}
              </Button>
            </div>
            {browsing && (
              <div data-new-session-registry className="flex min-w-0 flex-col gap-1">
                <div className="flex items-baseline gap-2">
                  <span className="text-xs">{gui.new_session_registry}</span>
                  {/* **When it was taken, on screen.** A pinned table
                      with no date reads as「现在有这些」, which is a
                      claim khor cannot make about a file from one day. */}
                  {taken && (
                    <span data-registry-taken className="text-xs text-muted-foreground">
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
                        className="flex min-w-0 flex-col text-xs"
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
              // never opens a terminal: two buttons look like the whole
              // world, and a third is the one thing they cannot guess.
              <span data-new-session-more-agents className="text-xs text-muted-foreground">
                {gui.new_session_more_agents}
              </span>
            )}
          </div>
              {/* **claude's own two forms**, right under its name and
                  only alive for it: the ruling keeps claude-in-a-terminal
                  reachable without making it a fourth kind. Greyed for
                  the others rather than hidden — a form that exists for
                  one agent and not another is a fact about the agent,
                  and a button that vanishes reads as khor having lost
                  it. */}
          <div className="flex gap-1">
            <Button
              size="sm"
              variant={form === "chat" ? "secondary" : "ghost"}
              data-new-session-chat
              data-on={form === "chat"}
              onClick={() => setForm("chat")}
            >
              {gui.view_chat}
            </Button>
            <Button
              size="sm"
              variant={form === "term" ? "secondary" : "ghost"}
              data-new-session-term
              data-on={form === "term"}
              disabled={agent !== "claude"}
              onClick={() => setForm("term")}
            >
              {gui.view_terminal}
            </Button>
          </div>
          {/* **Only for an agent that was registered.** khor finds
              claude's and codex's conversations by reading their files;
              this section is the *other* discovery path, and pointing
              it at them would paint 「还没问过」 about a question that
              does not apply to them. */}
          {registered(agent) && (
            <div data-agent-seen data-said={mark} className="flex min-w-0 flex-col gap-1 text-sm">
              <div className="flex items-baseline justify-between gap-2">
                <span className="text-muted-foreground">{gui.new_session_seen}</span>
                {/* **The only thing here that starts a process.** It is
                    a button because of that, and for no other reason:
                    seconds against 0.017s to read what was recorded. */}
                <Button
                  size="sm"
                  variant={mark === "unasked" ? "secondary" : "ghost"}
                  data-agent-ask
                  data-asking={asking === agent}
                  disabled={asking !== null}
                  onClick={() => ask(agent)}
                >
                  {asking === agent ? gui.new_session_seen_asking : gui.new_session_seen_ask}
                </Button>
              </div>
              {/* **Three sentences for three answers**, and they are
                  three because the next move differs: go ask, stop
                  asking, open a new one. Nothing is said while the read
                  is still out — an unanswered question is not a fact
                  about this agent (the `listed` rule, one field up). */}
              {sentence !== null && (
                <span data-agent-seen-text className="text-xs text-muted-foreground">
                  {sentence}
                </span>
              )}
              {rows.length > 0 && (
                <div className="flex max-h-40 min-w-0 flex-col gap-1 overflow-y-auto">
                  {rows.map((r) => (
                    <button
                      key={r.id}
                      type="button"
                      data-seen-row={r.id}
                      disabled={opening}
                      onClick={() => resume(r.id)}
                      className="flex w-full min-w-0 items-baseline justify-between gap-2 text-left text-xs hover:bg-accent"
                    >
                      {/* Its own name where it has one, and where it
                          lives where it does not — an id is not
                          something a person recognises. The cwd is
                          shown either way, because clicking lands
                          there rather than in the 目录 box above. */}
                      <span className="min-w-0 flex-1 truncate">{r.title ?? r.cwd}</span>
                      {r.title !== null && (
                        <span className="min-w-0 flex-1 truncate text-muted-foreground">
                          {r.cwd}
                        </span>
                      )}
                      <span className="shrink-0 tabular-nums text-muted-foreground">
                        {(r.updated_at ?? "").slice(0, 10)}
                      </span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
            </>
          )}
          {kind !== "tell" && (
            <>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_name}</span>
            <Input
              data-new-session-name
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && create()}
            />
          </label>
          {/* The box and its shortcuts, in one column. **Not one
              `label`**: a `button` inside a `label` is a control inside
              a control, and every click on a place would also be a
              click on the field. */}
          <div className="flex min-w-0 flex-col gap-1">
            <label className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.new_session_dir}</span>
              {/* **The box, and a way to go and look** (工作区批②).
                  Three roads to one field now — typing, 常去的地方
                  below, and the picker — and that is not three ways to
                  do one thing: the box reaches anything but needs the
                  path known, the shortcuts reach what is already worked
                  in, and the picker is the only one that can reach a
                  directory nobody has been to yet, on a machine that is
                  not this one. **Nothing is replaced**: the box is
                  still where the answer lands, and everything else
                  writes into it. */}
              <div className="flex min-w-0 items-center gap-1">
                <Input
                  data-new-session-dir
                  value={dir}
                  onChange={(e) => setDir(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && create()}
                />
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  data-new-session-pick
                  className="min-h-touch min-w-touch flex-none"
                  onClick={() => setPicking(true)}
                >
                  {gui.dir_picker_open}
                </Button>
              </div>
            </label>
            {/* **常去的地方: the first question, answered before it is
                asked.** A person walks into a directory and then starts
                something there, so the directories they already work in
                sit right under the box — picking one is a click instead
                of a typed path.

                Under the box rather than above it, and the box keeps
                taking anything: this is a shortcut, not a menu. A place
                that is not on the list (a directory nobody has opened a
                conversation in yet) has to stay typeable, and a list
                that replaced the box would have made those unreachable.

                Nothing is drawn when there is nothing — an empty list
                is the honest state of a fresh machine, and a heading
                over no rows is a promise of something missing. */}
            {places.length > 0 && (
              <div data-new-session-places className="flex min-w-0 flex-col gap-1">
                <span className="text-xs text-muted-foreground">
                  {gui.new_session_places}
                </span>
                <div className="flex max-h-36 min-w-0 flex-col gap-1 overflow-y-auto">
                  {places.map((p) => {
                    // **The last component is the name, the rest is
                    // where it lives** — the same split the 已有的对话
                    // rows use, and the reason the path is not printed
                    // whole: the end of a path is what a person
                    // recognises, and it is the end a narrow dialog
                    // cuts off.
                    //
                    // Cutting from the *left* instead was tried and is
                    // wrong: `direction: rtl` reorders the neutral
                    // characters a path is made of, and printed
                    // `~/Desktop/working/tokens` as
                    // `Desktop/working/tokens/~`. Splitting says the
                    // same thing without asking the bidi algorithm
                    // anything.
                    const cut = p.shown.lastIndexOf("/");
                    const name = cut < 0 ? p.shown : p.shown.slice(cut + 1);
                    const parent = cut <= 0 ? "" : p.shown.slice(0, cut);
                    return (
                      <button
                        key={p.path}
                        type="button"
                        data-place={p.path}
                        data-on={dir === p.path}
                        // The whole path goes into the box, not the
                        // shortened one: `~` is how a person reads it
                        // and `openSession` does expand it, but a place
                        // is picked by identity and the identity is
                        // absolute.
                        onClick={() => setDir(p.path)}
                        className={cn(
                          "flex w-full min-w-0 items-baseline gap-2 rounded px-1 text-left text-xs hover:bg-accent",
                          dir === p.path && "bg-accent",
                        )}
                      >
                        <span className="shrink-0 truncate">{name}</span>
                        <span className="min-w-0 flex-1 truncate text-muted-foreground">
                          {parent}
                        </span>
                        <span className="shrink-0 tabular-nums text-muted-foreground">
                          {gui.new_session_places_count(p.sessions)}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
            </>
          )}
          {error && (
            <p data-new-session-error className="text-sm text-destructive">
              {error}
            </p>
          )}
        </div>
        <DialogFooter>
          {kind === "tell" ? (
            <Button data-tell-send disabled={!text.trim() || opening} onClick={send}>
              {gui.send_it}
            </Button>
          ) : (
            <Button data-new-session-create disabled={opening} onClick={create}>
              {gui.create}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
    {/* **A sibling of the wizard, not a child of its content.** Both are
        portalled, so what decides the stacking is Radix's dismissable
        layer stack either way; a sibling keeps the wizard's own DOM the
        shape every probe in `smoke` already walks.

        **It answers into the box.** A directory on another machine is
        two answers, so picking one that is not where the wizard is
        pointed moves 在哪台 with it — otherwise the box would hold a
        path that means nothing on the machine still selected, which is
        exactly what the machine row's own reset exists to prevent. */}
    <DirPicker
      open={picking}
      onOpenChange={setPicking}
      machine={machine}
      onPick={(pick) => {
        if (pick.machine !== machine) setMachine(pick.machine);
        setDir(pick.path);
      }}
    />
    </>
  );
}
