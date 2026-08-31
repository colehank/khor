// The wizard (会话身份批B): opening a session is the first act, and it
// asks the four questions of the ruling — where (目录), which agent
// (智能体), which form (对话 or 终端 — the same two words the detail's
// face switch wears), and what to call it (optional; the directory
// names it otherwise). The row appears in the list like any other; the
// vendor's own uuid is its id on the chat form, so hooks and the disk
// sweep merge into it with no ceremony.
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
  openSession,
  registerKnownAgent,
  resumeSession,
  type AgentRow,
  type AgentSaid,
  type KnownAgent,
} from "@/api";
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

export function NewSessionDialog({
  open,
  onOpenChange,
  onOpened,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The freshly-minted session id — the caller may select its row. */
  onOpened: (id: string) => void;
}) {
  const [dir, setDir] = useState("~");
  const [title, setTitle] = useState("");
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
    setDir("~");
    setTitle("");
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
  }, [open]);

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

  const create = () => {
    if (opening) return;
    setOpening(true);
    setError(null);
    openSession(dir, title, form, agent)
      .then((id) => {
        onOpenChange(false);
        onOpened(id);
      })
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
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-new-session-dialog>
        <DialogHeader>
          <DialogTitle>{gui.new_session}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_dir}</span>
            <Input
              data-new-session-dir
              value={dir}
              onChange={(e) => setDir(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && create()}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_name}</span>
            <Input
              data-new-session-name
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && create()}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.new_session_agent}</span>
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
              <div data-new-session-registry className="flex flex-col gap-1">
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
                <div className="flex max-h-56 flex-col gap-1 overflow-y-auto">
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
                        className="flex items-baseline gap-2 text-xs"
                      >
                        <span className="shrink-0">{k.id}</span>
                        <span className="min-w-0 flex-1 truncate text-muted-foreground">
                          {k.name}
                        </span>
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
          </label>
          {/* **Only for an agent that was registered.** khor finds
              claude's and codex's conversations by reading their files;
              this section is the *other* discovery path, and pointing
              it at them would paint 「还没问过」 about a question that
              does not apply to them. */}
          {registered(agent) && (
            <div data-agent-seen data-said={mark} className="flex flex-col gap-1 text-sm">
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
                <div className="flex max-h-40 flex-col gap-1 overflow-y-auto">
                  {rows.map((r) => (
                    <button
                      key={r.id}
                      type="button"
                      data-seen-row={r.id}
                      disabled={opening}
                      onClick={() => resume(r.id)}
                      className="flex items-baseline justify-between gap-2 text-left text-xs hover:bg-accent"
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
                      <span className="shrink-0 text-muted-foreground">
                        {r.updated_at ?? ""}
                      </span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
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
          {error && (
            <p data-new-session-error className="text-sm text-destructive">
              {error}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button data-new-session-create disabled={opening} onClick={create}>
            {gui.create}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
