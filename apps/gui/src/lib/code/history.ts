import type { historyField } from "@codemirror/commands";
import type { EditorSelection, StateField } from "@codemirror/state";

type HistoryValue = typeof historyField extends StateField<infer Value> ? Value : never;

/** Owned by an open file/tab, never by a process-wide cache. Keep the
    immutable history value, not an EditorState (which holds old callbacks
    and extensions). This is memory only: JSON would discard the inverse
    effects that git/merge edits can carry in their undo entries. */
export type EditorHistorySlot = {
  saved?: { key: string; text: string; selection: EditorSelection; history: HistoryValue };
};
