// 网络 (docs/MORE.md 第二轮, and its 2026-09-28 correction).
//
// **A settings page, not a picture** (the owner, 2026-09-28). From
// 2026-09-20 (「网络就搞成地图」) the whole page was the map, with four
// buttons under it and the relays folded into 高级. The map lives on
// one surface now — the machines landing's overview, 10-layout's
// 「机器 → 那张图」 — and this page says what the network is to this
// machine in the rows every other 「…」 page is made of:
//
//   状态   本机 (its name and whether it is talking) · 已连接的机器 (how
//          many, and by which road) · 暂停同步 (a switch)
//   中继   使用中 · 自己的中继 and its entries · 只使用自己的中继
//   ——     离开网络
//
// and 邀请设备… / 加入网络… are the page's own buttons, at R0's right
// end (D9) — never a row of buttons in the page, which is what stood
// under the map.
//
// # Where the map's card went
//
// The map used to open a card over itself when a seat was pressed: the
// machine's rings, its session counts, 头像 and a one-press 删除
// (`MarkPlace`, retired with the map's second home). Each of those has
// one home already, and the card was the second: the rings are the
// machine's own page (a seat on the overview opens it), 待批 rides the
// seat, 头像 and 删除… are the machine row's menu — where 删除 asks
// first, like every other 删除 (docs/SHELL.md 第四轮).
//
// # Why the heavy act is a dialog and the light one is not
//
// 暂停同步 is one press and reversible, so it is a switch. 离开网络
// wants this machine's own name typed back, and the rule is the node's
// (`Node::leave`) rather than this page's — a page that grew its own
// would be two rules about one irreversible act, and the one that
// drifts is the one nobody runs.
import { useEffect, useState, type ReactNode } from "react";

import { fetchAbout, leaveNetwork, setPaused, type About, type DeviceRow } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cli, gui, msg } from "@/gen/catalog";
import { hopWord } from "@/words";
import { NetSettings } from "@/views/NetSettings";
import { MoreFact, MoreFrame, MoreSection } from "@/views/more/MorePage";

/** Between two facts on one line — the separator `ImportDialog` and
    额度's rows put between theirs. */
const DOT = " · ";

export function NetworkPage({
  devices,
  onClose,
  actions,
  onChanged,
}: {
  /** The shell's own polled table — **not fetched again here**. Two
      readers of one answer is how two surfaces come to show two
      different names for one machine (`IdentityPage`'s rule). */
  devices: DeviceRow[];
  onClose: () => void;
  /** 邀请设备… / 加入网络… for the narrow face, whose only head is this
      page's own — wide puts them in R0 (`App`), where every page's
      buttons are. */
  actions?: ReactNode;
  /** The device table changed under every surface on screen — the shell
      re-reads it (`App`'s `refresh`, the same one a forget gets). */
  onChanged: () => void;
}) {
  // 暂停 is read off this answer rather than off the device table's
  // `paused`, because the file is the thing `khor pause` writes and the
  // table is a report about it — and on a store that will not open,
  // `about` still answers (`khor_gui_core::about`).
  const [about, setAbout] = useState<About | null>(null);
  useEffect(() => {
    void fetchAbout().then(setAbout, () => setAbout(null));
  }, []);
  const [leaving, setLeaving] = useState(false);
  const [typedName, setTypedName] = useState("");
  // What khor said the last time 离开网络 was pressed. Both endings, in
  // the order the CLI prints them — the act, who heard, who did not —
  // because they are the node's sentences and a page that rewrote them
  // would be a second account of one act (`msg::left`).
  const [said, setSaid] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refused, setRefused] = useState<string | null>(null);

  const me = devices.find((d) => d.me) ?? null;
  const others = devices.filter((d) => !d.me);
  const alone = others.length === 0;
  // **Connected is a road, and the road is the node's word** (`hop`):
  // direct and relayed are the two that carry anything; 未连接 / serve
  // 未运行 / 未知 are machines in the table this one is not talking to,
  // and they are not counted as connected.
  const roads = (["direct", "relay"] as const)
    .map((k) => ({ k, n: others.filter((d) => d.hop === k).length }))
    .filter((r) => r.n > 0);
  const connected = roads.reduce((a, r) => a + r.n, 0);
  // **Whether this machine is talking**, in the words the rest of the
  // app already uses for it: 已暂停 when `khor pause` is on; serve 未运行
  // when every other machine's road says nobody here is holding the
  // connection (`Hop::NoServe` is about *this* end); otherwise 在线.
  const noServe = !alone && others.every((d) => d.hop === "no-serve");
  const state = about?.paused ? gui.machine_paused : noServe ? hopWord("no-serve") : gui.net_online;

  return (
    <MoreFrame page="network" onClose={onClose} actions={actions}>
      {error && (
        <p data-net-error className="text-callout text-destructive">
          {error}
        </p>
      )}
      {said?.map((line) => (
        <p key={line} data-net-left className="text-callout text-muted-foreground">
          {line}
        </p>
      ))}

      <MoreSection name="status" label={gui.axis_state}>
        {/* 本机: its name and whether it is talking — one fact row, the
            two halves the way the machines list says them (name · word). */}
        <MoreFact name="net-me" label={gui.machine_here}>
          {me ? (
            <>
              {me.shown}
              {about && (
                <>
                  {DOT}
                  <span data-net-state={about.paused ? "paused" : noServe ? "no-serve" : "online"}>{state}</span>
                </>
              )}
            </>
          ) : null}
        </MoreFact>
        {/* A machine that has paired with nobody gets the sentence the
            page has always said then: 「怎么再加一台」 is the only
            question there is, and the two buttons in the head answer it. */}
        <MoreFact name="net-connected" label={gui.net_connected}>
          {alone ? (
            <span data-net-alone>{gui.identity_alone}</span>
          ) : roads.length > 0 ? (
            gui.net_connected_n(connected, roads.map((r) => gui.net_via(r.n, hopWord(r.k))).join(DOT))
          ) : (
            gui.net_connected_bare(0)
          )}
        </MoreFact>
        {/* 暂停同步: **a switch** — one state, taken at once, undone by the
            same press. Not drawn on a machine that is in no network:
            `khor pause` there is an act with no audience. The fresh
            `About` is the answer: the switch shows what the file now
            says, never what the press assumed. **The rail paints off
            the device table, not off this page**, so the shell re-reads
            it too (the rail's face and this machine's row say 已暂停). */}
        {!alone && about && (
          <SwitchRow
            name="net-pause"
            label={gui.net_pause_sync}
            hint={gui.net_pause_note}
            on={about.paused}
            mark={{ "data-net-pause": "", "data-paused": about.paused ? "" : undefined } as Record<string, string>}
            onToggle={() => {
              setError(null);
              setPaused(!about.paused).then(
                (now) => {
                  setAbout(now);
                  onChanged();
                },
                (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
              );
            }}
          />
        )}
      </MoreSection>

      {/* 中继 — what 高级 held until 2026-09-28, open now: see
          `NetSettings` for why it is read on arrival. */}
      <NetSettings />

      {/* 离开网络: an action row with an ellipsis, the confirmation in a
          window (components/SettingsGroup 「危险的动作」) — the shape
          安全's 重置身份 has. Not on a machine in no network:
          `Node::leave` refuses by name there. */}
      {!alone && (
        <MoreSection name="leave">
          <ActionRow
            name="leave"
            label={gui.identity_leave_do}
            hint={gui.net_leave_note}
            actionLabel={gui.net_leave_ask}
            // Red word on clear ground, as this act has always been
            // drawn: on the push button's `fill-strong` the dark theme's
            // `destructive` reads 3.95, under the text floor.
            variant="ghost"
            danger
            disabled={!me}
            onPress={() => {
              setError(null);
              setRefused(null);
              setSaid(null);
              setTypedName("");
              setLeaving(true);
            }}
            mark={{ "data-net-leave": "" }}
          />
        </MoreSection>
      )}

      <Dialog open={leaving} onOpenChange={setLeaving}>
        <DialogContent data-net-leave-dialog>
          <DialogHeader>
            <DialogTitle>{gui.identity_leave_do}</DialogTitle>
          </DialogHeader>
          {/* The whole cost, and it is not red like the reset's: leaving
              is undone by a ticket, and painting the two the same colour
              would say they are the same size. */}
          <p data-net-leave-warn className="text-callout text-muted-foreground">
            {gui.identity_leave_warn}
          </p>
          <Input
            data-net-leave-name
            autoComplete="off"
            aria-label={gui.identity_leave_type_name}
            placeholder={gui.identity_leave_type_name}
            value={typedName}
            onChange={(e) => setTypedName(e.target.value)}
            className="min-h-touch w-full"
          />
          {refused && (
            <p data-net-leave-refused className="text-callout text-destructive">
              {refused}
            </p>
          )}
          <DialogFooter>
            <Button onClick={() => setLeaving(false)}>{gui.delete_keep}</Button>
            <Button
              variant="destructive"
              data-net-leave-do
              // Checked here only so the button is not live over an
              // empty box; the answer that counts is the node's, and it
              // comes back as a sentence.
              disabled={typedName.trim() !== (me?.shown ?? "") || !me}
              onClick={() => {
                setRefused(null);
                setSaid(null);
                leaveNetwork(typedName).then(
                  (left) => {
                    setTypedName("");
                    setLeaving(false);
                    setSaid([
                      msg.left,
                      ...(left.told.length ? [msg.left_told(left.told.join(cli.name_separator))] : []),
                      // The two endings stay two sentences all the way
                      // to the screen: a machine in `untold` goes on
                      // painting this one offline until somebody over
                      // there runs `khor forget`, which is a different
                      // thing left to do.
                      ...(left.untold.length
                        ? [msg.left_untold(left.untold.join(cli.name_separator), me?.shown ?? "")]
                        : []),
                    ]);
                    onChanged();
                  },
                  (e: unknown) => setRefused(String(e instanceof Error ? e.message : e)),
                );
              }}
            >
              {gui.identity_leave_do}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </MoreFrame>
  );
}
