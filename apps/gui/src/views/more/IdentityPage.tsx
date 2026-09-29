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
// - the face, the name (editable) and the id — what the network calls
//   this machine;
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

import { fetchAbout, inApp, renameMachine, type About, type DeviceRow } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { CopyId } from "@/views/DetailPane";
import { FaceSettings } from "@/views/FaceSettings";
import { MoreFact, MoreFrame, MoreSection } from "@/views/more/MorePage";

export function IdentityPage({
  devices,
  onClose,
  onRestyled,
}: {
  /** The shell's own polled table — **not fetched again here**. Two
      readers of one answer is how two surfaces come to show two
      different names for one machine, and the shell is already asking
      (`App.tsx`'s devices effect). */
  devices: DeviceRow[];
  onClose: () => void;
  /** A restyle repaints this machine everywhere at once — the shell
      refetches rows and devices together and every face on screen is
      painted from that one answer. */
  onRestyled: () => void;
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
  const [renaming, setRenaming] = useState(false);

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
      {/* **One settings group, no name of its own** — R0 says 身份. The
          machine's name is a row whose act is 改名… (it asks for the new
          name, so the ellipsis), then the id and the version as facts.

          ⚠️ 墓碑 (2026-09-27): this was a head of its own — the face at
          `avatar-lg` and the name as a `title-md` heading editable by
          double-click — which is a second title in the board and a
          setting drawn outside the four row kinds. The face is the
          editor's subject just below (`FaceSettings` draws it large), so
          the copy up here went; the rename is the row. `shown`, never
          `name`: the first is what this machine is called, the second
          is what the network files it under. */}
      <MoreSection name="machine">
        {me && (
          <ActionRow
            name="rename"
            label={<span data-identity-name>{me.shown}</span>}
            actionLabel={gui.rename_ask}
            onPress={() => setRenaming(true)}
            mark={{ "data-identity-rename": "" }}
          />
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
      {me && (
        <RenameDialog
          open={renaming}
          id={me.id}
          name={me.shown}
          onClose={() => setRenaming(false)}
          onRenamed={() => {
            setRenaming(false);
            onRestyled();
          }}
        />
      )}

      {/* 头像 (docs/MORE.md 头像页: 脸编辑器). **Empty machine — this
          one**: the page is about this machine, and the same editor
          opens on the map for any other. */}
      <MoreSection name="avatar" label={gui.settings_face}>
        <FaceSettings onChanged={onRestyled} />
      </MoreSection>

      {/* 本机开着哪几张界面 (docs/MORE.md 头像页). Drawn only once khor
          has answered — a list of faces painted from a default this file
          kept would be a page reporting a probe it never made. */}
      {about !== null && (
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

/**
 * 改名…: the name in a box, and the one act that writes it — the same
 * writer the list row's double-click uses (`renameMachine`, behind
 * `EditableName`). An emptied box gives the machine its reported name
 * back, as it does there. A refusal stays in the window in the node's
 * own sentence, under the box, so the fix is an edit.
 */
function RenameDialog({
  open,
  id,
  name,
  onClose,
  onRenamed,
}: {
  open: boolean;
  id: string;
  name: string;
  onClose: () => void;
  onRenamed: () => void;
}) {
  const [typed, setTyped] = useState(name);
  const [going, setGoing] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  useEffect(() => {
    if (open) {
      setTyped(name);
      setRefused(null);
      setGoing(false);
    }
  }, [open, name]);
  const write = () => {
    if (going) return;
    setGoing(true);
    setRefused(null);
    renameMachine(id, typed.trim()).then(
      () => {
        setGoing(false);
        onRenamed();
      },
      (e: unknown) => {
        setGoing(false);
        setRefused(String(e instanceof Error ? e.message : e) || gui.rename_failed);
      },
    );
  };
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent data-identity-rename-dialog>
        <DialogHeader>
          <DialogTitle>{gui.rename}</DialogTitle>
        </DialogHeader>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            write();
          }}
        >
          <Input
            data-identity-rename-input
            autoComplete="off"
            aria-label={gui.rename}
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            className="min-h-touch w-full"
          />
        </form>
        {refused !== null && (
          <p data-identity-rename-refused className="text-callout text-destructive">
            {refused}
          </p>
        )}
        <DialogFooter>
          <Button onClick={onClose}>{gui.delete_keep}</Button>
          <Button variant="primary" disabled={going} onClick={write}>
            {gui.rename}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
