// 自检 (docs/MORE.md 自检).
//
// **Batch A is the shell.** The list of checks — one row each for the
// version, the identity key, the store, the resident, the web face, the
// relays, the peers, the hooks, the agents, VS Code, the disk, the
// clock and the opener — is `khor doctor`'s answer, and it lands in
// batch B. An empty list drawn before then would be a page claiming
// everything is fine because it asked nothing.
import { MoreFrame } from "@/views/more/MorePage";

export function DoctorPage({ onClose }: { onClose: () => void }) {
  return <MoreFrame page="doctor" onClose={onClose} />;
}
