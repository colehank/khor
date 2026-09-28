// The machine's own relays — the settings screen's second section.
//
// **Two lists, and they are not the same kind of thing.** 自己的中继 is
// what the user typed here and the only part anything on this screen can
// remove. 在用 is every relay the machine actually binds with, the
// built-in tier included — shown because a screen that lists only what
// you typed is blank on a fresh machine and says nothing about the relay
// the connection is really standing on, and shown *without* controls
// because a delete button that refuses is worse than no button.
//
// The screen holds no relay state of its own: every write goes through
// `setRelays` and paints the answer, the same rule the face axes above
// run on. "Did it take" is never a question this layer answers from
// memory.
//
// ⚠️ **The one thing here that is not visible in the data:** the list is
// read when a node opens. The app opens one per call so it sees a change
// at once, but a running `khor serve` and every standing connection keep
// the list they started with. That is what `gui.relays_after_restart`
// is for, and it is not decoration — without it the screen shows a
// change that took and a network that did not move, with nothing
// connecting the two.
import { useCallback, useEffect, useState } from "react";

import { fetchRelays, setRelays, type RelayChoices } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { MoreSection } from "@/views/more/MorePage";

/**
 * **Mounting is opening, since 2026-09-19.** This took an `open` prop
 * while the settings screen was a dialog that stayed mounted with its
 * body hidden; the settings **page** is drawn only while it is the
 * thing on screen, so the mount is the event and a prop repeating it
 * would be a second answer to one question.
 */
export function NetSettings() {
  const [choices, setChoices] = useState<RelayChoices | null>(null);
  const [typed, setTyped] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    fetchRelays()
      .then((r) => {
        if (alive) setChoices(r);
      })
      .catch((e: unknown) => {
        if (alive) setError(String(e instanceof Error ? e.message : e));
      });
    return () => {
      alive = false;
    };
  }, []);

  // One door for every change — add and remove both send the whole list,
  // because that is the shape the node takes (`Node::set_relays`
  // replaces). Two paths that could disagree about what the list is
  // would be two paths to disagree about.
  const write = useCallback(async (next: string[], onlyMine: boolean) => {
    setError(null);
    try {
      setChoices(await setRelays(next, onlyMine));
      return true;
    } catch (e) {
      // The node's own sentence, which names the entry it refused. A
      // message invented here would be a second opinion about a rule
      // this layer does not hold — unwrapped the way every other
      // surface in this app unwraps one, because `String(someError)`
      // puts an English 「Error: 」 in front of a Chinese sentence.
      setError(String(e instanceof Error ? e.message : e));
      return false;
    }
  }, []);

  if (!choices) {
    return error ? (
      <div data-relays-error className="text-callout text-destructive">
        {error}
      </div>
    ) : null;
  }

  // **Two settings groups** (components/SettingsGroup), not a column of
  // its own headings and chips: 自己的中继 is rows — each relay an
  // action row whose one act is 去掉 (on the spot, so no ellipsis), the
  // box that adds one, and the 只用自己的 switch under the list it
  // governs; 在用 is facts, no controls.
  return (
    <div data-net-settings className="flex flex-col gap-group">
      <MoreSection name="relays-mine" label={gui.relays_mine}>
      {choices.mine.map((url) => (
        <ActionRow
          key={url}
          name="relay"
          rowMark={{ "data-relay-mine": url }}
          label={<span data-selectable className="block truncate font-mono">{url}</span>}
          actionLabel={gui.remove}
          variant="ghost"
          onPress={() => void write(choices.mine.filter((u) => u !== url), choices.only_mine)}
        />
      ))}

      <form
        className="flex items-center gap-2"
        onSubmit={async (e) => {
          e.preventDefault();
          const url = typed.trim();
          if (!url) return;
          // Cleared only on success: a refused address stays in the box
          // next to the sentence saying why, so the fix is an edit and
          // not a retype.
          if (await write([...choices.mine, url], choices.only_mine)) setTyped("");
        }}
      >
        <Input
          data-relay-add
          className="min-w-0 flex-1 font-mono"
          placeholder={gui.relays_hint}
          value={typed}
          onChange={(e) => setTyped(e.target.value)}
        />
        <Button type="submit" size="sm" className="min-h-touch" disabled={!typed.trim()}>
          {gui.relays_add}
        </Button>
      </form>

      {error && (
        <div data-relays-error className="text-callout text-destructive">
          {error}
        </div>
      )}

      {/* The switch, under the list it governs rather than above it:
          it is an answer to「加了这台之后原来那些还在不在」, and that
          question does not exist until there is a row. **A switch**
          (components/Switch): it is on or off and takes at once.
          ⚠️ 墓碑 (2026-09-27): it was a ringed `Button` wearing its own
          pressed state — 「this app has no checkbox」 — which is the
          one-off on/off look the four row kinds replaced. The note that
          says when it takes is the row's `note`. */}
      <SwitchRow
        name="relays-only-mine"
        label={gui.relays_only_mine}
        hint={<span data-relays-note>{gui.relays_after_restart}</span>}
        on={choices.only_mine}
        onToggle={() => void write(choices.mine, !choices.only_mine)}
        mark={{ "data-relays-only-mine": "" }}
      />
      </MoreSection>

      <MoreSection name="relays-in-use" label={gui.relays_in_use} mark={{ "data-relays-in-use": "" }}>
        {choices.in_use.map((url) => (
          <div key={url} data-selectable className="truncate font-mono text-callout text-muted-foreground">
            {url}
          </div>
        ))}
      </MoreSection>
    </div>
  );
}
