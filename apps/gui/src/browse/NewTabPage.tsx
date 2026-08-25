// A tab that has not been pointed anywhere: the places this person
// already goes, across every machine at once.
//
// **Mixed across machines on purpose.** "Where was I" is not a question
// about one disk — the directory somebody was digging through this
// morning is the thing they are looking for, and which machine it was
// on is how they recognise it, not how they find it. So the machine
// rides each row as its face, and the rows are ordered by the two
// things that actually rank them: a pin is a decision and sits on top,
// a visit is a fact and sorts by how lately (`placesInOrder`).
//
// The sentence for having neither is the one the files landing used to
// wear beside its machine list. That list is gone (标签页一期), and an
// empty state with nothing around it to explain it needs the sentence
// more than the old one did, not less.
import { useEffect } from "react";

import { MachineAvatar } from "@/components/Avatar";
import { Button } from "@/components/ui/button";
import type { DeviceRow } from "@/api";
import type { BrowseKind } from "@/browse/kind";
import { gui } from "@/gen/catalog";
import { placesInOrder, type Places } from "@/browse/use-places";
import { ago } from "@/words";

export function NewTabPage({
  kind,
  places,
  devices,
  onOpen,
  onGo,
}: {
  /** Whose places these are — this file asks it for one thing:
      what to call a place in a word. */
  kind: BrowseKind;
  places: Places;
  /** For the faces: a place is keyed by device id, and the picture
      belongs to the device table like everywhere else. */
  devices: DeviceRow[];
  onOpen: (device: string, path: string) => void;
  /** The way onward with nothing to show: this machine's home, the one
      browse that always exists. Absent only until the device table has
      answered — then the sentence stands alone rather than wearing a
      button with nowhere to go. */
  onGo?: () => void;
}) {
  // **Asked again every time this appears**, and it has to be: a pin
  // pressed in another tab a moment ago is exactly the pin a person
  // opens a fresh tab to use, and the answer this landing was holding
  // was fetched before that pin existed. Measured — a directory pinned
  // in one tab was missing from the next fresh tab until this was here.
  //
  // Cheap enough to be unconditional: two reads of local ledgers, on an
  // act a person performs deliberately.
  const { reload } = places;
  useEffect(reload, [reload]);

  const rows = placesInOrder(places);

  // Nobody has answered yet, so there is nothing true to say. **Not the
  // empty sentence**: "there is nothing pinned" is a claim about this
  // person, and a request in flight is not evidence for it.
  if (!places.asked) return null;

  if (rows.length === 0) {
    return (
      <div className="grid h-full place-items-center text-sm text-muted-foreground">
        <div className="flex flex-col items-center gap-2">
          <div data-no-places>{gui.no_pinned_dirs}</div>
          {onGo && (
            <Button size="sm" variant="secondary" data-go-pin-dir onClick={onGo}>
              {gui.go_pin_a_dir}
            </Button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div data-new-tab className="min-h-0 overflow-y-auto p-2">
      {rows.map(({ place, pinned }, i) => {
        // A heading where the kind changes, the way `SessionsList`
        // starts a group: the caller decides which section each row is
        // in, this only notices the boundary — so a section with no
        // rows cannot draw a heading over nothing.
        const first = i === 0 || rows[i - 1].pinned !== pinned;
        const leaf = kind.leafOf(place.place);
        const face = devices.find((d) => d.id === place.device)?.face ?? null;
        return (
          <div key={`${place.device}:${place.place}`}>
            {first && (
              <div
                data-places-section={pinned ? "pinned" : "recent"}
                className="px-3 pt-2 pb-1 text-xs text-muted-foreground"
              >
                {pinned ? gui.pinned_dirs : gui.recent_dirs}
              </div>
            )}
            <button
              type="button"
              data-place={place.place}
              data-place-device={place.device}
              onClick={() => onOpen(place.device, place.place)}
              className="flex w-full cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-left hover:bg-muted/50"
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
              {/* How lately, on the rows that have a "lately". A pin
                  carries none and shows none — an invented age would
                  be this screen claiming to know when a decision was
                  made. */}
              {place.at_ms !== null && (
                <span className="flex-none text-xs tabular-nums text-muted-foreground">
                  {ago(place.at_ms)}
                </span>
              )}
            </button>
          </div>
        );
      })}
    </div>
  );
}
