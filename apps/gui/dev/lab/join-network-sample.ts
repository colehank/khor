import type { JoinNetworkSample } from "../../src/lib/join-network-lab";
import { previewPairingCredential } from "./pairing";
import { labPairingTicket } from "./world";

const sample: JoinNetworkSample = {
  device: "Mac server",
  credential: previewPairingCredential,
  qr: labPairingTicket.qr,
};

export function getJoinNetworkLab() {
  const runtime = (window as Window & { __KHOR_LAB__?: { mode: string; scene: string } }).__KHOR_LAB__;
  return runtime?.mode === "frontend" && runtime.scene === "pairing" ? sample : null;
}
