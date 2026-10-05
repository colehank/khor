// 外观, 设置's first page (UX-16: 主题, 语言, 字号, 标记画法, 底栏顺序).
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
  /** This machine, whose face shows what 头像's rows draw (its system's
      mark, on the face, on the corner, or as the base with the face on
      the corner, as the two rows say). */
  me: DeviceRow | null;
  /** The shell re-reads the store after a change. */
  onSettingChanged: () => void;
}) {
  return (
    <MoreFrame page="appearance" onClose={onClose}>
      <ConfigSections
        page="appearance"
        onChanged={onSettingChanged}
        // 头像's two rows (UX-32) both show this machine's face with its
        // system's mark, assembled the way the two rows together say.
        previews={
          me
            ? {
                "kind-mark": <MachineFace face={me.face} system={me.vitals?.vitals.system ?? null} />,
                "corner-is": <MachineFace face={me.face} system={me.vitals?.vitals.system ?? null} />,
              }
            : undefined
        }
        // 角标是 is a question only under 角标 (用不上的控件不画).
        shownWhen={{ "corner-is": { key: "kind-mark", value: "corner" } }}
      />
      <TypeSizeSection />
      <NavOrderSection />
    </MoreFrame>
  );
}
