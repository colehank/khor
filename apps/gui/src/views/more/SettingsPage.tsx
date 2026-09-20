// 设置 (docs/MORE.md 设置).
//
// **What is here is what has a reader.** The face axes and the relays
// came whole from the dialog this used to be (批 A); 会话 and 工作区 are
// `khor config`'s keys (批 D), drawn from the node's own table so the
// page cannot come to group or word them differently than `khor config
// list` does; 存储 and 关于 state two facts and offer one act.
//
// **What is deliberately not here**, each because nothing behind it
// exists to switch (docs/MORE.md: 没有落地实现的键不画): 密度 (every
// spacing token would need two values — a typesetting pass, not a key),
// 通知 (the tree has no notification of any kind to turn off), 网页脸的
// 端口与「对局域网开放」 (the port is `KHOR_WEB_PORT` and the face binds
// every interface; there is no switch to move), 开机自启 (a real
// LaunchAgent, and no gate in this tree can verify one), 默认智能体 and
// 默认启动目录 (open value domains — a key whose values cannot be
// written down cannot be refused by name, and the first typo buys a
// wizard with nothing selected), and VS Code 开在弹窗还是系统浏览器 (the
// popup window is what the user asked for, and the other road cannot
// raise, hide or close what it opened). The reasons are in
// `docs/handoff/001.md`; this list is here so the next person reads them
// before adding a switch.
import { useEffect, useState } from "react";

import { fetchAbout, launchOn, type About } from "@/api";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { ConfigSections } from "@/views/more/ConfigRows";
import { MoreFact, MoreFrame, MoreSection } from "@/views/more/MorePage";

/** Bytes as a person reads them. **Binary steps and one decimal**, the
    same shape the machine card's disk readings take — a store is
    measured against a disk, and two units that disagree about what a
    「GB」 is would be two numbers nobody can compare. */
function bytes(n: bigint): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = Number(n);
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${i === 0 ? v : v.toFixed(1)} ${units[i]}`;
}

export function SettingsPage({
  onClose,
  onSettingChanged,
}: {
  onClose: () => void;
  /** A setting changed: the shell re-reads the store, and every screen
      that reads one of these keys is drawn from that answer rather than
      from what this page just sent. */
  onSettingChanged: () => void;
}) {
  const [about, setAbout] = useState<About | null>(null);

  // **No node is opened on the other side**, so this answers even on a
  // machine whose store refuses — which is the moment a person most
  // needs to be told which store that is (`khor_gui_core::about`).
  useEffect(() => {
    void fetchAbout().then(setAbout, () => setAbout(null));
  }, []);

  return (
    <MoreFrame page="settings" onClose={onClose}>
      {/* 头像 left this page on 2026-09-21 (docs/MORE.md 第二轮 地图:
          头像 = 这台机器是谁). The editor is on the avatar's own page
          now, where the thing it edits is already the subject — and on
          the map's machine card, for every other machine. */}

      <ConfigSections page="settings" onChanged={onSettingChanged} />

      {/* 存储 and 关于: read-only, and both come off the same answer.
          Nothing is drawn where it could not be read — a path or a size
          invented here would be a page describing a store it never
          looked at. */}
      {about !== null && (
        <>
          <MoreSection name="store" label={gui.settings_store}>
            <MoreFact name="store-path" label={gui.store_path}>
              {about.root}
            </MoreFact>
            <MoreFact name="store-size" label={gui.store_size}>
              {bytes(about.store_bytes)}
            </MoreFact>
            {/* The machine's own opener, the same road 「在那台机器上
                打开」 takes — empty `machine` is this one (`files::launch`). */}
            <Button
              variant="outline"
              data-store-open
              onClick={() => {
                void launchOn("", about.root).catch(() => {});
              }}
              className="min-h-touch self-start"
            >
              {gui.store_open}
            </Button>
          </MoreSection>

          <MoreSection name="about" label={gui.settings_about}>
            {/* The same word the identity card uses for the same fact
                (`identity-version`): one fact, one name. */}
            <MoreFact name="version" label={gui.identity_version}>
              {about.version}
            </MoreFact>
          </MoreSection>
        </>
      )}
    </MoreFrame>
  );
}
