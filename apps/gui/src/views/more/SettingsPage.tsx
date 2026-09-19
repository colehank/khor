// 设置 (docs/MORE.md 设置).
//
// **Batch A's version carries what already existed and nothing else**:
// the face axes and the machine's own relays, moved here whole from the
// dialog they used to be (`views/FaceSettings`). The rest of the page —
// 外观, 会话, 工作区, 网页脸, 通知, 启动, 存储, 关于 — arrives with
// `khor config` in batch D, and nothing is drawn for it in the meantime:
// a greyed switch for a key nothing stores is a promise this page cannot
// keep (docs/MORE.md: 没有落地实现的键不画).
import { FaceSettings } from "@/views/FaceSettings";
import { MoreFrame, MoreSection } from "@/views/more/MorePage";
import { gui } from "@/gen/catalog";

export function SettingsPage({
  onClose,
  onChanged,
}: {
  onClose: () => void;
  /** A restyle repaints this machine everywhere at once, because the
      shell refetches rows and devices together and every face on screen
      is painted from that one answer. */
  onChanged: () => void;
}) {
  return (
    <MoreFrame page="settings" onClose={onClose}>
      {/* The heading this section never had. In the dialog the three
          axes stood directly under the dialog's own title, so 「这是在
          设什么」 was answered by the frame; on a page with sections
          under one title the axes need to say whose they are. */}
      <MoreSection name="face" label={gui.settings_face}>
        <FaceSettings onChanged={onChanged} />
      </MoreSection>
    </MoreFrame>
  );
}
