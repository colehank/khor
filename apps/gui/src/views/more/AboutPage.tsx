// 关于, the last of 设置's seven pages (UX-16): which khor this is, and
// where its store is and how big — read-only, off one answer, with the
// one act a store has (open its directory).
//
// ⚠️ 墓碑 (UX-16): this was 设置 → 通用, which drew the node's 会话 and
// 工作区 rows above these; they are pages of their own now. The list of
// settings deliberately *not* offered, and why (密度, 通知, the web
// face's port, 开机自启, 默认智能体, 默认启动目录, VS Code's window), is
// in `docs/handoff/001.md` — read it before adding a switch.
import { useEffect, useState } from "react";

import { fetchAbout, launchOn, type About } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { gui } from "@/gen/catalog";
import { useHereHosts } from "@/lib/here";
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

export function AboutPage({ onClose }: { onClose: () => void }) {
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
      page="about"
      onClose={onClose}
      // Which khor this is — the same unit the machine page's second line
      // says it in (`card-version`), off the answer this page asked for.
      subtitle={about?.version ? gui.card_version(about.version) : undefined}
    >
      {/* 存储 and 关于: read-only, and both come off the same answer.
          Nothing is drawn where it could not be read — a path or a size
          invented here would be a page describing a store it never
          looked at. */}
      {about !== null && (
        <>
          {/* One card, no name of its own (the page's name says it): the
              version, then where the store is and how big. */}
          <MoreSection name="about">
            {/* The same word the identity card uses for the same fact
                (`identity-version`): one fact, one name. */}
            <MoreFact name="version" label={gui.identity_version}>
              {about.version}
            </MoreFact>
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
        </>
      )}
    </MoreFrame>
  );
}
