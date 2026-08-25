// The file landing's tabs, in the two shapes a browser has.
//
// **Wide is the strip**: every tab on screen at once, because the whole
// point of a tab is that you can see the other ones. **Narrow is a
// count and a switcher** — a phone shows one screen at a time
// (docs/UX.md 姿势), so the strip cannot be there and Chrome's own
// answer is the one people already know: the number of tabs is the
// button, and pressing it is where switching, closing and opening
// happen.
//
// That is not two designs. It is one model — a list of places, one of
// them open — drawn at the two widths, and neither face has a tab the
// other does not.
import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { IconClose, IconPlus } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** One tab as it is shown: what it calls itself, whose disk it is on,
    and where. Derived by the landing (it holds the device table); this
    file paints and decides nothing. */
export type TabFace = {
  id: string;
  label: string;
  face: Avatar | null;
  /** The whole path, for the switcher's second line and the wide tab's
      hover title. Empty on a fresh tab. */
  path: string;
};

export function TabStrip({
  faces,
  active,
  onFocus,
  onClose,
  onNew,
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  onNew: () => void;
}) {
  return (
    <div
      data-tab-strip
      // **The band is darker and the open tab is the page's own
      // colour**, which is the whole of how a tab says it is in front:
      // its bottom edge has no edge, so it reads as continuous with what
      // is under it, and the ones that are not open sit *in* the band
      // behind it. That is Chrome's arrangement, and it is not a
      // stylistic borrowing — it is the only one where "in front" is
      // said by the same surface that the content is on.
      //
      // The first spelling had it the other way round (a light tab on
      // the page ground, a rule under the strip) and the open tab read
      // as a floating pill with a line separating it from its own
      // content — measured on a screenshot, which is the only way that
      // kind of mistake shows.
      //
      // No `border-b`: the change of ground *is* the boundary, and a
      // rule as well would cut the open tab off from the page again.
      // `items-end` and a top-only radius do the rest.
      //
      // The strip scrolls rather than shrinking past `--tab-w-min`
      // (that token says why), and `flex-none` keeps it from being the
      // thing that gives when the window is short.
      className="flex flex-none items-end gap-1 overflow-x-auto bg-muted/40 px-2 pt-1"
    >
      {faces.map((f) => {
        const on = f.id === active;
        return (
          <div
            key={f.id}
            data-tab={f.id}
            data-on={on}
            title={f.path || undefined}
            className={cn(
              "group flex h-ctl-md w-tab max-w-tab min-w-tab-min flex-1 items-center gap-1 rounded-t-md pr-0.5 pl-2",
              on ? "bg-background text-foreground" : "text-muted-foreground hover:bg-background/50",
            )}
          >
            <button
              type="button"
              data-tab-open={f.id}
              onClick={() => onFocus(f.id)}
              className="flex min-w-0 flex-1 cursor-pointer items-center gap-1 text-left"
            >
              <MachineAvatar face={f.face} className="size-kind-mark flex-none" />
              <span data-tab-label className="min-w-0 flex-1 truncate text-sm">
                {f.label}
              </span>
            </button>
            <Button
              size="icon"
              variant="ghost"
              data-tab-close={f.id}
              aria-label={gui.close_tab}
              // Drawn on the open tab and under the pointer, the session
              // rows' rule: a close on every tab at once is the strip's
              // loudest repetition, and the open one needs it always
              // because that is the tab a person is finished with.
              className={cn(
                "size-ctl-sm flex-none",
                !on && "opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100",
              )}
              onClick={() => onClose(f.id)}
            >
              <IconClose />
            </Button>
          </div>
        );
      })}
      <Button
        size="icon"
        variant="ghost"
        data-tab-new
        aria-label={gui.new_tab}
        className="mb-1 flex-none"
        onClick={onNew}
      >
        <IconPlus />
      </Button>
    </div>
  );
}

/** The narrow face's way to the switcher: the count *is* the control,
    which is what makes it able to say how many without a second mark. */
export function TabCount({ n, onPress }: { n: number; onPress: () => void }) {
  return (
    <Button
      variant="ghost"
      size="icon"
      data-tab-count
      aria-label={gui.switch_tabs(n)}
      onClick={onPress}
      className="flex-none"
    >
      <span className="rounded-sm border px-1 text-xs tabular-nums">{n}</span>
    </Button>
  );
}

/**
 * The narrow face's switcher: every tab as a row, with the way to close
 * one and the way to open another.
 *
 * A screen rather than a sheet, because on this face it **is** the
 * place you are — the tab it replaces comes back the moment one is
 * picked, and a sheet over a screen you cannot see is a sheet over
 * nothing.
 */
export function TabSwitcher({
  faces,
  active,
  onFocus,
  onClose,
  onNew,
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  onNew: () => void;
}) {
  return (
    <div data-tab-switcher className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {faces.map((f) => (
          <div
            key={f.id}
            data-switch-tab={f.id}
            data-on={f.id === active}
            className={cn(
              "flex items-center gap-2 rounded-md px-2 py-2",
              f.id === active && "bg-muted",
            )}
          >
            <button
              type="button"
              data-switch-open={f.id}
              onClick={() => onFocus(f.id)}
              className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
            >
              <MachineAvatar face={f.face} className="size-avatar flex-none" />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm font-label">{f.label}</span>
                {/* The whole path under the name: on a phone the strip
                    is not there to give the tab context, so the row has
                    to carry it. Truncated from the tail — a slashed
                    path read right to left is not a path. */}
                {f.path && (
                  <span className="truncate text-xs text-muted-foreground">{f.path}</span>
                )}
              </span>
            </button>
            <Button
              size="icon"
              variant="ghost"
              data-switch-close={f.id}
              aria-label={gui.close_tab}
              // Always drawn here, unlike the wide strip: a phone has no
              // pointer to summon a control with, and a close that only
              // appears on hover is a close that does not exist
              // (`FilesPane`'s pin, same rule).
              className="flex-none"
              onClick={() => onClose(f.id)}
            >
              <IconClose />
            </Button>
          </div>
        ))}
      </div>
      <div className="flex-none border-t p-2">
        <Button variant="secondary" size="sm" data-tab-new onClick={onNew}>
          {gui.new_tab}
        </Button>
      </div>
    </div>
  );
}
