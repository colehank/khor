// NavRow — a setting that opens a sub-page inside the board.
// The current value is shown on the right with a › chevron.
// Pressing it pushes to the sub-page; R0 then shows a ‹ back button.
//
// `data-setting-row="nav"` is the gate anchor.
import { IconChevronRight } from "@/components/icons";
import { cn } from "@/lib/utils";

export function NavRow({
  name,
  label,
  hint,
  value,
  onPress,
  disabled = false,
}: {
  name: string;
  label: string;
  hint?: string;
  /** Current value shown before the chevron. Omit for no current value. */
  value?: string;
  onPress: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      data-setting-row="nav"
      data-setting-row-name={name}
      disabled={disabled}
      onClick={onPress}
      className={cn(
        "flex min-h-row-1 w-full items-center gap-row px-row py-in text-left",
        "hover:bg-secondary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
        "disabled:cursor-not-allowed disabled:opacity-50",
      )}
    >
      <div className="flex min-w-0 flex-1 flex-col gap-0">
        <span className="text-body">{label}</span>
        {hint && (
          <span className="text-callout text-muted-foreground">{hint}</span>
        )}
      </div>
      {/* components/SettingsGroup `nav`: the value in `body` secondary,
          then the family's chevron at 12 in tertiary — not a typed ›. */}
      <div className="flex flex-none items-center gap-in text-body text-muted-foreground">
        {value && <span>{value}</span>}
        <IconChevronRight className="size-3 text-tertiary" />
      </div>
    </button>
  );
}
