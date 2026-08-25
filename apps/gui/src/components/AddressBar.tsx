// Where a tab is, and the way to send it somewhere else.
//
// **One control, two states**, and that is the design rather than a
// saving: a breadcrumb at rest, the box with completions once it is
// pressed. Two controls — a path strip *and* a place to type — would be
// the failure `PaneBar`'s head describes about search boxes, one step
// worse: they would not merely look alike, they would be showing one
// path in two spellings, and a person would have to learn which of them
// is the one that goes.
//
// **Two segments, in order.** Which machine, then where on it — a path
// belongs to a disk, so the machine is committed first and becomes a
// chip, and only then is there a tree to complete against. Those are
// the omnibox's two roles unchanged (`Omnibox`'s `role`); nothing new
// was invented to hold either.
//
// **Enter does not promise arrival.** It says where to go; whether that
// place can be reached is the pane's answer, and the pane says so out
// loud (`FilesPane`'s refusal). Nothing here rewrites the breadcrumb on
// a submit — that follows a listing that actually came back, so a path
// that does not exist leaves the tab honestly where it still is.
import { useEffect, useRef, useState } from "react";

import type { DeviceRow } from "@/api";
import { IconFile } from "@/components/icons";
import { Omnibox, OmniChip, type Candidate, type OmniAxis } from "@/components/Omnibox";
import { gui } from "@/gen/catalog";
import { usePathCandidates } from "@/hooks/use-path-candidates";
import type { Place } from "@/hooks/use-places";

/** A place offered out of memory, with the word for where it came from.
    The hint is what keeps a remembered path from passing for a listing
    (`Candidate.hint`). */
export type Remembered = { place: Place; hint: string };

/** `/a/b/c` → the root and each step, each carrying the path pressing
    it goes to. */
function crumbs(at: string): { label: string; to: string }[] {
  const out = [{ label: "/", to: "/" }];
  let sofar = "";
  for (const part of at.split("/").filter(Boolean)) {
    sofar += `/${part}`;
    out.push({ label: part, to: sofar });
  }
  return out;
}

export function AddressBar({
  devices,
  machine,
  at,
  remembered,
  fallback,
  onGo,
  onClear,
  trailing,
}: {
  devices: DeviceRow[];
  /** The machine this tab is on, or null on a fresh tab. */
  machine: DeviceRow | null;
  /** Where the tab actually is — the path a listing came back about,
      absolute. `""` before anything has opened. */
  at: string;
  /** What this screen remembers on each machine, for when one stops
      answering. */
  remembered: Remembered[];
  /** Which machine a bare path goes to on a tab that has not picked
      one: this machine — the one browse that always exists, the same
      choice `go-pin-a-dir` makes. */
  fallback: DeviceRow | null;
  onGo: (device: string, path: string) => void;
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
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  // Raised the moment a machine is committed, lowered when its home
  // comes back. What it buys: the caret lands in the path segment with
  // that machine's own home already under it, instead of in an empty
  // box on a tree the person cannot spell yet.
  //
  // **A flag rather than "fill whenever the draft is empty"**, because
  // that rule fights the person: clearing the box to type somewhere
  // else entirely would refill it under their hands.
  const seeding = useRef(false);
  // The breadcrumb's own scroller, held at its right end: the step you
  // are standing on is the one worth seeing, and a scroller left at 0
  // shows the part every path on that machine has in common.
  const crumbBox = useRef<HTMLDivElement>(null);

  const { dir, entries, failed } = usePathCandidates(machine?.name ?? null, draft);

  useEffect(() => {
    if (!seeding.current || !at) return;
    seeding.current = false;
    // **Only into a box nobody has typed in.** The home comes back over
    // the wire, which can be seconds after the machine was committed —
    // long enough for somebody to have started spelling a path — and a
    // seed that landed then would erase what they had written. Measured:
    // committing a machine and typing straight away lost the typing.
    setDraft((d) => (d === "" ? (at.endsWith("/") ? at : `${at}/`) : d));
  }, [at]);

  // Every time the path changes — and every time the breadcrumb comes
  // back from the box, which is when the element exists again.
  useEffect(() => {
    const el = crumbBox.current;
    if (el) el.scrollLeft = el.scrollWidth;
  }, [at, editing]);

  const machines: OmniAxis = {
    key: "dev:",
    label: gui.devices_tab,
    candidates: devices.map((d) => ({ key: `dev:${d.name}`, label: d.name, face: d.face })),
  };

  // What the path segment offers. Live, it is the machine's own
  // listing — directories only, because a file is where a path stops
  // and offering one as a step to continue from is offering a step
  // nobody can take.
  //
  // **Refused, the offer changes kind, and it says so**: every
  // candidate is a place this screen remembers rather than one the
  // machine just named — dimmed, and carrying where it came from. An
  // empty list was the other option and it is the wrong one: "that
  // machine is not answering" and "that directory has no
  // subdirectories" are two facts, and an empty list paints them the
  // same.
  const pathCandidates: Candidate[] = failed
    ? remembered
        .filter((r) => r.place.device === machine?.id)
        .map((r) => ({ key: r.place.place, label: r.place.place, hint: r.hint, dim: true }))
    : entries
        .filter((e) => e.dir)
        .map((e) => ({ key: `${dir}${e.name}/`, label: `${e.name}/` }));

  const axes: OmniAxis[] = machine
    ? [machines, { key: "path:", label: gui.files_tab, role: "text", candidates: pathCandidates }]
    : [machines];

  const go = (text: string) => {
    const want = text.trim();
    if (!want) return;
    // A bare path on a tab that never picked a machine goes to this
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
    // and the resting breadcrumb would take the caret away in the
    // middle of writing the other half — the person picked a machine in
    // order to say where on it. This is what puts the caret in the path
    // segment; `seeding` above is what puts that machine's own home
    // under it.
    setEditing(true);
    onGo(found.id, "");
  };

  const chip: Candidate | null = machine
    ? { key: `dev:${machine.name}`, label: machine.name, face: machine.face }
    : null;

  // A fresh tab has no path to draw, so the box is the whole control:
  // there is no resting state to rest in.
  const boxUp = !machine || editing;
  const path = crumbs(at);

  const edit = () => {
    setDraft(at);
    setEditing(true);
  };

  return (
    <div data-address-bar className="flex min-h-ctl-lg flex-none items-center gap-1 border-b px-2">
      {boxUp ? (
        <Omnibox
          label={gui.address_dir}
          query={draft}
          onQuery={setDraft}
          axes={axes}
          chosen={chip ? [chip.key] : []}
          onToggle={onToggle}
          onSubmit={go}
          // Not the magnifier: this box does not look for things, it is
          // where you are (`Omnibox`'s `lead`).
          lead={<IconFile className="pointer-events-none flex-none text-muted-foreground" />}
          autoFocus={editing}
          // Escape or focus elsewhere puts the breadcrumb back — but
          // only where there is one. A fresh tab has nothing to fall
          // back to, and leaving would take its only control away.
          onLeave={machine ? () => setEditing(false) : undefined}
        />
      ) : (
        <div className="flex min-w-0 flex-1 items-center gap-1">
          {chip && <OmniChip candidate={chip} onRemove={() => onToggle(chip.key)} />}
          {/* The steps have **no gap between them** — that is what makes
              this read as a path rather than as a row of words. The
              container above holds the machine a `gap-1` off, because
              the machine is a different kind of thing; inside, the
              slashes are the whole of the spacing, and one `gap-1` per
              step (the first spelling) put four pixels on either side of
              every separator and turned `/var/folders/ml` into a list.
              Measured on a screenshot.

              **And no step truncates. The strip scrolls instead**, held
              at its right end (`crumbBox` below). Letting them share the
              width was the first spelling and it fails exactly where a
              path is longest: at 390 every middle step shrank to two
              characters and an ellipsis, so the whole address read
              `,v/f…/ı/w_5l2…/ʔkho…/b/tree-beta` — six truncations that
              name nothing, and the one segment worth reading squeezed
              between them. Measured on a screenshot at 390. Scrolled,
              the end of the path — the part that says where you are —
              is what shows, and the ancestors are still there to the
              left, still pressable. */}
          <div ref={crumbBox} className="flex min-w-0 flex-1 items-center overflow-x-auto">
          {path.map((c, i) => {
            const last = i === path.length - 1;
            return (
              <span key={c.to} className="flex flex-none items-center">
                {i > 1 && <span className="flex-none text-muted-foreground">/</span>}
                <button
                  type="button"
                  data-crumb={c.to}
                  // **The step you are standing on opens the box
                  // instead of going anywhere.** Going to where you
                  // already are is the one press on this strip that
                  // could do nothing at all, and an address bar needs a
                  // reliable way into its own editor — on a path deep
                  // enough to fill the row there is no blank tail left
                  // to press.
                  onClick={last ? edit : () => machine && onGo(machine.id, c.to)}
                  className="cursor-pointer rounded-sm px-0.5 text-sm whitespace-nowrap hover:bg-muted"
                >
                  {c.label}
                </button>
              </span>
            );
          })}
            {/* The rest of the strip is one more way in, so that
                pressing "somewhere on the address" does what it does in
                every other address bar. */}
            <button
              type="button"
              data-address-rest
              aria-label={gui.address_dir}
              onClick={edit}
              className="min-w-0 flex-1 cursor-text self-stretch"
            />
          </div>
        </div>
      )}
      {trailing}
    </div>
  );
}
