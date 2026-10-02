// Where a tab is, and the way to send it somewhere else.
//
// **One control, two states**, and that is the design rather than a
// saving: a resting face at rest, the box with completions once it is
// pressed. Two controls — a path strip *and* a place to type — would be
// the failure `PaneBar`'s head describes about search boxes, one step
// worse: they would not merely look alike, they would be showing one
// address in two spellings, and a person would have to learn which of
// them is the one that goes.
//
// **Two segments, in order, and only the first is this file's.** Which
// machine, then where on it — an address belongs to a machine, so the
// machine is committed first and becomes a chip, and only then is there
// anything to complete against. The machine half is the shell's,
// because it is the same question on every browsing landing (a device
// table, a name, a face). The second half arrives from the kind
// (`BrowseKind.useSecond`) and this file never looks inside what it
// hands back: a path and a URL are not one shape, and docs/DESIGN.md
// says why forcing them into one would cost more than it saves.
//
// **Enter does not promise arrival.** It says where to go; whether that
// place can be reached is the body's answer, and the body says so out
// loud. Nothing here rewrites the resting face on a submit — that
// follows something that actually came back, so an address that does
// not exist leaves the tab honestly where it still is.
import { useEffect, useRef, useState } from "react";

import type { DeviceRow } from "@/api";
import type { BrowseKind, Place } from "@/browse/kind";
import { IconFile } from "@/components/icons";
import { Omnibox, type Candidate, type OmniAxis } from "@/components/Omnibox";
import { gui } from "@/gen/catalog";

/** A place offered out of memory, with the word for where it came from.
    The hint is what keeps a remembered address from passing for a live
    answer (`Candidate.hint`). */
export type Remembered = { place: Place; hint: string };

export function AddressBar({
  tab,
  kind,
  devices,
  machine,
  at,
  remembered,
  fallback,
  onGo,
  onClear,
  trailing,
  leading,
}: {
  kind: BrowseKind;
  devices: DeviceRow[];
  /** The open tab's handle, handed to the kind's resting face. */
  tab: string;
  /** The machine this tab is on, or null on a fresh tab. */
  machine: DeviceRow | null;
  /** Where the tab actually is — what came back, in the kind's own
      spelling. `""` before anything has opened. */
  at: string;
  /** What this screen remembers on each machine, for when one stops
      answering. */
  remembered: Remembered[];
  /** Which machine a bare address goes to on a tab that has not picked
      one: this machine — the one browse that always exists, the same
      choice `go-pin-a-dir` makes. */
  fallback: DeviceRow | null;
  onGo: (device: string, place: string) => void;
  /** The chip came off: this tab is a fresh one again. */
  onClear: () => void;
  /**
   * What rides at the end of the bar. The narrow face's tab count goes
   * here rather than in a row of its own: two bars stacked would put a
   * rule across a phone's screen for no fact, and a count in a strip
   * that stops short of the button under it reads as two controls that
   * happen to be adjacent.
   */
  trailing?: React.ReactNode;
  /** Before the box: the narrow face's way back (the redesign). */
  leading?: React.ReactNode;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  // Raised the moment a machine is committed, lowered when its starting
  // point comes back. What it buys: the caret lands in the second
  // segment with that machine's own opening address already under it,
  // instead of in an empty box on a machine the person cannot spell
  // anything on yet.
  //
  // **A flag rather than "fill whenever the draft is empty"**, because
  // that rule fights the person: clearing the box to type somewhere
  // else entirely would refill it under their hands.
  const seeding = useRef(false);

  const second = kind.useSecond(machine, draft, remembered.map((r) => r.place));
  const Rest = kind.Rest;

  useEffect(() => {
    if (!seeding.current || !at) return;
    seeding.current = false;
    // **Only into a box nobody has typed in.** What a machine opens on
    // comes back over the wire, which can be seconds after the machine
    // was committed — long enough for somebody to have started spelling
    // an address — and a seed that landed then would erase what they
    // had written. Measured: committing a machine and typing straight
    // away lost the typing.
    setDraft((d) => (d === "" ? (at.endsWith("/") ? at : `${at}/`) : d));
  }, [at]);

  const machines: OmniAxis = {
    key: "dev:",
    label: gui.devices_tab,
    candidates: devices.map((d) => ({ key: `dev:${d.name}`, label: d.name, face: d.face })),
  };

  // What the second segment offers. Live, it is whatever the kind found
  // — untouched, because only the kind knows what a step in its own
  // tree looks like.
  //
  // **Refused, the offer changes kind, and it says so**: every candidate
  // becomes a place this screen remembers rather than one the machine
  // just named — dimmed, and carrying where it came from. An empty list
  // was the other option and it is the wrong one: "that machine is not
  // answering" and "there is nothing under there" are two facts, and an
  // empty list paints them the same. **This is the shell's, not the
  // kind's**: how a browsing landing says "could not get there" must not
  // differ between the two of them.
  const pathCandidates: Candidate[] = second.failed
    ? remembered
        .filter((r) => r.place.device === machine?.id)
        .map((r) => ({ key: r.place.place, label: r.place.place, hint: r.hint, dim: true }))
    : second.candidates;

  const axes: OmniAxis[] = machine
    ? [machines, { key: "place:", label: kind.words.segment, role: "text", candidates: pathCandidates }]
    : [machines];

  const go = (text: string) => {
    const want = text.trim();
    if (!want) return;
    // A bare address on a tab that never picked a machine goes to this
    // one. The address bar is executable, and the machine it executes
    // on is the one every other empty state here steps toward.
    const device = machine?.id ?? fallback?.id;
    if (!device) return;
    setEditing(false);
    onGo(device, want);
  };

  const onToggle = (key: string) => {
    const name = key.slice("dev:".length);
    const found = devices.find((d) => d.name === name);
    if (!found || (machine && found.id === machine.id)) {
      // Taking the chip off is leaving the machine, and what is left is
      // a tab pointed nowhere — which is a fresh tab, a state that
      // already knows what to draw.
      setDraft("");
      setEditing(false);
      onClear();
      return;
    }
    seeding.current = true;
    setDraft("");
    // **Stay in the box.** Committing the machine is half an address,
    // and the resting face would take the caret away in the middle of
    // writing the other half — the person picked a machine in order to
    // say where on it. This is what puts the caret in the second
    // segment; `seeding` above is what puts that machine's own opening
    // address under it.
    setEditing(true);
    onGo(found.id, "");
  };

  // ⚠️ **The machine's chip is gone from both faces** (2026-09-09), and
  // the shape it used to take is worth a tombstone rather than a blank:
  // `{ key: \`dev:${machine.name}\`, label: machine.name, face }`. It sat
  // in the resting row and in the open box, and in both places it was the
  // same size and shape as a tab one row above — a person reading the
  // screen saw two rows of tabs. The other half is that it stopped being
  // true: with one exit for the whole landing (docs/NET.md 可达性) the
  // machine is not a property of this tab's address.
  //
  // The machine is still on the tab, where it always was — the avatar —
  // and the tab's hover title spells it now. Switching it is still the
  // box, whose first axis it is.

  // A fresh tab has no address to draw, so the box is the whole control:
  // there is no resting state to rest in.
  const boxUp = !machine || editing;

  return (
    <div
      data-address-bar
      // Which tab this bar is about — the one in front — for the gate
      // that folds the narrow tab screen and checks nothing moved.
      data-address-tab={tab}
      // `--pane-head`, the height every other top row in this app wears
      // (第 5 条). It read `min-h-ctl-lg` and measured 45 for `PaneBar`'s
      // exact reason — the box inside carries the finger's floor and
      // won — so this was a third number nobody had written down.
      className="flex min-h-pane-head flex-none items-center gap-1 border-b px-2"
    >
      {leading}
      {boxUp ? (
        <Omnibox
          label={kind.words.address}
          query={draft}
          onQuery={setDraft}
          axes={axes}
          // **No chip while typing either** (2026-09-09). The machine
          // used to ride in the box as a removable chip beside the
          // address; with one exit for the whole landing (docs/NET.md
          // 可达性) it is not a property of what is being typed, and it
          // made the box read as two fields when it is one. Switching
          // machines is still here — the machine is the box's first axis
          // — it just is not drawn as something attached to the address.
          chosen={[]}
          onToggle={onToggle}
          onSubmit={go}
          // Not the magnifier: this box does not look for things, it is
          // where you are (`Omnibox`'s `lead`).
          lead={<IconFile className="pointer-events-none flex-none text-muted-foreground" />}
          autoFocus={editing}
          // **The one box in this app with something painted over it.**
          // A browsing tab's page is a native child webview and those
          // always paint above this page's DOM, so these completions
          // came out underneath the site — first blanking it, then
          // shoving it down, both judged wrong in front of somebody
          // (「不对,应该是浮在页面上的」). Floated, they are drawn in a
          // small window of the app's own, which is above a child view
          // by construction. A browser tab and a phone answer no and the
          // list is drawn here, which is right there: neither of them has
          // a page covering it either.
          float
          // Escape or focus elsewhere puts the resting face back — but
          // only where there is one. A fresh tab has nothing to fall
          // back to, and leaving would take its only control away.
          onLeave={machine ? () => setEditing(false) : undefined}
        />
      ) : (
        <div className="flex min-w-0 flex-1 items-center gap-1">
          {/* ⚠️ **The machine's chip used to sit here and no longer
              does** (2026-09-09). It was the same shape, the same size
              and one row under the tab strip, so it read as a second row
              of tabs — a person looking at it saw two tabs, one named
              after the page and one after the machine. The row at rest
              is now the address and nothing else.

              **The exit is not per tab any more either**, which is the
              other half of why it left: docs/NET.md 可达性 settled on one
              exit at a time for the whole landing, so a chip that sat
              inside one tab's address was drawing it as a property of
              that tab. Changing it is still the box — the machine is its
              first axis — and where the landing *shows* which exit it is
              on is open.

              **An element, never a call** (`BrowseKind.Rest` says what
              breaks). */}
          <Rest
            tab={tab}
            at={at}
            onGo={(place) => machine && onGo(machine.id, place)}
            onEdit={() => {
              setDraft(at);
              setEditing(true);
            }}
          />
        </div>
      )}
      {trailing}
    </div>
  );
}
