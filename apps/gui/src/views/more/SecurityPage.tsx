// 安全 (docs/MORE.md 安全).
//
// **Batch A is the shell.** What stands here — the app lock and its two
// keys, the web face's key, the paired machines' revocation, the three
// confirmations, 重置身份 — arrives with `khor lock` in batch C. Nothing
// is drawn for it in the meantime: a switch for a lock nothing stores
// would be a promise about a locked screen that never comes.
import { MoreFrame } from "@/views/more/MorePage";

export function SecurityPage({ onClose }: { onClose: () => void }) {
  return <MoreFrame page="security" onClose={onClose} />;
}
