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
import { Fragment } from "react";

import { IconFilter, IconPlus, IconSearch } from "@/components/icons";
import { Omnibox, type OmniAxis } from "@/components/Omnibox";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";

/** One choice in the filter menu. `key` is the backend's key, never a
    word this layer invented — see `PaneBar`'s filter prop.

    `axis` names the heading this option belongs under. The menu starts
    a heading where it **changes between neighbouring options** — the
    same way `SessionsList` starts a group heading, and for the same
    reason: the caller declares which axis each option is on, this only
    notices the boundary. An axis with no options therefore cannot draw
    a heading over nothing. */
export type FilterOption = { key: string; label: string; axis?: string };

export type PaneAction = { key: string; label: string; onSelect: () => void };

/** One way of laying the list out; `key` is the backend's arrangement
    key, never a word this layer invented. */
export type ArrangeOption = { key: string; label: string };

export function PaneBar({
  searchLabel,
  query,
  onQuery,
  filter,
  axes,
  actions,
  actionsLabel,
  filterLabel,
  arrange,
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
  /**
   * The axes the search box can narrow by, when the pane has any. Given
   * together with `filter`, because the two are one state: the chips in
   * the box and the ticks in the menu are the same keys, and the pane
   * hands both faces the same array.
   */
  axes?: OmniAxis[];
  filterLabel?: string;
  /**
   * How the list is laid out. Shares the filter's menu because both
   * answer "what am I looking at", and sits above the words with a rule
   * between them: these are exclusive (picking one drops the last),
   * the words are not, and the two marks differ so that reads off the
   * screen rather than having to be learned.
   */
  arrange?: {
    label: string;
    options: ArrangeOption[];
    chosen: string;
    onChoose: (key: string) => void;
  };
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
      // `min-h`, not `h`: chips make the box taller than one row, and a
      // bar with a fixed height would clip them or squeeze the list.
      className="flex min-h-ctl-lg flex-none items-center gap-1 border-b px-2"
    >
      {/* **One search slot, two shapes.** A pane that has axes to narrow
          by gets the omnibox; one that does not gets the plain box it
          always had. Not a second control and not a second box — the
          alternative is what this file's head warns about, two search
          fields that look alike and behave differently. */}
      {axes && axes.length > 0 && filter ? (
        <Omnibox
          label={searchLabel}
          query={query}
          onQuery={onQuery}
          axes={axes}
          chosen={filter.chosen}
          onToggle={filter.onToggle}
        />
      ) : (
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
      )}

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
            {arrange && (
              <>
                <DropdownMenuLabel>{arrange.label}</DropdownMenuLabel>
                <DropdownMenuRadioGroup value={arrange.chosen} onValueChange={arrange.onChoose}>
                  {arrange.options.map((o) => (
                    <DropdownMenuRadioItem
                      key={o.key}
                      value={o.key}
                      data-arrange-option={o.key}
                      // Same reason the word items stay open: choosing
                      // one is rarely the end of the thought.
                      onSelect={(e) => e.preventDefault()}
                    >
                      {o.label}
                    </DropdownMenuRadioItem>
                  ))}
                </DropdownMenuRadioGroup>
                <DropdownMenuSeparator />
              </>
            )}
            {filter.options.map((o, i) => (
              <Fragment key={o.key}>
                {o.axis && o.axis !== filter.options[i - 1]?.axis && (
                  <>
                    {i > 0 && <DropdownMenuSeparator />}
                    <DropdownMenuLabel data-filter-axis={o.axis}>{o.axis}</DropdownMenuLabel>
                  </>
                )}
                <DropdownMenuCheckboxItem
                  data-filter-option={o.key}
                  checked={filter.chosen.includes(o.key)}
                  onCheckedChange={() => filter.onToggle(o.key)}
                  // Ticking one is rarely the whole thought; closing the
                  // menu on the first tick makes the second one a chore.
                  onSelect={(e) => e.preventDefault()}
                >
                  {o.label}
                </DropdownMenuCheckboxItem>
              </Fragment>
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
