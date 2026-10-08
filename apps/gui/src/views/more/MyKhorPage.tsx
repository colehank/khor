// 我的 khor is one page, the dial (UX-26, the owner 2026-10-06, agreed on
// the prototype `audit/my-khor-dial.html`): no settings nav beside it,
// no card ground, no face block on top. Wide: the list column folds and
// ‹ goes back to 更多; phone: the content pages' deck (‹ · 「我的 khor」
// with the 地图/矩阵 segment · 「…」).
//
// **The dial.** This machine in the middle on the page's one olive (the
// `brand-halo` disc); three hairline rings — 30 ms, 120 ms and the rim
// 不通 — and every other machine at the radius its latency to this
// machine says (`lib/dial`: a log scale, no road = the rim), spread
// evenly in name order; a drag turns a seat round its ring and never
// changes its radius, because the radius is a reading. Names sit on the
// far side from the middle on a patch of the ground, so a road passes
// behind a name, never through it. Under the name a light in the band's
// colour (`lib/latency`, the same four bands the machine row's light
// wears) and the aside — the model, or when an unreachable machine was
// last seen. **Nothing about sessions on a seat**: that is the card's.
//
// **Roads at rest are this machine's only**, face edge to face edge,
// the band's colour, dashed by a relay, dotted with a × when none; no
// numbers at rest, no relay nodes (the relay's name is in the card and
// the matrix). **Look vs edit**: a mouse hovers a seat to look (that
// machine's roads to every machine draw, each with its ms) and
// right-clicks it for its card; a finger taps to look and long-presses
// for the card; the ground's right-click / long-press (and the phone's
// 「…」) is the add menu. No click-to-navigate anywhere: the card holds
// everything one can do to that machine, through the flows that exist
// (`MachineEditDialog`, `RootWordsDialog`, the leave / reset / forget
// confirmations, the invite / join / relay windows).
//
// **The matrix** (the 矩阵 segment): rows = the reporting machine,
// columns = to whom, each cell the ms on the band's wash with the road
// under it; the other machines' roads are what they wrote into the
// table (NET-12 `DeviceRow.roads`); an older peer that wrote none reads
// 未知, never 不通.
//
// Motion: a seat fades in on appearing and out on leaving; nothing else
// moves on its own.
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from "react";

import {
  fetchAbout,
  fetchRelays,
  fetchWaiting,
  fetchWorkspaces,
  inApp,
  leaveNetwork,
  resetIdentity,
  type About,
  type DeviceRow,
  type RelayChoices,
  type WorkspaceRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { DeleteDialog, type Pending } from "@/components/DeleteDialog";
import { GlassCircle } from "@/components/Glass";
import { MachineFace } from "@/components/SessionFace";
import { IconBack, IconMore } from "@/components/icons";
import { WindowControls } from "@/components/Region";
import { WINDOW_DRAG } from "@/components/shell";
import { Button } from "@/components/ui/button";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from "@/components/ui/context-menu";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { cli, config, gui, hop, msg } from "@/gen/catalog";
import { useCoarse } from "@/hooks/use-coarse";
import { useNarrow } from "@/hooks/use-narrow";
import { angleTo, cellOf, clip, firstHand, frame, labelOffset, linkOf, radiusFor, radiusOf, spread, type Link } from "@/lib/dial";
import { useFullScreen } from "@/lib/full-screen";
import { band, BAND_INK, type Band } from "@/lib/latency";
import { lastSeenMs } from "@/lib/topology";
import { cn } from "@/lib/utils";
import { CopyId } from "@/views/DetailPane";
import { MachineEditDialog } from "@/views/MachineEditDialog";
import { RelayAddDialog } from "@/views/NetSettings";
import { RootWordsDialog } from "@/views/PairDialogs";
import { span } from "@/words";

type View = "map" | "matrix";

/** What the card is about: this machine, or another by id. */
type Opened = { id: string; at: DOMRect | null };

/** Where remembered seat angles live, by machine id (rule 2: angles
    remembered locally; 「重新排布」 forgets them). */
const ANGLES_KEY = "khor.dial-angles";

function keptAngles(): Record<string, number> {
  try {
    const raw = window.localStorage.getItem(ANGLES_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : {};
    if (!parsed || typeof parsed !== "object") return {};
    const out: Record<string, number> = {};
    for (const [k, v] of Object.entries(parsed)) if (typeof v === "number" && Number.isFinite(v)) out[k] = v;
    return out;
  } catch {
    return {};
  }
}

function keepAngles(angles: Record<string, number>): void {
  try {
    window.localStorage.setItem(ANGLES_KEY, JSON.stringify(angles));
  } catch {
    // A private window keeps nothing; the seats spread evenly next time.
  }
}

/** The two seat sizes, read off the tokens so a face and the road
    clipped to its edge agree on one number. */
function seatSizes(): { seat: number; me: number; halo: number; touch: number } {
  const root = getComputedStyle(document.documentElement);
  const px = (v: string, fallback: number) => Number.parseFloat(root.getPropertyValue(v)) || fallback;
  return { seat: px("--dial-seat", 40), me: px("--dial-seat-me", 72), halo: px("--dial-halo", 22), touch: px("--touch", 44) };
}

/** A road's word for a card or a cell: 直连 / 中继 · <name> / 未连 /
    不通, with the ms (an idle edge's is its last reading); `null` (an
    older peer's silence) is 未知. */
function roadWord(l: Link | null): string {
  if (l === null) return gui.filter_unknown;
  if (l.road === "none") return gui.dial_key_none;
  const way = wayWord(l);
  return l.ms === null ? way : `${way} · ${gui.row_rtt(l.ms)}`;
}

/** The way alone, for the card's head. */
function wayWord(l: Link): string {
  if (l.road === "idle") return hop.quiet;
  return l.road === "direct" ? hop.direct : l.relay ? gui.dial_via(l.relay) : hop.relay;
}

export function MyKhorPage({
  devices,
  rows,
  onClose,
  onOpenMachine,
  onNewSession,
  onOpenWorkspaces,
  onOpenBlocked,
  onPin,
  onForget,
  onAdd,
  onOpenRelays,
  onChanged,
  onReset,
}: {
  /** The shell's own polled table — **not fetched again here** (two
      readers of one answer is how two surfaces disagree). */
  devices: DeviceRow[];
  /** The session rows, for the card's 会话 / 忙碌 / 待批 counts. */
  rows: { home: string; word: string }[];
  onClose: () => void;
  /** The card's 会话 ›: that machine's page, on its sessions. */
  onOpenMachine: (row: DeviceRow, body: "dashboard" | "sessions") => void;
  /** 新开终端 on that machine. */
  onNewSession: (row: DeviceRow) => void;
  /** 工作区 ›: the workspaces landing, narrowed to that machine. */
  onOpenWorkspaces: (row: DeviceRow) => void;
  /** 待批 ›: the sessions landing, narrowed to that machine's blocked rows. */
  onOpenBlocked: (row: DeviceRow) => void;
  /** 置顶 / 取消置顶 — rejects when it did not take. */
  onPin: (row: DeviceRow) => Promise<void>;
  /** 忘记这台机器 — `khor forget`; rejects with the node's sentence. */
  onForget: (row: DeviceRow) => Promise<void>;
  /** The add menu's 生成邀请票… and 用票配对… (and the card's 重新配对…). */
  /** 添加机器 (round join-d), on its first page or on khor 主机's. */
  onAdd: (start?: "pick" | "ticket") => void;
  /** This machine's 中继… ›: 设置 → 连接. */
  onOpenRelays: () => void;
  /** The device table changed (a rename, a face, leaving the network). */
  onChanged: () => void;
  /** This machine's identity was reset. */
  onReset: () => void;
}) {
  const narrow = useNarrow();
  const coarse = useCoarse();
  // The phone: the whole window (no first row, no landings' bar), as
  // every content page.
  useFullScreen(narrow);
  const [view, setView] = useState<View>("map");
  const [about, setAbout] = useState<About | null>(null);
  const [relays, setRelaysKnown] = useState<RelayChoices | null>(null);
  const [workspaces, setWorkspaces] = useState<WorkspaceRow[]>([]);
  useEffect(() => {
    let live = true;
    void fetchAbout().then((a) => live && setAbout(a), () => {});
    void fetchRelays().then((r) => live && setRelaysKnown(r), () => {});
    void fetchWorkspaces().then((w) => live && setWorkspaces(w), () => {});
    return () => {
      live = false;
    };
  }, []);
  /** This machine holds the network's root — 恢复词 is its alone, and
      only in the app (the web face is never given them). */
  const [holdsRoot, setHoldsRoot] = useState(false);
  useEffect(() => {
    if (!inApp) return;
    let live = true;
    fetchWaiting().then(
      (w) => live && setHoldsRoot(w.holds),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);

  const me = devices.find((d) => d.me) ?? null;
  const alone = devices.every((d) => d.me);
  // Seats in name order, so the dial reads the same every time.
  const others = useMemo(
    () => devices.filter((d) => !d.me).sort((a, b) => a.shown.localeCompare(b.shown, undefined, { numeric: true })),
    [devices],
  );

  // ── what is open: the look, the card, the menu, the windows ──
  const [looked, setLooked] = useState<string | null>(null);
  const [opened, setOpened] = useState<Opened | null>(null);
  const [editing, setEditing] = useState<DeviceRow | null>(null);
  const [showingWords, setShowingWords] = useState(false);
  const [addingRelay, setAddingRelay] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [forgetting, setForgetting] = useState<Pending | null>(null);
  const [typedName, setTypedName] = useState("");
  const [refused, setRefused] = useState<string | null>(null);
  /** What khor said the last time 离开网络 was pressed (`msg::left`). */
  const [said, setSaid] = useState<string[] | null>(null);
  const [angles, setAngles] = useState<Record<string, number>>(keptAngles);

  const close = useCallback(() => {
    setOpened(null);
    setLooked(null);
  }, []);
  // A machine that left the table while its card was open.
  useEffect(() => {
    if (opened && !devices.some((d) => d.id === opened.id)) close();
  }, [devices, opened, close]);
  useEffect(() => {
    if (!opened) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [opened, close]);

  const ask = (what: "leave" | "reset") => {
    close();
    setRefused(null);
    setSaid(null);
    setTypedName("");
    if (what === "leave") setLeaving(true);
    else setResetting(true);
  };

  const addMenu = (Item: typeof ContextMenuItem | typeof DropdownMenuItem, Sep: typeof ContextMenuSeparator | typeof DropdownMenuSeparator) => (
    <>
      <Item data-dial-add="add" onSelect={() => onAdd()}>
        {gui.add_machine_ask}
      </Item>
      <Item data-dial-add="relay" onSelect={() => setAddingRelay(true)}>
        {gui.relays_add_ask}
      </Item>
      <Sep />
      <Item
        data-dial-add="rearrange"
        onSelect={() => {
          setAngles({});
          keepAngles({});
        }}
      >
        {gui.dial_rearrange}
      </Item>
    </>
  );

  const segment = (mark: string) => (
    <SegmentedControl<View>
      label={gui.my_khor}
      value={view}
      onChange={(v) => {
        close();
        setView(v);
      }}
      mark={{ [mark]: true }}
      className="flex-none"
      options={[
        { key: "map", label: gui.dial_view_map, mark: { "data-dial-view": "map" } },
        { key: "matrix", label: gui.dial_view_matrix, mark: { "data-dial-view": "matrix" } },
      ]}
    />
  );

  const openedRow = opened ? (devices.find((d) => d.id === opened.id) ?? null) : null;
  const card =
    openedRow && me ? (
      openedRow.me ? (
        <MeCard
          me={me}
          about={about}
          relays={relays}
          holdsRoot={inApp && holdsRoot}
          alone={alone}
          onEdit={() => {
            close();
            setEditing(me);
          }}
          onWords={() => {
            close();
            setShowingWords(true);
          }}
          onRelays={() => {
            close();
            onOpenRelays();
          }}
          onLeave={() => ask("leave")}
          onReset={() => ask("reset")}
        />
      ) : (
        <MachineCard
          row={openedRow}
          rows={devices}
          sessions={rows}
          workspaces={workspaces.filter((w) => w.device === openedRow.id).length}
          coarse={coarse}
          onNewSession={() => {
            close();
            onNewSession(openedRow);
          }}
          onSessions={() => {
            close();
            onOpenMachine(openedRow, "sessions");
          }}
          onWorkspaces={() => {
            close();
            onOpenWorkspaces(openedRow);
          }}
          onBlocked={() => {
            close();
            onOpenBlocked(openedRow);
          }}
          onRepair={() => {
            close();
            onAdd("ticket");
          }}
          onEdit={() => {
            close();
            setEditing(openedRow);
          }}
          onPin={() => {
            close();
            onPin(openedRow).catch(() => {});
          }}
          onForget={() => {
            close();
            setForgetting({ name: openedRow.shown, says: gui.forget_machine_confirm, run: () => onForget(openedRow) });
          }}
        />
      )
    ) : null;

  return (
    <div data-dial-page data-narrow-page={narrow || undefined} className="relative flex h-full min-h-0 flex-1 flex-col bg-card">
      {/* **The wide face's first row** — the page's own, since the page
          stands alone in the window (rule 1): the way back where every
          detail screen's ‹ is, the name, the 地图/矩阵 segment at the
          right end with the page's buttons. The row is one of the
          window's handles (`WINDOW_DRAG`). */}
      {!narrow && (
        <div data-dial-top {...WINDOW_DRAG} className="flex h-pane-head flex-none items-center gap-in ps-wide-lead-start pe-row select-none">
          <GlassCircle aria-label={gui.back} data-more-close data-dial-back onClick={onClose}>
            <IconBack />
          </GlassCircle>
          <h1 className="min-w-0 flex-1 truncate text-title-sm font-title">{gui.my_khor}</h1>
          {segment("data-dial-segment")}
          <WindowControls />
        </div>
      )}
      <section
        data-more-page="me"
        data-dial-view={view}
        aria-label={gui.my_khor}
        className="relative min-h-0 flex-1 overflow-hidden"
      >
        {view === "map" ? (
          <ContextMenu>
            <ContextMenuTrigger asChild>
              <div data-dial-ground className="absolute inset-0">
                <Dial
                  me={me}
                  others={others}
                  rows={devices}
                  narrow={narrow}
                  coarse={coarse}
                  angles={angles}
                  looked={looked}
                  opened={opened?.id ?? null}
                  onLook={setLooked}
                  onOpen={(id, at) => setOpened({ id, at })}
                  onTurn={(id, angle) => {
                    setAngles((was) => ({ ...was, [id]: angle }));
                  }}
                  onTurned={() =>
                    setAngles((a) => {
                      keepAngles(a);
                      return a;
                    })
                  }
                />
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent data-dial-menu>{addMenu(ContextMenuItem, ContextMenuSeparator)}</ContextMenuContent>
          </ContextMenu>
        ) : (
          <Matrix me={me} others={others} rows={devices} narrow={narrow} onOpen={(id, at) => setOpened({ id, at })} />
        )}
        {/* The key: the four bands, each a light beside its word, and the
            count — tertiary, at the foot, out of the dial's way. */}
        <div
          data-dial-foot
          className={cn(
            "pointer-events-none absolute start-pane flex flex-wrap items-center gap-x-base gap-y-in text-caption text-tertiary tabular-nums",
            narrow ? "end-pane bottom-[calc(var(--glass-btn)+3*var(--gap-in)+var(--safe-bottom))]" : "bottom-pane",
          )}
        >
          <span data-dial-count>
            {gui.dial_count(
              devices.length,
              devices.filter((d) => {
                const l = firstHand(d, devices);
                return d.me || (l.road !== "idle" && band(l.ms) !== "none");
              }).length,
            )}
          </span>
          {(["fast", "mid", "slow", "none"] as const).map((b) => (
            <span key={b} className="inline-flex items-center gap-xxs">
              <Light band={b} />
              {b === "fast" ? gui.dial_key_fast : b === "mid" ? gui.dial_key_mid : b === "slow" ? gui.dial_key_slow : gui.dial_key_none}
            </span>
          ))}
        </div>
        {said?.map((line) => (
          <p key={line} data-net-left className="absolute inset-x-pane top-in text-callout text-muted-foreground">
            {line}
          </p>
        ))}
        {/* **The card**: a popover beside the face on the wide face, the
            phone's sheet on the narrow one (UX-21: the scrim closes it, a
            pull closes it, Esc closes it). */}
        {!narrow && opened && card && (
          <>
            <div data-dial-scrim className="absolute inset-0 z-10" onPointerDown={close} onContextMenu={(e) => e.preventDefault()} />
            <Float at={opened.at}>{card}</Float>
          </>
        )}
      </section>
      {narrow && (
        <Dialog open={opened !== null && card !== null} onOpenChange={(o) => !o && close()}>
          <DialogContent
            form
            data-dial-sheet
            data-dial-card
            // Nothing in the sheet asks for the focus on opening: a sheet
            // of facts and acts, not a form (the first act lit as if
            // chosen was the auto-focus).
            onOpenAutoFocus={(e) => e.preventDefault()}
            className="flex flex-col !gap-0 p-menu-inset"
          >
            <DialogTitle className="sr-only">{openedRow?.shown ?? gui.my_khor}</DialogTitle>
            {card}
          </DialogContent>
        </Dialog>
      )}
      {/* **The phone's deck** (rule 1): ‹ · the pill with the name and the
          segment · 「…」 for the add menu. */}
      {narrow && (
        <div data-dial-deck className="pointer-events-none absolute inset-x-0 bottom-[var(--safe-bottom)] flex flex-col items-stretch">
          <div className="pointer-events-auto flex h-[calc(var(--glass-btn)+2*var(--gap-in))] flex-none items-center gap-in px-row">
            <GlassCircle aria-label={gui.back} data-more-back data-more-close data-dial-back onClick={onClose}>
              <IconBack />
            </GlassCircle>
            <div data-dial-pill className="glass flex h-glass-btn min-w-0 flex-1 items-center gap-in rounded-full ps-row pe-1">
              <span className="min-w-0 flex-1 truncate text-body font-title">{gui.my_khor}</span>
              {segment("data-dial-segment")}
            </div>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <GlassCircle aria-label={gui.rail_more} data-dial-more>
                  <IconMore />
                </GlassCircle>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" side="top" sideOffset={8} data-dial-menu className="min-w-nav-menu">
                {addMenu(DropdownMenuItem, DropdownMenuSeparator)}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      )}

      {/* The flows the card opens: every one of them exists already. */}
      <MachineEditDialog row={editing} onClose={() => setEditing(null)} onChanged={onChanged} />
      <RootWordsDialog open={showingWords} onClose={() => setShowingWords(false)} />
      <RelayAddDialog open={addingRelay} onClose={() => setAddingRelay(false)} onWritten={setRelaysKnown} />
      <DeleteDialog pending={forgetting} onDone={() => setForgetting(null)} />

      <Dialog open={leaving} onOpenChange={setLeaving}>
        <DialogContent data-net-leave-dialog>
          <DialogHeader>
            <DialogTitle>{gui.identity_leave_do}</DialogTitle>
          </DialogHeader>
          {/* Not red like the reset's: leaving is undone by a ticket. */}
          <p data-net-leave-warn className="text-callout text-muted-foreground">
            {gui.identity_leave_warn}
          </p>
          <Input
            data-net-leave-name
            autoComplete="off"
            aria-label={gui.identity_leave_type_name}
            placeholder={gui.identity_leave_type_name}
            value={typedName}
            onChange={(e) => setTypedName(e.target.value)}
            className="min-h-touch w-full"
          />
          {refused && (
            <p data-net-leave-refused className="text-callout text-destructive">
              {refused}
            </p>
          )}
          <DialogFooter>
            <Button onClick={() => setLeaving(false)}>{gui.delete_keep}</Button>
            <Button
              variant="destructive"
              data-net-leave-do
              // Always pressable (`components/Why`); the node checks the
              // name too (`Node::leave`).
              onClick={() => {
                if (typedName.trim() !== (me?.shown ?? "")) {
                  setRefused(gui.identity_leave_type_name);
                  document.querySelector<HTMLElement>("[data-net-leave-name]")?.focus();
                  return;
                }
                setRefused(null);
                leaveNetwork(typedName).then(
                  (left) => {
                    setTypedName("");
                    setLeaving(false);
                    setSaid([
                      msg.left,
                      ...(left.told.length ? [msg.left_told(left.told.join(cli.name_separator))] : []),
                      ...(left.untold.length ? [msg.left_untold(left.untold.join(cli.name_separator), me?.shown ?? "")] : []),
                    ]);
                    onChanged();
                  },
                  (e: unknown) => setRefused(String(e instanceof Error ? e.message : e)),
                );
              }}
            >
              {gui.identity_leave_do}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={resetting} onOpenChange={setResetting}>
        <DialogContent data-reset-dialog>
          <DialogHeader>
            <DialogTitle>{gui.reset_title(me?.shown ?? "")}</DialogTitle>
          </DialogHeader>
          {/* Red: the one act here that cannot be undone. */}
          <p data-reset-warn className="text-callout text-destructive">
            {gui.reset_warn}
          </p>
          <Input
            data-reset-name
            autoComplete="off"
            aria-label={gui.reset_type_name}
            placeholder={gui.reset_type_name}
            value={typedName}
            onChange={(e) => setTypedName(e.target.value)}
            className="min-h-touch w-full"
          />
          {refused && (
            <p data-reset-refused className="text-callout text-destructive">
              {refused}
            </p>
          )}
          <DialogFooter>
            <Button onClick={() => setResetting(false)}>{gui.delete_keep}</Button>
            <Button
              variant="destructive"
              data-reset-do
              onClick={() => {
                if (typedName.trim() !== (me?.shown ?? "")) {
                  setRefused(gui.reset_type_name);
                  document.querySelector<HTMLElement>("[data-reset-name]")?.focus();
                  return;
                }
                setRefused(null);
                resetIdentity(typedName).then(
                  () => {
                    setResetting(false);
                    onReset();
                  },
                  (e: unknown) => setRefused(String(e instanceof Error ? e.message : e)),
                );
              }}
            >
              {gui.reset_do}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

/* ── the dial ─────────────────────────────────────────────────── */

type Placed = { row: DeviceRow; x: number; y: number; r: number; ang: number; link: Link; band: Band };

function Dial({
  me,
  others,
  rows,
  narrow,
  coarse,
  angles,
  looked,
  opened,
  onLook,
  onOpen,
  onTurn,
  onTurned,
}: {
  me: DeviceRow | null;
  others: DeviceRow[];
  rows: DeviceRow[];
  narrow: boolean;
  coarse: boolean;
  angles: Record<string, number>;
  looked: string | null;
  opened: string | null;
  onLook: (id: string | null) => void;
  onOpen: (id: string, at: DOMRect) => void;
  onTurn: (id: string, angle: number) => void;
  onTurned: () => void;
}) {
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [seat, setSeat] = useState(() => (typeof document === "undefined" ? { seat: 40, me: 72, halo: 22, touch: 44 } : seatSizes()));
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const read = () => {
      const r = el.getBoundingClientRect();
      setSize((was) => (was.w === r.width && was.h === r.height ? was : { w: r.width, h: r.height }));
      setSeat(seatSizes());
    };
    read();
    const watch = new ResizeObserver(read);
    watch.observe(el);
    return () => watch.disconnect();
  }, [narrow]);

  const f = frame(size.w, size.h, narrow, seat);
  const placed: Placed[] = useMemo(() => {
    const spreadAngles = spread(
      others.map((o) => o.id),
      angles,
    );
    return others.map((row) => {
      const link = firstHand(row, rows);
      const r = radiusOf(link, f.r0, f.r1);
      const ang = spreadAngles.get(row.id) ?? 0;
      return { row, x: f.cx + Math.cos(ang) * r, y: f.cy + Math.sin(ang) * r, r, ang, link, band: band(link.road === "none" ? null : link.ms) };
    });
  }, [others, rows, angles, f.cx, f.cy, f.r0, f.r1]);
  const at = (id: string): { x: number; y: number; half: number } | null => {
    if (me && id === me.id) return { x: f.cx, y: f.cy, half: seat.me / 2 };
    const p = placed.find((q) => q.row.id === id);
    return p ? { x: p.x, y: p.y, half: seat.seat / 2 } : null;
  };

  // Every pair once: this machine's roads show at rest, the rest when
  // looked at. What is not known between two other machines is not drawn.
  const edges = useMemo(() => {
    const all = me ? [me, ...others] : others;
    const out: { a: DeviceRow; b: DeviceRow; link: Link; far: boolean }[] = [];
    for (let i = 0; i < all.length; i += 1)
      for (let j = i + 1; j < all.length; j += 1) {
        const a = all[i]!;
        const b = all[j]!;
        const link = linkOf(a, b, rows);
        if (!link) continue;
        out.push({ a, b, link, far: !(a.me || b.me) });
      }
    return out;
  }, [me, others, rows]);

  // ── press, drag, look ──
  const drag = useRef<{ id: string; sx: number; sy: number; went: boolean; el: HTMLElement } | null>(null);
  const hold = useRef<number | null>(null);
  const clearHold = () => {
    if (hold.current !== null) {
      window.clearTimeout(hold.current);
      hold.current = null;
    }
  };
  const faceRect = (el: HTMLElement) => el.querySelector<HTMLElement>("[data-dial-face]")?.getBoundingClientRect() ?? el.getBoundingClientRect();
  const onDown = (e: ReactPointerEvent<HTMLDivElement>, id: string, isMe: boolean) => {
    if (e.button === 2) return;
    // The ground's long-press (the add menu) must not start under a seat.
    e.stopPropagation();
    const el = e.currentTarget;
    drag.current = { id, sx: e.clientX, sy: e.clientY, went: false, el };
    try {
      el.setPointerCapture(e.pointerId);
    } catch {
      // A synthetic pointer (a walk's dispatched event) has no capture to take.
    }
    clearHold();
    if (e.pointerType === "touch") {
      hold.current = window.setTimeout(() => {
        hold.current = null;
        drag.current = null;
        onOpen(id, faceRect(el));
      }, 480);
    }
    void isMe;
  };
  const onMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || (me && d.id === me.id)) return;
    const dx = e.clientX - d.sx;
    const dy = e.clientY - d.sy;
    if (!d.went && Math.hypot(dx, dy) < 6) return;
    clearHold();
    d.went = true;
    d.el.setAttribute("data-dragging", "");
    const r = box.current?.getBoundingClientRect();
    if (!r) return;
    onTurn(d.id, angleTo(f.cx, f.cy, e.clientX - r.left, e.clientY - r.top));
  };
  const onUp = (e: ReactPointerEvent<HTMLDivElement>) => {
    clearHold();
    const d = drag.current;
    if (!d) return;
    drag.current = null;
    d.el.removeAttribute("data-dragging");
    if (d.went) {
      onTurned();
      return;
    }
    // A tap looks (and a second tap on the same seat clears the look); a
    // click opens the card, as the right-click does.
    if (e.pointerType === "touch") onLook(looked === d.id ? null : d.id);
    else onOpen(d.id, faceRect(d.el));
  };
  const onCancel = () => {
    clearHold();
    const d = drag.current;
    drag.current = null;
    d?.el.removeAttribute("data-dragging");
  };

  // The open card's seat is the lit one; a hover elsewhere waits.
  const lit = opened ?? looked;
  const seatOf = (row: DeviceRow, p: Placed | null) => {
    const isMe = row.me;
    const half = isMe ? seat.me / 2 : seat.seat / 2;
    // The box a finger aims at is the finger floor (`--touch`) at least;
    // the face sits centred in it and keeps its own size, as every seat
    // on a map does (`MandalaMap`'s rule).
    const hit = Math.max(half, seat.touch / 2);
    const x = isMe ? f.cx : p!.x;
    const y = isMe ? f.cy : p!.y;
    // 不通 is the rim and a grey face; an idle seat (round NET-13) sits by
    // its last reading, its light at half ink, its face as any other.
    const idle = !isMe && p!.link.road === "idle";
    const off = !isMe && !idle && p!.band === "none";
    const lab = isMe ? { x: 0, y: half + 8, side: "c" as const, vert: "b" as const } : labelOffset(p!.ang, half + 8, narrow);
    const model = row.vitals?.vitals.model?.name ?? null;
    const seen = off || idle ? lastSeenMs(row) : null;
    return (
      <div
        key={row.id}
        data-dial-seat={row.id}
        data-me={isMe || undefined}
        data-off={off || undefined}
        data-idle={idle || undefined}
        data-on={lit === row.id || undefined}
        data-lat={isMe ? undefined : p!.band}
        data-angle={isMe ? undefined : p!.ang.toFixed(3)}
        data-radius={isMe ? undefined : Math.round(p!.r)}
        role="button"
        tabIndex={0}
        aria-label={isMe ? row.shown : gui.dial_look(row.shown)}
        className="absolute touch-none rounded-full select-none outline-none focus-visible:ring-2 focus-visible:ring-ring"
        style={{ left: x - hit, top: y - hit, width: hit * 2, height: hit * 2 }}
        onPointerDown={(e) => onDown(e, row.id, isMe)}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={onCancel}
        onPointerEnter={() => {
          if (!coarse && !drag.current) onLook(row.id);
        }}
        onPointerLeave={() => {
          if (!coarse && !drag.current && !opened) onLook(null);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onOpen(row.id, faceRect(e.currentTarget));
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onOpen(row.id, faceRect(e.currentTarget));
          }
        }}
      >
        <div data-dial-face className="absolute rounded-full" style={{ left: hit - half, top: hit - half, width: half * 2, height: half * 2 }}>
          {/* The one face assembly (UX-32): the machine's face with its
              system mark drawn the way the 外观 setting says — on the
              face, on the corner, or the mark as the base. */}
          <MachineFace face={row.face} system={row.vitals?.vitals.system ?? null} size={isMe ? "dialMe" : "dial"} />
        </div>
        <div
          data-dial-label
          data-side={lab.side}
          className={cn(
            "absolute flex flex-col whitespace-nowrap rounded-xs bg-card px-1 py-px",
            lab.side === "r" ? "items-start" : lab.side === "l" ? "items-end" : "items-center",
          )}
          style={{
            left: hit + lab.x,
            top: hit + lab.y,
            transform: `translate(${lab.side === "r" ? "0" : lab.side === "l" ? "-100%" : "-50%"}, ${lab.vert === "b" ? "0" : lab.vert === "t" ? "-100%" : "-50%"})`,
          }}
        >
          <span data-dial-name className={cn("leading-tight", isMe ? "text-body font-title" : "text-callout")}>
            {row.shown}
          </span>
          <span data-dial-aside className="flex items-center gap-xxs text-caption leading-tight text-tertiary">
            {isMe ? (
              gui.machine_here
            ) : (
              <>
                <Light band={p!.band} half={idle} />
                {off ? (seen === null ? gui.day_unreachable : gui.card_age(span(seen))) : idle ? (seen === null ? hop.quiet : gui.card_age(span(seen))) : (model ?? "")}
              </>
            )}
          </span>
        </div>
      </div>
    );
  };

  return (
    <div
      ref={box}
      data-dial
      data-look={lit !== null ? lit : undefined}
      className="absolute inset-0 touch-none"
      onPointerDown={(e) => {
        // A touch on the ground clears the look (a mouse's leaves it).
        if (e.pointerType === "touch" && !(e.target as HTMLElement).closest("[data-dial-seat]")) onLook(null);
      }}
    >
      <svg aria-hidden="true" className="absolute inset-0 size-full overflow-visible">
        {size.w > 0 && me && (
          <>
            {/* The halo under this machine: the one olive wash on the page. */}
            <circle data-dial-halo cx={f.cx} cy={f.cy} r={seat.me / 2 + seat.halo} className="fill-brand-halo" />
            {[
              [30, gui.dial_ring_fast],
              [120, gui.dial_ring_mid],
            ].map(([ms, word]) => {
              const r = radiusFor(ms as number, f.r0, f.r1);
              return (
                <g key={ms}>
                  <circle data-dial-ring={ms} cx={f.cx} cy={f.cy} r={r} />
                  <text data-dial-word x={f.cx} y={f.cy - r + 4} textAnchor="middle" className="fill-tertiary text-caption">
                    {word}
                  </text>
                </g>
              );
            })}
            <circle data-dial-ring="rim" data-rim="" cx={f.cx} cy={f.cy} r={f.r1} />
            <text data-dial-word x={f.cx} y={f.cy - f.r1 + 4} textAnchor="middle" className="fill-tertiary text-caption">
              {gui.dial_key_none}
            </text>
            {edges.map(({ a, b, link, far }) => {
              const pa = at(a.id);
              const pb = at(b.id);
              if (!pa || !pb) return null;
              const c = clip(pa, pb, pa.half, pb.half);
              const bd = band(link.road === "none" ? null : link.ms);
              const key = `${a.id}|${b.id}`;
              const on = lit !== null && (a.id === lit || b.id === lit);
              const mx = (c.x1 + c.x2) / 2;
              const my = (c.y1 + c.y2) / 2;
              return (
                <g key={key}>
                  <line
                    data-dial-edge={key}
                    data-lat={bd}
                    data-road={link.road}
                    data-far={far || undefined}
                    data-lit={on || undefined}
                    x1={c.x1}
                    y1={c.y1}
                    x2={c.x2}
                    y2={c.y2}
                    style={{ stroke: BAND_INK[bd] }}
                  />
                  {link.road === "none" ? (
                    <text
                      data-dial-x={key}
                      data-dial-word
                      data-far={far || undefined}
                      data-lit={on || undefined}
                      x={mx}
                      y={my + 4}
                      textAnchor="middle"
                      className="fill-tertiary text-body font-title"
                    >
                      ×
                    </text>
                  ) : (
                    link.ms !== null && (
                      <text
                        data-dial-ms={key}
                        data-dial-word
                        data-idle={link.road === "idle" || undefined}
                        data-lit={on || undefined}
                        x={mx}
                        y={my + 4}
                        textAnchor="middle"
                        className="text-caption tabular-nums"
                        style={{ fill: BAND_INK[bd] }}
                      >
                        {gui.row_rtt(link.ms)}
                      </text>
                    )
                  )}
                </g>
              );
            })}
          </>
        )}
      </svg>
      {size.w > 0 && me && seatOf(me, null)}
      {size.w > 0 && me && placed.map((p) => seatOf(p.row, p))}
    </div>
  );
}

/** A 7 pt light in a band's ink — beside a number or a word, never alone. */
function Light({ band: b, half = false }: { band: Band; half?: boolean }) {
  return (
    <i
      data-dial-light={b}
      data-half={half || undefined}
      aria-hidden="true"
      className={cn("inline-block size-1.5 flex-none rounded-full", half && "opacity-50")}
      style={{ background: BAND_INK[b] }}
    />
  );
}

/* ── the card ─────────────────────────────────────────────────── */

/** The popover beside a face on the wide face: placed to the right of
    the face, or to its left where the right has no room, and kept
    inside the page. */
function Float({ at, children }: { at: DOMRect | null; children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    const host = el?.parentElement;
    if (!el || !host) return;
    const h = host.getBoundingClientRect();
    const bw = el.offsetWidth;
    const bh = el.offsetHeight;
    const pad = 10;
    let x: number;
    let y: number;
    if (at && at.width > 0) {
      x = at.right - h.left + 12;
      y = at.top - h.top + at.height / 2 - bh / 2;
      if (x + bw > h.width - pad) x = at.left - h.left - bw - 12;
    } else {
      x = (h.width - bw) / 2;
      y = (h.height - bh) / 2;
    }
    x = Math.min(Math.max(x, pad), Math.max(pad, h.width - bw - pad));
    y = Math.min(Math.max(y, pad), Math.max(pad, h.height - bh - pad));
    setPos((was) => (was && was.left === x && was.top === y ? was : { left: x, top: y }));
  }, [at, children]);
  return (
    <div
      ref={ref}
      data-dial-card
      role="dialog"
      className={cn(
        "glass absolute z-20 w-[calc(var(--nav-menu)+2*var(--gap-base))] rounded-menu bg-glass-menu p-menu-inset text-foreground shadow-[var(--glass-in),var(--glass-shadow)]",
        "animate-in fade-in-0 zoom-in-95 duration-[var(--dur-180)] ease-[var(--ease-out)]",
        pos === null && "invisible",
      )}
      style={pos ?? undefined}
    >
      {children}
    </div>
  );
}

function CardHead({ row, aside, sub }: { row: DeviceRow; aside?: string | null; sub: ReactNode }) {
  return (
    <div data-dial-card-head className="flex items-center gap-row px-row pt-in pb-row">
      <MachineAvatar face={row.face} className="size-list-face flex-none" />
      <div className="min-w-0 flex-1">
        <div className="truncate text-body font-title">
          <span data-me-name={row.me || undefined}>{row.shown}</span>
          {aside && <span className="font-body text-muted-foreground"> {aside}</span>}
        </div>
        <div className="flex items-center gap-xxs text-callout text-muted-foreground tabular-nums">{sub}</div>
      </div>
    </div>
  );
}

function Fact({ name, label, children }: { name: string; label: string; children: ReactNode }) {
  return (
    <div data-dial-fact={name} className="contents">
      <dt className="text-tertiary">{label}</dt>
      <dd data-more-fact={name} className="min-w-0 truncate text-foreground">
        {children}
      </dd>
    </div>
  );
}

const FACTS = "grid grid-cols-[auto_1fr] gap-x-row gap-y-xxs px-row pb-in text-callout tabular-nums";

function Sep() {
  return <div className="mx-row my-xxs h-[var(--hair)] bg-border" />;
}

function Item({
  mark,
  label,
  k,
  destructive,
  onPress,
}: {
  mark: Record<string, string | boolean | undefined>;
  label: string;
  /** What stands at the item's end: a count, a ›, a shortcut. */
  k?: string;
  destructive?: boolean;
  onPress: () => void;
}) {
  return (
    <button
      type="button"
      {...mark}
      data-dial-item
      onClick={onPress}
      className={cn(
        "flex min-h-menu-item w-full cursor-default items-center gap-row rounded-menu-item px-row text-start text-body outline-none hover:glass-sel focus-visible:glass-sel",
        destructive && "text-destructive",
      )}
    >
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {k && <span className="flex-none text-callout text-tertiary tabular-nums">{k}</span>}
    </button>
  );
}

function MachineCard({
  row,
  rows,
  sessions,
  workspaces,
  coarse,
  onNewSession,
  onSessions,
  onWorkspaces,
  onBlocked,
  onRepair,
  onEdit,
  onPin,
  onForget,
}: {
  row: DeviceRow;
  rows: DeviceRow[];
  sessions: { home: string; word: string }[];
  workspaces: number;
  coarse: boolean;
  onNewSession: () => void;
  onSessions: () => void;
  onWorkspaces: () => void;
  onBlocked: () => void;
  onRepair: () => void;
  onEdit: () => void;
  onPin: () => void;
  onForget: () => void;
}) {
  const link = firstHand(row, rows);
  const b = band(link.road === "none" ? null : link.ms);
  const idle = link.road === "idle";
  const off = !idle && b === "none";
  const seen = lastSeenMs(row);
  const mine = sessions.filter((s) => s.home === row.name);
  const busy = mine.filter((s) => s.word === "busy").length;
  const blocked = mine.filter((s) => s.word === "blocked").length;
  const model = row.vitals?.vitals.model?.name ?? null;
  const others = rows.filter((o) => !o.me && o.id !== row.id);
  return (
    <>
      <CardHead
        row={row}
        aside={model}
        sub={
          off ? (
            <span data-dial-last-answer>{seen === null ? gui.day_unreachable : gui.dial_last_answer(span(seen))}</span>
          ) : (
            <>
              <Light band={b} half={idle} />
              <span data-dial-card-road>{link.ms === null ? wayWord(link) : `${gui.row_rtt(link.ms)} · ${wayWord(link)}`}</span>
            </>
          )
        }
      />
      <dl className={FACTS}>
        {others.map((o) => (
          <Fact key={o.id} name={`to-${o.id}`} label={gui.dial_to(o.shown)}>
            {roadWord(linkOf(row, o, rows))}
          </Fact>
        ))}
        <Fact name="sessions" label={config.session}>
          {mine.length === 0 ? gui.dial_none : busy > 0 ? gui.dial_some_busy(mine.length, busy) : gui.dial_some(mine.length)}
        </Fact>
      </dl>
      <Sep />
      {!off ? (
        <>
          {row.hosts && <Item mark={{ "data-dial-act": "new-terminal" }} label={gui.dial_new_terminal} k={coarse ? undefined : "⌘T"} onPress={onNewSession} />}
          <Item mark={{ "data-dial-act": "sessions" }} label={config.session} k={`${gui.dial_some(mine.length)} ›`} onPress={onSessions} />
          <Item mark={{ "data-dial-act": "workspaces" }} label={config.workspace} k={`${gui.dial_some(workspaces)} ›`} onPress={onWorkspaces} />
          {blocked > 0 && <Item mark={{ "data-dial-act": "blocked" }} label={gui.pending_title} k={`${gui.dial_some(blocked)} ›`} onPress={onBlocked} />}
        </>
      ) : (
        <>
          <Item mark={{ "data-dial-act": "sessions" }} label={config.session} k={`${gui.dial_some(mine.length)} ›`} onPress={onSessions} />
          <Item mark={{ "data-dial-act": "repair" }} label={gui.dial_repair} onPress={onRepair} />
        </>
      )}
      <Sep />
      <Item mark={{ "data-dial-act": "edit" }} label={gui.dial_rename} onPress={onEdit} />
      <Item mark={{ "data-dial-act": row.pinned ? "unpin" : "pin" }} label={row.pinned ? gui.unpin : gui.pin} onPress={onPin} />
      <Sep />
      <Item mark={{ "data-dial-act": "forget" }} label={gui.dial_forget} destructive onPress={onForget} />
    </>
  );
}

function MeCard({
  me,
  about,
  relays,
  holdsRoot,
  alone,
  onEdit,
  onWords,
  onRelays,
  onLeave,
  onReset,
}: {
  me: DeviceRow;
  about: About | null;
  relays: RelayChoices | null;
  holdsRoot: boolean;
  alone: boolean;
  onEdit: () => void;
  onWords: () => void;
  onRelays: () => void;
  onLeave: () => void;
  onReset: () => void;
}) {
  const hereKey = inApp ? "app" : "web";
  const faces = about?.faces ?? [];
  const up = faces.filter((f) => f.up).length;
  return (
    <>
      <CardHead row={me} sub={<span>{holdsRoot ? `${gui.machine_here} · ${gui.dial_holds_root}` : gui.machine_here}</span>} />
      <dl className={FACTS}>
        <Fact name="id" label={gui.identity_id}>
          {/* Twelve characters to recognise; pressing copies all 64. */}
          <CopyId id={me.id} face="fact" />
        </Fact>
        <Fact name="version" label={gui.identity_version}>
          {about?.version ?? ""}
        </Fact>
        {/* The faces this machine has open, and which one this is (a
            phone has neither, and no rows). */}
        {faces.length > 0 && (
          <div className="contents">
            <dt className="text-tertiary">{gui.dial_faces}</dt>
            <dd data-dial-faces className="flex min-w-0 flex-col text-foreground">
              <span>{gui.dial_faces_open(up)}</span>
              {faces.map((f) => (
                <span key={f.key} data-iface={f.key} data-on={f.up} className="flex min-w-0 items-baseline gap-in">
                  <span className="min-w-0 truncate">{f.name}</span>
                  {f.key === hereKey && f.up && (
                    <span data-iface-here className="flex-none text-muted-foreground">
                      {cli.face_current}
                    </span>
                  )}
                  <span data-iface-said className={cn("ms-auto flex-none", !f.up && "text-muted-foreground")}>
                    {f.said}
                  </span>
                </span>
              ))}
            </dd>
          </div>
        )}
      </dl>
      <Sep />
      <Item mark={{ "data-dial-act": "edit" }} label={gui.dial_rename} onPress={onEdit} />
      {holdsRoot && (
        <Item mark={{ "data-dial-act": "root-words", "data-root-words-open": "", "data-setting-row-name": "root-words" }} label={`${gui.root_words_title}…`} onPress={onWords} />
      )}
      <Item mark={{ "data-dial-act": "relays" }} label={gui.dial_relays} k={relays ? `${gui.dial_some(relays.in_use.length)} ›` : "›"} onPress={onRelays} />
      <Sep />
      {/* Not on a machine in no network: `Node::leave` refuses there. */}
      {!alone && <Item mark={{ "data-dial-act": "leave", "data-net-leave": "" }} label={gui.identity_leave_ask} destructive onPress={onLeave} />}
      <Item mark={{ "data-dial-act": "reset", "data-reset-open": "" }} label={gui.dial_reset} destructive onPress={onReset} />
    </>
  );
}

/* ── the matrix ───────────────────────────────────────────────── */

function Matrix({
  me,
  others,
  rows,
  narrow,
  onOpen,
}: {
  me: DeviceRow | null;
  others: DeviceRow[];
  rows: DeviceRow[];
  narrow: boolean;
  onOpen: (id: string, at: DOMRect) => void;
}) {
  const all = me ? [me, ...others] : others;
  // A cell is at least two touch widths wide, so a number and a relay's
  // name fit; the grid takes the width it can up to the detail cap.
  const grid: CSSProperties = { gridTemplateColumns: `auto repeat(${all.length}, minmax(calc(2 * var(--touch)), 1fr))` };
  return (
    <div
      data-dial-matrix
      className={cn(
        "absolute inset-0 flex justify-center overflow-auto px-pane pt-pane pb-[calc(var(--glass-btn)+4*var(--gap-in)+var(--safe-bottom))]",
        narrow ? "items-start" : "items-center",
      )}
    >
      <div className="flex w-full min-w-0 max-w-detail flex-col gap-in">
        <div data-dial-matrix-title className="text-callout text-muted-foreground">
          {gui.dial_matrix}
        </div>
        <div className="grid items-center gap-xxs" style={grid}>
          <div />
          {all.map((n) => (
            <div key={n.id} data-matrix-head={n.id} className="flex min-w-0 flex-col items-center gap-xxs pb-xxs text-caption text-muted-foreground">
              <MachineAvatar face={n.face} className="size-kind-mark flex-none" />
              <span className="max-w-full truncate">{n.shown}</span>
            </div>
          ))}
          {all.map((a) => (
            <div key={a.id} className="contents">
              <div data-matrix-row={a.id} className="flex min-w-0 items-center gap-in pe-in text-callout text-muted-foreground">
                <MachineAvatar face={a.face} className="size-kind-mark flex-none" />
                <span className="min-w-0 truncate">{a.shown}</span>
              </div>
              {all.map((b) => {
                if (a.id === b.id) return <div key={b.id} data-matrix-cell data-self="" className="h-touch rounded-sm" />;
                const l = cellOf(a, b, rows);
                const bd = l === null ? null : band(l.road === "none" ? null : l.ms);
                return (
                  <div
                    key={b.id}
                    data-matrix-cell={`${a.id}>${b.id}`}
                    data-lat={bd ?? undefined}
                    data-idle={l?.road === "idle" || undefined}
                    data-unknown={l === null || undefined}
                    title={`${a.shown} → ${b.shown} · ${roadWord(l)}（${gui.dial_reported_by(a.shown)}）`}
                    className="flex h-touch min-w-0 flex-col items-center justify-center rounded-sm bg-fill text-callout leading-tight tabular-nums"
                    onContextMenu={(e) => {
                      e.preventDefault();
                      onOpen(b.id, e.currentTarget.getBoundingClientRect());
                    }}
                  >
                    {l === null ? (
                      <span className="text-tertiary">{gui.filter_unknown}</span>
                    ) : l.road === "none" ? (
                      <span>{gui.dial_key_none}</span>
                    ) : (
                      <>
                        <span>{l.ms === null ? "" : gui.row_rtt(l.ms)}</span>
                        <small className="max-w-full truncate text-caption opacity-75">
                          {l.road === "idle" ? hop.quiet : l.road === "direct" ? hop.direct : (l.relay ?? hop.relay)}
                        </small>
                      </>
                    )}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
