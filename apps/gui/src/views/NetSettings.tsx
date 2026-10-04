// 中继 — the 网络 page's second group (docs/MORE.md; 2026-09-28).
//
// **Two lists, and they are not the same kind of thing.** 使用中 is
// every relay the machine actually binds with, the built-in tier
// included — shown because a screen that lists only what you typed is
// blank on a fresh machine and says nothing about the relay the
// connection is really standing on, and shown *as a fact* because a
// delete button that refuses is worse than no button. 自己的中继 is what
// the user added here and the only part this group can remove.
//
// **Open on arrival, and read on arrival** (2026-09-28). It sat folded
// under 高级 for a week, mounted only when the fold opened — on the
// judgment that reading it was 「a machine dialling on behalf of a
// control nobody opened」. It is not: `relays` reads this store's
// relay file and the configured tier (`khor_gui_core::relays`), with no
// network at all, the same kind of read as `about`. And the page it
// was folded on is two short groups now, one screen with room to spare
// — so the owner's sketch has it open, and a fold here would hide the
// one fact (使用中) a person opens this page to check.
//
// The group holds no relay state of its own: every write goes through
// `setRelays` and paints the answer. "Did it take" is never a question
// this layer answers from memory.
//
// **A key is typed once and never shown again** (2026-09-30, docs/NET.md
// 中继). The add window has a second box for the relay's shared key;
// after that the page only knows *whether* a relay is keyed (`keyed`,
// the row's note) — every later write names the relays to keep with no
// key, and the node keeps the one it stored (`Node::set_relays`).
//
// ⚠️ **The one thing here that is not visible in the data:** the list is
// read when a node opens. The app opens one per call so it sees a change
// at once, but a running `khor serve` and every standing connection keep
// the list they started with. That is what `gui.relays_after_restart`
// is for, and it is not decoration — without it the screen shows a
// change that took and a network that did not move, with nothing
// connecting the two.
import { useCallback, useEffect, useState } from "react";

import { fetchRelays, setPreferredRelay, setRelays, type RelayChoices, type RelayWant } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { MoreFact, MoreSection } from "@/views/more/MorePage";
import { Waiting } from "@/components/RegionState";

/** A relay to keep as it is stored — its key, if any, stays put. */
const kept = (url: string): RelayWant => ({ url, token: null });

/** **How a relay is reached, said in words** (round NET-7): HTTPS, or
    HTTP in the clear — the Beijing relay is both for a while, the same
    box over two schemes, and the address alone left the difference to
    a reader who knows what the letters mean. */
const scheme = (url: string) => (url.startsWith("https://") ? gui.relays_scheme_https : gui.relays_scheme_http);

/** Mounting is reading: the page is drawn only while it is on screen. */
export function NetSettings() {
  const [choices, setChoices] = useState<RelayChoices | null>(null);
  /** When the first asking began (`Waiting`, below). */
  const [since] = useState(() => Date.now());
  const [error, setError] = useState<string | null>(null);
  // The add window: its box, the node's refusal of what was in it, and
  // whether it was opened by the only-mine switch — then saving adds
  // the relay **and** turns the switch on, which is what the person
  // pressed the switch for (安全's PIN-then-lock, the same road).
  const [adding, setAdding] = useState(false);
  const [typed, setTyped] = useState("");
  const [typedKey, setTypedKey] = useState("");
  const [refused, setRefused] = useState<string | null>(null);
  const [arming, setArming] = useState(false);

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
  // replaces). The node's own sentence names the entry it refused.
  const write = useCallback(async (next: RelayWant[], onlyMine: boolean, onRefuse: (why: string) => void) => {
    try {
      setChoices(await setRelays(next, onlyMine));
      return true;
    } catch (e) {
      onRefuse(String(e instanceof Error ? e.message : e));
      return false;
    }
  }, []);

  const openAdd = (arm: boolean) => {
    setTyped("");
    setTypedKey("");
    setRefused(null);
    setArming(arm);
    setAdding(true);
  };

  return (
    <MoreSection name="relays" label={gui.net_relays} mark={{ "data-net-settings": "" }}>
      {error && (
        <div data-relays-error className="text-callout text-destructive">
          {error}
        </div>
      )}
      {/* Loading until the first answer (UX-4): the heading stood over
          nothing, and the next group's 离开网络 read as a relay. */}
      {!choices && !error && <Waiting since={since} />}
      {choices && (
        <>
          {/* 使用中: a fact, one address a line — nothing here can be
              removed, so nothing here offers to. */}
          <MoreFact name="relays-in-use" label={gui.relays_in_use}>
            <span data-relays-in-use className="flex flex-col items-end font-mono">
              {choices.in_use.map((url) => (
                <span key={url} data-relay-scheme={url.startsWith("https://") ? "https" : "http"}>
                  {url}
                  <span className="ml-2 font-sans text-muted-foreground">{scheme(url)}</span>
                </span>
              ))}
            </span>
          </MoreFact>

          {/* 首选中继 (round NET-2): which of the relays in use this
              machine homes on whenever it is reachable — 自动 is iroh's
              own choice, the nearest. The system's menu, like every
              choice of one among a few. Drawn only with more than one
              relay in use: with one there is nothing to prefer. */}
          {choices.in_use.length > 1 && (
            <PopupRow
              name="relays-preferred"
              label={gui.relays_preferred}
              hint={gui.relays_preferred_note}
              value={choices.preferred ?? ""}
              choices={[
                { value: "", label: gui.relays_preferred_auto },
                ...choices.in_use.map((url) => ({ value: url, label: `${url}（${scheme(url)}）` })),
              ]}
              onPick={(v) => {
                setError(null);
                setPreferredRelay(v || null)
                  .then(fetchRelays)
                  .then(setChoices)
                  .catch((e: unknown) => setError(String(e instanceof Error ? e.message : e)));
              }}
              mark={{ "data-relays-preferred": choices.preferred ?? "" }}
            />
          )}

          {/* 自己的中继: the row that adds one (its act asks for the
              address, so it opens a window and wears the ellipsis —
              ⚠️ 墓碑 2026-09-28: a text box and a button stood in the
              card here, a form in a row's place), then one row per
              entry whose one act is 移除, on the spot. */}
          <ActionRow
            name="relays-mine"
            label={gui.relays_mine}
            actionLabel={gui.relays_add_ask}
            onPress={() => openAdd(false)}
            mark={{ "data-relay-add": "" }}
          />
          {choices.mine.map((url) => (
            <ActionRow
              key={url}
              name="relay"
              rowMark={{ "data-relay-mine": url }}
              label={<span data-selectable className="block truncate font-mono">{url}</span>}
              hint={choices.keyed.includes(url) ? <span data-relay-keyed>{gui.relays_keyed}</span> : undefined}
              actionLabel={gui.remove}
              onPress={() =>
                void write(
                  choices.mine.filter((u) => u !== url).map(kept),
                  choices.only_mine,
                  setError,
                )
              }
            />
          ))}

          {/* The switch, under the list it governs: it answers
              「加了这台之后原来那些还在不在」. **With none of your own it
              asks for one first** — the node refuses only-mine over an
              empty list (`relays-only-mine-needs-one`), so a switch that
              just sent it would be a press whose one answer is no. */}
          <SwitchRow
            name="relays-only-mine"
            label={gui.relays_only_mine}
            hint={<span data-relays-note>{gui.relays_after_restart}</span>}
            on={choices.only_mine}
            onToggle={() => {
              setError(null);
              if (!choices.only_mine && choices.mine.length === 0) {
                openAdd(true);
                return;
              }
              void write(choices.mine.map(kept), !choices.only_mine, setError);
            }}
            mark={{ "data-relays-only-mine": "" }}
          />

          <Dialog open={adding} onOpenChange={setAdding}>
            <DialogContent form data-relay-add-dialog>
              <DialogHeader>
                <DialogTitle>{gui.relays_add}</DialogTitle>
              </DialogHeader>
              <form
                id="relay-add"
                className="flex flex-col gap-in"
                onSubmit={async (e) => {
                  e.preventDefault();
                  const url = typed.trim();
                  // Always pressable (`components/Why`): with no address,
                  // say so and put the caret in its box.
                  if (!url) {
                    setRefused(gui.need_relay);
                    document.querySelector<HTMLElement>("[data-relay-add-url]")?.focus();
                    return;
                  }
                  const token = typedKey.trim() || null;
                  // Closed only on success: a refused address stays in
                  // the box next to the sentence saying why, so the fix
                  // is an edit and not a retype.
                  if (
                    await write(
                      [...choices.mine.filter((u) => u !== url).map(kept), { url, token }],
                      arming || choices.only_mine,
                      setRefused,
                    )
                  ) {
                    setAdding(false);
                    setArming(false);
                  }
                }}
              >
                <Input
                  data-relay-add-url
                  autoComplete="off"
                  aria-label={gui.relays_add}
                  placeholder={gui.relays_hint}
                  value={typed}
                  onChange={(e) => setTyped(e.target.value)}
                  className="min-h-touch w-full font-mono"
                />
                {/* The relay's shared key, when its operator set one
                    (`khor relay --token`, iroh-relay's shared_token).
                    Masked: it is a secret, and this is the one moment
                    it is on screen at all. */}
                <Input
                  data-relay-add-key
                  type="password"
                  autoComplete="off"
                  aria-label={gui.relays_key}
                  placeholder={gui.relays_key_hint}
                  value={typedKey}
                  onChange={(e) => setTypedKey(e.target.value)}
                  className="min-h-touch w-full font-mono"
                />
              </form>
              {refused && (
                <p data-relays-refused className="text-callout text-destructive">
                  {refused}
                </p>
              )}
              <DialogFooter>
                <Button onClick={() => setAdding(false)}>{gui.delete_keep}</Button>
                <Button type="submit" form="relay-add" variant="primary" data-relay-add-do>
                  {gui.watch_add}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </>
      )}
    </MoreSection>
  );
}
