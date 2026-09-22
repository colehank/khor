// A tab that has not been pointed anywhere: the places this person
// already goes, across every machine at once.
//
// **Mixed across machines on purpose.** "Where was I" is not a question
// about one disk — the directory somebody was digging through this
// morning is the thing they are looking for, and which machine it was
// on is how they recognise it, not how they find it. So the machine
// rides each row as its face, and the rows sort by how lately
// (`placesInOrder`).
//
// ⚠️ 墓碑 (2026-09-22, docs/SHELL.md 第六轮): a pinned section sat above
// the recents — 「置顶的网页」, and 「置顶的目录」 before that — with an
// empty sentence of its own. A pin is the sidebar's pinned group now, one
// place for it the way a session's pin has one place, and the old rows
// were moved there (`Node::migrate_place_pins`). With nothing gone to
// yet the screen is empty — **no sentence**, and since 第七轮 no button
// either: a new one would be writing content to fill a space nobody
// asked to be filled.
//
// **A recent can be sent away, and until now nothing on any screen
// could do it.** `khor_sync::recents` has had the tombstone since it
// was written and `forget_recent` reaches it through the node and the
// api — the ledger was complete and the face had no door. The question
// this answers is the small one a shortcut list has to answer: somebody
// looks at a row they do not want offered again, and the thing they
// reach for has to be on that row. Not a setting, not a way to clear
// the whole list — one row, one press.
//
// **What the press does not mean is worth saying, because the ledger
// says it out loud**: forgetting is not a deletion, it is "stop
// suggesting this", and going back to the place brings it back on its
// own. So nothing here warns, confirms, or offers an undo — the undo is
// to go there, which is the same gesture as wanting it back.
import { useEffect } from "react";

import { MachineAvatar } from "@/components/Avatar";
import { IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { DeviceRow } from "@/api";
import type { BrowseKind, Place } from "@/browse/kind";
import { placesInOrder, type Places } from "@/browse/use-places";
import { gui } from "@/gen/catalog";
import { ago } from "@/words";

export function NewTabPage({
  kind,
  places,
  devices,
  narrow,
  onOpen,
  onForget,
}: {
  /** Whose places these are. This file asks it for two things: what
      to call a place in a word, and what to call the sections and the
      empty state — none of which the shell may spell itself, or the
      browser landing says 「目录」 (`BrowseKind.words`). */
  kind: BrowseKind;
  places: Places;
  /** For the faces: a place is keyed by device id, and the picture
      belongs to the device table like everywhere else. */
  devices: DeviceRow[];
  /** Which face this is, for the row action's visibility — see the
      forget button. */
  narrow: boolean;
  onOpen: (device: string, path: string) => void;
  /**
   * "Stop offering me this one."
   *
   * The shell's, not this file's: the recents ledger belongs to the node
   * and `BrowseLanding` is the one place that writes to it (the same
   * reason `recent_visit` is called there and not in a body).
   */
  onForget: (place: Place) => void;
}) {
  // **Asked again every time this appears**: a page visited in another
  // tab a moment ago is exactly the one a person opens a fresh tab to
  // go back to, and the answer this landing was holding was fetched
  // before that visit.
  //
  // Cheap enough to be unconditional: two reads of local ledgers, on an
  // act a person performs deliberately.
  const { reload } = places;
  useEffect(reload, [reload]);

  const rows = placesInOrder(places);

  // Nobody has answered yet, so there is nothing true to say.
  if (!places.asked) return null;

  // Nothing gone to yet: nothing in the middle (docs/SHELL.md 第七轮
  // 第 2 条). The button that stood here said 「去一台机器,借它的网上网」,
  // and this machine is on the net already — it read as if borrowing
  // were the only way out. Borrowing is picked in the address bar.
  if (rows.length === 0) return <div data-no-places className="h-full" />;

  return (
    <div data-new-tab className="min-h-0 overflow-y-auto p-2">
      {rows.map((place, i) => {
        const first = i === 0;
        const leaf = kind.leafOf(place.place);
        const face = devices.find((d) => d.id === place.device)?.face ?? null;
        return (
          <div key={`${place.device}:${place.place}`}>
            {first && (
              <div data-places-section="recent" className="px-3 pt-2 pb-1 text-xs text-muted-foreground">
                {kind.words.recent}
              </div>
            )}
            {/* The row is a wrapper that holds the way in and, on a
                recent, the way to be rid of it — `FilesPane`'s
                arrangement, and for its reason: **the button is the
                target**, and a row grown from outside would leave the
                pressable part its old height inside a taller stripe
                that lights up. The wash is the row's and the row is
                exactly as tall as what is in it. */}
            <div className="group flex items-center gap-1 rounded-md hover:bg-muted/50">
              <button
                type="button"
                data-place={place.place}
                data-place-device={place.device}
                onClick={() => onOpen(place.device, place.place)}
                // `min-h-touch` and the wash on the same box: this is a
                // list of one-line rows, so nothing but the floor decides
                // how tall a row is, and the hover paints exactly what
                // opens (tokens.css `--touch`). Measured before: 34.
                className="flex min-h-touch min-w-0 flex-1 cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-left"
              >
                <MachineAvatar face={face} className="size-kind-mark flex-none" />
                <span className="flex-none text-sm font-label">{leaf}</span>
                <span className="flex-none text-xs text-muted-foreground">{place.name}</span>
                <span
                  title={place.place}
                  className="min-w-0 flex-1 truncate text-xs text-muted-foreground"
                >
                  {place.place}
                </span>
                {/* How lately, on the rows that have a "lately". */}
                {place.at_ms !== null && (
                  <span className="flex-none text-xs tabular-nums text-muted-foreground">
                    {ago(place.at_ms)}
                  </span>
                )}
              </button>
              {/* 「Stop suggesting this」, on every row. Visibility is the
                  row-action rule this repo settled: **narrow draws it
                  always** — a phone has no pointer to summon a control
                  with — while wide reveals it under the pointer or
                  keyboard focus, because a mark on every one of a dozen
                  rows is this screen's loudest repetition. `min-*`
                  rather than a size: `cn()` cannot lift the icon
                  variant's `h-ctl-md`/`w-ctl-md` off (`button.tsx`). */}
                <Button
                  size="icon"
                  variant="ghost"
                  data-forget-place={place.place}
                  data-forget-device={place.device}
                  aria-label={gui.forget_place(kind.words.recent)}
                  className={cn(
                    "min-h-touch min-w-touch mr-3 flex-none text-muted-foreground",
                    !narrow &&
                      "opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100",
                  )}
                  onClick={() => onForget(place)}
                >
                  <IconClose />
                </Button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
