import type { SessionRow } from "../api";
import { gui } from "../gen/catalog";
import { UiButton } from "../ui/Button";
import { IconBack } from "../ui/Icons";
import { word } from "../words";

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
    <section className="detail">
      <header className="detail-header">
        {/* The back button exists only on the narrow face: there the
            list is genuinely off-screen, so "back" has somewhere to go.
            Wide keeps the list on screen — nothing to go back from. */}
        {narrow && (
          <UiButton className="back-btn" label={gui.back} onClick={onBack}>
            <IconBack />
          </UiButton>
        )}
        <span className="detail-title">{row ? row.title || row.id : ""}</span>
      </header>
      <div className="detail-body">
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
