// What one tab is showing: one machine's directory, painted in the
// node's order (it sorts, this pane never re-sorts — docs/UX.md).
// Navigation is the tree's own: down by pressing a directory, up by the
// 上一级 row, anywhere by the address bar above.
//
// **Where the tab is belongs to the tab, not to this pane** (标签页一期).
// It arrives as `path` and every move is reported back through
// `onPath`, which is what lets the strip title a tab, the address bar
// draw a breadcrumb, and a reload land where the person left off —
// three faces of one fact, and one place that owns it. The pane keeping
// its own idea of where it was is how those three drift apart.
//
// **There is no "back" on either face any more.** There used to be one
// on narrow, back to the machine list; that list is gone, so the
// button had nothing left to return to (docs/UX.md 返回 only exists
// where the list is really off screen). Going elsewhere is the
// switcher; going up is 上一级.
//
// Taking a file is the row's one action. The landing (the downloads
// directory) was chosen silently, so the row says where the file went;
// a failure wears its words on the same row — the pin's rule, because
// this app has nowhere that collects messages.
import { useEffect, useState } from "react";

import { fetchDirPins, fetchLs, pinDir, pullFile, type DirListing } from "@/api";
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago } from "@/words";

function sizeWord(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let u = 0;
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024;
    u += 1;
  }
  return `${v >= 100 ? Math.round(v) : Math.round(v * 10) / 10} ${units[u]}`;
}

/** What one pressed row is doing or has done. */
type Took = { path: string; state: "taking" | "landed" | "failed"; word: string };

export function FilesPane({
  machine,
  device,
  path,
  narrow,
  onPath,
  onLanded,
}: {
  /** The machine's name — what the node resolves. */
  machine: string;
  /** Its device id — what a directory pin is keyed by. */
  device: string;
  /** Where to open; "" is that machine's home. */
  path: string;
  narrow: boolean;
  /** Where the tab should now be. Called both for a step the person
      took and for the home a `""` resolved to — the tab holds the
      answer, this pane only ever asks. */
  onPath: (path: string) => void;
  /** A directory really opened. Separate from `onPath` because the two
      are different facts: `onPath` fires for an *intention* that may
      still be refused, this one only after a listing came back. */
  onLanded: (path: string) => void;
}) {
  const [listing, setListing] = useState<DirListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [took, setTook] = useState<Took | null>(null);
  // This machine's pinned paths. Refreshed with the pins call itself:
  // each toggle answers by re-asking, so the mark never guesses.
  const [pinned, setPinned] = useState<Set<string>>(new Set());

  const loadPins = () => {
    fetchDirPins()
      .then((rows) =>
        setPinned(new Set(rows.filter((r) => r.device === device).map((r) => r.path))),
      )
      .catch(() => {});
  };
  // The pane opens where the tab points — "" asks for home, and the
  // answer carries the real path, which is handed straight back so the
  // tab stops saying "home" and starts saying where that is.
  //
  // **The listing it already has is not asked for again**, which is
  // what keeps that hand-back from being a second round trip: the
  // resolved path arrives as a new `path`, and this sees it is the one
  // already on screen.
  useEffect(() => {
    let stopped = false;
    // Already here. Clearing the refusal is part of that: coming back
    // to a directory that is on screen must not leave the sentence
    // about the one that could not be reached standing over it.
    if (listing && listing.path === path) {
      setError(null);
      return;
    }
    fetchLs(machine, path)
      .then((l) => {
        if (stopped) return;
        setListing(l);
        setError(null);
        if (l.path !== path) onPath(l.path);
        // **After a real listing, with the machine's own spelling of the
        // path rather than whatever was typed.** What the shell does
        // with that — records a visit, re-reads its places — is the
        // shell's business (`BrowseLanding`); this only says that
        // something really opened.
        onLanded(l.path);
      })
      // The message is the node's, in the catalog's words.
      .catch((e) => {
        if (!stopped) setError(String(e instanceof Error ? e.message : e));
      });
    return () => {
      stopped = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, path]);

  useEffect(loadPins, [device]);

  const togglePin = (p: string) => {
    pinDir(machine, p, !pinned.has(p))
      .then(loadPins)
      .catch(() => {});
  };

  const up = () => {
    if (!listing) return;
    onPath(listing.path.slice(0, listing.path.lastIndexOf("/")) || "/");
  };

  const take = (path: string) => {
    setTook({ path, state: "taking", word: "…" });
    pullFile(machine, path)
      .then((dest) => setTook({ path, state: "landed", word: gui.took_file(dest) }))
      .catch((e) =>
        setTook({ path, state: "failed", word: String(e instanceof Error ? e.message : e) }),
      );
  };

  // No header: which machine and where on it are the address bar's, and
  // a second copy of them here would be two spellings of one fact one
  // poll apart. What is left is the listing.
  return (
    <section className="flex h-full min-w-0 flex-col">
      <div data-files-list className="min-h-0 flex-1 overflow-y-auto">
        {error ? (
          // **The one thing this must never be is blank.** Enter in the
          // address bar says where to go; arriving is a different fact,
          // and when it does not happen the refusal is the whole of what
          // this pane has to say — in the node's own sentence, at the
          // size of something meant to be read, not as a footnote under
          // an empty directory that was never opened.
          <div data-cannot-reach className="p-4 text-fact">
            {error}
          </div>
        ) : !listing ? null : (
          <>
            {listing.path !== "/" && (
              <button
                type="button"
                data-up
                onClick={up}
                className="flex w-full items-center gap-2 border-b px-4 py-2 text-left text-sm text-muted-foreground hover:bg-muted/50"
              >
                {gui.up_dir}
              </button>
            )}
            {listing.entries.length === 0 && (
              <div data-empty className="p-4 text-sm text-muted-foreground">
                {gui.empty_dir}
              </div>
            )}
            {listing.entries.map((e) => {
              const full = `${listing.path === "/" ? "" : listing.path}/${e.name}`;
              const faded = e.name.startsWith(".");
              return e.dir ? (
                <div
                  key={e.name}
                  className={cn(
                    "group flex items-center pr-2 hover:bg-muted/50",
                    faded && "text-muted-foreground",
                  )}
                >
                  <button
                    type="button"
                    data-dir={e.name}
                    onClick={() => onPath(full)}
                    className="flex min-w-0 flex-1 items-center gap-2 px-4 py-2 text-left text-sm"
                  >
                    <span className="min-w-0 flex-1 truncate">{e.name}/</span>
                  </button>
                  {/* The two states get the two words the row action rule
                      demands; the mark stays visible once set. */}
                  <Button
                    size="icon"
                    variant="ghost"
                    data-pin-dir={e.name}
                    data-on={pinned.has(full)}
                    aria-label={pinned.has(full) ? gui.unpin : gui.pin}
                    // The session rows' visibility rule, worn here too:
                    // narrow draws it always (the browser face put this
                    // pane on phones, 批⑦, and a phone has no pointer to
                    // summon it with); wide reveals it under the pointer
                    // or keyboard focus, because a pin on every row of a
                    // directory was the pane's loudest repetition. A
                    // pinned row's mark stays drawn on both faces — a
                    // fact, not an entry point (`SessionsList`).
                    className={cn(
                      "flex-none text-muted-foreground",
                      !narrow &&
                        "opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 data-[on=true]:opacity-100",
                    )}
                    onClick={() => togglePin(full)}
                  >
                    <IconPin pinned={pinned.has(full)} />
                  </Button>
                </div>
              ) : (
                <div
                  key={e.name}
                  data-file={e.name}
                  className={cn(
                    "group flex items-center gap-3 px-4 py-2 text-sm",
                    faded && "text-muted-foreground",
                  )}
                >
                  <span className="min-w-0 flex-1 truncate">{e.name}</span>
                  {took?.path === full ? (
                    <span
                      data-took={took.state}
                      className={cn(
                        "max-w-[50%] truncate text-xs",
                        took.state === "failed" ? "text-state-failed" : "text-muted-foreground",
                      )}
                    >
                      {took.word}
                    </span>
                  ) : (
                    <>
                      <span className="flex-none text-xs text-muted-foreground">
                        {sizeWord(e.size)}
                      </span>
                      {e.at_ms > 0 && (
                        <span className="hidden flex-none text-xs text-muted-foreground sm:inline">
                          {ago(e.at_ms)}
                        </span>
                      )}
                      <Button
                        size="sm"
                        variant="ghost"
                        data-take={e.name}
                        // Always drawn, unlike the pin above: it is the
                        // row's one action, a word rather than a repeated
                        // glyph, and hiding it would leave a file row
                        // with nothing to do.
                        className="flex-none"
                        onClick={() => take(full)}
                      >
                        {gui.take_file}
                      </Button>
                    </>
                  )}
                </div>
              );
            })}
            {listing.truncated && (
              // The cap is however many rows actually came — the number
              // belongs to the node, and a copy here would drift.
              <div className="p-4 text-xs text-muted-foreground">
                {cli.dir_truncated(listing.entries.length)}
              </div>
            )}
          </>
        )}
      </div>
    </section>
  );
}
