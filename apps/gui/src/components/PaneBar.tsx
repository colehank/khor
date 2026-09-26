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
import { Fragment, type CSSProperties } from "react";

import { IconFilter, IconPlus, IconSearch } from "@/components/icons";
import { Omnibox, type OmniAxis } from "@/components/Omnibox";
import { FIND_ALL, SideSeat } from "@/components/SideHead";
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
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

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
  onSubmit,
  actions,
  actionsLabel,
  filterLabel,
  arrange,
  create,
  seat,
}: {
  /** Names the search box and stands in as its placeholder. */
  searchLabel: string;
  /**
   * `"side"`: this is a sidebar's head, and it sits in the left block's
   * first row (docs/SHELL.md 第十三轮 第 4 条:「列表头原来那一行撤掉，
   * 搜索只有这一处」) — beside macOS's buttons, over the rail and the
   * list at once — rather than at the top of its own column. Still
   * written by the landing, next to the list it narrows, because what
   * the box does is the landing's; where it stands is the shell's
   * (`SideHead`). A head with no seat stays at the top of whatever it
   * is in (the tab switcher's box, the workspace's sessions column).
   */
  seat?: "side";
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
  /** Enter on free text, where the pane can act on it (files opens the
      path). Panes that only narrow a list pass none — see `Omnibox`. */
  onSubmit?: (text: string) => void;
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
  /**
   * The pane's **one** creation road, drawn as a plain "+" rather than
   * a menu. Given instead of `actions`, never beside it: two plus signs
   * on one bar is the second search box this file's head warns about.
   *
   * **`disabled` is allowed here and is not the judgment `actions`
   * makes.** A *menu* that greys its items teaches people not to open
   * menus, because finding out costs a press and a read. A single
   * button that visibly cannot be pressed costs a glance, and it is how
   * a pane says「这里将来能加东西」while the thing it opens is still a
   * batch away (工作区批①: the "+" is here, the cross-machine directory
   * picker is 批②).
   *
   * `mark` is a data attribute the caller hangs on it, for the
   * verification anchors that name this pane's "+" rather than any "+".
   */
  create?: { label: string; mark?: string; disabled?: boolean; onSelect?: () => void };
}) {
  const bar = (
    <div
      data-pane-bar
      // `min-h`, not `h`: chips make the box taller than one row, and a
      // bar with a fixed height would clip them or squeeze the list.
      //
      // **`--pane-head`, which is also the tab band's height**
      // (2026-09-20 第 5 条:「标签页的高度, 要和侧边栏搜索框的高度一致,
      // 保持和谐」). It read `min-h-ctl-lg` and *measured* 44, because
      // the search box inside it carries the finger floor and won — a
      // number nobody wrote against a band that was 48. One token now
      // says both, so they cannot drift apart again by one of them
      // being retuned.
      //
      // **Seated, no padding at the left and no rule under**: the row
      // it sits in already starts at the rail's glyph box (第十三轮 第 2
      // 条: the search box's left edge is that line) and is one piece
      // with the block under it.
      className={cn(
        "flex min-h-pane-head flex-none items-center gap-in",
        // `@container/bar`: the search slot below asks how wide this
        // bar is, not how wide the window is (第十五轮 第 4 条).
        seat === "side" ? "@container/bar min-w-0 flex-1 pr-1" : "border-b px-2",
      )}
      // A menu hung from this box starts at the list column, not over
      // the rail (tokens.css `--seat-menu-inset`).
      style={seat === "side" ? ({ "--menu-inset": "var(--seat-menu-inset)" } as CSSProperties) : undefined}
    >
      {/* **One search slot, two shapes.** A pane that has axes to narrow
          by gets the omnibox; one that does not gets the plain box it
          always had. Not a second control and not a second box — the
          alternative is what this file's head warns about, two search
          fields that look alike and behave differently. */}
      {/* **Too narrow for a box, the box is a magnifier** (docs/SHELL.md
          第十五轮 第 4 条:「放不下时缩到只剩放大镜（按钮形态，仍能点开搜
          索）」). The block's width has one owner now, so a list dragged
          narrow is a first row that narrow too — and the box gives way
          rather than holding the block open. Pressed, it opens the one
          search at its wide range, which is the search a column this
          narrow has room for (`SideHead`). Only in the block's row: a
          bar in a column of its own has that column's width to spend. */}
      {seat === "side" && (
        <Button
          variant="ghost"
          size="icon"
          aria-label={gui.find_everything}
          data-search-magnifier
          onClick={() => window.dispatchEvent(new Event(FIND_ALL))}
          className="hidden min-h-touch min-w-touch flex-none @max-3xs/bar:flex"
        >
          <IconSearch />
        </Button>
      )}
      <div className={cn("flex min-w-0 flex-1 items-center", seat === "side" && "@max-3xs/bar:hidden")}>
      {axes && axes.length > 0 && filter ? (
        <Omnibox
          label={searchLabel}
          query={query}
          onQuery={onQuery}
          axes={axes}
          chosen={filter.chosen}
          onToggle={filter.onToggle}
          onSubmit={onSubmit}
        />
      ) : (
        <div className="relative min-w-0 flex-1">
          <IconSearch className="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-tertiary" />
          <Input
            data-pane-search
            type="search"
            aria-label={searchLabel}
            placeholder={searchLabel}
            value={query}
            onChange={(e) => onQuery(e.target.value)}
            // `min-h-touch` — the finger floor (tokens.css `--touch`).
            // The omnibox half of this slot got it with the browse shell;
            // this is the plain box the panes without axes draw, and it
            // was still at the `--ctl-md` look's height. `min-h` rather
            // than a height so it beats the vendored `h-ctl-md` without
            // fighting it — tailwind-merge does not read `ctl-md` as a
            // height, so both classes would have survived and the winner
            // would have been whichever the stylesheet emitted last
            // (`button.tsx` wrote that trap down first).
            className={cn(
              "min-h-touch rounded-sm border-0 bg-fill pl-7 text-body shadow-none placeholder:text-tertiary focus-visible:ring-0 md:text-body",
              seat === "side" && "pr-10",
            )}
          />
          {/* Which range the box is on (第十三轮 第 4 条): typing narrows
              this column; the word says how to widen it. */}
          {seat === "side" && (
            <kbd
              data-search-scope
              className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 rounded-xs border px-xxs text-caption text-tertiary"
            >
              ⌘K
            </kbd>
          )}
        </div>
      )}
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
              // The finger floor, the same spelling the rail and the row
              // pins wear: the hit area grows, the glyph keeps its size,
              // and the ghost wash paints this box — so what lights up is
              // what is pressable.
              className="min-h-touch min-w-touch data-[on=true]:text-primary"
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
            <Button
              variant="ghost"
              size="icon"
              aria-label={actionsLabel}
              data-pane-new
              className="min-h-touch min-w-touch"
            >
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

      {create && (
        <Button
          variant="ghost"
          size="icon"
          aria-label={create.label}
          data-pane-new
          {...(create.mark ? { [create.mark]: "" } : {})}
          disabled={create.disabled}
          onClick={create.onSelect}
          // The finger floor, the same spelling the menu trigger above
          // wears — a control that cannot be pressed yet is still the
          // size it will be, so turning it on moves nothing.
          className="min-h-touch min-w-touch"
        >
          <IconPlus />
        </Button>
      )}
    </div>
  );
  return seat === "side" ? <SideSeat>{bar}</SideSeat> : bar;
}
