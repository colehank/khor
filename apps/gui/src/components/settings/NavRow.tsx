import { IconChevronRight } from "@/components/icons";

export function NavRow({ name, label, value, onOpen }: {
  name: string;
  label: string;
  value?: string;
  onOpen: () => void;
}) {
  return (
    <button
      type="button"
      data-setting-row="nav"
      data-setting-row-name={name}
      onClick={onOpen}
      className="flex w-full items-center gap-row rounded-md px-row py-in text-left text-body transition-colors hover:bg-fill focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
    >
      <span className="flex-none">{label}</span>
      <span className="min-w-0 flex-1 text-right text-callout text-muted-foreground">{value}</span>
      <IconChevronRight aria-hidden="true" data-setting-trail className="size-affordance flex-none text-muted-foreground" />
    </button>
  );
}
