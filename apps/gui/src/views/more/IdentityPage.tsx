// The avatar's page: this machine, as a card (docs/MORE.md 头像).
//
// **What the user asked the avatar for is not what it draws yet**, and
// the gap is written here rather than left to be discovered:
// 「点击左下角头像, 是选择接入已有khor网络,或建立新的khor网络, 退出Khor
// 网络, 暂停接入Khor网络」 (2026-09-18). Joining, leaving and pausing a
// *network* need the hypergraph first — a network is an edge with a
// name, a machine is a point, one machine can sit on several edges — and
// that ruling (2026-09-19: 「先不整网络吧, 有点复杂了. 可以是宏图」) put
// the whole shape after this batch. What is left of the press is the
// half that needs no hypergraph: **which machine am I, and who have I
// paired with** — plus the two verbs that already exist for pairing.
//
// So this page states facts and offers the doors that exist.
//
// > **2026-09-20: 退出 landed, 暂停 is still absent.** 「单网络的做通呀」 —
// > the user. A single network needs no hypergraph to be left: leaving
// > is `khor leave`, the half `khor forget` refuses to do, and it is on
// > this card because this card is where somebody stands when they ask
// > 「这台机器还在这张网里做什么」. 建立 / 接入 were already here as
// > 邀请 / 接入. 暂停 is not drawn yet, and a control that refuses is
// > still worse than one that is absent (docs/MORE.md: 没有落地实现的
// > 键不画).
//
// **The confirmation is the machine's own name typed back, and the node
// is what checks it** (`Node::leave`). Not a second press: leaving is
// one press away from a card somebody opened to look at a version
// number, and the two-press confirm on a row menu is for an act that
// names its target in the press. Here the target is the machine you are
// standing on, so typing it is what says you meant this one. The rule
// itself lives in the node for `lock::reset_identity`'s reason — a rule
// here as well would be two rules about one irreversible act.
import { useEffect, useState } from "react";

import { fetchAbout, leaveNetwork, type About, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { EditableName } from "@/components/EditableName";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cli, gui, msg } from "@/gen/catalog";
import { CopyId } from "@/views/DetailPane";
import { MoreFact, MoreFrame, MoreSection } from "@/views/more/MorePage";

export function IdentityPage({
  devices,
  onClose,
  onMesh,
  onInvite,
  onJoin,
  onLeft,
}: {
  /** The shell's own polled table — **not fetched again here**. Two
      readers of one answer is how two surfaces come to show two
      different names for one machine, and the shell is already asking
      (`App.tsx`'s devices effect). */
  devices: DeviceRow[];
  onClose: () => void;
  /** 网里的机器: the picture. The avatar used to open it directly and
      now opens this page, so the picture moved one press deeper rather
      than losing its only door — see the handoff for that ruling. */
  onMesh: () => void;
  onInvite: () => void;
  onJoin: () => void;
  /** The table changed under every surface on this screen — the shell
      re-reads it (`App`'s `refresh`, the same one a forget gets). */
  onLeft: () => void;
}) {
  // Asked once per opening, like the settings screen reads its face
  // choices: a version cannot change under a running app, but the
  // answer is cheap and a page that read it at mount would go stale
  // across an in-place upgrade the person then wonders about.
  const [about, setAbout] = useState<About | null>(null);
  useEffect(() => {
    void fetchAbout().then(setAbout, () => {
      // Silent: the version is a line on a card, and a sentence about
      // khor failing to read its own version is worse than the line
      // simply not being there. Everything else on this page comes
      // from the shell's table and is unaffected.
    });
  }, []);

  // What the box holds, and what khor said the last time 离开 was
  // pressed. `said` carries both endings in the order the CLI prints
  // them — the act, who heard, who did not — because they are the
  // node's sentences and a page that rewrote them would be a second
  // account of one act (`msg::left`).
  const [typedName, setTypedName] = useState("");
  const [said, setSaid] = useState<string[] | null>(null);
  const [leaveError, setLeaveError] = useState<string | null>(null);

  const me = devices.find((d) => d.me) ?? null;
  const others = devices.filter((d) => !d.me);

  return (
    <MoreFrame page="identity" onClose={onClose}>
      <div className="flex flex-wrap items-start gap-row">
        {/* The same derivation as the rail's foot and as this machine's
            row: one machine, one face, painted from what it reported.
            Bigger here, not different (`MachineCard`'s head). */}
        <MachineAvatar face={me?.face ?? null} className="size-avatar-lg" />
        <div className="flex min-w-0 flex-1 flex-col gap-2">
          {me && (
            <div className="flex min-w-0 flex-wrap items-center gap-row">
              {/* The editor every other name in this app uses — the
                  machine subject, which `MachineCard` already reaches
                  for. `shown`, never `name`: the first is what this
                  machine is called, the second is what the network
                  files it under. */}
              <EditableName
                kind="machine"
                id={me.id}
                title={me.shown}
                className="min-w-0 text-title font-title"
                inputClassName="min-w-0 text-title font-title"
              />
            </div>
          )}
          {me && (
            <MoreFact name="id" label={gui.identity_id}>
              {/* Twelve characters to recognise; pressing puts all
                  sixty-four on the clipboard (`CopyId`'s `id` face). */}
              <CopyId id={me.id} face="id" />
            </MoreFact>
          )}
          <MoreFact name="version" label={gui.identity_version}>
            {/* **This binary's**, never a claim about anybody else's:
                the device table carries no version, and `khor doctor`
                is where another machine's is asked for over the wire. */}
            <span className="tabular-nums">{about?.version ?? ""}</span>
          </MoreFact>
        </div>
      </div>

      <MoreSection name="paired" label={gui.identity_paired}>
        {others.length === 0 ? (
          <p className="text-sm text-muted-foreground">{gui.identity_alone}</p>
        ) : (
          <ul className="flex flex-col">
            {others.map((d) => (
              // Facts, not doors. A machine's card is one press away on
              // the 机器 landing, and a second way in from here would
              // be two surfaces owning one navigation — the thing that
              // makes two screens disagree about what pressing a
              // machine means.
              <li
                key={d.id}
                data-paired-machine={d.id}
                className="flex min-w-0 items-center gap-row py-1 text-sm"
              >
                <MachineAvatar face={d.face} className="size-avatar flex-none" />
                <span className="min-w-0 flex-1 truncate">{d.shown}</span>
                <span className="flex-none text-muted-foreground tabular-nums">
                  {d.id.slice(0, 12)}
                </span>
              </li>
            ))}
          </ul>
        )}
        <div className="flex flex-wrap gap-in">
          {/* The two verbs that already exist, and the same two the
              machines pane's 「+」 offers — one wizard each, opened by
              the shell (`App.tsx`'s `Sheet`). Repeated here rather than
              moved, because this page is where somebody who pressed
              their own face is standing when the question 「怎么再加一台」
              occurs to them. */}
          <Button variant="outline" data-identity-invite onClick={onInvite} className="min-h-touch">
            {gui.make_a_ticket}
          </Button>
          <Button variant="outline" data-identity-join onClick={onJoin} className="min-h-touch">
            {gui.join_with_a_ticket}
          </Button>
          <Button variant="ghost" data-identity-mesh onClick={onMesh} className="min-h-touch">
            {gui.mesh_map}
          </Button>
        </div>
      </MoreSection>

      {/* 离开这张网 (docs/MORE.md 离开). Drawn on a machine that is in a
          network and on no other: `Node::leave` refuses a machine that
          never joined one by name, so drawing it there would be an
          offer whose one answer is no — the rule that keeps 「忘掉」 off
          this machine's own row in `DevicesList`. */}
      {others.length > 0 && (
        <MoreSection name="leave" label={gui.identity_leave_section}>
          {/* The whole cost in one clause, and it is not red like the
              reset's: leaving is undone by a ticket, and painting the
              two the same colour would say they are the same size. */}
          <p data-identity-leave-warn className="text-sm text-muted-foreground">
            {gui.identity_leave_warn}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <Input
              data-identity-leave-name
              autoComplete="off"
              aria-label={gui.identity_leave_type_name}
              placeholder={gui.identity_leave_type_name}
              value={typedName}
              onChange={(e) => setTypedName(e.target.value)}
              className="min-h-touch min-w-0 flex-1"
            />
            <Button
              variant="ghost"
              data-identity-leave
              // Checked here only so the button is not live over an
              // empty box; the answer that counts is the node's, and it
              // is the one that comes back as a sentence.
              disabled={typedName.trim() !== (me?.shown ?? "") || !me}
              onClick={() => {
                setLeaveError(null);
                setSaid(null);
                leaveNetwork(typedName).then(
                  (left) => {
                    setTypedName("");
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
                    onLeft();
                  },
                  (e: unknown) => setLeaveError(String(e instanceof Error ? e.message : e)),
                );
              }}
              className="min-h-touch text-destructive"
            >
              {gui.identity_leave_do}
            </Button>
          </div>
        </MoreSection>
      )}
      {said?.map((line) => (
        <p key={line} data-identity-left className="text-sm text-muted-foreground">
          {line}
        </p>
      ))}
      {leaveError && (
        <p data-identity-leave-refused className="text-sm text-destructive">
          {leaveError}
        </p>
      )}
    </MoreFrame>
  );
}
