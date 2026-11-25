import type { Ticket } from "@/api";

export type JoinNetworkSample = {
  device: string;
  credential: string;
  qr: Ticket["qr"];
};

/** Replaced only by the Developer Lab's Vite entry. */
export function getJoinNetworkLab(): JoinNetworkSample | null {
  return null;
}
