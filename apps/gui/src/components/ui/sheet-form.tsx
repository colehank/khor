// The form inside a sheet (docs/design-system components/Dialog, 「sheet
// 里的表单」, 2026-09-28): questions in one column, right-aligned against
// their answers in the next — macOS's form habit — so a sheet reads down
// one edge instead of as a stack of label-over-control blocks. 新会话,
// 选择目录, 注册账户 and 导入 all stand on this one grid.
//
// **`min-w-0` on the grid and `minmax(0,1fr)` on its second column are
// load-bearing**: `DialogContent` is a grid and this is its item, and a
// grid item defaults to `min-width: auto` — one long path inside would
// push the whole sheet past its `max-w-lg` and `truncate` would never
// fire (the new-session wizard's scar, `check-acp` 17).
import type { HTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib/utils";

export function SheetForm({ className, children, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...rest}
      className={cn("grid min-w-0 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-row gap-y-row text-callout", className)}
    >
      {children}
    </div>
  );
}

/** A question's label, in the first column. `htmlFor` makes it a real
    `<label>` for a text box; a label beside a group of buttons stays a
    `<span>` (the group names itself with `aria-label`). */
export function FormLabel({
  htmlFor,
  children,
  mark,
}: {
  htmlFor?: string;
  children: ReactNode;
  mark?: Record<string, string>;
}) {
  const cls = "text-right text-muted-foreground";
  return htmlFor ? (
    <label htmlFor={htmlFor} {...mark} className={cls}>
      {children}
    </label>
  ) : (
    <span {...mark} className={cls}>
      {children}
    </span>
  );
}
