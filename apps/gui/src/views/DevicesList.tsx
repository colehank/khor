import { useMemo, useState } from "react";

import type { DeviceRow } from "@/api";
import { type OmniAxis } from "@/components/Omnibox";
import { type FilterOption } from "@/components/PaneBar";
import { EditableName } from "@/components/EditableName";
import { machineKind } from "@/components/SessionFace";
import { type RowAction } from "@/components/RowMenu";
import { SideList, type SideRow } from "@/components/SideList";
import { gui } from "@/gen/catalog";
import { type ListAxis, listAxes, listOptions, sieve } from "@/lib/list-filter";
import { connectionKey, connectionKeyWord, connectionWord, groupLabel, systemWord } from "@/words";
import { MachineEditDialog } from "@/views/MachineEditDialog";
import { PendingStrip } from "@/views/PairDialogs";

/**
 * **What the machines list can be narrowed by** (2026-09-29, the owner:
 * 「所有的session list应该都可以filter吧？」) — the sessions' filter,
 * the same menu and chips and the same rule (`lib/list-filter`), on the
 * three things a machine row says out loud:
 *
 * - `link:` 连接方式 — the word on the second line (`connectionKey`):
 *   本机, the road (直连 / 中继 / 未连接 / …), or 「经 <sponsor>」.
 *   **One value per sponsor, not one 「借网」 for all of them**: the row
 *   prints 「经 turing」, so the option prints the same words and a tick
 *   keeps exactly the rows that say them. Merged, the option would need
 *   a word no row carries, and one tick would stand for rows that read
 *   differently — the filter would be saying something the list does
 *   not. A network has a sponsor or two, so the menu does not grow.
 * - `sys:` 系统 — the family the corner draws (`vitals.system`). A
 *   machine that could not say (an older peer, a satellite, no reading
 *   yet) wears the plain machine mark, never a guess, and files under
 *   `""` 「未知」 — which is what that corner says.
 * - `state:` 状态 — 已暂停 and nothing else: the one state a row prints
 *   (`machine_paused`). A row that is not paused answers nothing here,
 *   so the axis is on offer only while some machine is paused (or a
 *   remembered tick is waiting to be undone).
 *
 * The first prefix is also where a bare key would land (`axisOf`);
 * nothing ever wrote one into this store.
 */
export const DEVICE_KEYS = ["link:", "sys:", "state:"] as const;
const devicePick = {
  "link:": (d: DeviceRow) => connectionKey(d),
  "sys:": (d: DeviceRow) => d.vitals?.vitals.system ?? "",
  "state:": (d: DeviceRow) => (d.paused ? "paused" : null),
};
/** The three axes as the sieve reads them: keys and answers, no words. */
const DEVICE_SIEVE = DEVICE_KEYS.map((key) => ({ key, pick: devicePick[key] }));
/** …and with their headings and words, for the menu and the box. A
    function, not a constant: the words are catalog reads (批 D3). */
export const deviceAxes = (): ListAxis<DeviceRow>[] => [
  {
    key: "link:",
    heading: gui.axis_connection,
    pick: devicePick["link:"],
    label: (v) => connectionKeyWord(v) ?? gui.filter_unknown,
  },
  {
    key: "sys:",
    heading: gui.axis_system,
    pick: devicePick["sys:"],
    label: (v) => (v === "" ? gui.filter_unknown : systemWord(v)),
  },
  {
    key: "state:",
    heading: gui.axis_state,
    pick: devicePick["state:"],
    label: (v) => (v === "paused" ? gui.machine_paused : v),
  },
];

/** The pane bar's half of it: the menu's options and the box's axes,
    one list read two ways (`listAxes`). No faces — none of the three
    values is a machine. */
export function useDeviceFilter(rows: DeviceRow[], ticked: string[]): { options: FilterOption[]; axes: OmniAxis[] } {
  return useMemo(() => {
    const options = listOptions(rows, ticked, deviceAxes(), DEVICE_KEYS);
    return { options, axes: listAxes(options, DEVICE_KEYS) };
  }, [rows, ticked]);
}

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
  loading = false,
  query,
  words = [],
  onPin,
  onForget,
  onOpen,
  selected,
  pinFailed,
  forgetRefused,
  onPause,
  pauseRefused,
  onRestyled,
}: {
  rows: DeviceRow[];
  /** Not answered yet: a skeleton rather than 「there are none」. */
  loading?: boolean;
  query: string;
  /** The ticked filter keys (`DEVICE_KEYS`), from the pane bar. */
  words?: string[];
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
  /** 暂停 / 恢复 on a row — `onPin`'s contract: a rejection keeps the
      menu open where the press was made. */
  onPause?: (row: DeviceRow) => Promise<void>;
  /** The node's sentence for a pause it refused, by row. */
  pauseRefused?: Map<string, string>;
  /** Something was written from 编辑… (a name, a face) — the shell
      re-reads the table. Absent, the row offers no 编辑…. (The name is
      the prop's from when 更改头像… was the item; kept so the shell's call
      site stands.) */
  onRestyled?: () => void;
}) {
  // Which machine the edit window is open on, if any.
  const [editing, setEditing] = useState<DeviceRow | null>(null);
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
    // **暂停 / 恢复, after 置顶** (docs/SHELL.md 第五轮 第 10 条; 删除 moved
    // behind it to the foot on 2026-09-28, audit 第 16 条).
    // On another machine's row it is this machine no longer talking to
    // that one — it runs on as it was; on this machine's own row it is
    // `khor pause`. Either way the word is what pressing it does, so it
    // is the opposite of the row's state. Absent where the caller has
    // nothing to do with it.
    ...(onPause
      ? [
          {
            key: d.paused ? "resume" : "pause",
            label: d.paused ? gui.resume : gui.pause,
            note: pauseRefused?.get(d.id),
            run: () => onPause(d),
          },
        ]
      : []),
    // **编辑…** (2026-09-30, the owner: 「右键→编辑，然后这里可以改名称、
    // 头像等诸多信息」): the one window where a machine's name and face
    // are changed (`MachineEditDialog`) — on every machine, this one
    // included; 全网同一份, anyone may set anyone's. It replaced 更改头像…
    // (2026-09-28), which opened the face half alone, and 这台机器's own
    // copies of both went with it. The ellipsis because a window follows.
    ...(onRestyled
      ? [
          {
            key: "edit",
            label: gui.edit_ask,
            run: () => setEditing(d),
          },
        ]
      : []),
    // **Not on this machine's own row.** `Node::forget_device` refuses
    // it by name — a table cannot be left from the inside — so the
    // item would exist only to be refused, an offer whose one answer
    // is no.
    ...(d.me
      ? []
      : [
          {
            key: "forget",
            // **删除, the word every sidebar's last item says** — one act
            // (`Node::forget_device`), one word. What deleting a machine
            // *means* is the sentence in the window that follows. **This
            // is the one way to it** since 2026-09-28: the map's card
            // forgot a machine on one press, and went with the map's
            // second home (`check-face` 「删除一台机器永远先问」).
            //
            // What the user once called 断开 is 暂停 above (第四轮: 「机器的
            // 断开指的是暂停」), which khor now has for one machine too.
            label: gui.delete,
            askLabel: gui.delete_ask,
            destructive: true,
            ask: gui.forget_machine_confirm,
            note: forgetRefused.get(d.id),
            run: () => onForget(d),
          },
        ]),
  ];
  // **The rows are `SideList`'s** (docs/SHELL.md 第四轮): the node's
  // order (pinned first), the pinned group and its fold, the box's one
  // rule. What is left here is what a machine row says — of the rows
  // the ticks leave standing (`DEVICE_SIEVE`).
  const side: SideRow[] = sieve(rows, words, DEVICE_SIEVE, DEVICE_KEYS).map((d) => {
    const connection = connectionWord(d);
    const system = d.vitals?.vitals.system ?? null;
    const modelName = d.vitals?.vitals.model?.name ?? null;
    return {
      id: d.id,
      group: d.pinned ? "pin" : "",
      pinned: d.pinned,
      face: { machine: d.face },
      // **The system family's own mark** (docs/SHELL.md 第五轮第 7 条改画法:
      // 「角标按系统」) — the node's `vitals.system`. A machine that could
      // not say (an older peer, a satellite, no reading yet) wears the
      // plain machine mark, never a guess. The model is not the mark's
      // any more; its name is the second line's (`sub`). On the face or
      // on its corner by the 外观 setting (`FaceMark`, 2026-10-02) — and
      // `MachineFace` is this same assembly for anywhere else.
      mark: machineKind(system),
      // Double-click to rename, the gesture a session row wears
      // (`EditableName`); `nested` because it sits inside the button
      // that opens the row. Only where a row opens anything.
      title: onOpen ? (
        <EditableName
          kind="machine"
          id={d.id}
          title={d.shown}
          className="block"
          inputClassName="w-full text-body"
          nested
        />
      ) : (
        <span className="block truncate">{d.shown}</span>
      ),
      name: d.shown,
      // How this machine is reached — the fact a person picks a row by;
      // this machine's own word is 本机 — and then what the box is, as
      // the machine names itself (`vitals.model.name`: 「型号文字进机器行
      // 的副标题」). Either can be missing; absent rather than an empty line.
      // A paused row says so first (第 10 条: 「暂停中的行变灰并标『已暂停』」).
      sub:
        d.paused || connection || modelName ? (
          <span className="flex min-w-0 items-baseline gap-1.5">
            {d.paused && <span data-row-paused>{gui.machine_paused}</span>}
            {connection && <span data-row-connection>{connection}</span>}
            {modelName && (
              <span data-row-model className="min-w-0 truncate">
                {modelName}
              </span>
            )}
          </span>
        ) : undefined,
      // **Both names are searchable**, and the id people paste: a renamed
      // machine must be findable by what it is called now and by the
      // name this person knew it by.
      find: `${d.shown} ${d.name} ${d.id}`,
      selected: onOpen !== undefined && d.id === selected,
      // A pane with nowhere to send a row has not earned actions on it.
      menu: onOpen ? actions(d) : [],
      onOpen: onOpen ? () => onOpen(d) : undefined,
      // Grey, the way SideList greys a row (`dim`) — the machine is still
      // in the table and still openable; this machine is not talking to it.
      dim: d.paused,
      rowData: {
        "data-device": d.name,
        "data-row": d.id,
        "data-pinned": d.pinned,
        "data-me": d.me,
        "data-paused": d.paused,
      },
      openData: { "data-row-open": "" },
    };
  });
  // The row it was opened on, re-read off the table so the title follows
  // a rename and the window goes if the machine does.
  const edited = editing ? (rows.find((d) => d.id === editing.id) ?? null) : null;
  return (
    <>
      {/* 待批 (docs/CONNECT.md 根不在场): who waits for the root holder,
          above the rows it will join. Draws nothing while nobody waits. */}
      <PendingStrip />
      <SideList
        rows={side}
        loading={loading}
        query={query}
        filtered={words.length > 0}
        label={groupLabel}
        // An empty device table cannot happen — this machine is always in
        // it — so zero rows is always a filter, said in its own words:
        // the box's or the ticks' (`empty` is what `SideList` draws when
        // the ticks have left it nothing to hand over).
        empty={gui.no_matches}
        none={gui.no_matches}
      />
      {/* An errand — 办完就回去 — so a window over the list, not a page.
          Picking a face keeps it open (the table is re-read, the window
          stays): only 完成 or Esc closes it. */}
      <MachineEditDialog row={edited} onClose={() => setEditing(null)} onChanged={() => onRestyled?.()} />
    </>
  );
}
