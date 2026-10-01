import { useMemo, useState } from "react";

import { StatusWord } from "@/components/StatusWord";
import { takeover, takeoverTerm, type SessionRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { sessionMark } from "@/components/BrandIcons";
import { type RowAction } from "@/components/RowMenu";
import { SideList, type SideRow } from "@/components/SideList";
import { EditableName } from "@/components/EditableName";
import { IconDevices } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { useSetting } from "@/settings";
import { closeWarning } from "@/views/DetailPane";
import { sieve } from "@/lib/list-filter";
import { SESSION_SIEVE } from "@/lib/session-filter";
import { AXES, ago, groupLabel, placeLabels, quietFor } from "@/words";

/**
 * Which rows the pane bar's search and filter leave standing.
 *
 * The filter matches on the state **key**, the one the node sent — never
 * on the displayed word, and never on a state this layer worked out for
 * itself (docs/UX.md 状态呈现). Search reads what the row shows plus its
 * id, since the id is what the CLI prints and what people paste.
 *
 * **The machine a row came from is part of what the row shows**, so it
 * is part of what search reads — on a list gathering sessions from
 * several machines, "which ones are on turing" is the obvious question
 * to ask a search box, and it used to come back empty.
 *
 * Only reported rows carry that name, which is the same rule as
 * everything else here rather than a gap: `source` is the offline axis,
 * set on rows another device told us about, and a row living on this
 * machine prints no machine name. Search matches what is on the screen —
 * so a local row is not findable by this machine's name, because the row
 * never says it.
 */
export function visibleSessions(rows: SessionRow[], query: string, ticked: string[]) {
  const q = query.trim().toLowerCase();
  // **AND across axes, OR within one** — every list's rule, in one place
  // (`lib/list-filter`'s `sieve`), against what each row answers on the
  // four axes (`SESSION_SIEVE`: the node's keys, never the words drawn).
  return sieve(rows, ticked, SESSION_SIEVE, AXES).filter(
    (r) =>
      q === "" ||
      `${r.title} ${r.id} ${r.source?.device ?? ""} ${r.place ?? ""}`
        .toLowerCase()
        .includes(q),
  );
}

// Order and words come from the node untouched: the list never
// re-derives a judgment the backend already made (docs/UX.md). That
// includes the order pinned rows arrive in — `Node::sessions` floats
// them, this file only paints the mark. **There is no comparison
// function in here, and adding one is the bug.**
export function SessionsList({
  rows,
  loading = false,
  query,
  words,
  selected,
  onSelect,
  onPin,
  onClose,
  pinFailed,
  closeRefused,
  showVia = true,
}: {
  rows: SessionRow[];
  /** Not answered yet: a skeleton rather than 「there are none」. */
  loading?: boolean;
  query: string;
  words: string[];
  selected: string | null;
  onSelect: (row: SessionRow) => void;
  /** **Rejects when the write did not take** — that is how the menu
      knows to stay open on the item that was pressed, instead of
      closing over a row that did not move (`RowMenu`, `App`). */
  onPin: (row: SessionRow) => Promise<void>;
  /** Rejects the same way, and for the same reason; a refusal also
      lands in `closeRefused` as a whole sentence. */
  onClose: (row: SessionRow) => Promise<void>;
  /** Rows whose last pin attempt did not take — see `App`. */
  pinFailed: Set<string>;
  /** What the node said when it would not close a row, by row id. */
  closeRefused: Map<string, string>;
  /** Whether a row says which machine it came through (工作区批③).
      **Off where the surface already said it once**: the workspace
      pane's column is one machine's directory, named in the header
      directly above it, and a face repeated on every row underneath is
      the same fact said twice — which docs/UX.md spends on nothing.
      On by default, because the sessions landing gathers rows from the
      whole mesh and there the machine is the thing the eye scans by. */
  showVia?: boolean;
}) {
  /** What the node said when it would not change a row's form, by row
      id. Owned here rather than in `App` because nothing else needs
      it: unlike a pin, the conversion has no optimistic state to keep
      — the row comes back in its new form on the next poll, or it does
      not come back at all and this sentence says why. */
  const [turnRefused, setTurnRefused] = useState<Map<string, string>>(new Map());
  // The ticked words narrow here; the box is `SideList`'s one rule,
  // against what each row says it can be found by (`find` below) — the
  // same four things `visibleSessions` reads, so the two agree.
  const ticked = visibleSessions(rows, "", words);
  // Place headings, told apart where two checkouts share a name
  // (`placeLabels`). Over the rows on screen, because those are the
  // groups that can collide with each other.
  const placeNames = useMemo(() => placeLabels(ticked.map((r) => r.group)), [ticked]);
  /**
   * What a row can be told to do, on right-click and on long-press.
   *
   * **Two items, and the second one is why there is a menu at all.** A
   * menu holding one thing is a menu that teaches people not to open
   * menus, which is the same judgment `App` makes about an empty "+".
   *
   * **Rename is deliberately not here.** It already has a way in — the
   * title's own double-click and F2 (`EditableName`) — and a second
   * path to one action is the thing this repo keeps taking back out.
   * The two that are here had no way in that a finger could reach:
   * the pin was a hover-only button, and close lived one screen away in
   * the detail header. 复制 id came the same road on 2026-09-02 — the
   * detail header sheds it (「id用户知道也没用」), and its one real
   * reader, a person wiring this session up from another machine,
   * finds it here beside the other things done *to* a row.
   */
  // 危险动作二次确认 · 接管会话 (docs/MORE.md 安全, the key lands in
  // `khor_node::config`). **Only an explicit 「off」 takes the question
  // away.** `useSetting` answers `null` while khor has not said — and
  // that has to read as 「ask」, which is what this key's default is:
  // a guard that vanished because an answer was late would be the one
  // failure this switch must not have.
  const askTakeover = useSetting("confirm-takeover") !== "off";

  /**
   * Changing what a session **is**: the same act both ways round, and
   * the row's menu is where it belongs.
   *
   * **It used to be a tab.** The detail pane carried a 终端/对话 strip
   * whose second face, on a conversation, was a card reading "there is
   * no terminal here, press this to make one" — a conversion wearing a
   * view's clothes, on a strip that cost every session two controls.
   * Now the pane shows the form the host is actually running
   * (`DetailPane`) and this is the one way to change it, beside 置顶
   * and 关闭 which also change a row rather than look at one.
   *
   * **Absent, not greyed, where it cannot happen.** A row on another
   * machine, a shell with no conversation to have, a vendor khor has
   * no shim for: none of them has a second form, so none of them gets
   * a word about one.
   *
   * Both directions carry a confirm, because both end something that
   * is running: which form is ending is the one thing a person must
   * not have to guess from the label (`takeover_*` in the catalog).
   */
  const turnInto = (r: SessionRow): RowAction[] => {
    const refused = turnRefused.get(r.id);
    const said = (e: unknown) =>
      setTurnRefused((m) => new Map(m).set(r.id, String(e instanceof Error ? e.message : e)));
    // Somebody else's machine hosts it; the act happens where the
    // process is.
    if (r.source) return [];
    if (r.kind === "gui") {
      return [
        {
          key: "to-a-terminal",
          label: gui.to_a_terminal,
          confirm: !askTakeover
            ? undefined
            : r.word === "busy"
              ? gui.takeover_term_busy
              : gui.takeover_term,
          note: refused,
          run: async () => {
            try {
              await takeoverTerm(r.id);
            } catch (e) {
              said(e);
              throw e;
            }
          },
        },
      ];
    }
    // Only a vendor khor can speak for has a conversation to move
    // into — the same judgment the offer inside the read-only chat
    // used to make.
    if (r.category !== "claude") return [];
    return [
      {
        key: "to-a-chat",
        label: gui.to_a_chat,
        confirm: !askTakeover ? undefined : r.word === "busy" ? gui.takeover_busy : gui.takeover_calm,
        note: refused,
        run: async () => {
          try {
            await takeover(r.id);
          } catch (e) {
            said(e);
            throw e;
          }
        },
      },
    ];
  };
  // **置顶 first, this column's own, then 删除 last behind a line**
  // (2026-09-28 audit 第 16 条, Apple HIG; it was 置顶, 删除 first —
  // docs/SHELL.md 第二轮). Every sidebar's menu opens on the same word
  // and closes on the same word, so a hand that learned them on one list
  // finds them on the next — and the one that cannot be taken back is no
  // longer between 置顶 and 复制 ID.
  const actions = (r: SessionRow): RowAction[] => [
    {
      key: "pin",
      // The word is what pressing it *does*, so it is the opposite of
      // the row's current state — which is also what makes this item
      // say what the state is. When the last attempt did not take, the
      // word becomes the one for what failed, said on the very thing
      // that was pressed (docs/UX.md 做了但没变化 / 失败). It tracks the
      // action that was **attempted**, so it stays the opposite of
      // `r.pinned` read off the un-refreshed row.
      label: pinFailed.has(r.id)
        ? r.pinned
          ? gui.unpin_failed
          : gui.pin_failed
        : r.pinned
          ? gui.unpin
          : gui.pin,
      failed: pinFailed.has(r.id),
      run: () => onPin(r),
    },
    ...turnInto(r),
    {
      // The id is paste material, not reading material. The menu
      // closing is the success face; a clipboard refusal rejects, so
      // the menu stays open where the press was made (RowMenu's rule:
      // rejecting is how an action says it did not happen).
      key: "copy-id",
      label: gui.copy_id,
      run: () => navigator.clipboard.writeText(r.id),
    },
    {
      key: "close",
      // 删除, and what deleting *this* row does is the sentence
      // between the two presses — this row's own, from the one place
      // that map lives (`DetailPane.closeWarning`). Writing the kinds
      // out again here would be a second source that compiles, runs,
      // and says the wrong thing.
      label: gui.delete,
      askLabel: gui.delete_ask,
      destructive: true,
      ask: closeWarning(r.kind, r.imported, r),
      note: closeRefused.get(r.id),
      run: () => onClose(r),
    },
  ];
  const Mark = (r: SessionRow) => {
    const M = sessionMark(r.kind, r.category);
    return <M className="size-kind-mark-glyph" />;
  };
  const side: SideRow[] = ticked.map((r) => ({
    id: r.id,
    group: r.group,
    pinned: r.pinned,
    // The machine's face carries the row; the halo is the state, the
    // corner the kind (`FaceMark`, which `SessionFace` also wears).
    face: <MachineAvatar face={r.face} className="size-avatar" />,
    halo: r.word,
    badge: { mark: r.category ?? r.kind, icon: Mark(r), data: { "data-kind-mark": r.category ?? r.kind } },
    // Double-click the title to rename it in place — the component the
    // detail header wears, so the two cannot drift. `nested`: it sits
    // inside the button that opens the row.
    title: (
      <EditableName
        id={r.id}
        title={r.title}
        className="block font-title"
        inputClassName="w-full font-title"
        nested
      />
    ),
    name: r.title,
    // The preview line: the newest thing said or shown, and the machine
    // a reported row came from — both 「where this row has been」.
    sub:
      r.last || r.source ? (
        <span className="flex items-baseline gap-2 overflow-hidden whitespace-nowrap">
          {r.last && (
            <span data-last className="min-w-0 flex-1 truncate">
              {r.last}
            </span>
          )}
          {r.source && (
            <span data-row-source className="flex-none">
              {gui.from_device(r.source.device)}
            </span>
          )}
        </span>
      ) : undefined,
    // The right column: the state word (this row's `fact`) over the age
    // (`aux`) — state is never colour alone, so the word stays.
    //
    // **The word ends the line, so it and the age share one right edge
    // down the whole list** (2026-09-28 audit 第 7 条). The machine chip
    // and the unread count used to come after it, and a row that carried
    // one had its word 22px left of the rows that did not — a column of
    // words that did not line up. Whatever else rides this line stands
    // to the word's left.
    trailing: (
      <span className="flex flex-none flex-col items-end gap-xxs text-caption text-tertiary">
        <span className="flex items-center gap-xxs">
          {showVia && r.via && (
            <span
              data-via={r.via}
              data-via-glyph={r.via_face ? undefined : ""}
              className="grid size-kind-mark flex-none place-items-center"
            >
              {/* **No face, the machine glyph** — not `MachineAvatar`'s
                  blank. At a row's 32 the blank holds a seat until the
                  face lands; at 18 beside a word it read as a picture
                  that failed to load (audit 第 7 条). A thing with no
                  identity wears a glyph, never an invented face
                  (components/Avatar「要缩写的场合…用一个字形」). */}
              {r.via_face ? (
                <MachineAvatar face={r.via_face} className="size-kind-mark" />
              ) : (
                <IconDevices className="size-kind-mark-glyph" />
              )}
            </span>
          )}
          {r.unread > 0 && (
            <span
              data-unread
              className="min-w-4 rounded-full bg-badge px-1 text-center leading-4 text-badge-foreground"
            >
              {r.unread}
            </span>
          )}
          <StatusWord word={r.word} />
        </span>
        {r.at_ms > 0 &&
          (r.quiet ? (
            <span data-quiet-note className="text-callout">
              {quietFor(r.at_ms)}
            </span>
          ) : (
            <span className="text-callout tabular-nums">{ago(r.at_ms)}</span>
          ))}
      </span>
    ),
    find: `${r.title} ${r.id} ${r.source?.device ?? ""} ${r.place ?? ""}`,
    selected: r.id === selected,
    dim: r.quiet,
    old: r.stale,
    menu: actions(r),
    onOpen: () => onSelect(r),
    // The anchors this list has always answered to — the gates and the
    // smoke read them, and they are facts about a session, not about a
    // sidebar.
    rowData: {
      "data-row": r.id,
      // The row's shape (shell / tui / gui / chat …): what an import
      // landed as is a fact the gates read (docs/SHELL.md 第八轮).
      "data-kind": r.kind,
      "data-imported": r.imported,
      "data-mux": r.mux,
      "data-word": r.word,
      "data-pinned": r.pinned,
      "data-quiet": r.quiet,
      "data-source": r.source?.device,
      "data-home": r.home,
      "data-title": r.title,
    },
    openData: { "data-row-open": "" },
  }));
  return (
    <SideList
      rows={side}
      loading={loading}
      query={query}
      filtered={words.length > 0}
      label={(g) => placeNames.get(g) ?? groupLabel(g)}
      // 「Nothing here」 and 「nothing matched」 are different facts.
      // A tick that left nothing is 「nothing matched」 too.
      empty={rows.length === 0 ? gui.empty_sessions : gui.no_matches}
      none={gui.no_matches}
      old={{ more: (n) => gui.old_sessions(n), less: gui.collapse_old }}
    />
  );
}
