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
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import { useSettings } from "@/settings";
import { MoreSection, MoreSwitch, type MorePage } from "@/views/more/MorePage";

/** How many choices still read as a ladder of switches. Past this a row
    is a menu: 标签页上限 has 48 (off, 4…50 — docs/SHELL.md 第三轮), and
    48 switches wrapping down the page is a wall, not a setting. */
const LADDER_MAX = 8;

/** One setting: its name on the left, its ladder under it. */
function Row({
  row,
  onPick,
}: {
  row: ConfigRow;
  onPick: (name: string, value: string) => void;
}) {
  return (
    // `data-value` carries the **key**, not the word: verification that
    // had to read the label would be reading the catalog to check the
    // catalog (`MoreSection`'s own split).
    <div data-config-row={row.name} data-value={row.value} className="flex flex-col gap-1">
      <span className="text-callout">{row.label}</span>
      {row.choices.length > LADDER_MAX ? (
        // **The same keys either way**: each item wears the anchor a
        // switch would, so a gate presses `data-config-choice` whichever
        // shape the row took.
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="secondary"
              size="sm"
              data-config-open={row.name}
              className="min-h-touch min-w-touch self-start"
            >
              {row.choices.find((c) => c.value === row.value)?.label ?? row.value}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="max-h-(--radix-dropdown-menu-content-available-height) overflow-y-auto">
            <DropdownMenuRadioGroup value={row.value} onValueChange={(v) => onPick(row.name, v)}>
              {row.choices.map((c) => (
                <DropdownMenuRadioItem key={c.value} value={c.value} data-config-choice={`${row.name}:${c.value}`}>
                  {c.label}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      ) : (
      <div className="flex flex-wrap gap-1">
        {row.choices.map((c) => (
          <MoreSwitch
            key={c.value}
            mark={{ "data-config-choice": `${row.name}:${c.value}` }}
            label={c.label}
            on={row.value === c.value}
            onPress={() => onPick(row.name, c.value)}
          />
        ))}
      </div>
      )}
    </div>
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
    return <p className="text-callout text-muted-foreground">{gui.backend_unreachable}</p>;
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
