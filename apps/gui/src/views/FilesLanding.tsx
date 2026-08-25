// The files landing, whole: tabs, an address bar, and whatever the open
// tab is showing.
//
// **There is no machine list here any more** (标签页一期, 用户裁决
// 2026-08-25). A landing is one of two shapes in this app and this one
// changed shape: 会话 and 机器 are **收件箱型** — a list you keep an eye
// on, with a detail beside it — while 文件 is **浏览型**, where the list
// was never the thing being watched. Which machine is a step on the way
// to a directory, not a thing to keep a column of screen on while you
// read one, and a browser is the shape that already knows this: you say
// where you are going, and the going is what stays on screen. The
// column that came back is the content's.
//
// One tab is (a machine, a path). That pair is the tab's whole
// identity, it is what the strip titles, what the address bar draws,
// and what survives a reload (`useDirTabs`) — one fact with three
// faces, held in one place so they cannot disagree.
import { useState } from "react";

import type { DeviceRow } from "@/api";
import { AddressBar, type Remembered } from "@/components/AddressBar";
import { TabCount, TabStrip, TabSwitcher, type TabFace } from "@/components/TabStrip";
import { gui } from "@/gen/catalog";
import type { DirTabs } from "@/hooks/use-dir-tabs";
import { placesInOrder, usePlaces } from "@/hooks/use-places";
import { FilesPane } from "@/views/FilesPane";
import { NewTabPage } from "@/views/NewTabPage";

/** The leaf of a path — what a tab calls itself, the way a browser tab
    wears a page's title rather than its URL. A machine with nothing open
    on it wears its own name; a tab pointed nowhere wears the word for
    what it is. */
function tabLabel(path: string, machine: DeviceRow | null): string {
  if (!machine) return gui.new_tab;
  if (!path) return machine.name;
  return path.slice(path.lastIndexOf("/") + 1) || path;
}

export function FilesLanding({
  devices,
  tabs,
  narrow,
}: {
  devices: DeviceRow[];
  tabs: DirTabs;
  narrow: boolean;
}) {
  // Narrow only: the switcher is up. Wide ignores it — the strip is
  // already every tab at once, and a switcher over it would be a second
  // way to do a thing that is not hidden.
  const [switching, setSwitching] = useState(false);
  const places = usePlaces();

  const deviceOf = (id: string | null) => (id ? (devices.find((d) => d.id === id) ?? null) : null);
  const machine = deviceOf(tabs.current.device);
  const me = devices.find((d) => d.me) ?? null;

  const faces: TabFace[] = tabs.tabs.map((t) => {
    const d = deviceOf(t.device);
    return { id: t.id, label: tabLabel(t.path, d), face: d?.face ?? null, path: t.path };
  });

  // Every place this screen remembers, each carrying the word for which
  // list it came from. The address bar falls back to these when a
  // machine stops answering, and the hint is what stops a remembered
  // path passing for a listing.
  const remembered: Remembered[] = placesInOrder(places).map(({ place, pinned }) => ({
    place,
    hint: pinned ? gui.pinned_dirs : gui.recent_dirs,
  }));

  const goTo = (device: string, path: string) => {
    tabs.goTo(device, path);
    if (narrow) setSwitching(false);
  };

  const address = (trailing?: React.ReactNode) => (
    <AddressBar
      devices={devices}
      machine={machine}
      at={tabs.current.path}
      remembered={remembered}
      fallback={me}
      onGo={goTo}
      onClear={tabs.clear}
      trailing={trailing}
    />
  );

  const content =
    machine && tabs.current.device ? (
      <FilesPane
        // Keyed by the tab, not by where the tab is: a tab that moves is
        // the same tab, and re-keying on the path would throw away the
        // scroll position on every step down the tree. Keyed by
        // *machine* too, because a path from one disk means nothing on
        // another and the pane must not carry one across.
        key={`${tabs.current.id}:${tabs.current.device}`}
        machine={machine.name}
        device={machine.id}
        path={tabs.current.path}
        narrow={narrow}
        onPath={(path) => tabs.goTo(machine.id, path)}
        // A directory really opened, so the memory of where this person
        // goes has one more entry — asked for again rather than guessed
        // at locally, because the node is the one keeping that ledger
        // (`khor_sync::recents`) and a local copy would be a second one.
        onLanded={() => places.reload()}
      />
    ) : (
      <NewTabPage
        places={places}
        devices={devices}
        onOpen={goTo}
        onGo={me ? () => goTo(me.id, "") : undefined}
      />
    );

  if (narrow) {
    return (
      <section data-files-landing className="flex h-full min-h-0 flex-col">
        {address(<TabCount n={tabs.tabs.length} onPress={() => setSwitching(true)} />)}
        <div className="min-h-0 flex-1 overflow-hidden">
          {switching ? (
            <TabSwitcher
              faces={faces}
              active={tabs.current.id}
              onFocus={(id) => {
                tabs.focus(id);
                setSwitching(false);
              }}
              onClose={tabs.close}
              onNew={() => {
                tabs.add();
                setSwitching(false);
              }}
            />
          ) : (
            content
          )}
        </div>
      </section>
    );
  }

  return (
    <section data-files-landing className="flex h-full min-h-0 flex-col">
      <TabStrip
        faces={faces}
        active={tabs.current.id}
        onFocus={tabs.focus}
        onClose={tabs.close}
        onNew={tabs.add}
      />
      {address()}
      <div className="min-h-0 flex-1 overflow-hidden">{content}</div>
    </section>
  );
}
