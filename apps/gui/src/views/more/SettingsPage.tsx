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
import { ActionRow } from "@/components/settings/ActionRow";
import { gui } from "@/gen/catalog";
import { useHereHosts } from "@/lib/here";
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

/** A path that breaks after its slashes, never inside a name (the audit
    fix plan F4.9; N23: 「/Volumes/disk0/kho / r-redesign-home」). */
function SlashBreaks({ text }: { text: string }) {
  const parts = text.split("/");
  return (
    <>
      {parts.map((p, i) => (
        <span key={i}>
          {p}
          {i < parts.length - 1 && (
            <>
              /<wbr />
            </>
          )}
        </span>
      ))}
    </>
  );
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
  /** Whether this machine has something to open a directory with — not
      a phone (docs/IOS.md 第二步: nothing is opened there, and the
      refusal said 「设 KHOR_LAUNCHER」, advice for a computer). */
  const opens = useHereHosts();

  // **No node is opened on the other side**, so this answers even on a
  // machine whose store refuses — which is the moment a person most
  // needs to be told which store that is (`khor_gui_core::about`).
  useEffect(() => {
    void fetchAbout().then(setAbout, () => setAbout(null));
  }, []);

  return (
    <MoreFrame
      page="settings"
      onClose={onClose}
      // Which khor this is — the same unit the machine page's second line
      // says it in (`card-version`), off the answer this page asked for.
      subtitle={about?.version ? gui.card_version(about.version) : undefined}
    >
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
            {/* **One row: where it is, and the act on it.** The path is
                the row's value and 打开这个目录 its button — the machine's
                own opener, the same road 「在那台机器上打开」 takes; empty
                `machine` is this one (`files::launch`).
                ⚠️ 墓碑 (2026-09-28): 「store 在哪儿」 was two rows, one
                holding the path and one the button, the title twice. */}
            {opens ? (
              <ActionRow
                name="store-open"
                label={gui.store_path}
                value={
                  <span data-more-fact="store-path">
                    <SlashBreaks text={about.root} />
                  </span>
                }
                actionLabel={gui.store_open}
                mark={{ "data-store-open": "" }}
                onPress={() => {
                  void launchOn("", about.root).catch(() => {});
                }}
              />
            ) : (
              // The path is still a fact worth reading; the act is not drawn.
              <MoreFact name="store-path" label={gui.store_path}>
                <span data-more-fact="store-path">
                  <SlashBreaks text={about.root} />
                </span>
              </MoreFact>
            )}
            <MoreFact name="store-size" label={gui.store_size}>
              {bytes(about.store_bytes)}
            </MoreFact>
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
