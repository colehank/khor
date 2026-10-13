import type { PillNotice } from "@/lib/pill-notices";

/** What is happening between this operator and a session, drawn on the
 * pill perimeter. It can reconnect or upload while the agent is busy
 * or idle. A failed transfer never changes the session's work state.
 *
 * TabFace.session supplies the avatar's state; SessionRow.unread
 * supplies the title's attention mark. Neither is an interaction.
 */
export type SessionInteraction = {
  /** The file currently going in; the pill uses its stem as its title. */
  drop?: { name: string };
  /** Reconnect/transfer failures, most urgent first; not unread messages. */
  notices?: PillNotice[];
};

export type InteractionSignal = PillNotice["kind"] | "upload" | "settling";

export function interactionSignal(interaction?: SessionInteraction | null): InteractionSignal | undefined {
  return interaction?.notices?.[0]?.kind ?? (interaction?.drop ? "upload" : undefined);
}
