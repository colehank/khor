// What one browser tab is showing: a page opened through some machine's
// network (docs/NET.md 借网), and whether that worked.
//
// **Thin on purpose** (浏览器接壳批). This used to be the whole landing —
// its own header, its own address box, its own list of pinned pages. All
// three moved: the machine's name is the address bar's chip, the address
// box is the address bar, and the pins are the fresh tab. What is left is
// the half that is genuinely about borrowing a network: doing it, saying
// whether it worked, and keeping the page if it is worth keeping.
//
// **Opening is a real act, and that is why it happens here rather than
// in the shell.** `BrowseLanding`'s `onPlace` only says *where to go*;
// for a directory that costs a listing, but for a page it opens a real
// borrow session on the far machine — something nobody can use is
// something nobody thinks to close (`api.ts` on `openWeb`). So the body
// is the one that decides when to spend it, and the shell is told only
// once it worked (`onLanded` → the visit is recorded).
import { useEffect, useState } from "react";

import { fetchWebPins, openWeb, pinWeb } from "@/api";
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** A page's scheme is optional to type; a bare host means https. */
export function normalize(raw: string): string | null {
  const t = raw.trim();
  if (!t) return null;
  return /^[a-z]+:\/\//i.test(t) ? t : `https://${t}`;
}

/** What the open action is doing or has done, said in place. */
type Opening = { url: string; state: "opening" | "opened" | "failed"; word: string };

export function BrowserPane({
  machine,
  device,
  url,
  onOpened,
}: {
  /** The exit machine's name — whose network a page leaves through. */
  machine: string;
  /** Its device id — what a web pin is keyed by. */
  device: string;
  /** The page this tab is pointed at; `""` before one has been named. */
  url: string;
  /** It really opened. Only on success: a borrow that was refused is
      not a place this person has been. */
  onOpened: (url: string) => void;
}) {
  const [opening, setOpening] = useState<Opening | null>(null);
  // This machine's pinned pages — only so the control below can show
  // which of its two jobs it is about to do.
  const [pins, setPins] = useState<string[]>([]);

  const loadPins = () => {
    fetchWebPins()
      .then((rows) => setPins(rows.filter((r) => r.device === device).map((r) => r.url)))
      .catch(() => {});
  };
  useEffect(loadPins, [device]);

  // **One borrow per address, and only when there is one.** The tab
  // arriving pointed at a page is the same act as somebody pressing
  // Enter on it — both come through as a new `url`, and neither should
  // spend a second borrow on a page already open.
  useEffect(() => {
    const want = normalize(url);
    if (!want) return;
    let stopped = false;
    setOpening({ url: want, state: "opening", word: gui.web_opening(machine) });
    openWeb(machine, want).then(
      () => {
        if (stopped) return;
        setOpening({ url: want, state: "opened", word: gui.web_opened(machine) });
        onOpened(want);
      },
      (e) => {
        if (stopped) return;
        setOpening({
          url: want,
          state: "failed",
          word: String(e instanceof Error ? e.message : e),
        });
      },
    );
    return () => {
      stopped = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, url]);

  const here = normalize(url);
  const pinned = here !== null && pins.includes(here);

  return (
    <section className="flex h-full min-w-0 flex-col">
      {opening && (
        // **The one thing this must never be is blank.** A borrow that
        // was refused is the whole of what this pane has to say, in the
        // node's own sentence — the same rule the files body follows for
        // a directory it could not list.
        <div
          data-web-status={opening.state}
          data-cannot-reach={opening.state === "failed" ? "" : undefined}
          className={cn(
            "flex-none px-4 py-3",
            opening.state === "failed" ? "text-fact" : "text-sm text-muted-foreground",
          )}
        >
          {opening.word}
        </div>
      )}
      {/* Nothing else, and deliberately no sentence explaining that.
          Before an address is named the address bar above is the whole
          affordance and it already has the caret; a line of prose under
          it would be the decoration docs/UX.md 装饰性说明归零 forbids.
          What khor cannot do here is show the page — the borrow hands
          back a proxy port and only the app's own window can point at
          it (`api.ts`), so this space stays the borrow's report. */}
      <div className="min-h-0 flex-1" />
      {here && (
        // The one place a page becomes a shortcut. Two words for one
        // control, the pin's rule: it shows what pressing it will do.
        <div className="flex-none border-t p-2">
          <Button
            variant="ghost"
            size="sm"
            data-pin-web
            data-on={pinned}
            aria-label={pinned ? gui.unpin : gui.pin}
            className="text-muted-foreground"
            onClick={() => {
              pinWeb(machine, here, !pinned)
                .then(loadPins)
                .catch(() => {});
            }}
          >
            <IconPin pinned={pinned} />
            <span className="ml-2">{pinned ? gui.unpin : gui.pin}</span>
          </Button>
        </div>
      )}
    </section>
  );
}
