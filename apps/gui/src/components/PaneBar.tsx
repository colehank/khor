// The top of every list pane. No page in this app carries a title
// (docs/UX.md): the pane's name is an aria-label on the pane itself,
// invisible but read aloud, and the strip along the top is what you can
// *do* here — search always, plus whatever that pane can filter and
// create.
//
// **One file, composed by parameters.** mandala ended up with two search
// boxes that looked alike and behaved differently; the second copy is how
// that starts, so panes pass props here rather than growing their own
// bar. Anything with focus, keyboard or overlay semantics is Radix by way
// of shadcn — a hand-rolled menu gets the easy 80% and none of the rest.
//
// Every control carries a name from the catalog. A control the eye reads
// by its shape still has to say what it is out loud.
import { IconFilter, IconPlus, IconSearch } from "@/components/icons";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";

/** One choice in the filter menu. `key` is the backend's key, never a
    word this layer invented — see `PaneBar`'s filter prop. */
export type FilterOption = { key: string; label: string };

export type PaneAction = { key: string; label: string; onSelect: () => void };

export function PaneBar({
  searchLabel,
  query,
  onQuery,
  filter,
  actions,
  actionsLabel,
  filterLabel,
}: {
  /** Names the search box and stands in as its placeholder. */
  searchLabel: string;
  query: string;
  onQuery: (q: string) => void;
  /**
   * Omitted where a pane has nothing to filter by. The options are the
   * caller's business, and the rule there is that they come from what the
   * backend actually sent — this app never re-derives a state
   * (docs/UX.md 状态呈现), so it must not enumerate states either.
   */
  filter?: { options: FilterOption[]; chosen: string[]; onToggle: (key: string) => void };
  filterLabel?: string;
  /**
   * What "+" opens. Only things that work today go in here: a menu item
   * that greys out or does nothing teaches people not to open the menu.
   */
  actions?: PaneAction[];
  actionsLabel?: string;
}) {
  return (
    <div
      data-pane-bar
      className="flex h-ctl-lg flex-none items-center gap-1 border-b px-2"
    >
      <div className="relative min-w-0 flex-1">
        <IconSearch className="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-muted-foreground" />
        <Input
          data-pane-search
          type="search"
          aria-label={searchLabel}
          placeholder={searchLabel}
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          className="border-0 bg-transparent pl-7 shadow-none focus-visible:ring-0"
        />
      </div>

      {filter && filterLabel && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label={filterLabel}
              data-pane-filter
              data-on={filter.chosen.length > 0}
              className="data-[on=true]:text-primary"
            >
              <IconFilter />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {filter.options.map((o) => (
              <DropdownMenuCheckboxItem
                key={o.key}
                data-filter-option={o.key}
                checked={filter.chosen.includes(o.key)}
                onCheckedChange={() => filter.onToggle(o.key)}
                // Ticking one is rarely the whole thought; closing the
                // menu on the first tick makes the second one a chore.
                onSelect={(e) => e.preventDefault()}
              >
                {o.label}
              </DropdownMenuCheckboxItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}

      {actions && actions.length > 0 && actionsLabel && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={actionsLabel} data-pane-new>
              <IconPlus />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {actions.map((a) => (
              <DropdownMenuItem key={a.key} data-new-item={a.key} onSelect={a.onSelect}>
                {a.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
}
