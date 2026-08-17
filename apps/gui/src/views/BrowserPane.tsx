// The browser landing's second step (docs/NET.md 借网): one machine as
// the exit, an address bar that opens a page through its network, and
// this machine's pinned pages below. There is no "back" — the machine
// list on the left never left (the desktop posture; narrow keeps its own
// back in the header like every detail).
//
// Opening builds a proxied window in the app; over the dev bridge there
// is no window, so the same action just resolves the borrow. Either way
// the row (or the bar) says what happened — this app has nowhere that
// collects messages, so a status wears its words in place (the pin's
// rule).
import { useEffect, useState } from "react";

import { fetchWebPins, openWeb, pinWeb } from "@/api";
import { IconBack, IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** A page's scheme is optional to type; a bare host means https. */
function normalize(raw: string): string | null {
  const t = raw.trim();
  if (!t) return null;
  return /^[a-z]+:\/\//i.test(t) ? t : `https://${t}`;
}

/** What the open action is doing or has done, said in place. */
type Opening = { url: string; state: "opening" | "opened" | "failed"; word: string };

export function BrowserPane({
  machine,
  device,
  initialUrl = null,
  narrow,
  onBack,
}: {
  /** The exit machine's name — whose network a page leaves through. */
  machine: string;
  /** Its device id — what a web pin is keyed by. */
  device: string;
  /** A page to open on arrival (a pinned shortcut was clicked). */
  initialUrl?: string | null;
  narrow: boolean;
  onBack: () => void;
}) {
  const [address, setAddress] = useState("");
  const [opening, setOpening] = useState<Opening | null>(null);
  // This machine's pinned pages. Refreshed with the pins call itself, so
  // the marks never guess.
  const [pins, setPins] = useState<{ url: string }[]>([]);

  const loadPins = () => {
    fetchWebPins()
      .then((rows) => setPins(rows.filter((r) => r.device === device).map((r) => ({ url: r.url }))))
      .catch(() => {});
  };
  useEffect(() => {
    loadPins();
    // A shortcut clicked from the pinned list arrives as initialUrl and
    // opens once, its address filled in so the bar shows what opened.
    if (initialUrl) {
      setAddress(initialUrl);
      open(initialUrl);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, device, initialUrl]);

  const open = (raw: string) => {
    const url = normalize(raw);
    if (!url) return;
    setOpening({ url, state: "opening", word: gui.web_opening(machine) });
    openWeb(machine, url)
      .then(() => setOpening({ url, state: "opened", word: gui.web_opened(machine) }))
      .catch((e) =>
        setOpening({ url, state: "failed", word: String(e instanceof Error ? e.message : e) }),
      );
  };

  const isPinned = (url: string) => pins.some((p) => p.url === url);
  const togglePin = (url: string) => {
    pinWeb(machine, url, !isPinned(url))
      .then(loadPins)
      .catch(() => {});
  };

  return (
    <section className="flex h-full min-w-0 flex-col">
      <header className="flex h-ctl-lg flex-none items-center gap-2 border-b px-3">
        {narrow && (
          <Button variant="ghost" size="icon" aria-label={gui.back} data-back onClick={onBack}>
            <IconBack />
          </Button>
        )}
        <span className="truncate font-semibold">{machine}</span>
      </header>
      <form
        className="flex flex-none items-center gap-2 border-b p-2"
        onSubmit={(e) => {
          e.preventDefault();
          open(address);
        }}
      >
        <Input
          data-web-address
          value={address}
          onChange={(e) => setAddress(e.target.value)}
          placeholder={gui.web_address(machine)}
          className="min-w-0 flex-1"
        />
        <Button type="submit" data-web-open disabled={!normalize(address)}>
          {gui.web_open}
        </Button>
      </form>
      {opening && (
        <div
          data-web-status={opening.state}
          className={cn(
            "flex-none truncate px-4 py-2 text-xs",
            opening.state === "failed" ? "text-state-failed" : "text-muted-foreground",
          )}
        >
          {opening.word}
        </div>
      )}
      <div data-web-pins className="min-h-0 flex-1 overflow-y-auto">
        {pins.length === 0 ? (
          <div className="p-4 text-sm text-muted-foreground">{gui.no_pinned_webs}</div>
        ) : (
          pins.map((p) => (
            <div key={p.url} className="group flex items-center gap-3 px-4 py-2 text-sm hover:bg-muted/50">
              <button
                type="button"
                data-web-pin-open={p.url}
                onClick={() => open(p.url)}
                className="min-w-0 flex-1 truncate text-left"
              >
                {p.url}
              </button>
              <Button
                size="icon"
                variant="ghost"
                data-unpin-web={p.url}
                aria-label={gui.unpin}
                className="flex-none text-muted-foreground"
                onClick={() => togglePin(p.url)}
              >
                <IconPin pinned />
              </Button>
            </div>
          ))
        )}
      </div>
      {/* The address bar can pin what is not yet pinned — the one place a
          page becomes a shortcut. Shown only when the typed address is a
          real url and not already kept. */}
      {normalize(address) && !isPinned(normalize(address)!) && (
        <div className="flex-none border-t p-2">
          <Button
            variant="ghost"
            size="sm"
            data-pin-web
            className="text-muted-foreground"
            onClick={() => togglePin(normalize(address)!)}
          >
            <IconPin pinned={false} />
            <span className="ml-2">{gui.pin}</span>
          </Button>
        </div>
      )}
    </section>
  );
}
