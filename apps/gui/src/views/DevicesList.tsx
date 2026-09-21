import type { DeviceRow } from "@/api";
import { EditableName } from "@/components/EditableName";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { connectionWord } from "@/words";

/** Search matches the name people call the machine and the id they
    paste. The id left the row's face (2026-09-02 — the card carries it
    now), but it is still the string people hold, so it still matches. */
export function visibleDevices(rows: DeviceRow[], query: string) {
  const q = query.trim().toLowerCase();
  if (q === "") return rows;
  // **Both names are searchable.** A renamed machine must be findable
  // by what it is called now, and one somebody else renamed must still
  // answer to the name this person knew it by — neither is the wrong
  // thing to type.
  return rows.filter((d) => `${d.shown} ${d.name} ${d.id}`.toLowerCase().includes(q));
}

/**
 * The machine list. Three panes drew it once — devices, files and
 * browser — because "which machine" is the first step of all three;
 * the latter two grew their own starting points and only the devices
 * pane draws this now.
 *
 * # Whether a row goes anywhere is the caller's to say
 *
 * The rule has not changed since the batch that wrote it down: **no
 * affordance may suggest a destination that does not exist**, because
 * one that answers nothing teaches people to stop trying the ones that
 * do. What changed is the input — the devices pane now opens a machine
 * card, so on that pane the row is a button, lights under the pointer
 * and is reachable by keyboard. A pane without a second step would pass
 * no `onOpen` and its rows would stay inert.
 *
 * That is why this is a prop and not a flag read from a landing name:
 * the thing that decides is "is there somewhere to go", and the only
 * code that knows it is the code that would take you there. A pane that
 * grows a destination turns its rows into buttons by handing one over,
 * and there is no way to get the button without it.
 *
 * # The row's actions are its menu, not a button on it
 *
 * The pin sat on the row's right end as its own button until 2026-09-06
 * (「机器左侧 list,与 session 一样,是右键,置顶或删除,取消现在的置顶
 * icon」). It is the session row's `RowMenu` now — right-click, a long
 * press, the keyboard's Menu key — for the reason that file gives: a
 * control that lives on the row is one a phone has no room for, and a
 * menu is one gesture for one job on both lists. What the pin's shape
 * used to say about the row's state, the menu's first item says at the
 * moment somebody asks: it reads 置顶 or 取消置顶. The list still floats
 * a pinned machine to the top, which is the node's own order and not
 * this file's.
 *
 * The second item is `khor forget` — the machine leaves the network,
 * for everyone (`Node::forget_device`). Two presses, and the sentence
 * between them is the catalog's; the refusal, when there is one, is the
 * node's own and stays on the menu where the press was made.
 */
export function DevicesList({
  rows,
  query,
  onPin,
  onForget,
  onOpen,
  selected,
  pinFailed,
  forgetRefused,
}: {
  rows: DeviceRow[];
  query: string;
  /** Resolves when the pin took and rejects when it did not — the menu
      reads which to decide between closing and staying open (`RowMenu`). */
  onPin: (row: DeviceRow) => Promise<void>;
  /** Shows the machine the door. Same contract as `onPin`: a rejection
      is how the menu learns the machine is still here. */
  onForget: (row: DeviceRow) => Promise<void>;
  /** Where a row goes, when it goes anywhere. Absent on a pane that
      has no second step — see above. */
  onOpen?: (row: DeviceRow) => void;
  selected?: string | null;
  /** Rows whose last pin attempt did not take — see `App`. */
  pinFailed: Set<string>;
  /** The node's sentence for a forget it refused, by row — cleared the
      moment a later attempt on that row gets anywhere (`App`). */
  forgetRefused: Map<string, string>;
}) {
  const shown = visibleDevices(rows, query);
  // An empty device table cannot happen — this machine is always in it —
  // so the only way to reach zero rows is by filtering, and that gets
  // said in the words for it rather than borrowing the sessions pane's.
  if (shown.length === 0) {
    return rows.length === 0 ? null : (
      <div data-empty className="p-4 text-sm text-muted-foreground">
        {gui.no_matches}
      </div>
    );
  }
  const actions = (d: DeviceRow): RowAction[] => [
    {
      key: "pin",
      // The word is what pressing it *does*, so it is the opposite of
      // the row's current state — which is also what makes this item
      // say what the state is. When the last attempt did not take, the
      // word becomes the one for what failed, said on the very thing
      // that was pressed (docs/UX.md 做了但没变化 / 失败).
      label: pinFailed.has(d.id)
        ? d.pinned
          ? gui.unpin_failed
          : gui.pin_failed
        : d.pinned
          ? gui.unpin
          : gui.pin,
      failed: pinFailed.has(d.id),
      run: () => onPin(d),
    },
    // **Not on this machine's own row.** `Node::forget_device` refuses
    // it by name — a table cannot be left from the inside — so the
    // item would exist only to be refused, an offer whose one answer
    // is no.
    ...(d.me
      ? []
      : [
          {
            key: "forget",
            // **删除, the word every sidebar's second item says**
            // (docs/SHELL.md 第二轮:「置顶与删除的操作统一为置顶与删除」),
            // and the map's card says it too — one act
            // (`Node::forget_device`), one word. What deleting a
            // machine *means* (it leaves the network, for everyone) is
            // the sentence between the two presses.
            //
            // **No 断开 after it.** The ruling named one for machines;
            // khor has no act that drops a single peer — `forget` is
            // this item and `pause` is the whole machine — and an item
            // with nothing behind it is not offered.
            label: gui.delete,
            confirm: gui.forget_machine_confirm,
            note: forgetRefused.get(d.id),
            run: () => onForget(d),
          },
        ]),
  ];
  return (
    <div>
      {shown.map((d) => (
        <div
          key={d.id}
          data-device={d.name}
          data-row={d.id}
          data-pinned={d.pinned}
          data-me={d.me}
          className={cn(
            "flex items-center",
            // Only a row that opens something reacts to the pointer.
            onOpen && "hover:bg-secondary",
            onOpen && d.id === selected && "bg-accent hover:bg-accent",
          )}
        >
          {/* **The menu's trigger is the button, not the strip around
              it** — the session row's arrangement, for its reason: the
              trigger carries an `onPointerDown` (that is the long
              press), and a press handler on a bare `<div>` is a target
              only a pointer can find (`check-face`'s section 4). On the
              button it sits on something the DOM already admits is a
              target: focus reaches it, and a keyboard's Menu key fires
              `contextmenu` on the focused element.

              The inert branch offers no menu, and that is not an
              oversight: a span is not a target either, and a pane with
              nowhere to send a row is a pane that has not earned
              actions on it (the rule at the head). No pane draws that
              branch today. */}
          {onOpen ? (
            <RowMenu items={actions(d)}>
              <button
                type="button"
                data-row-open
                onClick={() => onOpen(d)}
                className="flex min-w-0 flex-1 items-center gap-3 py-2 pr-2 pl-4 text-left"
              >
                <Identity row={d} rename />
              </button>
            </RowMenu>
          ) : (
            <span className="flex min-w-0 flex-1 items-center gap-3 py-2 pr-2 pl-4">
              <Identity row={d} />
            </span>
          )}
        </div>
      ))}
    </div>
  );
}

/** What a row says about a machine, the same either way — so the two
    branches above differ in whether they answer a click and in nothing
    else a reader can see.

    The face, then the name over how this machine is reached — the
    session row's arrangement (2026-09-02:「左侧头像,右上机器名,右下
    连接方式」). The reading stack that used to sit between them was the
    user's word for 抽象/看不明白 and is gone; the id under the name went
    onto the card. The connection line took their place because it is
    the fact a person picks a row by — and this machine reaches itself
    over nothing, so its word is 本机, which is also why the name no
    longer carries the `(本机)` bracket. */
function Identity({ row, rename }: { row: DeviceRow; rename?: boolean }) {
  const connection = connectionWord(row);
  return (
    <>
      <MachineAvatar face={row.face} className="size-avatar" />
      <span className="min-w-0 flex-1">
        {/* Double-click to rename, the same component and the same
            gesture a session row wears (`EditableName`) — 2026-09-11:
            「session 现在列表双击名称即可重命名,我需要机器也一样」.
            `nested` because the name sits inside the button that opens
            the row: a control inside a control is invalid markup and
            worse behaviour, so the keyboard path is the card's, reached
            by opening the row.

            **Only on the branch that answers a click.** The inert
            branch has no menu either, and for the same reason stated
            above it: a pane with nowhere to send a row has not earned
            actions on it. */}
        {rename ? (
          <EditableName
            kind="machine"
            id={row.id}
            title={row.shown}
            className="block text-fact"
            inputClassName="w-full text-fact"
            nested
          />
        ) : (
          <span className="block truncate text-fact">{row.shown}</span>
        )}
        {/* Absent rather than an empty line when there is nothing to
            say: a machine whose road khor cannot name yet keeps a
            one-line row instead of a blank second line. */}
        {connection && (
          <span data-row-connection className="block truncate text-aux text-muted-foreground">
            {connection}
          </span>
        )}
      </span>
    </>
  );
}
