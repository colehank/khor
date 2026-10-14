import { useEffect, useRef, useState } from "react";
import { setConfig } from "@/api";
import { gui } from "@/gen/catalog";
import { useSettings, useSettingsReread } from "@/settings";
import { useWindowSupport, windowChoice } from "@/lib/window-surface";
import { PopupRow } from "@/components/settings/PopupRow";
import { Slider } from "@/components/settings/Slider";
import { MoreSection } from "@/views/more/MorePage";

export function WindowAppearanceSection() {
  const rows = useSettings();
  const support = useWindowSupport();
  const stored = windowChoice(rows);
  const reread = useSettingsReread();
  const [opacity, setOpacity] = useState(stored?.opacity ?? 100);
  const [error, setError] = useState<string | null>(null);
  const desired = useRef<number | null>(null);
  const queue = useRef(new Map<string, string>());
  const sending = useRef(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    if (stored && (desired.current === null || desired.current === stored.opacity)) { desired.current = null; setOpacity(stored.opacity); }
  }, [stored?.opacity]);
  const change = (name: string, value: string) => {
    queue.current.set(name, value);
    if (sending.current) return;
    sending.current = true;
    void (async () => {
      while (queue.current.size) {
        const [key, next] = queue.current.entries().next().value!;
        queue.current.delete(key);
        try { await setConfig(key, next); if (alive.current) { setError(null); reread(); } }
        catch (e) { if (alive.current) { setError(e instanceof Error ? e.message : String(e)); if (key === "window-opacity") { desired.current = null; setOpacity(stored?.opacity ?? 100); } } }
      }
      sending.current = false;
    })();
  };
  if (!support.supported || !stored) return null;
  const choices = rows?.find(r => r.name === "window-material")?.choices ?? [];
  const effective = stored.material === "frost" && !support.blur ? "clear" : stored.material;
  return <>
    <MoreSection name="window" label={gui.window_surface_title}>
      <PopupRow name="window-material" label={gui.window_surface_material} value={effective} choices={choices.filter(c => c.value !== "frost" || support.blur)} onPick={v => change("window-material", v)} />
      {effective !== "solid" && <div data-setting-row="slider" data-setting-row-name="window-opacity" className="flex flex-col gap-in px-row py-in">
        <div className="flex items-center justify-between text-body"><span>{gui.window_surface_opacity}</span><output className="text-callout tabular-nums text-muted-foreground">{opacity}%</output></div>
        <Slider label={gui.window_surface_opacity} min={75} max={100} value={opacity} ticks={6} small={<span className="text-caption">75%</span>} large={<span className="text-caption">100%</span>} onChange={v => { desired.current = v; setOpacity(v); change("window-opacity", String(v)); }} />
      </div>}
    </MoreSection>
    {error && <p role="status" className="mb-group text-callout text-destructive">{error}</p>}
  </>;
}
