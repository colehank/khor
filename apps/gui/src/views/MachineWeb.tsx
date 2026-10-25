/**
 * **A machine's network, in place on its page** (the redesign, round 14,
 * the owner: 浏览 shows in place, like 文件). The machine head's 浏览 is a
 * switch of the page's body: pressed, the body is one page browsed
 * through this machine — the address field, the machine fixed to this
 * one and shown by its face at the field's left, and the page under it.
 * The landing stays 机器, and nothing is added to the 浏览 landing's list:
 * one page, no tab strip, the same reasoning as the files.
 */
import { useEffect, useState } from "react";

import type { DeviceRow } from "@/api";
import { PageField } from "@/browse/PageField";
import { BrowserPane } from "@/views/BrowserPane";
import { releaseBorrows, releasePages } from "@/views/held";

export function MachineWeb({ machine, devices }: { machine: DeviceRow; devices: DeviceRow[] }) {
  // The page's own name for the native view this body holds: one per
  // machine, never one of the 浏览 landing's tabs.
  const tab = `machine-web:${machine.id}`;
  const [at, setAt] = useState("");
  useEffect(() => {
    releasePages(at ? [tab] : [], tab);
    releaseBorrows(at && !machine.me ? [machine.id] : [], tab);
  }, [at, tab, machine.id, machine.me]);
  useEffect(() => () => {
    releasePages([], tab);
    releaseBorrows([], tab);
  }, [tab]);
  return (
    <div data-machine-web className="flex min-h-0 flex-1 flex-col">
      <div data-page-field-seat className="flex flex-none px-page pt-xxs pb-in">
        <PageField tab={tab} machine={machine} devices={devices} at={at} onGo={setAt} fixed />
      </div>
      <div className="min-h-0 flex-1 overflow-hidden">
        {at !== "" && (
          <BrowserPane
            retentionOwner={tab}
            tab={tab}
            machine={machine.name}
            device={machine.id}
            url={at}
            here={machine.me}
            onTitle={() => {}}
            onOpened={() => {}}
            onWent={setAt}
            onPopped={setAt}
          />
        )}
      </div>
    </div>
  );
}
