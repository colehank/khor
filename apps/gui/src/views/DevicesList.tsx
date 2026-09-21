import type { DeviceRow } from "@/api";
import { EditableName } from "@/components/EditableName";
import { MachineAvatar } from "@/components/Avatar";
import { IconDevices } from "@/components/icons";
import { type RowAction } from "@/components/RowMenu";
import { SideList, type SideRow } from "@/components/SideList";
import { gui } from "@/gen/catalog";
import { connectionWord, groupLabel } from "@/words";

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
            // (docs/SHELL.md 第二轮), and the map's card says it too —
            // one act (`Node::forget_device`), one word. What deleting
            // a machine *means* is the sentence between the two presses.
            //
            // **No 断开 after it.** khor has no act that drops a single
            // peer — `forget` is this item and `pause` is the whole
            // machine — and an item with nothing behind it is not offered.
            label: gui.delete,
            ask: gui.forget_machine_confirm,
            note: forgetRefused.get(d.id),
            run: () => onForget(d),
          },
        ]),
  ];
  // **The rows are `SideList`'s** (docs/SHELL.md 第四轮): the node's
  // order (pinned first), the pinned group and its fold, the box's one
  // rule. What is left here is what a machine row says.
  const side: SideRow[] = rows.map((d) => {
    const connection = connectionWord(d);
    return {
      id: d.id,
      group: d.pinned ? "pin" : "",
      pinned: d.pinned,
      face: <MachineAvatar face={d.face} className="size-avatar" />,
      // 「在设备, 是头像+机器型号icon」. **No reading names a model yet**
      // — the vitals carry cores, memory, GPUs and khor's version, never
      // what the box is — so every machine wears the generic machine
      // mark until the node reports one (docs/SHELL.md 第四轮, builder
      // 就地更正).
      badge: { mark: "machine", icon: <IconDevices className="size-kind-mark-glyph" /> },
      // Double-click to rename, the gesture a session row wears
      // (`EditableName`); `nested` because it sits inside the button
      // that opens the row. Only where a row opens anything.
      title: onOpen ? (
        <EditableName
          kind="machine"
          id={d.id}
          title={d.shown}
          className="block text-fact"
          inputClassName="w-full text-fact"
          nested
        />
      ) : (
        <span className="block truncate text-fact">{d.shown}</span>
      ),
      // How this machine is reached — the fact a person picks a row by;
      // this machine's own word is 本机. Absent rather than an empty line.
      sub: connection ? <span data-row-connection>{connection}</span> : undefined,
      // **Both names are searchable**, and the id people paste: a renamed
      // machine must be findable by what it is called now and by the
      // name this person knew it by.
      find: `${d.shown} ${d.name} ${d.id}`,
      selected: onOpen !== undefined && d.id === selected,
      // A pane with nowhere to send a row has not earned actions on it.
      menu: onOpen ? actions(d) : [],
      onOpen: onOpen ? () => onOpen(d) : undefined,
      rowData: { "data-device": d.name, "data-row": d.id, "data-pinned": d.pinned, "data-me": d.me },
      openData: { "data-row-open": "" },
    };
  });
  return (
    <SideList
      rows={side}
      query={query}
      label={groupLabel}
      // An empty device table cannot happen — this machine is always in
      // it — so zero rows is always a filter, said in its own words.
      none={gui.no_matches}
    />
  );
}
