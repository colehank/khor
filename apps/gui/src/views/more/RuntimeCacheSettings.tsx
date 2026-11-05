import { useEffect, useState } from "react";
import { inApp, runtimeCacheUsage, type RuntimeCacheUsage } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { gui } from "@/gen/catalog";
import { formatBytes } from "@/lib/format-bytes";
import { MoreFact, MoreSection } from "./MorePage";

export function RuntimeCacheSettings({ enabled }: { enabled: boolean }) {
  const [usage, setUsage] = useState<RuntimeCacheUsage | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!enabled || !inApp) return;
    let current = true;
    void runtimeCacheUsage().then(value => { if (current) setUsage(value); }, () => { if (current) setFailed(true); });
    return () => { current = false; };
  }, [enabled]);
  if (!enabled || !inApp) return null;
  if (!usage && !failed) return null;
  const clean = async () => {
    if (busy) return;
    setBusy(true); setFailed(false);
    try { setUsage(await runtimeCacheUsage(usage !== null)); }
    catch { setFailed(true); }
    finally { setBusy(false); }
  };
  return <MoreSection name="runtime-cache" label={gui.update_runtime_cache}>
    {usage && <>
      <MoreFact name="runtime-retained" label={gui.update_runtime_retained} note={gui.update_runtime_retained_hint}>{formatBytes(usage.retained_bytes)}</MoreFact>
      <MoreFact name="runtime-active-old" label={gui.update_runtime_active_old} note={gui.update_runtime_active_hint}>{formatBytes(usage.active_old_bytes)}</MoreFact>
      {Number(usage.unknown_items) > 0 && <MoreFact name="runtime-unknown" label={gui.update_runtime_unknown}>{String(usage.unknown_items)}</MoreFact>}
    </>}
    <ActionRow name="runtime-clean" label={failed ? gui.update_runtime_failed : gui.update_runtime_idle}
      value={usage ? formatBytes(usage.idle_bytes) : undefined} actionLabel={usage ? gui.update_runtime_clean : gui.region_retry}
      busy={busy || (!usage && !failed)} onPress={() => { void clean(); }} />
  </MoreSection>;
}
