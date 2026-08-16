import type { SessionRow } from "@/api";
import { IconBack } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { word } from "@/words";

// Per-kind faces (chat stream, transfer card, terminal) land with their
// own batches; until then the pane states the row's facts and nothing
// else — no invented copy (docs/UX.md 文案: 装饰性说明归零).
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
    </section>
  );
}
