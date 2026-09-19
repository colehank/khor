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
// So this page states facts and offers the two existing doors. It does
// not offer a 退出 or a 暂停, because there is nothing behind either
// word yet and a control that refuses is worse than a control that is
// absent (docs/MORE.md: 没有落地实现的键不画).
import { useEffect, useState } from "react";

import { fetchAbout, type About, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { EditableName } from "@/components/EditableName";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { CopyId } from "@/views/DetailPane";
import { MoreFact, MoreFrame, MoreSection } from "@/views/more/MorePage";

export function IdentityPage({
  devices,
  onClose,
  onMesh,
  onInvite,
  onJoin,
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
    </MoreFrame>
  );
}
