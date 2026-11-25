import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconChevronRight } from "@/components/icons";
import { cn } from "@/lib/utils";

export const fileTreeButtonClass = "flex min-w-0 flex-1 items-center gap-2 self-stretch text-left";
export const fileTreeSelectedClass = "font-title text-foreground";

export function fileTreeIndent(depth: number) {
  // Start the first level on the page's left line.
  return { paddingLeft: `calc(var(--tree-inset) + var(--tree-step) * ${depth})` };
}

type RowProps = ComponentPropsWithoutRef<"div"> & {
  depth: number;
  name: string;
  directory?: string;
  open?: boolean;
  highlight?: boolean;
  mark?: ReactNode;
};

export function FileTreeRow({ depth, name, directory, open, highlight, mark, className, children, ...props }: RowProps) {
  const isDirectory = directory !== undefined;
  return (
    <div
      {...props}
      data-tree-entry
      data-tree-directory={directory}
      data-file={isDirectory ? undefined : name}
      data-depth={depth}
      data-open={open}
      style={fileTreeIndent(depth)}
      className={cn(
        "group relative flex min-h-[var(--tree-row)] items-center gap-2 pr-3 text-callout",
        isDirectory
          ? "hover:bg-muted/50"
          : "isolate before:absolute before:inset-y-0 before:inset-x-2 before:-z-10 before:rounded-md before:content-['']",
        !isDirectory && (highlight ? "before:bg-fill-strong" : "hover:before:bg-fill"),
        name.startsWith(".") && "text-muted-foreground",
        className,
      )}
    >
      {/* Each row draws the portion of its ancestors' guides that crosses it. */}
      {Array.from({ length: depth }, (_, level) => (
        <span
          key={level}
          data-guide={level}
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 w-px bg-border"
          style={{ left: `calc(var(--tree-inset) + var(--tree-step) * ${level} + var(--tree-slot) / 2)` }}
        />
      ))}
      {children}
      {/* Mirror the leading slot so directory and file marks share a column. */}
      <span data-tree-mark-slot className="flex size-[var(--tree-slot)] flex-none items-center justify-center" aria-hidden={isDirectory || mark === undefined ? true : undefined}>
        {mark}
      </span>
    </div>
  );
}

type LabelProps = { name: string; expanded?: boolean; className?: string; children?: ReactNode };

export function FileTreeRowLabel({ name, expanded, className, children }: LabelProps) {
  if (expanded !== undefined) {
    return (
      <>
        <span data-tree-slot className="flex size-[var(--tree-slot)] flex-none items-center justify-center" aria-hidden="true">
          <IconChevronRight className={cn("size-affordance flex-none transition-transform", expanded && "rotate-90")} />
        </span>
        {children ?? <span data-dir-name className={cn("min-w-0 flex-1 truncate", className)}>{name}</span>}
      </>
    );
  }
  return (
    <>
      <FileKindIcon name={name} className="h-[var(--tree-slot)] w-[var(--tree-slot)] items-center ps-[calc(var(--tree-slot)/2-var(--affordance)/2+var(--tree-icon-nudge))] [&>svg]:!size-4" />
      {children ?? <span data-file-name className="min-w-0 flex-1 truncate">{name}</span>}
    </>
  );
}
