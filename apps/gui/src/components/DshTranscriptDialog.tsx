import { useEffect, useRef, useState } from "react";

import { dshTranscript, type DshLine } from "@/api";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { Dialog, DialogHeader, DialogTitle, TaskActions } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";

/** What each kind of line is called on screen. */
const who = (line: DshLine): string => {
  switch (line.who) {
    case "user":
      return gui.dsh_read_speaker_user;
    case "assistant":
      return gui.dsh_read_speaker_assistant;
    case "thought":
      return gui.dsh_read_speaker_thought;
    case "tool":
      return gui.dsh_read_speaker_tool;
    case "tool-result":
      return gui.dsh_read_speaker_tool_result;
    default:
      return line.who;
  }
};

/**
 * 读一下: one DSH conversation, as the lines its own events state.
 *
 * Read-only by construction — DSH owns the conversation, and this is the
 * same reader `khor agents dsh <session>` prints from, so a face and a
 * terminal cannot disagree about what was said. Tool calls show their
 * name and input; a failed result says so.
 *
 * The list is long (a real session here is twelve thousand lines), so it
 * scrolls inside the dialog and draws as plain text: a replay rich enough
 * to fold and colour is the protocol's job, not this door's.
 */
export function DshTranscriptDialog({
  open,
  id,
  title,
  onClose,
}: {
  open: boolean;
  id: string | null;
  title: string;
  onClose: () => void;
}) {
  const [lines, setLines] = useState<DshLine[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    setLines(null);
    setError(null);
    if (!open || id === null) return;
    dshTranscript(id).then(
      (rows) => {
        if (alive.current) setLines(rows);
      },
      (why: unknown) => {
        if (alive.current) setError(String((why as Error)?.message ?? why));
      },
    );
    return () => {
      alive.current = false;
    };
  }, [open, id]);
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogHeader>
        <DialogTitle>{title}</DialogTitle>
      </DialogHeader>
      <div className="max-h-[60vh] overflow-y-auto px-row py-in" data-dsh-transcript>
        {error !== null ? (
          <RegionState state="error" message={error} />
        ) : lines === null ? (
          <RegionState state="loading-never" />
        ) : (
          lines.map((line, index) => (
            <div key={index} data-dsh-line={line.who} className="mb-in">
              <span className="text-caption text-muted-foreground">{who(line)}</span>
              <div className="whitespace-pre-wrap break-words text-body">{line.text}</div>
            </div>
          ))
        )}
      </div>
      <TaskActions
        busy={false}
        dirty={false}
        cancel={undefined}
        primary={
          <Button variant="primary" onClick={onClose}>
            {gui.dsh_read}
          </Button>
        }
      />
    </Dialog>
  );
}
