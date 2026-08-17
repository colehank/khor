// The files landing's second step: one machine's directory, painted in
// the node's order (it sorts, this pane never re-sorts — docs/UX.md).
// Navigation is the tree's own: down by pressing a directory, up by
// the 上一级 row. There is no "back" — the machine list on the left
// never left (the desktop posture; narrow keeps its own back in the
// header like every detail).
//
// Taking a file is the row's one action. The landing (the downloads
// directory) was chosen silently, so the row says where the file went;
// a failure wears its words on the same row — the pin's rule, because
// this app has nowhere that collects messages.
import { useEffect, useState } from "react";

import { fetchDirPins, fetchLs, pinDir, pullFile, type DirListing } from "@/api";
import { IconBack, IconPin } from "@/components/icons";
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
  initialPath = "",
  narrow,
  onBack,
}: {
  /** The machine's name — what the node resolves. */
  machine: string;
  /** Its device id — what a directory pin is keyed by. */
  device: string;
  /** Where to open; "" is that machine's home. */
  initialPath?: string;
  narrow: boolean;
  onBack: () => void;
}) {
  const [listing, setListing] = useState<DirListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [took, setTook] = useState<Took | null>(null);
  // This machine's pinned paths. Refreshed with the pins call itself:
  // each toggle answers by re-asking, so the mark never guesses.
  const [pinned, setPinned] = useState<Set<string>>(new Set());

  const navigate = (path: string) => {
    fetchLs(machine, path)
      .then((l) => {
        setListing(l);
        setError(null);
      })
      // The message is the node's, in the catalog's words.
      .catch((e) => setError(String(e instanceof Error ? e.message : e)));
  };
  const loadPins = () => {
    fetchDirPins()
      .then((rows) =>
        setPinned(new Set(rows.filter((r) => r.device === device).map((r) => r.path))),
      )
      .catch(() => {});
  };
  // The pane opens where it was pointed — "" asks for home, and the
  // answer carries the real path so the way down can be spelled.
  useEffect(() => {
    navigate(initialPath);
    loadPins();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, initialPath]);

  const togglePin = (path: string) => {
    pinDir(machine, path, !pinned.has(path))
      .then(loadPins)
      .catch(() => {});
  };

  const up = () => {
    if (!listing) return;
    const parent = listing.path.slice(0, listing.path.lastIndexOf("/")) || "/";
    navigate(parent);
  };

  const take = (path: string) => {
    setTook({ path, state: "taking", word: "…" });
    pullFile(machine, path)
      .then((dest) => setTook({ path, state: "landed", word: gui.took_file(dest) }))
      .catch((e) =>
        setTook({ path, state: "failed", word: String(e instanceof Error ? e.message : e) }),
      );
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
        {/* The whole path rides the title; what shows truncates from
            the tail — rtl tricks scramble a slashed path's order. */}
        <span
          title={listing?.path ?? ""}
          className="min-w-0 flex-1 truncate text-sm text-muted-foreground"
        >
          {listing?.path ?? ""}
        </span>
      </header>
      <div data-files-list className="min-h-0 flex-1 overflow-y-auto">
        {error ? (
          <div className="p-4 text-sm text-muted-foreground">{error}</div>
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
                    onClick={() => navigate(full)}
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
                    className={cn(
                      "flex-none text-muted-foreground",
                      !pinned.has(full) && "invisible group-hover:visible",
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
                        className="invisible flex-none group-hover:visible"
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
