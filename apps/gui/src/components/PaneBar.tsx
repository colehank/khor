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
import { Fragment, useLayoutEffect, useRef, useState, type CSSProperties } from "react";

import { GlassCapsule, GlassCircle, GlassItem } from "@/components/Glass";
import { IconClose, IconFilter, IconPlus, IconSearch } from "@/components/icons";
import { Omnibox, type OmniAxis } from "@/components/Omnibox";
import { SearchHint, SideSeat } from "@/components/SideHead";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { endsOnEnter } from "@/lib/field-swipe";
import { keyLabel, useNewAct } from "@/lib/shortcuts";
import { band } from "@/lib/scroll-edge";
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

export type PaneAction = { key: string; label: string; onSelect: () => void; afterClose?: boolean };

// ⚠️ 墓碑 (2026-09-30): `PageActions` stood here — a landing's own acts
// drawn a second time as push buttons at R0's right end (新会话… 导入… on
// 会话, 邀请设备… 加入网络… on 机器 and on 「…」 → 网络, 登记… on 额度),
// beside the ＋ that already offered them. The owner: 「订阅页右上角不要
// 有登记按钮，这个功能属于搜索框旁边的 ＋。类似的冗余不要出现在 app 里」.
// **An act that makes something has one home, the ＋ below**; R0's right
// end is for verbs on the one thing that is open (a machine's four, a
// workspace's two, 安全's 立即锁定), and a row's acts are its menu
// (10-layout 「一个动作只有一个家」).

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
  filterLabel,
  arrange,
  create,
  newIsTab = false,
  seat,
  band: bandProp,
  title,
  inline = false,
}: {
  /** The landing's name, said as the list page's title where the bar
      is seated (the redesign; it used to ride only the pane's
      aria-label). */
  title?: string;
  /** Keep the side toolbar in a compound heading already seated by its owner. */
  inline?: boolean;
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
   *
   * `"head"`: the box stands in a narrow screen's head (`NarrowHead`),
   * which already sits in the first row — so no portal, no rule under,
   * no ⌘K hint and no ⌘N; the narrow tab switcher's, beside its 「‹」.
   */
  seat?: "side" | "head";
  /** A head standing over its own column's scroller (the workspace's
      sessions column): it is a band (`lib/scroll-edge`), so the rows pass
      under it on glass and its rule is the band's, drawn only while a
      row is under it. */
  band?: boolean;
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
   * What "+" opens: **every act on this landing that makes something
   * not yet there** — 新会话 and 导入, 邀请设备 and 加入网络, 登记 — and
   * this is the only place they are drawn (2026-09-30). One is pressed
   * straight through, two or more are a menu. The first is also ⌘N.
   * Only things that work today go in here: a menu item that greys out
   * or does nothing teaches people not to open the menu.
   */
  actions?: PaneAction[];
  /**
   * The pane's **one** creation road, drawn as a plain "+" rather than
   * a menu. Given instead of `actions`, never beside it: two plus signs
   * on one bar is the second search box this file's head warns about.
   *
   * **`disabled` means it is not drawn** (the audit fix plan F1.9, the
   * owner's progressive disclosure). ⚠️ 墓碑: it drew a greyed "+" that
   * said 「这里将来能加东西」.
   *
   * `mark` is a data attribute the caller hangs on it, for the
   * verification anchors that name this pane's "+" rather than any "+".
   */
  create?: { label: string; mark?: string; disabled?: boolean; onSelect?: () => void };
  /** What this 「+」 makes is a tab (a session, a page): ⌘T runs it too. */
  newIsTab?: boolean;
}) {
  // **⌘N is this 「+」** (`lib/shortcuts`, 2026-09-28) — the landing's one
  // 「+」 is the one seated in the first row, so only that bar lends it,
  // by its own name (the menu bar's 新建 item reads it).
  const primary = actions?.[0] ?? (create?.onSelect && !create.disabled ? create : null);
  const newKey = keyLabel("new");
  useNewAct(
    seat === "side" && primary?.onSelect ? { label: primary.label, run: primary.onSelect, tab: newIsTab } : null,
  );
  // **Seated, the bar is the list page's head** (the redesign,
  // 2026-10-02: Apple's Preview on a phone, Notes in a window): the
  // landing's name, then filter and ＋ in one glass capsule, then search
  // as a glass circle of its own. Narrow puts the name on the row; wide
  // gives the row to the window's corner and the buttons, and the name a
  // row of its own under it (below). The search circle opens the box in
  // the row's place; a query left in it keeps it open.
  const pending = useRef<PaneAction | null>(null);
  useLayoutEffect(() => () => { pending.current = null; }, []);
  const glass = seat === "side";
  const banded = !!bandProp && seat === undefined;
  const [searching, setSearching] = useState(query !== "");
  const open = !glass || searching || query !== "";
  /** **The glass head controls everywhere** (the audit fix plan F4.7;
      W17): seated or not — the workspace's sessions column, the tab
      switcher — filter and ＋ are a glass capsule's items. ⚠️ 墓碑: bare
      ghost buttons off the seat. */
  const Plus = GlassItem;
  /** Ends a search the circle opened, and gives the caret back to the
      circle (W-V4) — it reappears where the field was. */
  const endSearch = (from: HTMLElement | null) => {
    const row = from?.closest("[data-pane-bar]");
    onQuery("");
    setSearching(false);
    requestAnimationFrame(() => row?.querySelector<HTMLElement>("[data-search-open]")?.focus());
  };
  const box = (
      <div
        className="flex min-w-0 flex-1 items-center"
        // **Escape, the Mac's way** (W-V4): the first clears what is
        // typed, the next — or the first, on an empty box — ends the
        // search. An open list of completions takes its own Escape first.
        onKeyDownCapture={(e) => {
          if (!glass || e.key !== "Escape" || e.nativeEvent.isComposing) return;
          const t = e.target as HTMLElement;
          if (t.getAttribute("aria-expanded") === "true") return;
          e.preventDefault();
          e.stopPropagation();
          if (query !== "") onQuery("");
          else endSearch(t);
        }}
      >
        {axes && axes.length > 0 && filter ? (
          <Omnibox
            label={searchLabel}
            query={query}
            onQuery={onQuery}
            axes={axes}
            chosen={filter.chosen}
            onToggle={filter.onToggle}
            onSubmit={onSubmit}
            field
            // **The same ⌘K the plain box wears** (components/SearchField:
            // one anatomy in L0). Sessions and 额度 narrow by chips, and
            // their box was the one L0 box with no hint (2026-09-28 audit).
            trail={seat === "side" ? <SearchHint /> : undefined}
          />
        ) : (
          <div data-search-wrap className="relative min-w-0 flex-1">
            <IconSearch className="pointer-events-none absolute top-1/2 left-2 z-10 -translate-y-1/2 text-tertiary" />
            <Input
              data-pane-search
              data-search-field
              type="search"
              // The return key ends the box under a finger, and the
              // keyboard goes (UX-43 §3).
              enterKeyHint="search"
              aria-label={searchLabel}
              placeholder={searchLabel}
              value={query}
              onChange={(e) => onQuery(e.target.value)}
              onKeyDown={endsOnEnter}
              // The look is the text field's own (`ui/input`, components/
              // TextField, 2026-09-28): `fill`, `radius-sm`, `body`, the
              // `ring` on focus, and the finger floor (`min-h-touch`,
              // tokens.css `--touch`) that used to be written here — a
              // `min-h` because tailwind-merge does not read `ctl-md` as a
              // height, so only a floor beats `h-ctl-md` (`button.tsx`
              // wrote that trap down first). What is left here is room for
              // the magnifier and, seated, the key hint.
              className={cn("pl-7", seat === "side" && "pr-10 @max-3xs/bar:pr-2")}
            />
            {/* A clear button inside the field once it holds something
                (F3.4; N20) — the × beside it leaves search. */}
            {query !== "" && (
              <button
                type="button"
                data-search-clear
                aria-label={gui.search_clear}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => onQuery("")}
                className="absolute top-1/2 right-2 z-10 grid size-6 -translate-y-1/2 place-items-center rounded-full bg-fill-strong text-muted-foreground"
              >
                <IconClose className="size-affordance" />
              </button>
            )}
            {/* Which range the box is on (第十三轮 第 4 条): typing narrows
                this column; the word says how to widen it. */}
            {seat === "side" && query === "" && (
              <span className="pointer-events-none absolute top-1/2 right-2 flex -translate-y-1/2">
                <SearchHint />
              </span>
            )}
          </div>
        )}
        </div>
  );
  const acts = (
    <>
      {filter && filterLabel && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <GlassItem aria-label={filterLabel} data-pane-filter on={filter.chosen.length > 0}>
              <IconFilter />
            </GlassItem>
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

      {/* **＋ is every act that makes something on this landing, and
          their only home** (2026-09-30, 10-layout 「一个动作只有一个家」):
          one act is pressed straight through; two or more drop a menu —
          the same on the wide face and the narrow one. ⚠️ 墓碑: from
          2026-09-26 ＋ was the first act alone and the rest stood at R0's
          right end (`PageActions`, retired above), with this menu only on
          the narrow face, which has no R0. That put every act twice on
          the wide face; the owner took R0's copy away. */}
      {actions && actions.length > 1 ? (
        <DropdownMenu onOpenChange={(open) => { if (open) pending.current = null; }}>
          <DropdownMenuTrigger asChild>
            <Plus aria-label={gui.new_item} data-pane-new>
              <IconPlus />
            </Plus>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" onCloseAutoFocus={(event) => {
            const action = pending.current;
            pending.current = null;
            if (!action) return;
            event.preventDefault();
            action.onSelect();
          }}>
            {actions.map((a, i) => (
              <DropdownMenuItem key={a.key} data-new-item={a.key} onSelect={() => { if (a.afterClose) pending.current = a; else a.onSelect(); }}>
                {a.label}
                {/* The first act is the one ⌘N runs (`useNewAct` above),
                    and only from the seated bar. */}
                {i === 0 && seat === "side" && newKey && <DropdownMenuShortcut>{newKey}</DropdownMenuShortcut>}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      ) : actions && actions.length > 0 && (
        <Plus aria-label={actions[0].label} data-pane-new onClick={actions[0].onSelect}>
          <IconPlus />
        </Plus>
      )}

      {create && !create.disabled && (
        <Plus
          aria-label={create.label}
          data-pane-new
          {...(create.mark ? { [create.mark]: "" } : {})}
          onClick={create.onSelect}
          // The finger floor, the same spelling the menu trigger above
          // wears — a control that cannot be pressed yet is still the
          // size it will be, so turning it on moves nothing.
        >
          <IconPlus />
        </Plus>
      )}
    </>
  );
  const hasActs = !!(filter && filterLabel) || !!actions?.length || !!(create && !create.disabled);
  // **The phone's list head, at both widths** (the redesign; wide since
  // the owner's refinement of 方案 B): the landing's name, then 筛选 / ＋
  // in one glass capsule, then the search circle, which opens the box in
  // this row. On the wide face it is the left column's second row, under
  // the window's own.
  const bar = glass ? (
    <div
      data-pane-bar
      data-searching={open || undefined}
      data-search-glass
      className="@container/bar flex min-h-glass-btn min-w-0 flex-1 items-center gap-in"
      style={{ "--menu-inset": "var(--seat-menu-inset)" } as CSSProperties}
    >
      {open ? (
        <>
          {box}
          <GlassCircle
            aria-label={gui.search_close}
            data-search-close
            onClick={(e) => endSearch(e.currentTarget)}
          >
            <IconClose />
          </GlassCircle>
        </>
      ) : (
        <>
          {title ? (
            <h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">
              {title}
            </h1>
          ) : (
            <span className="flex-1" />
          )}
          {hasActs && <GlassCapsule>{acts}</GlassCapsule>}
          <GlassCircle
            aria-label={gui.search_open}
            data-search-open
            onClick={(e) => {
              const row = e.currentTarget.parentElement;
              setSearching(true);
              // The box takes the row on the next frame; the caret goes
              // into it, the way pressing search on a phone brings up the
              // keyboard.
              requestAnimationFrame(() => row?.querySelector<HTMLInputElement>("input")?.focus());
            }}
          >
            <IconSearch />
          </GlassCircle>
        </>
      )}
    </div>
  ) : (
    <div
      data-pane-bar
      // The one search field everywhere (F3.4; W06): glass, as the list's.
      data-search-glass
      // `min-h`, not `h`: chips make the box taller than one row, and a
      // bar with a fixed height would clip them or squeeze the list.
      ref={banded ? band : undefined}
      data-band={banded || undefined}
      className={cn(
        "flex min-h-pane-head flex-none items-center gap-in",
        seat === "head" ? "min-w-0 flex-1" : banded ? "px-2" : "border-b px-2",
      )}
    >
      {box}
      {hasActs && <GlassCapsule>{acts}</GlassCapsule>}
    </div>
  );
  if (!glass || inline) return bar;
  return (
    <>
      <SideSeat>{bar}</SideSeat>
    </>
  );
}

