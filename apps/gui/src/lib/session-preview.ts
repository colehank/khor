import type { SessionRow } from "@/api";

/** Content shared by list rows and transient notices; prompt frames are not content. */
export function sessionContentPreview(row: Pick<SessionRow, "last" | "doing">): string | null {
  if (row.doing?.command) return row.doing.command;
  return row.last && /[\p{L}\p{N}]/u.test(row.last) ? row.last : null;
}
