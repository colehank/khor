# Draft v2 — continuing work, and declaring workspaces (Windows)

Revision of v1 after review round 1. Written in English this time: the first
round read the Chinese draft through a GBK-decoding shell and got mojibake.

## 0. What changed from v1, and why

Round 1 was right on four points, and v2 adopts them:

1. **"Continue existing work" is the product, not "label rows better."** v1 called
   the owner's complaint a labeling problem. It is a *contract* problem: for a
   given session, on a given machine, which ways to continue it exist, and what
   control each one gives you.
2. **Read-only is a capability, not a lifecycle state.** v1 proposed a three-state
   row label (对话 / 终端 / 只读) that mixed presentation, interaction model and
   access into one exhaustive taxonomy. That collides with the six execution
   states. v2 keeps the six states untouched and treats "can I act on this?" as
   a separate, per-row **action set**.
3. **Declarations identify scope.** A workspace is not "a name for some
   directories"; it is a declared **scope**: exact (machine, path) members, and
   the right column is still an explicit machine+directory query over session
   cwd. A collection may legitimately store a human-given name; that is not an
   "exception" to any principle.
4. **Discovery must not overclaim.** The measured derivation is narrower than
   "all project directories with agent records", and the remote path silently
   turns refusal into absence. v2 states coverage out loud.

## 1. Facts, separated by how they are known

| Fact | Status |
|---|---|
| The machine has 3 shell sessions and 1 `observe/` record; with the Windows conversation filter on, one row is visible | measured |
| A khor-hosted live conversation is created by 新建会话 → Codex · 对话 and has an input box | measured (previous round) |
| `observe/` rows are imported vendor transcripts; their face is read-only **as a viewer** | implemented (`SESSION.md`: delegated sub-task viewers show no input box, no approvals) |
| Codex observations get a menu action 「在终端里继续」 → `codex resume <id>` in a terminal khor hosts, restricted to `kind === "observe" && !source && category === "codex"` | implemented (`SessionsList.tsx:~200`) |
| `agent_driver::resume_argv` already knows **two** vendors: `codex resume <id>` and `dsh tui --resume <id>` | implemented |
| The DSH record on this machine therefore has **no** continue action today, because the UI gate names only codex | measured + code |
| `khor agents tell` sends a message into a live codex thread | implemented |
| The codex ACP host supports `LoadSessionRequest` / `thread/resume`, i.e. a recorded codex thread can be **continued as a hosted conversation** | implemented in the host, **not exposed by any row action** |
| A phone or browser can send tasks/approvals/stops to a hosted session over the authenticated network | documented (`AGENT.md`) |
| Any of that has ever been exercised from a phone **on Windows** | **not verified** |
| `Node::places` = Claude records + Codex records + khor's own live records, folded to git roots; **DSH is not included**; no agent provenance | code |
| A remote `Reach::Places` that refuses is translated into an empty list, deliberately ("a refusal translated into an absence", `link.rs`) | code, documented |

## 2. Part A — "continue this work" as a contract

For every session row, the question is not "what label is it" but **what can I do
with it, here and from my phone**. The concrete routes that exist or are within
reach:

| Row | Route | What you get | Control |
|---|---|---|---|
| live khor conversation (`gui`) | open it | input box, approvals, stop | full, from any device |
| live terminal (`shell`/`tui`) | attach | keyboard | full, from any device |
| vendor observation **with a native id** (codex, dsh) | 「继续」→ resume argv in a khor-hosted terminal | the vendor's own conversation continues in a terminal | terminal control; **not** the vendor's running UI |
| vendor observation **with a native id** (codex only) | 「接手为对话」→ `LoadSessionRequest` into the hosted ACP conversation | a live khor conversation continuing that thread | full (input, approvals, stop) |
| vendor observation **without a native id** | none | read only | none |
| any row on an offline machine | none (refused with the machine's own reason) | declared as unreachable | none |

**A1.** Rows expose only the actions their session supports; the action set is the
contract. "Read-only" is shown where the set is empty — as a capability hint, not
as a state word (the six states stay as they are).

**A2.** The cheapest real gap is the DSH resume gate: the argv exists, the UI gate
names one vendor. Widening it is small and makes the owner's own record
continuable today.

**A3.** The bigger, more valuable gap is "continue as a conversation" for codex
threads: the host supports it, no surface offers it. This is what "remote control
of an existing conversation" actually means.

**A4.** Remote control must be *stated* where it applies: a live conversation or
terminal says which machine runs it and that another device may continue it. An
observation says who holds it and via which route it can be continued (or that
none exists).

**A5.** Withdraw v1's "empty state = no conversations". Imported sessions are
sessions; the hint may say 「没有正在进行的对话」 and offer 派发, but it must not
claim the list is empty.

## 3. Part B — declaring workspaces from discovered locations

**B1. Discovery is of *locations*, not of workspaces.** Candidates come from each
added machine's recorded activity; the UI must say what it actually covers:
which agents were read, that paths were folded to git roots, and that a machine
which refuses contributes nothing (rather than "zero").

**B2. Declaration = selecting exact members.** Creating a workspace means choosing
one or more (machine, path) members and naming the scope. Optional: a manual
directory with no agent history at all. Everything undeclared stays usable in the
session list.

**B3. Collections are scopes, not containers of sessions.** A named collection
with N members scopes the same session list; a session belongs to a collection
because its cwd is under a member on that member's machine — not because it was
"added". Overlapping collections are allowed and both show the session; the right
column remains an explicit machine+directory query, deduplicated by session id.

**B4. ＋ must target one concrete member.** A collection-level ＋ is ambiguous
(which folder does the new conversation run in?). ＋ belongs on a member, or asks
for one; the collection page states that choice instead of guessing.

**B5. No silent suppression.** Low-frequency locations stay visible (perhaps
collapsed), temporary-looking paths are shown as they are, and nothing is hidden
because it "looks like noise". Dismissal is explicit and reversible.

**B6. The 2026-09-27 ruling.** That ruling removed path recommendations from the
＋ sheet ("简单一个输入框"). Discovery here must not become a recommendation list in
that sheet: it belongs to a separate, explicitly entered workspace flow, and the
＋ sheet keeps its empty box.

## 4. Windows prerequisites (before either part ships)

1. **Path identity.** `sessionInWorkspace` compares strings today; separator,
   case, extended-length prefix (`\\?\`) and root boundary differences can empty
   the right column silently. Normalise, and apply **the execution machine's**
   rules, not the viewer platform's.
2. **Absolute paths on the wire.** Display forms like `~/…` must never be typed
   into `cmd.exe`; resume/open commands carry absolute paths for the target shell.
3. **Vendor resolution.** `vendor_bin` covers two shims today; with Codex
   installed outside `PATH`, every route that spawns a vendor program must use it
   (takeover preflight, conversation-to-terminal, native send, wizard terminal
   choice).
4. **No tmux assumption.** There is no attachment route into an arbitrary
   existing terminal on Windows; khor must host the session it continues.

## 5. Staging (revised)

- **Stage 0 — prerequisites:** §4.1–4.3, plus the path-identity unit checks.
- **Stage 1 — continue:** widen the resume gate to every vendor `resume_argv`
  knows (A2); expose 「continue as a conversation」 for codex threads (A3);
  state the route on the row/detail (A4). **Verified end-to-end on Windows with a
  phone**, not just rendered.
- **Stage 2 — declare:** the workspace page's declaration entry: discovered
  locations with stated coverage, multi-select exact members, name, create (B1–B2).
- **Stage 3 — collections:** multi-member scopes in the right column and the
  session list (B3–B5).
- **Cut from v1:** "business-first" global grouping of the session list (round 1
  was right: it changes the list's meaning before the scope model exists).

## 6. Acceptance cases (must be described before implementing)

1. A live vendor thread: what a person can do from the desktop, and from a phone.
2. A historical transcript with a native id: continue in a terminal; continue as a
   conversation (codex only); what identity the new session has.
3. A historical transcript whose native id is missing: only reading, stated.
4. An offline machine: every route refuses with that machine's own reason.
5. An empty workspace store: declare from discovered locations, or manually, or
   leave everything undeclared — all three are valid.

## 7. Questions for round 2

1. Is the route table (§2) complete? Which continuation routes am I missing
   (approvals? stop? a thread that is *currently running* elsewhere)?
2. Is "capability, not state" the right model, and where should the capability be
   rendered so it does not read as a second state vocabulary?
3. How should coverage be disclosed without turning the panel into a warning wall?
4. Is Stage 1 (continue) really the right first product step on Windows, or does
   declaration come first?
5. What in v2 still overclaims, and what is missing from the acceptance cases?
