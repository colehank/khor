/** Khor and ACP conversations share navigation regardless of their session ID. */
export function isConversationKind(kind: string | undefined): boolean {
  return kind === 'gui' || kind === 'broadcast';
}
