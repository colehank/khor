// 网络 (docs/MORE.md 第二轮: 地图 —— 两个入口,一样东西只住一处).
//
// 「不同功能分开, 设置里有网络,本机里也有网络. 网络就搞成地图. 更多里的一个
// 单独的页面… 构建用户只觉得用户地图」 — the user, 2026-09-20.
//
// # What this page is, and what that cost elsewhere
//
// **The whole page is the picture.** `MarkPlace` used to be a screen of
// its own, reached by pressing this machine's face and then a row on the
// identity card; it is this page's body now and has no other door. A map
// in two places is the same map twice, and the second one is always the
// one somebody finds first.
//
// Four things came here from three other surfaces, and every one of them
// left the place it came from:
//
// - **已配对的机器** was a list on the identity card *and* a list on 安全.
//   Neither is left. A machine is a seat on the map; pressing it opens
//   its card, and 移除设备 is in that card because that is where the
//   machine you mean is named.
// - **邀请设备 / 加入网络** were buttons on the identity card. They are
//   about this network, not about which machine this is.
// - **暂停 / 恢复 and 离开网络** were two sections on the identity card.
//   Same sentence: 这台机器和这张网的关系.
// - **中继** was a section of 设置. It is behind 高级 here — 「不是所有的
//   设置都要去铺在界面上」 — because it is the one control on this page
//   that most people never touch and nobody arrives looking for.
//
// # Why the heavy act is a dialog and the light one is not
//
// 暂停 is one press and reversible, so it is a button that says which way
// it goes. 离开网络 wants this machine's own name typed back, and the rule
// is the node's (`Node::leave`) rather than this page's — a page that
// grew its own would be two rules about one irreversible act, and the one
// that drifts is the one nobody runs. It is a dialog rather than a
// section because a page that is 一屏以内 cannot also carry a warning
// paragraph and a text box for an act almost nobody performs.
import { useEffect, useState } from "react";

import { EmptyState } from "@/components/RegionState";
import { fetchAbout, leaveNetwork, setPaused, type About, type DeviceRow, type SessionRow } from "@/api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cli, gui, msg } from "@/gen/catalog";
import { MarkPlace } from "@/views/MarkPlace";
import { NetSettings } from "@/views/NetSettings";
import { MoreFold, MoreFrame } from "@/views/more/MorePage";

export function NetworkPage({
  rows,
  devices,
  onClose,
  onInvite,
  onJoin,
  onChanged,
  onPickWaiting,
}: {
  rows: SessionRow[];
  /** The shell's own polled table — **not fetched again here**. Two
      readers of one answer is how two surfaces come to show two
      different names for one machine (`IdentityPage`'s rule). */
  devices: DeviceRow[];
  onClose: () => void;
  onInvite: () => void;
  onJoin: () => void;
  /** The device table changed under every surface on screen — the shell
      re-reads it (`App`'s `refresh`, the same one a forget gets). */
  onChanged: () => void;
  onPickWaiting: (row: SessionRow) => void;
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

  const me = devices.find((d) => d.me) ?? null;
  const alone = devices.filter((d) => !d.me).length === 0;

  return (
    <MoreFrame page="network" onClose={onClose} bleed>
      {/* The picture takes whatever is left after the acts under it.
          A machine that has paired with nobody gets a sentence instead:
          a one-seat map answers nothing, and 「怎么再加一台」 is the only
          question there is at that moment. */}
      <div className="min-h-0 min-w-0 flex-1">
        {alone ? (
          <EmptyState data-net-alone>{gui.identity_alone}</EmptyState>
        ) : (
          <MarkPlace rows={rows} devices={devices} onChanged={onChanged} onPickWaiting={onPickWaiting} />
        )}
      </div>

      {/* The four acts, in one row under the picture (docs/MORE.md
          第二轮: 页上四个动作). Pause before leave — the reversible one
          first, which is the order a person reads. */}
      <div className="flex flex-none flex-col gap-2 border-t p-pane">
        <div data-net-acts className="flex flex-wrap items-center gap-in">
          <Button data-net-invite onClick={onInvite} className="min-h-touch">
            {gui.make_a_ticket_ask}
          </Button>
          <Button data-net-join onClick={onJoin} className="min-h-touch">
            {gui.join_with_a_ticket_ask}
          </Button>
          {/* Both of these are about a network this machine is in, so
              neither is drawn on a machine that is in none: `Node::leave`
              refuses by name there, and `khor pause` on a machine with
              nobody to be quiet towards is an act with no audience. */}
          {!alone && about && (
            <Button
              data-net-pause
              data-paused={about.paused ? "" : undefined}
              onClick={() => {
                setError(null);
                // The fresh `About` is the answer: the page shows what
                // the file now says, never what the press assumed.
                setPaused(!about.paused).then(
                  (now) => {
                    setAbout(now);
                    // **The rail paints off the device table, not off
                    // this page.** Pausing changes a fact both of them
                    // state, so the shell re-reads it here.
                    onChanged();
                  },
                  (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
                );
              }}
              className="min-h-touch"
            >
              {about.paused ? gui.resume : gui.pause}
            </Button>
          )}
          {!alone && (
            <Button
              variant="ghost"
              data-net-leave
              onClick={() => {
                setError(null);
                setSaid(null);
                setTypedName("");
                setLeaving(true);
              }}
              className="min-h-touch text-destructive"
            >
              {/* It opens the window that asks for the name: the
                  ellipsis (components/Button). The window's own title
                  and button keep the bare verb. */}
              {gui.identity_leave_ask}
            </Button>
          )}
        </div>
        {about?.paused && (
          <p data-net-paused className="text-callout text-muted-foreground">
            {gui.identity_paused_now}
          </p>
        )}
        {said?.map((line) => (
          <p key={line} data-net-left className="text-callout text-muted-foreground">
            {line}
          </p>
        ))}
        {error && (
          <p data-net-error className="text-callout text-destructive">
            {error}
          </p>
        )}
        {/* 中继, folded (docs/MORE.md 第二轮: 中继折进「高级」).
            **Mounted only while the fold is open** — a relay list is two
            fetches, and a page that made them to draw nothing is a
            machine dialling on behalf of a control nobody opened. */}
        <MoreFold name="advanced" label={gui.advanced}>
          <NetSettings />
        </MoreFold>
      </div>

      <Dialog open={leaving} onOpenChange={setLeaving}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{gui.identity_leave_do}</DialogTitle>
          </DialogHeader>
          {/* The whole cost in one clause, and it is not red like the
              reset's: leaving is undone by a ticket, and painting the
              two the same colour would say they are the same size. */}
          <p data-net-leave-warn className="text-callout text-muted-foreground">
            {gui.identity_leave_warn}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <Input
              data-net-leave-name
              autoComplete="off"
              aria-label={gui.identity_leave_type_name}
              placeholder={gui.identity_leave_type_name}
              value={typedName}
              onChange={(e) => setTypedName(e.target.value)}
              className="min-h-touch min-w-0 flex-1"
            />
            <Button
              variant="ghost"
              data-net-leave-do
              // Checked here only so the button is not live over an
              // empty box; the answer that counts is the node's, and it
              // comes back as a sentence.
              disabled={typedName.trim() !== (me?.shown ?? "") || !me}
              onClick={() => {
                setError(null);
                setSaid(null);
                leaveNetwork(typedName).then(
                  (left) => {
                    setTypedName("");
                    setLeaving(false);
                    setSaid([
                      msg.left,
                      ...(left.told.length
                        ? [msg.left_told(left.told.join(cli.name_separator))]
                        : []),
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
                  (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
                );
              }}
              className="min-h-touch text-destructive"
            >
              {gui.identity_leave_do}
            </Button>
          </div>
          {error && (
            <p data-net-leave-refused className="text-callout text-destructive">
              {error}
            </p>
          )}
        </DialogContent>
      </Dialog>
    </MoreFrame>
  );
}
