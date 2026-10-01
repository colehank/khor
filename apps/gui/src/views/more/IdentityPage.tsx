// The avatar's page: **this machine, and nothing about the network**
// (docs/MORE.md 第二轮 地图: 头像 = 这台机器是谁。「…」= 这个 app).
//
// # What left this card on 2026-09-20, and why
//
// It used to carry the paired machines, 邀请 / 接入, a door to the mesh,
// 暂停 / 恢复 and 离开这张网 — five things about *the network*, on the
// page somebody opens to ask *which machine am I*. 「不同功能分开」 —
// the user. All five are on 网络 now, and none of them is here as well:
// a second copy is how two surfaces come to disagree, and the one
// somebody finds first is never the one that was updated.
//
// What is left is the two halves of 这台机器是谁:
//
// - the face, the name and the id — what the network calls this
//   machine. **Stated, not edited** (2026-09-30, the owner: 「『我』的里面
//   不要有更改头像的设置，这在机器页也能设置，不要冗余」): the name and the
//   face are changed in one window, from the machine's row on the
//   machines page (右键 → 编辑…, `MachineEditDialog`), for this machine
//   as for every other. This page carried a 改名… row and the whole face
//   editor until then — a second door to the same two facts;
// - **本机开着哪几张 khor 界面, 当前是哪张** (「头像这, 还可以显示, 本机
//   运行了哪些脸. 当前是哪张脸」). The facts come from the node
//   (`khor_node::face::faces`, which `khor faces` prints) — the desktop
//   app's own pid file and a probe of the web face's port. The page does
//   not guess: a face khor cannot see is drawn as 未打开, never omitted.
//
// 「当前」 is the one thing on this page the node cannot answer, because
// it is a fact about the viewer rather than about the machine: the app
// and the browser are looking at one khor and each is a different
// 「this one」. `inApp` (`api.ts`) is that fact, and it is asked once
// here rather than carried in the answer.
import { useEffect, useState } from "react";

import { fetchAbout, inApp, type About, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { CopyId } from "@/views/DetailPane";
import { MoreFact, MoreFrame, MoreSection } from "@/views/more/MorePage";

export function IdentityPage({
  devices,
  onClose,
}: {
  /** The shell's own polled table — **not fetched again here**. Two
      readers of one answer is how two surfaces come to show two
      different names for one machine, and the shell is already asking
      (`App.tsx`'s devices effect). */
  devices: DeviceRow[];
  onClose: () => void;
}) {
  // Asked once per opening, like the settings screen reads its face
  // choices: a version cannot change under a running app, but the
  // answer is cheap and a page that read it at mount would go stale
  // across an in-place upgrade the person then wonders about.
  //
  // **The list of faces rides on the same answer** and is therefore as
  // fresh as the page: the web face can come up while the card is open,
  // and the honest cost of not polling is one reopening.
  const [about, setAbout] = useState<About | null>(null);
  useEffect(() => {
    void fetchAbout().then(setAbout, () => {
      // Silent: the version is a line on a card, and a sentence about
      // khor failing to read its own version is worse than the line
      // simply not being there. The name and the id come from the
      // shell's table and are unaffected.
    });
  }, []);

  const me = devices.find((d) => d.me) ?? null;
  // Which face this person is looking through. The desktop shell says
  // so about itself; everything else is a browser, which is the web
  // face — and in neither case is it something the node can be asked.
  const hereKey = inApp ? "app" : "web";

  return (
    <MoreFrame
      page="identity"
      onClose={onClose}
      // The machine page's second line for this machine: its name and the
      // khor it runs — R0 says 「这台机器」, this says which one.
      subtitle={
        me ? (
          <>
            {me.shown}
            {about?.version && (
              <>
                <span aria-hidden="true"> · </span>
                {gui.card_version(about.version)}
              </>
            )}
          </>
        ) : undefined
      }
    >
      {/* **One settings group of facts, no name of its own** — R0 says
          这台机器. The name, with the face it wears beside it, then the
          id and the version. `shown`, never `name`: the first is what
          this machine is called, the second is what the network files
          it under.

          ⚠️ 墓碑 (2026-09-30): the name was an action row, 改名… (a window
          with a box), and the face editor was its own section under it.
          Both went — the owner: 「『我』的里面不要有更改头像的设置，这在机器
          页也能设置，不要冗余」. The one place either is edited is the
          machine row's 编辑… (`MachineEditDialog`). And before that
          (2026-09-27) this was a head of its own — the face at
          `avatar-lg` and a `title-md` name — a second title. */}
      <MoreSection name="machine">
        {me && (
          <MoreFact name="name" label={gui.machine_name}>
            <span data-identity-name className="inline-flex max-w-full items-center gap-in align-middle">
              <MachineAvatar face={me.face} className="size-kind-mark flex-none" />
              <span className="min-w-0 break-all">{me.shown}</span>
            </span>
          </MoreFact>
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
      </MoreSection>

      {/* 本机开着哪几张界面 (docs/MORE.md 头像页). Drawn only once khor
          has answered — a list of faces painted from a default this file
          kept would be a page reporting a probe it never made. */}
      {/* A phone has neither face, and says so with no section rather
          than an empty one (`khor_node::face::faces`, docs/IOS.md). */}
      {about !== null && about.faces.length > 0 && (
        <MoreSection name="faces" label={gui.identity_faces}>
          {/* **Each face is a row of the card**, as tall as every other
              settings row and with the hairline between them.
              ⚠️ 墓碑 (2026-09-28): one `<ul>` of 24-high callout lines in
              one padded box — the group read as a caption, not rows. */}
          {about.faces.map((f) => (
            <div
              key={f.key}
              data-iface={f.key}
              data-on={f.up}
              data-more-row
              className="flex min-w-0 items-center gap-row text-body"
            >
              <span className="flex min-w-0 flex-1 items-baseline gap-in">
                <span className="min-w-0 truncate">{f.name}</span>
                {/* 当前 says which of these the reader is looking
                    through — a note on the name, in `secondary`, the
                    way macOS marks 「this Mac」 in a list of devices.
                    ⚠️ 墓碑 (2026-09-28): it was olive, and a marked
                    choice is not one of `brand-strong`'s three posts. */}
                {f.key === hereKey && (
                  <span data-iface-here className="flex-none text-muted-foreground">
                    {cli.face_current}
                  </span>
                )}
              </span>
              {/* The whole right-hand side is the node's sentence —
                  已打开 with the port when there is one, 未打开 when
                  there is not. Assembled over there so a port can
                  never be printed beside 未打开 (`FaceRow::said`). */}
              <span
                data-iface-said
                data-setting-trail
                className={cn("flex-none text-right", f.up ? "" : "text-muted-foreground")}
              >
                {f.said}
              </span>
            </div>
          ))}
        </MoreSection>
      )}
    </MoreFrame>
  );
}
