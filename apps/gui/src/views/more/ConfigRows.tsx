// The settings rows themselves (docs/MORE.md 设置), drawn wherever the
// node's table says they belong.
//
// **Two pages draw these and neither owns them.** 设置 takes 会话 and
// 工作区; 安全 takes 危险动作二次确认, because that is where docs/MORE.md
// puts it. Which section a key lives in is the node's answer
// (`khor_node::config::Key::section`) and travels on the row — a
// grouping written here would be a second table of the same fact, one
// that compiles and runs and puts a key under the wrong heading the day
// somebody adds one.
//
// **Every word on screen comes from the answer**, never from a match in
// this file: `label` and each choice's `label` are catalog lookups the
// Rust side already did (`ConfigRow`). So the only thing here is which
// button is lit and what pressing one sends.
//
// **Nothing is drawn where the store could not be read.** A settings row
// painted from a default this file kept would be a switch reporting a
// value nothing stored — the 灰着的开关 docs/MORE.md refuses, wearing a
// state.
import { useState } from "react";

import { setConfig, type ConfigRow } from "@/api";
import { PopupRow } from "@/components/settings/PopupRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { gui } from "@/gen/catalog";
import { RegionState } from "@/components/RegionState";
import { useSettings, useSettingsReread } from "@/settings";
import { MoreSection, type MorePage } from "@/views/more/MorePage";

/**
 * One setting, as the one row kind a closed set of choices is
 * (docs/SHELL.md 第十七轮 §2): `current value ⌃⌄`, and the system's menu
 * when pressed.
 *
 * ⚠️ 墓碑 (第十七轮): up to eight choices were a ladder of `MoreSwitch`
 * buttons and past eight a Radix menu. Two shapes for one question, and
 * the second was a menu khor drew itself — the thing 「系统的东西交给系统
 * 画」 forbids. Both are one native `<select>` now.
 *
 * `data-config-row` / `data-value` carry the **key**, not the word, so a
 * gate never reads the catalog to check the catalog.
 */
function Row({
  row,
  onPick,
}: {
  row: ConfigRow;
  onPick: (name: string, value: string) => void;
}) {
  const mark = { "data-config-row": row.name, "data-value": row.value };
  // **Two states, on and off, is a switch** (components/SettingsGroup:
  // switch is 「一件立刻生效、只有两种状态的事」; the spec's own example
  // is 删除前确认). A popup offering 开 / 关 was the pick-one-of-several
  // control dressed over a yes/no. Decided by the node's value keys, not
  // by the words, so a key that grows a third value falls back to the
  // popup by itself.
  const keys = row.choices.map((c) => c.value).sort().join(",");
  if (keys === "off,on") {
    const on = row.value === "on";
    return (
      <SwitchRow
        name={row.name}
        label={row.label}
        hint={row.note ?? undefined}
        on={on}
        onToggle={() => onPick(row.name, on ? "off" : "on")}
        rowMark={mark}
      />
    );
  }
  return (
    <PopupRow
      name={row.name}
      label={row.label}
      // What the setting does, when the catalog has a sentence for it
      // (`ConfigRow.note`, 2026-10-02) — the node's word like the label.
      hint={row.note ?? undefined}
      value={row.value}
      choices={row.choices.map((c) => ({ value: c.value, label: c.label }))}
      onPick={(v) => onPick(row.name, v)}
      mark={mark}
    />
  );
}

export function ConfigSections({
  page,
  onChanged,
}: {
  /** Which 「…」 page is asking. Which sections that is, and in what
      order, is the node's answer — the rows arrive grouped and each one
      says which page draws it, so this file keeps no list. */
  page: MorePage;
  /** The shell re-reads after a change, so what is on screen is what the
      node says rather than what this page just sent. */
  onChanged: () => void;
}) {
  const rows = useSettings();
  const reread = useSettingsReread();
  const [error, setError] = useState<string | null>(null);

  // Two-argument `then`, and the re-read is never in the chain: a read
  // that threw after a change that took would report the change as
  // failed while the store is already wearing it (`FaceSettings`'s
  // rule).
  const press = (name: string, value: string) => {
    setError(null);
    setConfig(name, value).then(
      () => onChanged(),
      (e: unknown) => setError(String(e instanceof Error ? e.message : e)),
    );
  };

  if (rows === null) {
    // 出错 (第十七轮 §6): the machine's own sentence where the rows would
    // be, and 重试 — never a row painted from a default this file kept.
    return <RegionState state="error" message={gui.backend_unreachable} onRetry={reread} />;
  }

  const mine = rows.filter((r) => r.page === page);

  return (
    <>
      {error && (
        <div data-config-error className="text-callout text-destructive">
          {error}
        </div>
      )}
      {/* A heading starts where the section changes between
          neighbouring rows — the rows arrive grouped (`config::KEYS`,
          and a gate keeps them that way), which is what lets this hold
          no list of sections. A section with no keys on this build
          simply has no rows and so draws nothing: 没有落地实现的键不画,
          one level up, where an empty heading is a promise the page
          cannot keep. */}
      {mine.map((r, i) =>
        i > 0 && mine[i - 1].section === r.section ? null : (
          <MoreSection key={r.section} name={r.section} label={r.section_label}>
            {mine
              .filter((o) => o.section === r.section)
              .map((o) => (
                <Row key={o.name} row={o} onPick={press} />
              ))}
          </MoreSection>
        ),
      )}
    </>
  );
}
