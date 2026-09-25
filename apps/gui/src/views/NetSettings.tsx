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
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

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

  return (
    <div data-net-settings className="flex flex-col gap-2">
      <h2 className="text-callout font-label text-muted-foreground">{gui.relays_mine}</h2>

      {choices.mine.map((url) => (
        <div key={url} data-relay-mine className="flex items-center gap-2">
          <span className="min-w-0 flex-1 truncate font-mono text-callout">{url}</span>
          <Button
            variant="ghost"
            size="sm"
            className="min-h-touch"
            onClick={() => write(choices.mine.filter((u) => u !== url), choices.only_mine)}
          >
            {gui.remove}
          </Button>
        </div>
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
          question does not exist until there is a row. Pressed-state on
          a `Button` because this app has one control family and no
          checkbox in it — the same reason the face options are buttons. */}
      <Button
        variant="ghost"
        size="sm"
        data-relays-only-mine
        data-on={choices.only_mine}
        aria-pressed={choices.only_mine}
        className={cn(
          // **A ring in both states, not only when it is on.** Off is
          // the state this control ships in and the state somebody has
          // to discover it in — and measured on the glass (2026-09-18),
          // a ghost button with no edge reads as one more line of prose
          // between the input and the note under it. The ring is what
          // says「按得动」; its weight and ink are what say which way it
          // is set.
          "min-h-touch self-start rounded-md px-2 text-caption",
          choices.only_mine
            ? "text-primary ring-2 ring-primary"
            : "text-muted-foreground ring-1 ring-border",
        )}
        onClick={() => write(choices.mine, !choices.only_mine)}
      >
        {gui.relays_only_mine}
      </Button>

      <p data-relays-note className="text-caption text-muted-foreground">
        {gui.relays_after_restart}
      </p>

      <h2 className="mt-2 text-callout font-label text-muted-foreground">{gui.relays_in_use}</h2>
      <ul data-relays-in-use className="flex flex-col gap-1">
        {choices.in_use.map((url) => (
          <li key={url} className="truncate font-mono text-caption text-muted-foreground">
            {url}
          </li>
        ))}
      </ul>
    </div>
  );
}
