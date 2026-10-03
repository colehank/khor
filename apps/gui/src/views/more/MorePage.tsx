// The four pages behind 「…」 and the avatar (docs/MORE.md), and the one
// frame all four wear.
//
// **A page, not a dialog, and that is the ruling rather than a taste.**
// 设置 was a `Dialog` until 2026-09-19 — 「进一步点击都是独立的界面」 —
// and the difference shows up in three places a dialog cannot follow:
// a page takes the whole region beside the rail (a dialog is a box over
// whatever was behind it, which on the narrow face means a screen
// pretending to be a sheet), a page's head is a drag region like every
// other landing's head (第八轮 第 59 条), and a page is the thing on
// screen — so the rail still answers 我在哪一页 while it is open.
//
// **What a page does *not* get is a rail seat.** These are not landings
// (`App.tsx`'s `Landing`): a landing is a list beside a detail and is
// where you are; one of these is opened, read, and closed back to
// wherever you were. That is the same structural line the mark's place
// sits on, and it is why the open page is held apart from the landing
// rather than being another value of it.
import { createContext, useContext, useState, type ReactNode } from "react";

import { IconBack, IconChevronRight, IconClose } from "@/components/icons";
import { PageAction, PageSubtitle } from "@/components/Region";
import { NarrowHead } from "@/components/SideHead";
import { GlassCircle } from "@/components/Glass";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { config, gui } from "@/gen/catalog";

/**
 * Which section of 更多 is open. `null` is none — the landing underneath is
 * what shows.
 *
 * The keys are the values of `data-more-page`, so what a gate reaches
 * for and what the shell switches on are one string.
 *
 * Section order in L2 (spec 第十七轮 §1): 通用(settings), 外观(appearance),
 * 网络(network), 安全(security), 身份(identity), 诊断(doctor).
 */
export type MorePage = "settings" | "appearance" | "network" | "security" | "identity" | "doctor";

/** The ordered list of all 更多 sections, for the L2 section list.
    The order is the spec order; items are {key, label}. */
export const morePageOrder: readonly MorePage[] = [
  "settings",
  "appearance",
  "network",
  "security",
  "identity",
  "doctor",
];

/** The section names, in one place, so the list item and the page head
    cannot come to call the same section two things.

    ⚠️ **A function, not a `const`** (批 D3): these are catalog reads,
    and a module-level record would keep whichever language the page
    loaded in for the life of the tab — while the menu that opens these
    pages, one file over, had already changed. */
export const moreName = (page: MorePage): string =>
  ({
    settings: gui.settings,
    appearance: config.appearance,
    network: gui.network,
    security: gui.security,
    identity: gui.identity,
    doctor: gui.doctor,
  })[page];

/** The name the page's head says, for the sections inside it: a group
    named the same is the head said twice, and `MoreSection` leaves that
    name off. A context rather than a prop because the sections that can
    echo it are not all spelled on the page — `ConfigSections` draws the
    node's (`khor_node::config::SECTIONS`), and 通用 and 外观 each open on
    a section named after their page. */
const PageName = createContext<string | null>(null);

/**
 * The frame: a head with the page's name and its way out, and a column
 * of sections under it.
 *
 * **The name is drawn, unlike every landing's** (docs/UX.md: no pane
 * wears its name where you can see it). A landing's name is answered by
 * the rail glyph that is lit; nothing in the rail is lit for one of
 * these, so the only thing on screen that could say which page this is
 * is the page. The close button is the other half of the same sentence:
 * a landing is never closed, and one of these always is.
 */
export function MoreFrame({
  page,
  onClose,
  actions,
  subtitle,
  children,
}: {
  page: MorePage;
  /** Out of the page: back to the landing it was opened over on the
      wide face, one screen back to 更多's list on the narrow one (the
      shell decides which, `App` `leaveMorePage`). */
  onClose: () => void;
  /** The page's own buttons (D9) — verbs on what the page is about, never
      an act that makes something new (those are a landing's ＋, 2026-09-30;
      网络's 邀请设备… / 加入网络… left on that day): 安全's 立即锁定.
      **One mechanism on both faces** (`PageAction`): at R0's
      right end on the wide face, at the end of the page's head in the
      window's first row on the narrow one (`NarrowHead`). */
  actions?: ReactNode;
  /** **R0's second line** (2026-09-30, the A pages 「和工作区看齐」):
      the fact this page has already asked for that says where the thing
      it is about stands — which khor, whether the network is talking,
      whether the lock is on. **Absent where the page asks nothing**
      (外观: its rows are the settings themselves) — no second line
      rather than an invented one. Wide only: the narrow head is one
      line, the name. */
  subtitle?: ReactNode;
  // ⚠️ 墓碑 (2026-09-28): `bleed` stood here — the body filling the frame
  // instead of a reading column, for the one page that was a picture
  // (网络, 「网络就搞成地图」). 网络 is rows now and the map lives on the
  // machines overview; no page asks for it.
  /** Optional, and it stays optional on purpose: two of these four pages
      are a head and nothing else until their backend lands (batches B
      and C), and a placeholder section saying so would be the 「灰着的
      开关」 docs/MORE.md refuses. */
  children?: ReactNode;
}) {
  return (
    <PageName.Provider value={moreName(page)}>
      <section
        data-more-page={page}
        aria-label={moreName(page)}
        // Settings groups are cards on the canvas (components/SettingsGroup),
        // so a page of rows stands on `background`. **The one place the
        // frame reaches into the board** (10-layout's written exception):
        // since 2026-09-30 that reads as paper on the frame — each group a
        // sheet of the board's white / black, the page round them the
        // window's grey, like System Settings.
        className="flex h-full min-h-0 flex-col bg-background"
      >
        {subtitle && (
          <PageSubtitle>
            <span
              data-more-head-sub
              className="min-w-0 truncate text-callout text-muted-foreground tabular-nums"
            >
              {subtitle}
            </span>
          </PageSubtitle>
        )}
        {/* The page's own head. **On the wide face it is not drawn**
            (`app.css` `[data-board] [data-more-head]`): R0 above the
            board already says the name and holds the way out, and only
            the page's buttons travel up there (`PageAction`).

            **On the narrow face it is the window's first row**
            (`NarrowHead`, 2026-09-28) — 「‹」 and the name, like every
            other detail screen there. ⚠️ 墓碑: it was 「×」 in a bar of
            its own under L0's magnifier, the one detail screen whose way
            out was not 「返回」 (10-layout 窄档: 「列表一屏 ⇄ 详情一屏，
            R0 左上角是返回」). 「‹」 goes one screen back, to 更多's list. */}
        <NarrowHead>
          {(seated) =>
            seated ? (
              <header data-more-head className="flex min-w-0 flex-1 items-center gap-in self-stretch">
                {/* The canvas's settings head (the redesign): the way back as
                    a glass circle, the page's name centred on the row. */}
                <GlassCircle aria-label={gui.back} data-more-back onClick={onClose}>
                  <IconBack />
                </GlassCircle>
                <h1 className={cn("min-w-0 flex-1 truncate text-center text-title-sm font-title", !actions && "pr-glass-btn")}>
                  {moreName(page)}
                </h1>
                {/* No R0 on this face: `PageAction` leaves them here,
                    in the one place a page's buttons are — the end of its
                    head row, the same slot R0's `data-page-actions` is. */}
                {actions && (
                  <div data-page-actions className="flex flex-none items-center gap-in">
                    <PageAction>{actions}</PageAction>
                  </div>
                )}
              </header>
            ) : (
              <header
                data-more-head
                className="flex min-h-ctl-lg flex-none items-center gap-row border-b px-3"
              >
                <h1 className="min-w-0 flex-1 truncate text-title-md font-title">{moreName(page)}</h1>
                {actions && <PageAction>{actions}</PageAction>}
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={gui.close}
                  data-more-close
                  onClick={onClose}
                  className="min-h-touch min-w-touch"
                >
                  <IconClose className="size-affordance" />
                </Button>
              </header>
            )
          }
        </NarrowHead>
        <div data-nav-clear className="min-h-0 flex-1 overflow-y-auto p-pane">
          {/* Capped at a reading measure and centred: these pages are
              prose and rows rather than a table, and a settings row
              stretched across a 1400 window puts its label and its
              control at opposite ends of the screen. */}
          <div className="mx-auto flex w-full max-w-measure flex-col gap-group">{children}</div>
        </div>
      </section>
    </PageName.Provider>
  );
}

/**
 * One section of one page: a heading and what stands under it.
 *
 * `name` is the stable key and `label` the word, the split `FaceSettings`
 * already makes on its axes and for the same reason — verification that
 * had to find a section by its translated text would be reading the
 * catalog to check the catalog.
 */
export function MoreSection({
  name,
  label,
  mark,
  children,
}: {
  name: string;
  /** The group's name over its card. **Absent for a page's first group
      when the page's own name already says it** (System Settings' top
      group has none either) — a heading that repeats R0 is the second
      title the Toolbar rule refuses. A name equal to the page's is
      dropped here whoever passed it (`check-face` 4I). */
  label?: string;
  /** Extra anchors on the card — a page's older gates find its rows by
      the list they sit in. */
  mark?: Record<string, string>;
  children: ReactNode;
}) {
  const page = useContext(PageName);
  const shown = label && label !== page ? label : undefined;
  return (
    // components/SettingsGroup: the name above the card and outside it
    // (`group` 11/600 tertiary, in by `gap-row`), the card `card` on the
    // page's `canvas` at `radius-lg`, one hairline between rows starting
    // where the text does. `app.css` `[data-settings-card]` draws the
    // lines and pads whatever in it is not a settings row.
    <section data-more-section={name} className="flex min-w-0 flex-col gap-xxs">
      {shown && <h2 className="ml-row text-group text-muted-foreground">{shown}</h2>}
      <div data-settings-card {...mark} className="flex min-w-0 flex-col">
        {children}
      </div>
    </section>
  );
}

/**
 * A section that is folded away until somebody asks for it.
 *
 * 「不是所有的设置都要去铺在界面上, 显得非常杂乱」 — the user,
 * 2026-09-20. **Closed on arrival, always**, and the state is not
 * stored: a fold that remembered would be a page that is a different
 * height on two machines for a reason nobody can see, and the whole
 * point of folding is that the page has one shape when you open it.
 *
 * It is a `<details>` rather than a button and a conditional render,
 * because the browser already gives that element the open/closed
 * semantics a reader needs — a screen reader announces it, and the
 * summary is a real focus stop with no `aria-expanded` for this file to
 * keep in step with its own state.
 */
export function MoreFold({
  name,
  label,
  children,
}: {
  name: string;
  label: string;
  children: ReactNode;
}) {
  // **The body is not in the document while the fold is shut.** A
  // `<details>` keeps its children and hides them, which for a section
  // that fetches on mount would mean khor dialling on behalf of a
  // control nobody opened — and a probe that only ever runs invisibly is
  // one nothing on screen can report the failure of.
  const [open, setOpen] = useState(false);
  return (
    <details
      data-more-fold={name}
      open={open}
      onToggle={(e) => setOpen((e.currentTarget as HTMLDetailsElement).open)}
      className="group flex min-w-0 flex-col"
    >
      <summary
        data-more-fold-open
        // The finger floor on both axes: 「高级」 is two characters, and
        // the marker beside them does not make a target (`MoreSwitch`'s
        // own measurement).
        // In by the row's inset, so the chevron stands on the column's
        // line with the rows' words (F2.7; W08).
        className="flex min-h-touch list-none items-center gap-in ps-row text-callout font-label text-muted-foreground [&::-webkit-details-marker]:hidden"
      >
        {/* Its ink on the section's left line (F2.7: the arrow drew at
            42 against 36) — the glyph's own box is wider than its ink. */}
        <IconChevronRight className="-ms-1.5 size-affordance flex-none transition-transform group-open:rotate-90" />
        {label}
      </summary>
      {open && <div className="flex min-w-0 flex-col gap-group pt-2">{children}</div>}
    </details>
  );
}

// ⚠️ 墓碑 (第十七轮): `MoreSwitch` — a `Button` wearing its own on/off
// state, one per choice in a ladder — stood here. Every setting is one of
// the four rows in `components/settings` now (SwitchRow, PopupRow,
// NavRow, ActionRow), and a ladder of buttons was the shape they replace.
//
// ⚠️ 墓碑 (2026-09-27): `NavRow` was written with those three and never
// had a caller — no setting on these pages opens a page of its own, and
// a row kind with no row is a promise. It went; the fourth kind stays in
// the spec (components/SettingsGroup `nav`) for the first setting that
// genuinely has a sub-page, which is when it should be written again.

/**
 * A fact with a name: the left half names it, the right half is the
 * value. Used wherever one of these pages states something rather than
 * offering a control — an id, a version, a path.
 *
 * The value keeps its own element and its own mark, so a gate reads the
 * value and never the label plus the value run together.
 */
export function MoreFact({
  name,
  label,
  children,
  note,
}: {
  name: string;
  label: string;
  children: ReactNode;
  /** **A sentence about the row, on its own line under the label**
      (UX-4, decided): left-aligned and muted, like a session row's second
      line. A long one (a connection's last error) beside the value wrapped
      into a ragged right column. */
  note?: ReactNode;
}) {
  const row = (
    <>
      {/* The label keeps its line: a long value (a store path on a phone,
          docs/IOS.md) wraps beside it instead of squeezing it to one
          character a line. */}
      <span className="flex-1 whitespace-nowrap">{label}</span>
      <span data-more-fact={name} data-setting-trail data-selectable className="min-w-0 [overflow-wrap:anywhere] text-right text-muted-foreground">
        {children}
      </span>
    </>
  );
  if (!note) {
    return (
      // `data-more-row`: a row of a settings card like the four kinds —
      // their height, their trailing edge (app.css, `check-face` 4I).
      <div data-more-row className="flex min-w-0 items-center gap-row text-body">
        {row}
      </div>
    );
  }
  return (
    <div data-more-row className="flex min-w-0 flex-col justify-center text-body">
      <div className="flex min-w-0 items-center gap-row">{row}</div>
      <span data-more-note={name} data-selectable className="min-w-0 text-left text-callout text-muted-foreground [overflow-wrap:anywhere]">
        {note}
      </span>
    </div>
  );
}
