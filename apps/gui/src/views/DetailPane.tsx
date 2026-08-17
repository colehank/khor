import type { SessionRow } from "@/api";
import { IconBack } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { ChatView } from "@/views/ChatView";
import { word } from "@/words";

// Per-kind faces: a GUI session gets the conversation (ChatView); the
// rest (transfer card, terminal) land with their own batches, and until
// then the pane states the row's facts and nothing else — no invented
// copy (docs/UX.md 文案: 装饰性说明归零). Keyed by the row's **kind**,
// never its id: a GUI session's id is spelled `tui/…` on purpose (the
// vendor-session agreement — `gui_host` module head), so the id is an
// address and the kind is the behaviour.
export function DetailPane({
  row,
  narrow,
  onBack,
}: {
  row: SessionRow | null;
  narrow: boolean;
  onBack: () => void;
}) {
  return (
    <section className="flex h-full min-w-0 flex-col">
      <header data-detail-header className="flex h-ctl-lg flex-none items-center gap-2 border-b px-3">
        {/* The back button exists only on the narrow face: there the
            list is genuinely off-screen, so "back" has somewhere to go.
            Wide keeps the list on screen — nothing to go back from. */}
        {narrow && (
          <Button variant="ghost" size="icon" aria-label={gui.back} data-back onClick={onBack}>
            <IconBack />
          </Button>
        )}
        <span className="truncate font-semibold">{row ? row.title || row.id : ""}</span>
      </header>
      {row && row.kind === "gui" ? (
        // Keyed so switching sessions remounts the chat: its cursor,
        // frames and attachment all belong to one conversation.
        <ChatView key={row.id} id={row.id} />
      ) : (
        <div className="grid flex-1 place-items-center text-center text-sm text-muted-foreground">
          {row ? (
            <div>
              <div style={{ color: `var(--state-${row.word})` }}>{word(row.word)}</div>
              <div>{row.kind}</div>
            </div>
          ) : (
            <div>{gui.pick_a_session}</div>
          )}
        </div>
      )}
    </section>
  );
}
