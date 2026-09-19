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
import type { ReactNode } from "react";

import { IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * Which of the four is open. `null` is none — the landing underneath is
 * what shows.
 *
 * The keys are the values of `data-more-page`, so what a gate reaches
 * for and what the shell switches on are one string.
 */
export type MorePage = "settings" | "security" | "doctor" | "identity";

/** The four names, in one place, so the menu item and the page head
    cannot come to call the same page two things.

    ⚠️ **A function, not a `const`** (批 D3): these are catalog reads,
    and a module-level record would keep whichever language the page
    loaded in for the life of the tab — while the menu that opens these
    pages, one file over, had already changed. */
export const moreName = (page: MorePage): string =>
  ({
    settings: gui.settings,
    security: gui.security,
    doctor: gui.doctor,
    identity: gui.identity,
  })[page];

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
  children,
}: {
  page: MorePage;
  /** Back to the landing this was opened over. */
  onClose: () => void;
  /** Optional, and it stays optional on purpose: two of these four pages
      are a head and nothing else until their backend lands (batches B
      and C), and a placeholder section saying so would be the 「灰着的
      开关」 docs/MORE.md refuses. */
  children?: ReactNode;
}) {
  return (
    <section
      data-more-page={page}
      aria-label={moreName(page)}
      className="flex h-full min-h-0 flex-col bg-card"
    >
      {/* The head row, so its blank moves the window — the rule every
          landing's first row keeps (第八轮 第 59 条). `deep` is what
          makes the whole row draggable while the close button keeps
          its press: tauri excludes controls under a deep region. */}
      <header
        data-more-head
        data-tauri-drag-region="deep"
        className="flex min-h-ctl-lg flex-none items-center gap-row border-b px-3"
      >
        <h1 className="min-w-0 flex-1 truncate text-title font-title">{moreName(page)}</h1>
        <Button
          variant="ghost"
          size="icon"
          aria-label={gui.close}
          data-more-close
          onClick={onClose}
          // The only way out of this page on the narrow face, which is
          // the reason it carries the finger floor rather than the
          // icon size (`DetailPane`'s back button, same argument).
          className="min-h-touch min-w-touch"
        >
          <IconClose />
        </Button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto p-pane">
        {/* Capped at a reading measure and centred: these pages are
            prose and rows rather than a table, and a settings row
            stretched across a 1400 window puts its label and its
            control at opposite ends of the screen. */}
        <div className="mx-auto flex w-full max-w-measure flex-col gap-group">{children}</div>
      </div>
    </section>
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
  children,
}: {
  name: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <section data-more-section={name} className="flex min-w-0 flex-col gap-2">
      <h2 className="text-sm font-label text-muted-foreground">{label}</h2>
      {children}
    </section>
  );
}

/**
 * One toggle in this app's one control family: a `Button` wearing its
 * own state, the way the relay switch and the face options do. There is
 * no checkbox in `ui/`, and this is not the place to start a second
 * family.
 *
 * **`mark` rather than a `name` prop**, because the two callers are read
 * by two different gates under two different attributes — the lock
 * switch answers to `data-security-switch` (check-face has pressed it
 * since 批 C) and a settings choice to `data-config-choice`. One
 * attribute for both would have meant renaming one of them on a page
 * nothing else about this change touches.
 */
export function MoreSwitch({
  mark,
  label,
  on,
  onPress,
}: {
  mark: Record<string, string>;
  label: string;
  on: boolean;
  onPress: () => void;
}) {
  return (
    <Button
      variant="ghost"
      size="sm"
      {...mark}
      data-on={on}
      aria-pressed={on}
      onClick={onPress}
      className={cn(
        // A ring in both states: off is the state this ships in and the
        // state somebody has to discover it in (`NetSettings`'s own
        // measurement, 2026-09-18).
        // Both axes: 「永不」 is two characters, and `px-2` around them
        // is 38 wide — under the floor on the axis nobody thinks to
        // check (the 修 button on 自检 learned the same thing).
        "min-h-touch min-w-touch self-start rounded-md px-2 text-xs",
        on ? "text-primary ring-2 ring-primary" : "text-muted-foreground ring-1 ring-border",
      )}
    >
      {label}
    </Button>
  );
}

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
}: {
  name: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-row text-sm">
      <span className="flex-none text-muted-foreground">{label}</span>
      <span data-more-fact={name} className="min-w-0 break-all">
        {children}
      </span>
    </div>
  );
}
