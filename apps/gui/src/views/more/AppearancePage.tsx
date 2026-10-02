// 外观 (docs/SHELL.md 第十七轮 §1): the second section of 更多.
//
// What the whole app is drawn in — 主题 and 界面语言. The rows are the
// node's (`khor_node::config::SECTIONS` puts `appearance` and `language`
// on this page), drawn as `PopupRow`s by `ConfigSections`, so this file
// keeps no list of settings and cannot come to word one differently than
// `khor config list` does.
//
// 头像 is not here: the face editor lives on 身份, the page about this
// machine (docs/MORE.md 第二轮 地图).
import type { DeviceRow } from "@/api";
import { MachineFace } from "@/components/SessionFace";
import { ConfigSections } from "@/views/more/ConfigRows";
import { MoreFrame } from "@/views/more/MorePage";
import { NavOrderSection, TypeSizeSection } from "@/views/more/AppearanceRows";

export function AppearancePage({
  onClose,
  onSettingChanged,
  me,
}: {
  onClose: () => void;
  /** This machine, whose face shows what 类型标记 draws (its system's
      mark, on the face or in the corner as the setting says). */
  me: DeviceRow | null;
  /** The shell re-reads the store after a change. */
  onSettingChanged: () => void;
}) {
  return (
    <MoreFrame page="appearance" onClose={onClose}>
      <ConfigSections
        page="appearance"
        onChanged={onSettingChanged}
        previews={me ? { "kind-mark": <MachineFace face={me.face} system={me.vitals?.vitals.system ?? null} /> } : undefined}
      />
      <TypeSizeSection />
      <NavOrderSection />
    </MoreFrame>
  );
}
