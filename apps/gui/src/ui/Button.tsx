// The one button. Pages compose it; they never restyle heights or
// paddings locally — that is how mandala grew four button heights.
import type { ReactNode } from "react";

export function UiButton({
  label,
  onClick,
  className,
  children,
}: {
  /** Accessible name; also the tooltip. */
  label: string;
  onClick?: () => void;
  className?: string;
  children: ReactNode;
}) {
  return (
    <button type="button" aria-label={label} title={label} className={className} onClick={onClick}>
      {children}
    </button>
  );
}
