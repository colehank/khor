# Draft v3 (final plan) — continuing work, and declaring scopes (Windows)

Revision after review round 2. Round 2 said: direction sound; execution ownership,
discovery completeness and the workspace rollout still lack enforceable contracts.
v3 answers those three, and keeps the route table as the centre of the plan.

## 1. Model decisions (resolving round-2 flaw 3)

**One scope model: a workspace IS a named scope over one or more (machine, path)
members.** There is no second "collection" layer.

- A single-directory declaration is the degenerate case: one member.
- Existing rows in `khor_sync::workspaces` (which identify individual
  machine/path pairs) **migrate as one-member scopes**, keeping the directory
  basename as the name. The doc gains a version; an older khor reading a newer
  doc ignores the unknown field (`SESSION.md` openness).
- Operations, specified now rather than at Stage 3:
  - **rename**: name is stored, so rename is allowed; members are unaffected.
  - **add member**: any (machine, path) that exists on that machine; a member may
    be added twice only if it is the same pair (deduplicated by pair).
  - **remove member**: removes the scope edge only. **No file, no session, no
    directory is touched**; sessions under a removed member simply stop appearing
    in that scope (they stay in the session list under their place).
  - **delete scope**: same — edges only.
  - **empty scope**: allowed and visible ("0 folders"), because a scope under
    construction is a real state; it is not hidden.
  - **overlapping scopes**: allowed. A session appears in every scope whose member
    contains it, deduplicated by session id; the right column is always the
    explicit query "sessions whose cwd is under member M on member M's machine".
- Stage 2 therefore ships **single-member declarations only** (independently
  usable), and Stage 3 adds members. No stage ships multi-member creation before
  the query semantics exist.

## 2. Part A — the continuation contract (round-2 flaw 1)

**A continuation route is a promise about one operation, on one machine, with named
controls.** Native identity alone is not ownership. Every route below must state:

| field | meaning |
|---|---|
| identity | the session row's id, and the vendor's native id when it has one |
| execution machine | where the process runs (a device id, never a display name) |
| current writer | the process that is writing now: khor's hosted session, the vendor's own daemon, a terminal, or **none/unknown** |
| ownership transition | what happens to the writer when this route is taken (acquire, queue, refuse) |
| resulting identities | ids the operation creates (a new hosted session id; the native id it continues) |
| readiness | what must be true before the button is enabled |
| failure | the sentence a person sees, per concrete reason |
| controls | exactly which of these it gives: send, approve, interrupt turn, terminate process, keyboard input |

**Route table (proposed; each row must be marked implemented / proposed / verified):**

| Row | Route | Controls | Ownership |
|---|---|---|---|
| live khor conversation (`gui`) | open | send, approve, interrupt, terminate | khor-owned; one writer; remote devices share it |
| live terminal (`shell`/`tui`) | attach | keyboard input, terminate | khor-owned pty; one writer |
| observed codex thread, not running | resume in vendor TUI (`codex resume <id>`) | keyboard input | vendor process; khor hosts the window |
| observed codex thread, not running | **continue as hosted conversation** (`LoadSessionRequest` / `thread/resume`) — *proposed; host supports it, no surface offers it* | send, approve, interrupt | khor acquires; the vendor daemon must not be running that thread |
| observed codex thread, **running elsewhere** | **refused** with the reason (writer occupied) — *not offered* | none | — |
| observed dsh thread | resume in vendor TUI (`dsh tui --resume <id>`) — *proposed: widen the gate; argv exists* | keyboard input | vendor process |
| observed dsh thread | send a turn (`khor agents tell`'s dsh lane: headless turn) — *implemented, gate widening needed* | send | khor's headless turn; the vendor's own store is written |
| observed thread with no native id | none; "viewing transcript only" | none | — |
| any row on an offline machine | refused; **report the connection failure** (a machine cannot report its own current reason while offline) | — | — |

**Explicitly not offered** (stated in the UI when relevant, not silently absent):
attaching to an arbitrary external Windows terminal; approvals/stops for a process
khor does not own; automatic migration of a conversation to another machine.

**Capability rendering.** "Read-only" is used only when the content is readable and
the **known control set is empty**. Rename/pin/copy do not count as control.
Offline, checking, missing credentials and an occupied writer each get their own
sentence next to the disabled action — an empty button set cannot distinguish
them. The six lifecycle words and their rendering stay exactly as they are; the
capability shows as concrete verbs (the action set), not as a second state word.

**A5 (empty state), corrected.** The list says 「没有匹配的会话」 when the filter
hides rows, and 「还没有派发过任务」 only when activity was actually checked and
none exists. Imported sessions are sessions; the hint never claims the list is
empty.

## 3. Part B — discovery: outcome metadata before rendering (round-2 flaw 2)

Disclosure cannot repair discarded evidence, so **discovery must carry its own
outcome as data**, and the panel renders that data:

```
Discovery {
  machines: [{ device, status: answered|unreachable|refused, why, checked_at }]
  rows:     [{ device, raw_path, folded: bool, session_count, last_ms,
               agents: [claude|codex|…|unknown], hit_scan_limit: bool,
               root_missing: bool }]
  limits:   { vendor caps applied, transcripts sampled for cwd }
}
```

- **Compact summary above results**: 「依据 Claude、Codex 与 khor 记录 · 按仓库归并」
  plus 「已查 2/3 台机器」.
- **Expand** to show: DSH not scanned yet; Codex scanned at most N rollouts; cwd
  taken from a sample of Claude transcripts; paths whose root is missing; per
  machine failure with retry.
- **A refusing machine is "未查"**, never "没有" — the remote response needs to
  carry the reason (today `link.rs` turns every refusal into an empty list; that
  must become a stated outcome first).
- **Folded paths are marked**, and the raw path remains available; a non-repository
  fallback name is shown when folding found no root.
- Discovery only *proposes locations*. Creating a scope = selecting exact
  (machine, path) members and naming it. Manual entry for a directory with no
  history stays available. Dismissal is explicit and reversible. The ＋ sheet
  keeps the owner's 2026-09-27 rule (an empty box, no recommendations).

## 4. Windows prerequisites (Stage 0)

1. **Path identity**: normalise separator, case, `\\?\` prefix, trailing
   separator, root boundary; compare with **the execution machine's** rules. Fix
   `sessionInWorkspace` before anything ships that depends on it.
2. **Absolute paths on the wire**: display forms must never be typed into
   `cmd.exe`.
3. **Vendor resolution**: every spawn path uses `vendor_bin` (today: two shims;
   Codex is installed outside `PATH`).
4. **No tmux assumption**: khor hosts the session it continues; there is no
   attach route into an arbitrary existing terminal on Windows.

## 5. Staging

- **Stage 0** — prerequisites (§4). Small, testable, no product decisions.
- **Stage 1** — continuation contract: widen the gate to every vendor whose
  `resume_argv`/send lane exists (dsh resume + send; codex already), add
  "continue as hosted conversation" for codex threads, render the route with its
  controls and the not-offered list. **Verified Windows→phone**: one route, proven,
  not just rendered.
- **Stage 2** — declaration: workspace page entry with discovery outcome metadata,
  single-member creation (independently usable), migration of existing rows.
- **Stage 3** — members: add/remove/rename/delete/empty scopes, the right column
  as the explicit per-member query, overlapping scopes.
- **Cut**: "business-first" global grouping of the session list.

## 6. Acceptance cases

Existing five (live thread, historical transcript, missing native id, offline
machine, empty store) plus:

1. simultaneous desktop and phone requests on one hosted conversation;
2. a lost acknowledgement and a retry, with no duplicate execution;
3. an active or unknown external writer;
4. missing executable / credentials / cwd;
5. preserved history and identities across a continuation;
6. approval and stop across a disconnect;
7. partial discovery vs a successful zero;
8. overlapping scopes, Windows path boundaries, member-targeted creation, and
   scope removal that deletes no files or sessions.

## 7. What is still unverified

- Windows↔phone behaviour of any route (all routes above are source-level).
- Hosted codex continuation against a thread that a vendor daemon is currently
  running.
- Discovery outcome metadata: today's wire has no field for it, so this is work,
  not rendering.
- The owner's phone pairing on this machine.

## 8. Question for round 3

1. Is the per-operation contract in §2 now enforceable — anything still
   unstated that a button would promise?
2. Is "one scope model, migrate single-member" the right resolution of round-2
   flaw 3?
3. Is the discovery `Discovery` outcome shape sufficient to distinguish partial
   from empty everywhere it will be shown?
4. What would you cut from Stage 1 to reach a provable Windows→phone route
   sooner?
