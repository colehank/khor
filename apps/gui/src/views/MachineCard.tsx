import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore, type CSSProperties, type ReactNode } from "react";
import { createPortal } from "react-dom";

import type { About, DeviceRow, HooksState, Load, Strain, WatchedDir } from "@/api";
import { fetchAbout, fetchLoad, inApp, setWatch } from "@/api";
import { CodeButton, type CodeWindow } from "@/components/CodeWindow";
import { DetailFrame } from "@/components/DetailFrame";
import { MachineFace } from "@/components/SessionFace";
import { PageAvatar, PageSubtitle } from "@/components/Region";
import { RegionState } from "@/components/RegionState";
import { SectionHead } from "@/components/SectionHead";
import { Bar } from "@/components/Bar";
import { Face, Glyph, type Pip } from "@/components/Face";
import { DIAL_CARD_SIZE, DIAL_CARD_STROKE, hugRatio } from "@/components/Face";
/* ⚠️ **Tombstone: this card imported `RING_SIZE_CARD` / `RING_STROKE_CARD`
   until 2026-09-09.** They were the size its dials were drawn at, then
   the reference `dialFor` scaled from — and both are gone from here,
   because the user's ruling is that these dials are **the 額度 card's
   dials**: same size, same stroke, same two constants
   (`DIAL_CARD_SIZE` / `DIAL_CARD_STROKE` in `components/Face`).
   `components/Ring` still owns those two for the ring it draws; what
   ended is this card reading them. */
import { AxisLabel, PlotSettings } from "@/components/PlotControls";
import { PLOT, dotRadius, dotSpan, everyRowHas, labelSeat, tickFractions, usePlotBox, xTickY } from "@/lib/plot";
import { IconBack, IconBrowser, IconChevronDown, IconChevronUp, IconCode, IconFiles, IconMore, IconPlus, IconSessions } from "@/components/icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { GlassCapsule, GlassCircle, GlassItem } from "@/components/Glass";
import { useNarrow } from "@/hooks/use-narrow";
import { NarrowHead } from "@/components/SideHead";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PopupButton } from "@/components/ui/popup-button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cli, gui, msg } from "@/gen/catalog";
import { fontOf, measurer } from "@/components/PathCut";
import { tickSet } from "@/lib/ruler";
import { useKept } from "@/lib/kept";
import { useTravel } from "@/lib/travel";
import { cn } from "@/lib/utils";
import { mount as mountDay } from "@/day/day-core.js";
import type { DayPanel, DayWords } from "@/day/day-core.js";
import "@/day/day.css";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useHiddenRows, useOverflows, useTight } from "@/hooks/use-tight";
import { pagesHere } from "@/lib/here";
import { CopyId } from "@/views/DetailPane";
import { ago, lasting, linkWord, systemWord, count } from "@/words";

/**
 * One machine, opened from the devices pane.
 *
 * **This is the destination the machine rows were waiting for.** The
 * previous batch left those rows inert on a stated judgment — no
 * affordance may suggest a destination that does not exist — and the
 * judgment has not changed, only its input: there is a destination now,
 * so on *this* pane the row becomes a button. The files and browser
 * panes still list machines with nowhere to send you, and their rows
 * stay exactly as they were until their own batches give them one.
 *
 * No title here (components/Toolbar「身份只出现一次」): the machine's
 * name is R0's, with its row's avatar and its model. The board opens on
 * **one strip of facts** (`Strip`, 2026-10-01) — no face, no name — and
 * the readings follow.
 */
export function MachineCard({
  row,
  narrow,
  onBack,
  hooks,
  hooksFailed,
  onInstallHooks,
  onUninstallHooks,
  actions,
  body,
}: {
  row: DeviceRow | null;
  narrow: boolean;
  /** **What the head's switches put in place of the dashboard** (the
      redesign, 2026-10-02): the machine's sessions or its files, drawn by
      the shell, which holds the rows and the switches. Absent, the
      dashboard. */
  body?: ReactNode;
  onBack: () => void;
  /** This machine's hooks, or `null` until the first answer is in. */
  hooks: HooksState | null;
  hooksFailed: boolean;
  onInstallHooks: () => void;
  onUninstallHooks: () => void;
  /** The page's own buttons on the narrow face, which has no R0 over a
      detail screen: they sit at the right end of the back bar, the same
      place R0 puts them wide (第十七轮 §3). */
  actions?: ReactNode;
}) {
  return (
    <section data-machine-card className="flex h-full min-w-0 flex-col">
      {/* **A strip only where something has to live in it.** Back exists
          on the narrow face, where the list really is off-screen; wide
          keeps the list beside this, so there is nothing to go back
          from. ⚠️ Rewritten 2026-09-27: the strip carried no name while
          the card's first line was the name; that line is gone (the
          name is R0's, said once — components/Toolbar), so on narrow,
          where there is no R0, this strip is where it is said. */}
      {narrow && (
        // **`min-h`, not `h`** — the same move the session detail's bar
        // made, for the same reason: the bar's look is still `--ctl-lg`,
        // but a 44 target inside a fixed 40 would light up a band that
        // is cut off, and the floor's own rule is that what paints the
        // pressed state paints the grown box (tokens.css `--touch`).
        // Seated in the window's first row (`NarrowHead`, 2026-09-28):
        // it was a second bar under L0's magnifier.
        <NarrowHead>
        {(seated) => (
        <header className={seated ? "flex min-w-0 flex-1 items-center gap-row self-stretch" : "flex min-h-ctl-lg flex-none items-center gap-row border-b px-3"}>
          {/* The canvas's machine head (the redesign, 2026-10-02): the way
              back as a glass circle, the machine's own face, its name
              over one line of facts, and its verbs in a glass capsule. */}
          <GlassCircle size="head" aria-label={gui.back} data-back onClick={onBack}>
            <IconBack />
          </GlassCircle>
          {row && <MachineFace face={row.face} system={row.vitals?.vitals.system ?? null} size="head" />}
          <span data-head-name className="flex min-w-0 flex-1 flex-col">
            <span className="min-w-0 truncate text-title-md font-title">{row?.shown ?? ""}</span>
            {row && <HeadModel row={row} yields />}
          </span>
          {actions}
        </header>
        )}
        </NarrowHead>
      )}
      {/* **R0: the row's avatar, the name, the model** (2026-10-01, the
          owner: 「机器页面的标题，改成第一行名称，第二行型号就行了，不写
          khor 版本和链接方式」; the avatar 2026-10-02). The shell says the
          name; this page writes the two things around it, seated in R0 by
          the shell like the workspace's location:
          - the avatar is **the one its list row wears** (`HeadAvatar`);
          - the second line is the **model** alone, and is not drawn when
            the machine did not report one (「读不到就空——不编」).
          ⚠️ 墓碑 (2026-09-30 晚 → 10-01): the second line was 「型号 ·
          khor 版本 · 连接」 (`MachineCaption`); the version and the road
          are the strip's, said there once. Nothing on the narrow face,
          which has no R0 — the strip says the model there, first. */}
      {row && <HeadAvatar row={row} />}
      {row && (
        <PageSubtitle>
          <HeadModel row={row} />
        </PageSubtitle>
      )}
      {row && body !== undefined ? (
        body
      ) : row && (
        // **Keyed by the machine**, so every piece of state on this card
        // belongs to the machine it was made on: a sort somebody set on
        // one machine's table is not an opinion about the next one's,
        // and neither is a turned-over dial or a scroll position.
        //
        // **And no `narrow` below this line.** Every part of the card
        // that changes shape measures its own box now — the drawer's
        // rows, the process table — because the card is not the window,
        // and a window breakpoint left a 106px overflow band the first
        // time it was tried (`use-tight.ts`). The strip above still
        // asks the shell: whether there is a way back is a fact about
        // the screen, not about a box on it.
        <Body
          key={row.id}
          row={row}
          // The head says the model at both widths now (the redesign's
          // narrow head has a second line), so the band does not.
          sayModel={false}
          hooks={hooks}
          hooksFailed={hooksFailed}
          onInstallHooks={onInstallHooks}
          onUninstallHooks={onUninstallHooks}
        />
      )}
    </section>
  );
}

/**
 * **R0's avatar over a machine: the very one its list row wears**
 * (2026-10-02, the owner through the lead: 「机器页页头的头像不要画裸的
 * 系统标记了，直接用列表那一行的同一个机器头像组件」). The same pieces
 * the machines' list hands its row (`DevicesList` → `SideList` →
 * `FaceMark`): the machine's face at `--avatar` 32 and its system
 * family's mark (`machineMark`, the plain machine glyph for one that could
 * not say) the way a row draws it — so a change to how a row draws a face
 * and its mark (「浮现」 / 「角标」) is a change here too, with nothing to
 * keep in step. 32 is the two title lines; `PageAvatar` seats it and takes
 * the face's hairline ring away, which a 1× screen draws in steps.
 *
 * ⚠️ 墓碑 (2026-10-01 → 10-02): the bare system mark, first at 18 on a
 * paper disc, then at 32 on nothing (「头像仅仅是类别头像哈，不是机器带
 * 角标的」).
 */
/** The head's second line: **the model alone**, and nothing when the
    machine did not report one (the owner, 2026-10-01: 「第二行型号就行
    了」). The khor version and the reading's age are the facts band's. */
function HeadModel({ row, yields = false }: { row: DeviceRow; yields?: boolean }) {
  const model = row.vitals?.vitals.model?.name;
  const [box, setBox] = useState<HTMLSpanElement | null>(null);
  const [gone, setGone] = useState(false);
  // **On the narrow head the model yields first** (P2, 2026-10-03, the
  // owner's ruling): where the line cannot be said whole it is not said
  // at all, rather than cut to 「Mac mini (2…」, and the name keeps the
  // row's room; the name then truncates to its floor, and only then do
  // the verbs fold (`MachineVerbs`). Measured against the name's column
  // (the parent), so a line that fits again comes back.
  useLayoutEffect(() => {
    if (!yields || !box || !model) return;
    const column = box.parentElement;
    if (!column) return;
    const fit = () => setGone(measurer(fontOf(box))(model) > column.clientWidth + 0.5);
    fit();
    const watch = new ResizeObserver(fit);
    watch.observe(column);
    return () => watch.disconnect();
  }, [yields, box, model]);
  if (!model) return null;
  return (
    <span
      ref={setBox}
      data-machine-head-sub
      data-yielded={gone || undefined}
      className={cn("min-w-0 truncate text-callout text-muted-foreground", gone && "invisible h-0")}
    >
      {model}
    </span>
  );
}

function HeadAvatar({ row }: { row: DeviceRow }) {
  const system = row.vitals?.vitals.system ?? null;
  // The list row's own avatar, drawn by the one component that knows the
  // two ways a kind is drawn (浮现 / 角标, `MachineFace`) — so the head
  // follows the Appearance setting with the row.
  return (
    <PageAvatar kind={system ?? "machine"}>
      <MachineFace face={row.face} system={system} size="head" />
    </PageAvatar>
  );
}

/** One machine's card, from its head to its hook. Split from the
    section above only so that `key` can reset every bit of state on it
    when the machine changes. */
function Body({
  row,
  sayModel,
  hooks,
  hooksFailed,
  onInstallHooks,
  onUninstallHooks,
}: {
  row: DeviceRow;
  /** The narrow face has no R0, so the model R0 says is the strip's to
      say there (`Strip`). A fact about the screen, like the way back. */
  sayModel: boolean;
  hooks: HooksState | null;
  hooksFailed: boolean;
  onInstallHooks: () => void;
  onUninstallHooks: () => void;
}) {
  const [sort, setSort] = useState<Sort>(SORT_FIRST);
  const { load, why, shown, asking, seen } = useLoad(row, sort);
  // **A region that has never been read shows its skeleton, not a blank
  // or half a card** (components/RegionState `loading-never`, the state
  // the lists already wear). Two holes it fills: a machine with no
  // reading in the device table and no process answer yet (the whole
  // card below the facts line), and the process table before its first
  // answer (its own slot). **Anything kept wins**: the readings come
  // with the device row and the process table from `useKept`, so a
  // machine seen before draws its last content on the first frame and
  // never this — the fast gate's second claim.
  // **A machine known to be out of reach is said so at once** (P6,
  // 2026-10-03): the device table already says 「未连接」, so the page
  // does not wait out the node's deadline behind skeletons. The asking
  // goes on quietly underneath (`useLoad` retries), and the first answer
  // that lands replaces the sentence.
  const far = isFar(row) && load === null;
  /** What the process table says when there is nothing to show: the
      machine by name, in the words a person reads, when the trouble is
      that it cannot be reached — the node's own sentence (its timeout and
      the paths it tried) is the hover's. Any other refusal (an older
      khor declining a column) is said as the node said it. */
  const unreachable =
    far || (why !== null && [row.name, row.shown].some((n) => why === msg.machine_unreachable_timeout(n)));
  const lost = unreachable ? gui.machine_cant_reach(row.shown) : why;
  const answered = load !== null || lost !== null;
  const never = !row.vitals && !answered;
  return (
        <div data-nav-clear className="min-h-0 flex-1 overflow-y-auto py-4">
          {/* The pane's width, capped and centred (`DetailFrame`, UX-22). */}
          <DetailFrame className="flex flex-col gap-row">
            <Strip row={row} sayModel={sayModel} />
            {never ? (
              <RegionState state="loading-never" />
            ) : (
              <>
                <Readings row={row} day={load?.cpu_day ?? null} dayLost={load === null && lost !== null} top={load?.top ?? []} />
                {/* **No process table for a machine that cannot host**
                    (docs/IOS.md 第二步): a phone's app may not see other
                    processes, so its table is always empty — and an empty
                    table reads as 「nothing running」, a claim about a
                    machine khor cannot look inside. */}
                {!row.hosts ? null : answered ? (
                  <Busy
                    load={load}
                    why={lost}
                    whyDetail={why}
                    sort={shown ?? sort}
                    asking={asking}
                    onSort={setSort}
                    seen={seen}
                  />
                ) : (
                  <RegionState state="loading-never" />
                )}
              </>
            )}
          {/* The subscription's arcs left this card on 2026-09-02
              (「机器页面不需要有token usage的环」) — the 额度 landing's
              foot is their one home now, where 额度 is read.
              `QuotaLanding`'s tombstone carries the whole road —
              `QuotaRings` itself went on 2026-09-06 when the band became
              a row. */}
          {/* **Only on this machine's card**, because the file it edits
              is on this machine. Putting it on every card would be a
              control that silently applies to one machine out of a list.
              Installing on a machine you are looking at from here
              belongs to the batch that adds remote verbs, and when it
              lands, this is where it goes. */}
          {/* …and only on a machine with a claude to hook: a phone has
              none, and its 「装好了」 was about a file nobody reads
              (docs/IOS.md 第二步; `install_hooks` refuses there too). */}
          {row.me && row.hosts && (
            <Hooks
              state={hooks}
              failed={hooksFailed}
              onInstall={onInstallHooks}
              onUninstall={onUninstallHooks}
            />
          )}
          </DetailFrame>
        </div>
  );
}

/**
 * **The machine's facts, one strip at the top of its board** (2026-10-01,
 * the owner looked at three directions and picked A, 「一条事实带」). Each
 * fact is a cell — a small label over its value — and the cells stand in
 * one row with a hairline between every two:
 *
 * ```
 * 系统    │ khor 版本 │ 连接 │ 已运行 │ 读数    │ 机器 id      │ 界面
 * macOS   │ 0.1.14    │ 直连 │ 125 天 │ 42 秒前 │ ffffffffffff │ 桌面 app · 网页
 * ─────────────────────────────────────────────────────────── ← hairline
 *   CPU  GPU  内存  磁盘 ……
 * ```
 *
 * **Who it is is R0's, said once**: the row's avatar, the name, the
 * model. The board has no face (R0 and the list row beside it wear it)
 * and no name; the strip is what the machine *is* and how it *is now*, the facts
 * you look up rather than read on arrival — so they take one line of
 * about forty pixels and the dials are the first look again.
 *
 * - **A cell**: the label in `caption` `tertiary`, the value in `body`
 *   `ink` (14 + 18 = 32 tall), numbers tabular, the id mono. Content
 *   starts at the cell's left; the hairline is the cell's left border,
 *   `gap-row` before its words.
 * - **The row is spread end to end** (`justify-content: space-between`
 *   over `auto` tracks): the first value starts at the board's padding
 *   and the last one ends at it, and what is left over goes between the
 *   cells, before each line. Chosen over equal columns because the cells
 *   are not equally wide (「直连」 beside twelve characters of id): equal
 *   tracks leave the short ones floating in air and fold the row sooner;
 *   spread, seven cells stay one line down to a strip of about 36rem, and
 *   folded, the tracks still line the rows up into columns.
 * - **It folds by the board, not the window** (`@container/strip`): one
 *   row from 36rem; two rows from 24rem; three below that. Every row
 *   holds the same count — `ceil(n / rows)`, so only the last can be
 *   short, by less than a row — in the same tracks, `gap-row` between
 *   rows. A row's first cell shows no line: the grid is pulled left by
 *   one line and its gap under a horizontal clip, so the first column's
 *   lines stand outside the strip.
 * - **The order**: 系统 · khor 版本 · 连接 · 已运行 · 读数 · 机器 id · 界面
 *   — what it is, then how it is now, then the long values last so a
 *   squeezed row cuts them rather than the short ones. The narrow face
 *   has no R0, so the model opens the strip there (`sayModel`).
 *
 * **Every cell is a fact the page already has**, and a fact with nothing
 * honest to say is no cell — never a dash, never a guess, never an empty
 * slot (「读不到就空——不编」):
 *
 * - 系统 — the family the list row's corner and R0's mark draw
 *   (`vitals.system`);
 * - khor 版本 — `vitals.version`, the khor that machine runs;
 * - 连接 — `connectionWord`, the word the list row prints: 本机, the road
 *   (`hop`), or 「经 <sponsor>」;
 * - 已运行 — `vitals.uptime_s`, nothing for 0 (an older node);
 * - 读数 — how old the reading is: an age for a machine reached over the
 *   wire, the clock of its taking for this one;
 * - 机器 id — the row's id, twelve characters, and pressing copies all
 *   sixty-four (`CopyId`); mono, because it is read character by character;
 * - 界面 — this machine only: which khor faces are open, from the node
 *   (`about.faces`, `khor faces`). Nobody can answer it for another
 *   machine, so its strip has no such cell.
 *
 * The anchors the smoke and the gates read ride the values: `data-hop` /
 * `data-machine-here` / `data-via-sponsor` on 连接 (the road compared
 * against `khor devices`), `data-vitals-unit="version"` on the version
 * and `data-vitals-age` / `data-updated` on 读数 — the readings the smoke
 * counts against the CLI's line, once on the page.
 *
 * ⚠️ 墓碑 (2026-09-30 → 10-01): this was `Contact`, a contact card — the
 * face at 72, the name beside it in `title-md`, and the facts in a
 * `label · value` table built on one grid with `subgrid`s so the labels
 * ended where the face ended. The owner, on his 1× screen: 「机器页内容部
 * 分还是很丑」. The name was said in R0 and again on the card, the face
 * repeated the list row beside it, and a hundred and fifty pixels at the
 * top of the board went to the facts read least, pushing the dials down.
 * Before that (09-27 → 09-30) it was `Head`, one line of `callout` facts.
 */
/** This machine cannot be reached now (`hop` 「quiet」): what the page
    shows of it is its last reading, said so (P5, P6). */
function isFar(row: DeviceRow): boolean {
  return !row.me && row.hop === "quiet";
}

function Strip({ row, sayModel }: { row: DeviceRow; sayModel: boolean }) {
  const far = isFar(row);
  const v = row.vitals?.vitals ?? null;
  const age = row.vitals?.age_ms ?? 0;
  // **This machine's open faces** — the one fact on the strip only the
  // node here can answer, asked once per board (the identity page's
  // reason: cheap, and fresh enough at the cost of one reopening).
  const [about, setAbout] = useState<About | null>(null);
  useEffect(() => {
    if (!row.me) return;
    let live = true;
    void fetchAbout().then(
      (a) => live && setAbout(a),
      () => {
        // Silent: a cell on a strip; its absence says as much as a
        // sentence about khor failing to ask itself.
      },
    );
    return () => {
      live = false;
    };
  }, [row.me]);
  const days = v ? Math.floor(v.uptime_s / 86_400) : 0;
  const hours = v ? Math.floor(v.uptime_s / 3_600) : 0;
  const model = v?.model?.name ?? null;
  const upFaces = row.me && about ? about.faces.filter((f) => f.up) : [];
  const here = inApp ? "app" : "web";
  const facts: [string, string, ReactNode][] = [];
  if (sayModel && model) facts.push(["model", gui.card_model, model]);
  if (v?.system) facts.push(["system", gui.axis_system, systemWord(v.system)]);
  if (v?.version) {
    facts.push([
      "version",
      gui.identity_version,
      <span data-vitals-unit="version" className="tabular-nums">
        {v.version}
      </span>,
    ]);
  }
  // Three shapes for one slot (`connectionWord`'s three). This machine
  // reaches itself over nothing — 本机, deliberately **not** `data-hop`:
  // the smoke asserts this machine's strip carries no road.
  const road = row.me ? (
    <span data-machine-here>{gui.machine_here}</span>
  ) : row.sponsor ? (
    <span data-via-sponsor={row.sponsor}>{cli.via_sponsor(row.sponsor)}</span>
  ) : row.hop ? (
    <span data-hop={row.hop}>{linkWord(row.hop, row.heard_ms)}</span>
  ) : null;
  if (road) facts.push(["connection", gui.card_connection, road]);
  if (v && v.uptime_s > 0) {
    facts.push([
      "uptime",
      gui.card_uptime,
      <span data-uptime className="tabular-nums">
        {days >= 1 ? count.days(days) : count.hours(hours)}
      </span>,
    ]);
  }
  if (row.vitals) {
    facts.push([
      "reading",
      gui.card_reading,
      far ? (
        // 「无法获取 · 上次 HH:MM」 (P5): what the row says, with the clock
        // of the last reading rather than an age that keeps growing.
        <span data-vitals-age data-far className="tabular-nums">
          {gui.reading_unreachable(clock(Date.now() - age))}
        </span>
      ) : !row.me && age > 0 ? (
        <span data-vitals-age className="tabular-nums">
          {gui.card_age(ago(Date.now() - age))}
        </span>
      ) : (
        <span data-updated className="tabular-nums">
          {clock(Date.now() - age)}
        </span>
      ),
    ]);
  }
  facts.push([
    "id",
    gui.identity_id,
    // Twelve characters to recognise; pressing copies all sixty-four.
    // The button keeps the pointer's floor (`--hit`, taller than the
    // value's line): it is centred on a box exactly the value's line
    // tall, so the target overhangs the line evenly and the twelve
    // characters stand on the same baseline as every other value.
    <span className="flex h-(--text-body--line-height) min-w-0 items-center">
      <CopyId id={row.id} face="fact" />
    </span>,
  ]);
  if (upFaces.length > 0) {
    facts.push([
      "faces",
      gui.card_faces,
      <span data-strip-faces>
        {upFaces.map((f, i) => (
          <span key={f.key} data-strip-face={f.key} data-here={f.key === here}>
            {i > 0 && <span aria-hidden="true"> · </span>}
            {f.name}
          </span>
        ))}
      </span>,
    ]);
  }
  const n = facts.length;
  return (
    // The board's own width decides how many rows (`@container`), and a
    // hairline under the strip is the one edge between it and the dials
    // — `gap-base` of paper over the line, the board's `gap-row` under it.
    // **`overflow-x: clip`**, not hidden: the first column's lines are
    // cut off sideways while the id's taller hit floor still overhangs
    // its line up and down.
    <div data-machine-strip className="@container/strip min-w-0 overflow-x-clip border-b pb-base">
      <dl
        data-strip-grid
        // The three counts a row can hold, one per fold; the class list
        // picks one by the board's width.
        style={
          {
            "--strip-one": n,
            "--strip-two": Math.ceil(n / 2),
            "--strip-three": Math.ceil(n / 3),
          } as CSSProperties
        }
        className={cn(
          // **Equal columns** (the audit fix plan F6.6, W24): each fact
          // takes the same share, so the lines between them fall at
          // even steps. ⚠️ 墓碑: `auto` columns spread apart by
          // `justify-between`, whose gaps followed the words.
          "grid gap-x-row gap-y-row",
          // Pulled left by one cell's line and gap, so a row's first cell
          // keeps its words on the board's padding and its line outside.
          "-ml-[calc(var(--gap-row)+var(--hair))]",
          "grid-cols-[repeat(var(--strip-three),minmax(0,1fr))]",
          "@sm/strip:grid-cols-[repeat(var(--strip-two),minmax(0,1fr))]",
          "@xl/strip:grid-cols-[repeat(var(--strip-one),minmax(0,1fr))]",
        )}
      >
        {facts.map(([k, label, value]) => (
          <div key={k} data-strip-fact={k} className="flex min-w-0 flex-col border-l pl-row">
            <dt data-strip-label className="truncate text-caption text-muted-foreground">
              {label}
            </dt>
            <dd data-strip-value data-selectable className="min-w-0 truncate text-body text-foreground">
              {value}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/**
 * The card's verbs, in one row — the pane stops being a pure dashboard
 * here (docs/KHOR.md 等价: the CLI can already `open --on`, `ls` and
 * `borrow` this machine from any terminal, so the card owes the same
 * moves).
 *
 * **Three verbs, one per landing this machine can be the subject of**
 * (2026-09-03:「右侧是终端、文件、借网三个操作按钮」): a session on it,
 * its disk, its network. 复制 id left this row the same day — the id is
 * on the card, and pressing it copies (`CopyId`'s `id` face), so a
 * button that fetched a visible string was a control with nothing left
 * to do.
 *
 * **Icons, and the rail's own three** (2026-09-06:「开会话、看文件、借网,
 * 用侧边栏 icon 替代。点击后进入对应的侧边栏内容」). Each verb lands on
 * one landing — sessions, files, browser — and the glyph the rail wears
 * for that landing is the one word for it this app already has; a
 * second vocabulary for the same three places is what the word buttons
 * were. The word survives as the name a screen reader hears and a
 * tooltip shows, so nothing is lost to somebody who cannot read a
 * glyph, and the catalog keeps owning it.
 *
 * **开会话 opens the sessions pane's wizard, with this machine picked**
 * — it used to open a bare shell over there directly, and could fail
 * (the far machine away), so it wore a failure word and a disable.
 * Opening a dialog cannot fail; the refusal, if one comes, lands in the
 * wizard where the four answers are, which is where a person can
 * change one. The other two are local navigation and never could fail.
 *
 * **Toolbar buttons, not rail glyphs** (2026-09-28 UI 审计). They
 * were the rail's own drawing at the rail's size (`--icon-rail`, 32) in
 * a 28 button — the heaviest marks in R0, next to a 15 title. They are
 * R0's page buttons, so they wear what R0's other icon button (a 「…」
 * page's close) wears: a `--ctl-md` ghost square, the hit floor from
 * `--hit` (`min-h-touch` reads it), a name in a tooltip. The glyph is
 * still the rail's — the one word this app has for each place — drawn
 * at its own 24 (2026-09-30, the owner: 「顶部右侧的几个 icon 太小了,
 * 明显没有把空间充分利用上」; at 20, the 「…」 list's size, three of them
 * left a 38 row mostly air). components/Icon: a 24 glyph never goes
 * down to the 16 canvas.
 */
export function MachineVerbs({
  row,
  onNewSession,
  code,
  sessionsOn,
  onSessions,
  filesOn,
  onFiles,
  webOn,
  onWeb,
}: {
  row: DeviceRow;
  onNewSession: (row: DeviceRow) => void;
  /** **This machine's VS Code, with no folder** (2026-09-30, the owner:
      「机器页也加一个 VS Code 图标，打开这台机器的 VS Code，不指定目录」).
      The open workspace's switch and this one are one flow
      (`components/CodeWindow`): drawn only where this machine has an
      editor and never on the web face, the first press on a machine asks,
      a second press brings the window forward. Held by the shell rather
      than here — this row of buttons is drawn in R0 on the wide face and
      in the narrow head on the other, and a remount across the breakpoint
      would drop the window's handle. */
  code?: CodeWindow;
  /** **会话 and 文件 are switches of this page's body** (the redesign,
      2026-10-02): pressed, the body is this machine's sessions, or its
      files in place (`MachineFiles` — the owner: 「文件按钮点击后，不要跳转
      到工作区」); pressed again, the dashboard. The shell keeps them
      exclusive. Opening a new session here is ＋, which the capsule shows
      while the sessions are the body. */
  sessionsOn: boolean;
  onSessions: () => void;
  filesOn: boolean;
  onFiles: () => void;
  /** 浏览 in place, the third switch (round 14). */
  webOn: boolean;
  onWeb: () => void;
}) {
  const narrow = useNarrow();
  type Verb = {
    key: string;
    word: string;
    Icon: (p: { className?: string }) => ReactNode;
    on?: boolean;
    /** A companion the switch before it reveals while on: drawn inside
        that switch's chosen pill, still a button of its own. */
    rides?: boolean;
    run: () => void;
  };
  const verbs: Verb[] = [
    { key: "sessions", word: gui.machine_sessions, Icon: IconSessions, on: sessionsOn, run: onSessions },
    ...(sessionsOn && row.hosts
      ? [{ key: "new-session-on", word: gui.open_term_on_ask, Icon: IconPlus, rides: true, run: () => onNewSession(row) }]
      : []),
    { key: "browse-files", word: gui.browse_files, Icon: IconFiles, on: filesOn, run: onFiles },
    // 浏览, where this face can open a page at all (`pagesHere`; it used
    // to ask whether this machine hosts, which a phone never does though
    // it browses): in place since round 14, so it is a switch at every
    // width — the narrow head folds it into 「…」 when it does not fit.
    ...(pagesHere ? [{ key: "borrow-web", word: gui.borrow_web, Icon: IconBrowser, on: webOn, run: onWeb }] : []),
  ];
  const coded = !!code?.offered;
  // **A switch and its companion share one chosen pill** (the owner,
  // 2026-10-02: 「这个+要和session在一个圈里面」): while 会话 is on, the
  // pill holds its glyph and the ＋ side by side — two buttons, one
  // drawing. The pill is the switch's own fill moved one level out, so it
  // grows by exactly the ＋ the capsule grows by.
  const item = (measure: boolean, { key, word, Icon, on, run }: Verb, inPill: boolean) => (
    <GlassItem
      key={key}
      aria-label={word}
      aria-pressed={on === undefined ? undefined : on}
      data-verb={measure ? undefined : key}
      on={inPill ? undefined : on}
      onClick={run}
      // The probe's copy is never pointed at.
      tip={measure ? false : undefined}
      className={cn(inPill && on && "text-[var(--rail-on-ink)]")}
    >
      <Icon />
    </GlassItem>
  );
  const pair = (measure: boolean) => (
    <>
      <GlassCapsule size="head" data-machine-verbs={measure ? undefined : ""}>
        {verbs.map((v, i) => {
          if (v.rides) return null;
          const riders = [];
          for (let j = i + 1; j < verbs.length && verbs[j].rides; j++) riders.push(verbs[j]);
          if (riders.length === 0) return item(measure, v, false);
          return (
            <div key={v.key} data-verb-pill={measure ? undefined : v.key} className="flex flex-none items-center rounded-full glass-sel">
              {/* **The ＋ on the left of 会话** (the audit fix plan F3.2;
                  W02): the capsule stands at the head's right end and
                  grows leftward, so 会话 — and 文件, 浏览 after it — keep
                  their x when the ＋ appears; the second press on the
                  same spot turns 会话 off. */}
              {riders.map((r) => item(measure, r, true))}
              {item(measure, v, true)}
            </div>
          );
        })}
      </GlassCapsule>
      {/* **VS Code is a button of its own** (the owner, 2026-10-02: 「作为一个
          新的页面，不要和已有的 icon 合并为一个 icon 组」). The capsule's
          verbs switch what this page shows; VS Code opens a page of its
          own, so it is a glass circle beside them. Drawn only where there
          is an editor to open; chosen while its window is up — still a
          button, not a switch. */}
      {coded && code && (
        <div className="glass grid size-head-ctl flex-none place-items-center rounded-full">
          <CodeButton
            code={code}
            mark={measure ? undefined : { "data-verb": "code" }}
            className="h-head-item-h w-head-item-h rounded-full data-[on=true]:glass-sel"
          />
        </div>
      )}
    </>
  );

  // **Narrow: when they do not fit, they fold into one 「…」** (the owner,
  // 2026-10-02: 「窄窗口在放不下的情况下，就干脆这几个按钮收为一个 "..."」).
  // Decided by measuring, not by counting or by a width: the unfolded
  // pair is laid out unseen (`measure`), and the head folds when what the
  // name would keep beside it is under its floor (`--head-name-floor`).
  // The name and the buttons share the row's flexible room, so that sum
  // does not move when the buttons fold — no flicker at the edge.
  const box = useRef<HTMLDivElement>(null);
  const probe = useRef<HTMLDivElement>(null);
  const [folded, setFolded] = useState(false);
  const verbsKey = `${verbs.map((v) => v.key).join(",")}|${coded}`;
  useLayoutEffect(() => {
    if (!narrow) {
      setFolded(false);
      return;
    }
    const el = box.current;
    const row = el?.parentElement;
    if (!el || !row) return;
    const check = () => {
      const name = row.querySelector<HTMLElement>("[data-head-name]");
      const full = probe.current?.getBoundingClientRect().width ?? 0;
      if (!name || full === 0) return;
      const cs = getComputedStyle(row);
      const gap = parseFloat(cs.columnGap) || 0;
      const floor = parseFloat(cs.getPropertyValue("--head-name-floor")) || 0;
      const room = name.getBoundingClientRect().width + el.getBoundingClientRect().width;
      setFolded(room - gap - full < floor);
    };
    check();
    const ro = new ResizeObserver(check);
    ro.observe(row);
    return () => ro.disconnect();
  }, [narrow, verbsKey]);

  const anyOn = sessionsOn || filesOn || webOn || !!code?.up;
  // The chosen verb's piece slides to the one pressed (F7.2, `lib/travel`).
  useTravel(
    "verbs",
    () => box.current?.querySelector<HTMLElement>("[data-machine-verbs]") ?? null,
    sessionsOn ? "sessions" : filesOn ? "files" : webOn ? "web" : null,
  );
  return (
    <div ref={box} data-machine-head-buttons data-folded={folded || undefined} className="relative flex flex-none items-center gap-in">
      {narrow && (
        <div ref={probe} aria-hidden inert className="pointer-events-none invisible absolute top-0 right-0 flex items-center gap-in">
          {pair(true)}
        </div>
      )}
      {!folded ? (
        pair(false)
      ) : (
        <div className="glass grid size-head-ctl flex-none place-items-center rounded-full">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                data-slot="button"
                data-verbs-more
                data-on={anyOn}
                aria-label={gui.rail_more}
                className={cn(
                  "relative grid size-head-item-h place-items-center rounded-full text-foreground",
                  "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring",
                  "data-[on=true]:glass-sel data-[state=open]:glass-sel",
                )}
              >
                <IconMore />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="end"
              sideOffset={8}
              data-verbs-menu
              onCloseAutoFocus={(e) => e.preventDefault()}
              className="min-w-nav-menu"
            >
              {verbs.map(({ key, word, Icon, on, run }) => (
                <DropdownMenuItem
                  key={key}
                  data-verb={key}
                  data-on={on || undefined}
                  onSelect={run}
                  className={cn(
                    "[&_svg]:size-nav-glyph",
                    on && "glass-sel text-[var(--rail-on-ink)]",
                  )}
                >
                  <Icon />
                  {word}
                </DropdownMenuItem>
              ))}
              {coded && code && (
                <DropdownMenuItem
                  data-verb="code"
                  data-on={code.up || undefined}
                  onSelect={code.want}
                  className={cn(
                    "[&_svg]:size-nav-glyph",
                    code.up && "glass-sel",
                  )}
                >
                  <IconCode />
                  {gui.code_tab}
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      )}
    </div>
  );
}

/**
 * The hook, and the one button that turns it on or off.
 *
 * **The button is the report.** It shows the job it will do next, so
 * "install" on screen means the hooks are not in place — one control
 * saying one thing, rather than a status line and a button that can
 * disagree with each other. Failure lands on the same button, in the
 * failure colour and under the word for what failed, exactly as a pin
 * does and for the same reason: this app has nowhere that collects
 * messages, and 做了但没变化 may not look like 失败 (docs/UX.md).
 *
 * Nothing is drawn until the state has been read once. A button guessing
 * "not installed" while the answer is still coming would offer to do
 * something already done, and the press would look like it did nothing.
 */
function Hooks({
  state,
  failed,
  onInstall,
  onUninstall,
}: {
  state: HooksState | null;
  failed: boolean;
  onInstall: () => void;
  onUninstall: () => void;
}) {
  if (!state) return null;
  const on = state.installed;
  return (
    <div data-hooks data-installed={on} className="flex flex-col gap-2 pt-pane">
      <Button
        size="sm"
        data-hooks-toggle
        data-failed={failed}
        onClick={on ? onUninstall : onInstall}
        className="min-h-touch self-start data-[failed=true]:text-state-failed"
      >
        {failed
          ? on
            ? gui.uninstall_hooks_failed
            : gui.install_hooks_failed
          : on
            ? gui.uninstall_hooks
            : gui.install_hooks}
      </Button>
      {/* What pressing it buys — **and only while it has not been
          pressed**. Before, this is the reason to press; after, it is a
          remark about something already true, and this app zeroes those
          rather than shortening them (docs/UX.md 文案: 装饰性说明归零).
          The word it promises is one of the six the list already shows,
          which is the only vocabulary this line is allowed (不解释实现);
          the catalog entry carries the rest of that judgment, including
          the tombstone for the second line that used to sit here. */}
      {!on && (
        <div data-hooks-buy className="text-callout text-muted-foreground">
          {gui.hooks_buy_you}
        </div>
      )}
    </div>
  );
}

/** One region of the card, in a box of its own.
 *
 * **The boxes arrived when the card stopped being one column of
 * paragraphs** (2026-09-03): readings, the day, the top and the hook
 * are four answers to four questions, and with nothing but headings
 * between them a person had to find the seams. What a box may not do is
 * make a second column — a row of two boxes side by side is a row that
 * cannot line up, and 「很容易对不齐」 is exactly what the user saw when
 * one was tried. So every panel is full width and they stack. */
/* ⚠️ **Tombstone: `Panel` was defined here until 2026-09-08.** It is
   `components/Panel` now, with its reasoning — the 額度 card's usage
   half had grown flat sections instead of cards, and a block of
   readings looking like a card on one surface and like a stretch of
   page on the other is the plainest form of 「two products」. */
/* ⚠️ **And the boxes went on 2026-09-28** (the owner: 「板里去掉框」).
   The seams the boxes made are one hairline between sections now, and
   each section has a small heading row (`components/SectionHead`, the
   24-hour one drawn by the day panel itself); the two products are still
   one — the 额度 card's usage half took the same shape the same day, and
   `components/Panel` retired with no users. */

/**
 * What the machine is doing, and how old that answer is.
 *
 * **Three states, three different things to say.** Never reached is not
 * an old reading and neither is a zero; an old reading is not the
 * present. Collapsing any two of them would put a number on the screen
 * that means something other than what it says (docs/SESSION.md 离线).
 *
 * **Strain is painted, and the colour is a new one** (user ruling
 * 2026-08-17 flipped the earlier "no severity colour" decision — the
 * trigger written on it was exactly 用户要). What survives from the old
 * ruling is its real content: colour here is spoken for — 中断 owns
 * amber, 失败 owns red, and a 95% disk wearing either would dress a
 * machine as a session. So strain has its own token (`--strain`), one
 * hue for one new thing, and the step from tight to critical is weight
 * rather than a second hue. The judgment itself — which resources, at
 * what thresholds, why — is `khor_core::Fill::strain`, minted into the
 * row by gui-core; this file only paints the word it was handed.
 *
 * # Four dials, and every one of them is a whole
 *
 * CPU, GPU, 内存, 磁盘 (2026-09-03:「就是 CPU、GPU、Mem、Disk。GPU 和
 * Disk 是汇总的,下面分硬件显示 bar」). A dial may only draw something
 * that has **one** denominator — that is what lets an arc say how full
 * without being learnt — so the two that are made of parts draw the
 * sum and hand the parts to a row apiece underneath.
 *
 * **The disk dial's percentage is the sum; its colour is the worst part
 * it is made of.** A machine with a full 1 TB scratch disk beside an
 * empty 2 TB one averages to comfortable, and the average is a true
 * number that answers the wrong question — what a person needs to know
 * is that *something* is out of room. The colour is not invented here
 * either: it is the worst of the strains gui-core already minted per
 * volume (`khor_core::Fill::strain`), so this file still only picks
 * among words it was handed.
 *
 * **`data-vitals-unit` marks a reading, and a whole made of parts is
 * not one.** The GPU and disk dials carry no unit when the parts are
 * there — the rows below are the readings `khor devices` prints, one
 * per card and one per volume, and the smoke holds the two faces to
 * that count. On an older node that reports only aggregates, the
 * aggregate *is* the reading and the dial is the unit.
 */
function Readings({
  row,
  day,
  dayLost = false,
  top,
}: {
  row: DeviceRow;
  day: Load["cpu_day"] | null;
  /** The day was asked for and the machine did not answer: the slot says
      so under its heading rather than holding its height empty. */
  dayLost?: boolean;
  /** The face's own process rows, for the two drawers that have no
      parts to show — see `Drawers`. */
  top: Load["top"];
}) {
  const far = isFar(row);
  // **Nothing is open on arrival, and the slot below the dials shows
  // the day.** It used to open the leftmost dial, because the slot had
  // nothing else to hold and an empty box under four dials is a space
  // waiting to be discovered. It has something else to hold now: the
  // 24-hour figure moved in here (2026-09-04), so the resting state of
  // this block is 「four readings now, and the day behind them」 and a
  // ring is pressed to trade the day for what that ring is made of.
  //
  // Pressing the open one closes it — back to the day. The press is a
  // toggle, and a control whose press does nothing when it is already
  // on is a control that feels stuck (`Dial`'s `onOpen`, which has
  // always sent `""` for that).
  const [open, setOpen] = useState("");
  if (!row.vitals) {
    return (
      <div data-vitals-never className="text-callout text-muted-foreground">
        {cli.vitals_never}
      </div>
    );
  }
  const { vitals: v, volume_strains } = row.vitals;
  const cards = v.gpus;
  const gpuMean = cards.length > 0 ? cards.reduce((a, c) => a + c.util_pct, 0) / cards.length : null;
  const gpuMem = cards.some((c) => c.mem)
    ? cards.reduce(
        (acc, c) => (c.mem ? { used: acc.used + c.mem.used, total: acc.total + c.mem.total } : acc),
        { used: 0, total: 0 },
      )
    : null;
  // The disk whole: every volume this machine reports, summed. An
  // older node that reports one disk and no volumes has that disk as
  // its whole; one that could not read either has no denominator, and
  // then only the track is drawn.
  const vols = v.volumes;
  const disk =
    vols.length > 0
      ? vols.reduce(
          (acc, x) => ({ used: acc.used + x.fill.used, total: acc.total + x.fill.total }),
          { used: 0, total: 0 },
        )
      : v.disk;
  const diskStrain =
    vols.length > 0
      ? (volume_strains.find((w) => w === "critical") ?? volume_strains.find((w) => w === "tight") ?? null)
      : row.vitals.disk_strain;
  const peak = (xs: (number | null)[]) => {
    const seen = xs.filter((x): x is number => x !== null);
    return seen.length > 0 ? Math.round(Math.max(...seen)) : null;
  };
  // **The rim marks, and only where real parts exist.** A card and a
  // volume are things the aggregate is made *of*; the processes under
  // the CPU and memory dials are occupants of a reading, not parts of
  // it, so those two dials carry no marks (`Face` carries why).
  const cardPips: Pip[] = cards.map((c, i) => ({ word: c.name || `GPU${i}`, v: c.util_pct }));
  const volPips: Pip[] = vols.map((x, i) => ({
    word: x.mount,
    v: x.fill.total > 0 ? (x.fill.used / x.fill.total) * 100 : null,
    strain: volume_strains[i] ?? null,
  }));
  const cpuPeak = day ? peak(day.buckets) : null;
  const memPeak = day ? peak(day.mem) : null;
  const left = (fill: { used: number; total: number }) => cli.fill_left(bytes(Math.max(0, fill.total - fill.used)));
  //   `--gap-group`, **read off the page, not mirrored**. ⚠️ 墓碑: this
  //   was a literal 18 with a note that nothing drawn depended on it —
  //   and the shrink below does: `room` is the row less the gaps, so
  //   when the token moved to 20 (第十五轮) the two dials at 390 and 360
  //   came out 2px too wide and hung off the row's edge, which is what
  //   `check-face` (「the row's edges read … -2.0 on the right」) said.
  /** How small the dials may go to keep four in a line (2026-09-28).
      0.8 first; the owner's 934-wide window (a ~540 row) still fell to
      2 × 2 with the pair at the two edges, and 「允许一行缩小的底线变化，
      主要是满足和谐的间距」 — so 0.6: four in a line at the standard gap
      down to a ~490 row. The centre reading still has to fit its ring at
      the drawn size (`check-face` ④ measures that at every width). */
  const FOUR_ACROSS_FLOOR = 0.6;
  const GAP_GROUP =
    Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--gap-group")) || 20;
  //   **How many dials fit on a line, and how big each one is** —
  //   2026-09-09, the user:「机器的环,要铺平宽度,现在左右空太多。窄屏
  //   不允许一行只有一个环。我们一共四个,窄屏就一行两个」.
  //
  //   ⚠️ **Declared, not left to `flex-wrap`.** Wrapping puts as many
  //   as fit on each line, so a row between three dials wide and four
  //   wide lands on **3 + 1** — one dial alone on the second line, the
  //   exact thing the ruling forbids. Measured 2026-09-09 before this
  //   changed: at 900 and 560 viewport (rows 490 and 502) the disk dial
  //   sat by itself. Four or two, decided here, and no other count can
  //   come out.
  //
  //   ⚠️ **The row's own width, never the viewport's** — and that is
  //   measured, not preferred: the row is 642 wide on a 700 viewport
  //   and **490 on a 900 one**, because the list pane appears in
  //   between. A `md:` breakpoint would pick four columns for the
  //   narrower row of the two.
  //
  //   ⚠️ **The size is fixed now, so 「four across」 is a question about
  //   the row, not about the dial**: four boxes and three of the
  //   smallest gaps either fit or they do not. The rung this replaces
  //   asked whether a *share* of the row was still wide enough — that
  //   was the shape when the dial grew to fill its share, and the user
  //   ruled that shape out on 2026-09-09.
  const { box: vitalsBox, w: vitalsW } = usePlotBox({ w: 990, h: 106 });
  //   **One size, and it is the 額度 card's own** (2026-09-09, the
  //   user:「机器卡片环的大小,就是现在额度页的大小。我的意思是,环之间
  //   的间距,排布,平铺屏幕宽度」). Not `dialFor(cell)`: a size that
  //   filled its share made the dial a function of the window, and
  //   「和额度页一样大」 is a statement about a number, not about a
  //   share.
  //
  //   ⚠️ **The same two constants, not the same-looking pair.** The
  //   scaled stroke this replaces would have drawn 5.12 at this size
  //   where the 額度 card draws 5 — a 2.4% thicker arc on a dial the
  //   user had just called the same one. `DIAL_CARD_STROKE`'s own head
  //   says why 5 goes with 169 (it is the machine card's 4 in the
  //   proportion the sizes are), so taking both from there is what
  //   makes this literally the same drawing.
  const dialBox = DIAL_CARD_SIZE * hugRatio(DIAL_CARD_SIZE, DIAL_CARD_STROKE);
  //   Four across while four boxes and three of the smallest gaps fit;
  //   otherwise two. **Still declared, never `flex-wrap`** — see below.
  //
  //   **Four across down to 60% of that size** (2026-09-28, the owner
  //   chose it over 2 × 2 with more air: 「避免 3+1，就是 2 或 4」 — and
  //   of the two, four in a line). A ~650 row, the common one with the
  //   list beside the card, holds four at ~82%, a ~540 one at ~67%; at
  //   full size both fell to 2 × 2 with the pair pushed to the edges.
  //   Below 60% the row is narrow enough that two read better than four
  //   small ones.
  const cols = (vitalsW - 3 * GAP_GROUP) / 4 >= FOUR_ACROSS_FLOOR * dialBox ? 4 : 2;
  //   **And where two do not fit either, the dial gives way** — the
  //   only rung left once 「one alone on a line」 is forbidden outright
  //   and the size is fixed. Measured 2026-09-09: two boxes plus the
  //   floor gap need 334.1, and a 390-wide phone gives this row 332 —
  //   **the smallest ordinary phone is already inside the last 2px of
  //   the full size**, and a 360-wide one (12 mini, 13 mini) is 30px
  //   short of it.
  //
  //   ⚠️ **The gap floor wins and the dial yields, not the other way
  //   round.** They cannot both hold: 18 between two 158s wants 334 in
  //   a row of 332. Giving way on the gap would make it a remainder
  //   again on exactly the screens where things are tightest — and the
  //   09-08 ruling that made it a stated number was about the wide
  //   posture, where there is room to state it.
  //
  //   **One ratio moves both numbers**, so a shrunken dial is the same
  //   drawing rather than a thinner one at the same diameter — see
  //   `DIAL_CARD_STROKE`, whose 5 is the machine card's old 4 in the
  //   proportion the sizes are. ⚠️ **That pairing is what makes the
  //   centre reading keep fitting**: at a fixed stroke the inner
  //   diameter falls faster than the type does, which is exactly how
  //   `100%` came to overflow at size 88 (57.25 against 56) — and with
  //   the stroke in proportion, an 88-sized dial's inner would have
  //   been 57.4 and it would have fitted. **The gate reads the fit at
  //   the smallest size this actually draws**, rather than trusting
  //   that sentence.
  const room = (vitalsW - (cols - 1) * GAP_GROUP) / cols;
  const shrink = Math.min(1, room / dialBox);
  const dial = { size: DIAL_CARD_SIZE * shrink, stroke: DIAL_CARD_STROKE * shrink };
  return (
    // **On the board itself, not in a box** (2026-09-28, the owner:
    // 「机器详情：板里去掉框」). The dials and the slot under them were a
    // hairline-bordered card on the board, and the processes another;
    // they are two sections of the board now, one hairline between them
    // (the slot's own top rule) and no border, ground or inset of their
    // own. `data-machine-section` names each for the gate that holds it.
    <section data-machine-section="readings" className="flex flex-col">
      {/* ⚠️ **A grid of a declared column count, since 2026-09-09** —
          and the two rulings it replaces are worth keeping, because
          each one was right about the row it was looking at.

          First it was `justify-around`, which spread the row's leftover
          as air: measured 2026-09-08 at 1400, four 123.5 boxes in a 990
          row put 109.7 between boxes — about 190 between one arc's edge
          and the next — against an arc 60 wide (the user:「环的间距也
          太远了」). **The direction was the opposite of the obvious
          one**: a narrower dial gives `justify-around` more to spread,
          so shrinking the boxes would have widened the gaps.

          Then it was `justify-center gap-group`, which made the gap a
          stated number instead of a remainder — and left the leftover
          at the two ends: **221 of air on each side at 1400** (the
          user, 2026-09-09:「要铺平宽度,现在左右空太多」).

          Now the four divide the row and fill their share, so there is
          no leftover to place anywhere. `flex-wrap` had to go with it:
          it packs as many as fit per line, which on a middling row is
          **3 + 1** — and 「窄屏不允许一行只有一个环」 forbids exactly
          that. The count is decided above and the gate holds it at six
          widths. */}
      <div
        ref={vitalsBox}
        data-vitals
        data-vitals-cols={cols}
        // **An unreachable machine's dials are its last reading, greyed**
        // (P5, 2026-10-03): a data state, the way the list greys the
        // row of a machine it cannot reach (`SideList` `dim`), so the
        // numbers do not read as live. The facts say since when.
        data-far={far || undefined}
        className={cn("grid gap-group", far && "opacity-55")}
        //   **Content-sized columns pushed to the two edges**, so the
        //   row's spare width lands *between* the dials and the outer
        //   two sit flush. `gap-group` stays as the floor the spare
        //   width is added to, not as the gap itself.
        style={{ gridTemplateColumns: `repeat(${cols}, max-content)`, justifyContent: "space-between" }}
      >
        <Dial
          dial="cpu"
          open={open === "cpu"}
          onOpen={setOpen}
          name="cpu"
          word={cli.vitals_name_cpu}
          fraction={v.cpu_pct / 100}
          sentence={cli.vitals_cpu(Math.round(v.cpu_pct), v.cores)}
          facts={[cli.cores_sub(v.cores), cpuPeak !== null ? cli.day_peak(`${cpuPeak}%`) : null]}
          size={dial.size}
          stroke={dial.stroke}
        />
        {cards.length > 0 && gpuMean !== null ? (
          <Dial
            dial="gpu"
            open={open === "gpu"}
            onOpen={setOpen}
            word={cli.vitals_name_gpu}
            pips={cardPips}
            fraction={gpuMean / 100}
            sentence={cli.vitals_gpu(Math.round(gpuMean), cards.length)}
            facts={[
              gpuMem ? cli.vitals_vram(bytes(gpuMem.used), bytes(gpuMem.total)) : cli.gpu_sub_nomem(cards.length),
            ]}
            size={dial.size}
            stroke={dial.stroke}
          />
        ) : (
          v.gpu && (
            <Dial
              dial="gpu"
              open={open === "gpu"}
              onOpen={setOpen}
              name="gpu"
              word={cli.vitals_name_gpu}
              fraction={v.gpu.util_pct / 100}
              sentence={cli.vitals_gpu(Math.round(v.gpu.util_pct), v.gpu.cards)}
              // An older node prints 显存 as a sentence of its own, and
              // the count of readings has to match — so on that node the
              // memory line *is* a unit, here on the back of the dial.
              facts={[
                v.gpu.mem ? (
                  <span
                    data-vitals-unit="vram"
                    data-vitals-detail="vram"
                    data-strain={row.vitals.vram_strain ?? undefined}
                    className="data-[strain]:text-strain data-[strain=critical]:font-label"
                  >
                    {cli.vitals_vram(bytes(v.gpu.mem.used), bytes(v.gpu.mem.total))}
                  </span>
                ) : (
                  cli.gpu_sub_nomem(v.gpu.cards)
                ),
              ]}
              size={dial.size}
              stroke={dial.stroke}
            />
          )
        )}
        <Dial
          dial="mem"
          open={open === "mem"}
          onOpen={setOpen}
          name="mem"
          word={cli.vitals_name_mem}
          fraction={v.mem.total > 0 ? v.mem.used / v.mem.total : null}
          strain={row.vitals.mem_strain}
          sentence={cli.vitals_mem(bytes(v.mem.used), bytes(v.mem.total))}
          facts={[left(v.mem), memPeak !== null ? cli.day_peak(`${memPeak}%`) : null]}
          size={dial.size}
          stroke={dial.stroke}
        />
        <Dial
          dial="disk"
          open={open === "disk"}
          onOpen={setOpen}
          word={cli.vitals_name_disk}
          pips={volPips}
          fraction={disk && disk.total > 0 ? disk.used / disk.total : null}
          strain={diskStrain}
          unknown={!disk}
          sentence={
            disk
              ? vols.length > 0
                ? cli.disk_whole(bytes(disk.total), vols.length)
                : cli.vitals_disk(bytes(disk.used), bytes(disk.total))
              : cli.vitals_disk_unknown
          }
          facts={[disk ? left(disk) : null]}
          size={dial.size}
          stroke={dial.stroke}
        />
      </div>
      {/* **One slot, one height, two things it can show.** See
          `--slot-h`: the figure and the drawers are the same height, so
          the card does not move under a press. */}
      <div data-ring-slot>
      <Drawers
        row={row}
        top={top}
        open={open}
        heads={{
          cpu: { kind: "occupants", word: cli.vitals_name_cpu, count: cli.procs_sub, fraction: v.cpu_pct / 100 },
          gpu: {
            kind: "parts",
            word: cli.vitals_name_gpu,
            count: cli.gpu_sub_nomem,
            fraction: gpuMean !== null ? gpuMean / 100 : v.gpu ? v.gpu.util_pct / 100 : null,
          },
          mem: {
            kind: "occupants",
            word: cli.vitals_name_mem,
            count: cli.procs_sub,
            fraction: v.mem.total > 0 ? v.mem.used / v.mem.total : null,
            strain: row.vitals.mem_strain,
          },
          disk: {
            kind: "parts",
            word: cli.vitals_name_disk,
            count: cli.vols_sub,
            fraction: disk && disk.total > 0 ? disk.used / disk.total : null,
            strain: diskStrain,
          },
        }}
      />
        {/* **The day lives in the same slot as the drawers**
            (2026-09-04). It is not a fourth thing under the dials, it
            is what this slot shows when no ring is pressed — and the
            two share one height so pressing a ring does not make the
            card jump.

            **Hidden, never unmounted.** `smoke.mjs` counts the
            readings on this card against what `khor devices` prints,
            and a block that unmounted its figure or its rows would make
            the card disagree with the terminal — which reads as a
            broken backend rather than as a closed drawer. */}
        <div data-load-day hidden={open !== ""}>
          {day ? (
            <DayChart day={day} />
          ) : (
            // **Not a box of nothing** (2026-10-03): until the day came, and
            // for good when the machine did not answer, the slot held its
            // whole height blank between the dials and 进程, its heading
            // gone with it. The heading stays; under it the skeleton the
            // process table wears while it is asked (`RegionState`), or,
            // once the asking failed, the words an unreachable row wears.
            <div data-day-lost={dayLost || undefined} className="flex h-full flex-col">
              <span className="flex h-(--slot-head-h) flex-none items-center text-group text-muted-foreground">
                {cli.day_title}
              </span>
              {dayLost ? (
                <span className="text-callout text-muted-foreground">{gui.day_unreachable}</span>
              ) : (
                <div data-skeleton aria-busy="true" className="min-h-0 flex-1 rounded-md bg-fill" />
              )}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

/**
 * The parts the two aggregate dials are made of: one row per GPU card,
 * then one per volume (2026-09-03:「GPU 和 Disk 是汇总的,下面分硬件显示
 * bar」).
 *
 * **One grid for both groups, and that is the whole point of the
 * component.** They were two blocks with two layouts once, and the bars
 * did not start on the same pixel — 「很容易对不齐」. Sharing a grid
 * makes alignment structural rather than a pair of numbers somebody has
 * to keep equal. The bar column is `1fr`, so the rows follow the card
 * rather than reserving a width the window may not have.
 *
 * **Each row is the unit** for one of `khor devices`' sentences and
 * carries that sentence word for word for the smoke to read (`sr-only`:
 * the row prints the same facts laid out, not the sentence twice).
 *
 * A volume is named by its mount point, spelled the way the machine
 * spells it (2026-09-03:「就写 /Volumes/disk0 就可以」). The short name
 * it wore before answered a question nobody asked — which of these is
 * "the disk" — and lost the one thing a person picks a volume by.
 */
function Drawers({
  row,
  top,
  open,
  heads,
}: {
  row: DeviceRow;
  /** The face's own two hundred process rows. **The CPU and memory
      drawers are computed from these and nothing is asked of the node**
      — there is no per-core array and no memory breakdown on the wire
      (`khor_core::Vitals` is one `cpu_pct` and one `Fill`), so what can
      honestly be shown there is who is taking the most. */
  top: Load["top"];
  open: string;
  /** Each group's heading: the reading's own glyph and its word. **The
      count is not passed in** — it is taken from the rows this
      component actually renders, so the number and the list cannot
      disagree. A machine whose node reports 「4 cards」 while sending no
      per-card list would otherwise print 「4 卡」 over an empty drawer,
      which is a heading lying about the thing directly under it.

      `kind` is **what the rows under it are**, and it is the one place
      that fact is written down. A card and a volume are *parts* of the
      ring above — their readings add up to it. A process is only an
      *occupant*, and the few listed here do not add up to anything;
      there are several thousand more. Six of either draws identically,
      so the difference has to be said, and it is said **in the count's
      own grammar**: `parts` wear a bare count (「3 卷」), `occupants`
      wear a qualifier in front of the number (「占用最多的 6 个进程」).

      That used to be a footnote under the drawer instead
      (`top-share-note`, 「占用最多的几个,加起来不等于上面那个环」).
      2026-09-04:「也不要,很蠢」. What went is the apology, not the
      fact — a line that explains the block above it after the fact
      costs a row and is read last, while the heading is read first and
      is where somebody is already looking for what these rows are.
      `check-face` measures the grammar, on both arms. */
  heads: Record<
    string,
    { kind: "parts" | "occupants"; word: string; fraction: number | null; strain?: Strain | null; count: (n: number) => string }
  >;
}) {
  if (!row.vitals) return null;
  const { vitals: v, volume_strains } = row.vitals;
  const cards = v.gpus;
  const vols =
    v.volumes.length > 0
      ? v.volumes.map((vol, i) => ({
          unit: "vol",
          word: mountWord(vol.mount),
          fill: vol.fill,
          strain: volume_strains[i] ?? null,
          sentence: cli.vitals_vol(volWord(vol.mount), bytes(vol.fill.used), bytes(vol.fill.total)),
        }))
      : v.disk
        ? [
            {
              unit: "disk",
              word: cli.vitals_name_disk,
              fill: v.disk,
              strain: row.vitals.disk_strain,
              sentence: cli.vitals_disk(bytes(v.disk.used), bytes(v.disk.total)),
            },
          ]
        : [
            {
              unit: "disk",
              word: cli.vitals_name_disk,
              fill: null,
              strain: null,
              sentence: cli.vitals_disk_unknown,
            },
          ];
  // The busiest few, by the reading the drawer is under. Six, because
  // the drawer shows four without scrolling and the fifth and sixth are
  // what the scroll is for.
  const busiest = (by: "cpu" | "mem") =>
    [...top]
      .sort((a, b) => (by === "cpu" ? b.cpu_pct - a.cpu_pct : Number(b.mem) - Number(a.mem)))
      .slice(0, 6);
  // **An occupant's bar is drawn against the biggest occupant shown,
  // not against the machine.** Against the machine it is invisible and
  // therefore says nothing: turing's hungriest process is 41% of one
  // core on a box with sixty-four of them, so a share-of-machine bar
  // for the row at the top of the list draws a hairline and every row
  // below it draws nothing at all — a column that is blank for every
  // value it can hold is not a reading.
  //
  // What it does say is 「how these few compare to each other」, which
  // is the question the drawer is answering. **It is not a fraction of
  // the ring above**, and the foot says so in as many words — that
  // sentence is doing real work here rather than being a formality.
  const against = (xs: number[]) => Math.max(1, ...xs);
  // **What each drawer actually draws.** The heading counts these, not
  // what the node claims elsewhere — see `heads`.
  // The watched directories: the machine's own answer, one row each in
  // one of three shapes, under the volumes (2026-09-06 关注目录). Absent
  // altogether — not empty — for a machine that never answered.
  const watched: WatchedDir[] = row.watched?.rows ?? [];
  const drawn: Record<string, number> = {
    cpu: busiest("cpu").length,
    gpu: cards.length,
    mem: busiest("mem").length,
    disk: vols.length,
  };
  const box = useRef<HTMLDivElement>(null);
  const scroll = useRef<HTMLDivElement>(null);
  // **What is out of sight right now**, so the heading can say it —
  // and stop saying it the moment somebody scrolls to the bottom.
  const hidden = useHiddenRows(scroll, open);
  // What the three columns need before they stop fitting: the two fixed
  // ones, the two gaps between them, and enough track left for a bar to
  // still read as a length. Written from the tokens rather than guessed
  // — `--part-name` 220 + `--part-num` 172 + two `--gap-row` 12 + 60.
  const tight = useTight(box, 476);
  return (
    <>
      {/* **One box, fixed height, and every group inside it.** The
          groups that are not open are hidden rather than dropped: the
          card's readings are counted against what `khor devices` prints
          (`smoke.mjs`'s `sameNumberOfReadings`), and a drawer that
          unmounted three of them would make the card disagree with the
          terminal — which reads as a broken backend, not as a drawer.
          Hidden elements are still in the document, so the count holds
          and only the eye is spared. */}
      <div
        ref={box}
        data-drawer={open}
        data-tight={tight ? "" : undefined}
        // **The slot decides the height now**, not this box: the day
        // and the drawers share one, so pressing a ring cannot move
        // anything below (`--slot-h`, and `[data-slot]` in app.css).
        // It used to carry `min-h-drawer-h max-h-drawer-h`, a height
        // written for a block that stood on its own.
        hidden={open === ""}
        // **The rule above the slot and the room under it belong to the
        // slot**, not to this box: they were here, and the day beside
        // it had neither, so the drawer's heading sat 13px below the
        // reading words — two things that have to be the same line
        // (`[data-ring-slot]`, and the assertion that caught it).
        className="flex h-full flex-col text-callout tabular-nums"
      >
        {heads[open] && (
          <div
            data-drawer-head
            data-drawer-kind={heads[open].kind}
            className="mb-in flex items-center gap-in text-callout text-muted-foreground"
          >
            <Glyph fraction={heads[open].fraction} strain={heads[open].strain} />
            <span>{heads[open].word}</span>
            <span aria-hidden="true">·</span>
            <span data-drawer-count={drawn[open]} className="tabular-nums">
              {heads[open].count(drawn[open])}
            </span>
            {/* **The number is on the element, not in the sentence.**
                It used to be read out — 「还有 3 个在下面,拖一下」 —
                and 2026-09-04 the answer to that was 「你不觉得这个画很
                蠢吗,不要这个。下滑查看更多」. Nobody acts on the
                three: seeing that there is more is what makes somebody
                scroll, and whether it is three or seven changes
                nothing they do next.

                **What did not change is when it appears.** It is still
                counted off the rows the bottom edge actually clips, so
                it is absent when everything fits and goes the moment
                somebody reaches the end — a hint that is always there
                is not a hint, it is part of the heading. `check-face`
                reads the count off `data-drawer-more`, which is why
                the words could go and the state could stay. */}
            {hidden > 0 && <span data-drawer-more={hidden}>· {cli.drawer_more}</span>}
          </div>
        )}
        {/* **The rows scroll, the heading does not.** A heading that
            scrolled away would take with it the one line saying which
            reading these rows belong to — and four rows fit here without
            scrolling at all, which is what makes 「just fits」 and 「just
            does not」 different on screen. */}
        <div
          ref={scroll}
          data-drawer-scroll
          className="flex min-h-0 flex-1 flex-col overflow-y-auto overscroll-contain"
          style={{ scrollSnapType: "y proximity" }}
        >
        {/* **One pitch in every drawer, and the rows sit in the middle**
            (2026-09-28, the owner: CPU's bars read comfortably, the
            disk's were spaced too far). The rows used to share the slot's
            height (`minmax(row, 1fr)`), so four volumes plus the watch
            editor were pulled 43 apart against CPU's 29 — and pushed past
            the slot, so the drawer scrolled with a row to spare. Now every
            row keeps `--drawer-row`, and what is left of the slot goes
            above and below the block (`my-auto`: a centre that collapses
            to the top when the rows overflow, so they still scroll),
            which also keeps 09-04's 「不要…下面空掉了」 — no hole under
            the last row. */}
        <div data-drawer-body className="my-auto">
        <div data-parts="cpu" hidden={open !== "cpu"}>
          {busiest("cpu").map((r, _i, all) => (
            <Occupant
              key={r.pid}
              row={r}
              share={r.cpu_pct / against(all.map((x) => x.cpu_pct))}
              value={`${Math.round(r.cpu_pct)}%`}
            />
          ))}
        </div>
        <div data-parts="gpu" hidden={open !== "gpu"}>
          {cards.map((c, i) => {
            const util = Math.round(c.util_pct);
            const sentence = c.mem
              ? cli.vitals_gpu_card_mem(c.name || cli.vitals_name_gpu, util, bytes(c.mem.used), bytes(c.mem.total))
              : cli.vitals_gpu_card(c.name || cli.vitals_name_gpu, util);
            return (
              <div key={`gpu${i}`} data-part={i}>
                <span
                  data-part-name
                  data-vitals-unit="gpu"
                  data-gpu-card={c.name || `GPU${i}`}
                  className="min-w-0 gap-in"
                >
                  <span className="flex-none font-label">GPU{i}</span>
                  {/* The driver's name, when it gave one; a card that
                      will not name itself (Apple's) is "GPU0" and
                      nothing more — not "GPU0 GPU". */}
                  {c.name && <span className="ml-in min-w-0 truncate text-muted-foreground">{c.name}</span>}
                  <span data-vitals-detail="gpu" className="sr-only">
                    {sentence}
                  </span>
                </span>
                <Bar pct={util} strain={null} />
                <span data-part-num className="justify-end gap-row text-right">
                  <span data-gpu-util={util}>{util}%</span>
                  <span className="text-muted-foreground">
                    {c.mem ? `${bytes(c.mem.used)} / ${bytes(c.mem.total)}` : ""}
                  </span>
                  {c.temp_c != null && (
                    <span className="text-muted-foreground">{`${Math.round(c.temp_c)}°C`}</span>
                  )}
                </span>
              </div>
            );
          })}
        </div>
        <div data-parts="mem" hidden={open !== "mem"}>
          {busiest("mem").map((r, _i, all) => (
            <Occupant
              key={r.pid}
              row={r}
              share={Number(r.mem) / against(all.map((x) => Number(x.mem)))}
              value={bytes(r.mem)}
            />
          ))}
        </div>
        <div data-parts="disk" hidden={open !== "disk"}>
          {vols.map((r, i) => {
            const pct = r.fill && r.fill.total > 0 ? (r.fill.used / r.fill.total) * 100 : null;
            return (
              // By position, not by word: a Mac reports its data volume
              // and `/` as two mounts that can both print as `/`.
              <div key={`vol${i}`} data-part={i}>
                <span
                  data-part-name
                  data-vitals-unit={r.unit}
                  data-vitals-unknown={r.fill ? undefined : ""}
                  className="min-w-0 truncate"
                >
                  {r.word}
                  <span
                    data-vitals-detail={r.unit}
                    data-strain={r.strain ?? undefined}
                    className="sr-only data-[strain]:text-strain data-[strain=critical]:font-label"
                  >
                    {r.sentence}
                  </span>
                </span>
                <Bar pct={pct} strain={r.strain} />
                <span
                  data-part-num
                  data-strain={r.strain ?? undefined}
                  className="justify-end text-right text-muted-foreground data-[strain]:text-strain data-[strain=critical]:font-label"
                >
                  {r.fill ? `${bytes(r.fill.used)} / ${bytes(r.fill.total)}` : cli.vitals_disk_unknown}
                </span>
              </div>
            );
          })}
        </div>
        {/* **The watched directories, under the volumes and apart from
            them, in one column** (2026-09-06 关注目录, and the ruling
            「一个路径只出现一次」): their own heading and their own grid —
            the heading above counts volumes, and a directory is not a
            volume, it is a place on one. What a row is, is the state of
            its path: the machine answered it (a reading — bar, size or
            the sentence for a missing / unlisted directory — and a unit,
            because `khor devices` prints that sentence), it is saved and
            the machine has not answered yet (还没问到, no unit: the
            terminal prints nothing for it either), or it is only in the
            draft here (未保存, no bar, no unit — it is not a reading and
            neither face may count it). A saved path taken out of the
            draft keeps its row, greyed and marked 未保存, until the save
            makes it true. `WatchList` draws the rows and the editor's
            own line under them; a satellite's rows have no editor (its
            readings are sampled by its sponsor over ssh, and its row has
            no khor to answer `Watched` — an editor there would save a
            list nothing reads). */}
        <WatchList row={row} watched={watched} open={open === "disk"} />
        </div>
        </div>
      </div>
      {/* **Nothing under the drawer.** A sentence used to sit here
          saying that the rows above do not add up to the ring
          (`top-share-note`); 2026-09-04:「也不要,很蠢」.

          The fact it was defending is real and is still said — it
          moved **into the heading's grammar**, where `kind` explains
          it: `occupants` count themselves with a qualifier in front of
          the number, `parts` with a bare count. That is better than
          the footnote was, and not only shorter: the footnote sat
          *after* the block it was about, so it could only correct a
          reading somebody had already taken, while the heading is
          above the rows and is read on the way in. */}
    </>
  );
}

/**
 * The disk drawer's watched directories, one column, with the editor's
 * line under them (2026-09-06:「磁盘点击后,可以有一个编辑,用户可以在这
 * 增删目录,而后更新关注磁盘」, and the ruling that a path appears once).
 *
 * **Directories, not volumes.** A person names the place they work in
 * — `/data/models`, a scratch dir on a mount they cannot spell — and
 * the machine answers which volume that is and how much it has left
 * (`khor_core::WatchedDir`). The list is the setting and rides that
 * machine's row in the device table; the readings arrive on the next
 * visit, which is why the save says nothing itself: the row turning
 * from 还没问到 into a size is the report.
 *
 * **A draft, saved whole.** Adding and removing edit a local copy, and
 * 保存 sends the copy — one call, one refusal, rather than a call per
 * keystroke racing the poll. The draft follows the row's list only when
 * that list changes under it (somebody else saved, or this save came
 * back through the poll); a poll that repeats the list it already has
 * does not touch what is being typed. The node validates (absolute, no
 * `..`, no duplicate) and its sentence is printed under the list — the
 * same sentence `khor watch` prints, one voice.
 *
 * **Every row is the same three cells the volumes use**, so the two
 * groups line up (`Drawers` has why one grid beats two), grown to the
 * finger floor because a row here carries a button — `min-height`
 * beats the grid's fixed `height`, `ui/button.tsx`'s scar.
 */
function WatchList({ row, watched, open }: { row: DeviceRow; watched: WatchedDir[]; open: boolean }) {
  const savedKey = row.watch.join("\n");
  const [base, setBase] = useState(savedKey);
  const [draft, setDraft] = useState<string[]>(row.watch);
  const [typed, setTyped] = useState("");
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  useEffect(() => {
    if (savedKey === base) return;
    setBase(savedKey);
    setDraft(savedKey === "" ? [] : savedKey.split("\n"));
  }, [savedKey, base]);
  const saved = base === "" ? [] : base.split("\n");
  const editable = row.sponsor === null;
  const dirty = draft.join("\n") !== base;
  // Saved paths first, in the table's order, then the draft's new ones
  // in the order they were typed — one row per path, whatever its state.
  const paths = [...saved, ...draft.filter((p) => !saved.includes(p))];
  const add = () => {
    const p = typed.trim();
    if (!p || draft.includes(p)) return;
    setDraft((d) => [...d, p]);
    setTyped("");
  };
  const save = () => {
    if (saving || !dirty) return;
    setSaving(true);
    setFailed(null);
    setWatch(row.name, draft)
      .then((kept) => {
        setBase(kept.join("\n"));
        setDraft(kept);
      })
      .catch((e) => setFailed(String(e instanceof Error ? e.message : e)))
      .finally(() => setSaving(false));
  };
  if (paths.length === 0 && !editable) return null;
  return (
    <>
      <div data-watch-head hidden={!open} className="mt-row mb-in text-callout text-muted-foreground">
        {gui.watch_dirs}
      </div>
      <div data-parts="watch" hidden={!open}>
        {paths.map((p, i) => {
          const reading = watched.find((w) => w.path === p) ?? null;
          const inSaved = saved.includes(p);
          const inDraft = draft.includes(p);
          const pending = inSaved === inDraft ? null : inDraft ? "add" : "remove";
          const fill = reading?.volume?.fill ?? null;
          const pct = fill && fill.total > 0 ? (fill.used / fill.total) * 100 : null;
          const shape = reading
            ? reading.volume
              ? "on"
              : reading.missing
                ? "missing"
                : "unlisted"
            : inSaved
              ? "asked"
              : "pending";
          const sentence = reading
            ? reading.volume
              ? cli.vitals_watch(p, volWord(reading.volume.mount), bytes(Math.max(0, reading.volume.fill.total - reading.volume.fill.used)), bytes(reading.volume.fill.total))
              : reading.missing
                ? cli.vitals_watch_missing(p)
                : cli.vitals_watch_unlisted(p)
            : null;
          const number = fill
            ? `${bytes(Math.max(0, fill.total - fill.used))} / ${bytes(fill.total)}`
            : reading
              ? reading.missing
                ? cli.vitals_watch_missing("").trim()
                : cli.vitals_watch_unlisted("").trim()
              : inSaved
                ? cli.vitals_never
                : "";
          return (
            <div
              key={p}
              data-part={i}
              data-watch-row={p}
              data-watch-shape={shape}
              data-pending={pending ?? undefined}
              className={cn(pending === "remove" && "opacity-55")}
            >
              {/* **The unit rides only a row the machine answered** —
                  that is the row `khor devices` prints a sentence for,
                  and the count on both faces is the count of those. */}
              <span
                data-part-name
                data-vitals-unit={reading ? "watch" : undefined}
                className="min-h-touch min-w-0 gap-in"
              >
                <span className="min-w-0 truncate">{p}</span>
                {reading?.volume && (
                  <span className="ml-in flex-none text-muted-foreground">{mountWord(reading.volume.mount)}</span>
                )}
                {sentence && (
                  <span data-vitals-detail="watch" className="sr-only">
                    {sentence}
                  </span>
                )}
              </span>
              {reading ? <Bar pct={pct} strain={null} /> : <span data-part-bar className="min-h-touch" />}
              <span data-part-num className="min-h-touch justify-end gap-in text-right text-muted-foreground">
                {number && <span>{number}</span>}
                {pending && (
                  <span data-watch-unsaved className="text-callout">
                    {gui.watch_unsaved}
                  </span>
                )}
                {editable && pending !== "remove" && (
                  <Button
                    variant="ghost"
                    size="sm"
                    data-watch-remove={p}
                    className="min-h-touch"
                    onClick={() => setDraft((d) => d.filter((q) => q !== p))}
                  >
                    {gui.remove}
                  </Button>
                )}
              </span>
            </div>
          );
        })}
      </div>
      {/* **The editor sticks to the bottom of the drawer**, because it is
          this drawer's workbench and not its last row. It sat at the end
          of the scrolling column when it arrived, and the drawer's
          viewport is 175px on a 720-tall window: measured 2026-09-06 the
          whole editor was at y=208 with the scroller ending at y=200 —
          **not one pixel of it on screen**, and no hint that it was
          there (the 「下滑查看更多」 mark counts clipped *rows*, and the
          editor is not one). A control nobody can see is the same as a
          control that is not there, which is what the disk drawer was
          asked to grow.

          `bg-card` because it must not be see-through with rows sliding
          under it, and that is the ground the panel around it paints.

          **It does not fix a drawer too short for its own content** —
          the rows still scroll behind it and one row's worth of room
          goes to the editor. What it fixes is that the way *in* is
          always on screen. */}
      {editable && (
        <div
          data-watch-editor
          data-dirty={dirty}
          hidden={!open}
          className="sticky bottom-0 mt-in flex flex-col gap-in bg-card pt-in text-callout"
        >
          <div className="flex min-w-0 items-center gap-in">
            <Input
              data-watch-input
              aria-label={gui.new_session_dir}
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && add()}
            />
            {/* Save is drawn while there is something to save (F1.9). */}
            {/* Always pressable (`components/Why`): with nothing typed,
                the press puts the caret in the box. */}
            <Button
              size="sm"
              data-watch-add
              className="min-h-touch"
              onClick={() => (typed.trim() ? add() : document.querySelector<HTMLElement>("[data-watch-input]")?.focus())}
            >
              {gui.watch_add}
            </Button>
            {(dirty || saving) && (
              <Button
                size="sm"
                data-watch-save
                data-failed={failed !== null}
                className="min-h-touch data-[failed=true]:text-state-failed"
                disabled={saving}
                aria-busy={saving || undefined}
                onClick={save}
              >
                {failed !== null ? gui.watch_save_failed : gui.watch_save}
              </Button>
            )}
          </div>
          {failed !== null && (
            <div data-watch-error className="text-callout text-destructive">
              {failed}
            </div>
          )}
        </div>
      )}
    </>
  );
}

/** One occupant of a reading: a process, its share of the machine, and
    what it is taking. **Not a part** — see `Drawers`. */
function Occupant({ row, share, value }: { row: Load["top"][number]; share: number | null; value: string }) {
  return (
    <div data-part={row.pid} data-occupant={row.pid}>
      <span data-part-name className="min-w-0 gap-in truncate">
        <span className="truncate">{row.name}</span>
        {row.arg && <span className="ml-in min-w-0 truncate text-muted-foreground">{row.arg}</span>}
      </span>
      <Bar pct={share === null ? null : share * 100} strain={null} />
      <span data-part-num className="justify-end text-right">{value}</span>
    </div>
  );
}

/* `Bar` moved to `components/Bar.tsx` on 2026-09-07: the 額度 page's
   window rows draw the same thing, and a second copy of a primitive has
   nothing keeping the two in step (that component's head carries the
   whole account, including why the quota bars pass no `strain`). The
   DOM it emits is unchanged — `data-part-bar` on the cell,
   `data-part-ink` on the ink. */


/** The wall clock for one instant, hours and minutes in the reader's
    own locale — the head's 更新于. (It printed the day chart's two ends
    as well until 2026-09-04; the window is drawn on that figure's axis
    and does not need writing out beside it.) */
function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** How deep the table goes. **A bound, and it is honest about being
    one**: the window shows the top 200 of whichever column it is
    ranked by, ranked on the machine itself. Not "every process" —
    turing has ten thousand, and 800 KB every five seconds to show nine
    rows is the kind of traffic the mesh should not spend on a screen.
    Two hundred is past any question a person asks a card ("what is
    eating this machine") and small enough to be free (~16 KB). */
const TOP_LIMIT = 200;
/** How many rows the table shows before 显示全部 (F4.3). */
const TOP_SHOWN = 20;

/** How often an open card re-asks. Five seconds is the vitals beat the
    rings above already move on; the table and the band moving on
    another would make the card describe two moments. */
const LOAD_MS = 5_000;
/** …and how long to wait after a refusal before asking again: a far
    machine that is away costs a dial each time, and the sentence on
    the card does not get truer by being re-fetched every five seconds. */
const LOAD_RETRY_MS = 30_000;

/** Which column the table is ordered by, and which way.
 *
 * **Never absent, and never a third state** (2026-09-03:「默认按 CPU
 * 降序。不存在现在这样,一个列名可以点三次,升、降、空」). A press
 * turns the column over and that is all it does: a control whose third
 * press lands somewhere the reader cannot name is a control that reads
 * as broken. The table opens on CPU, biggest first — the question a
 * person opens a machine card to ask. */
type Sort = { col: SortCol; dir: "up" | "down" };
const SORT_FIRST: Sort = { col: "cpu", dir: "down" };
type SortCol = "pid" | "name" | "user" | "cpu" | "mem" | "gpu" | "vram" | "time" | "status" | "cputime" | "io";

/**
 * **The table's columns, in the order they are drawn, with everything
 * that differs between them written once.**
 *
 * The head word, the track width and the alignment used to live in
 * three places — a list of tracks, a run of `<Column>`s, and a run of
 * cells — and 2026-09-05 the table went from seven columns to eleven.
 * Three lists of eleven kept in agreement by somebody remembering is
 * the arrangement that already failed once at seven: a machine with
 * per-process 显存 and no per-process GPU had six cells meeting five
 * tracks, and 「命令」 collapsed to 55px. **The gate that catches that
 * is still there** (`tracks` against heads) — this is what stops it
 * having to fire.
 *
 * The order is the one the user named (2026-09-05): the pid you type
 * next, then what the process *is* (状态 · 用户), then the two times,
 * then the quantities, and **「命令」 last** because it is the only
 * column whose content has no bound, so it is the one that can absorb
 * width without opening a canyon between two numbers.
 */
const COLS = {
  pid: { head: () => cli.top_pid, w: "var(--cw-pid)", fr: 1, align: "left" },
  status: { head: () => cli.top_status, w: "var(--cw-status)", fr: 1, align: "left" },
  user: { head: () => cli.top_user, w: "var(--cw-user)", fr: 1, align: "left" },
  time: { head: () => cli.top_time, w: "var(--cw-run)", fr: 1, align: "right" },
  cputime: { head: () => cli.top_cputime, w: "var(--cw-cputime)", fr: 1, align: "right" },
  cpu: { head: () => cli.vitals_name_cpu, w: "var(--cw-cpu)", fr: 1, align: "right" },
  gpu: { head: () => cli.vitals_name_gpu, w: "var(--cw-gpu)", fr: 1, align: "right" },
  mem: { head: () => cli.vitals_name_mem, w: "var(--cw-mem)", fr: 1, align: "right" },
  vram: { head: () => cli.vitals_name_vram, w: "var(--cw-vram)", fr: 1, align: "right" },
  io: { head: () => cli.top_io, w: "var(--cw-io)", fr: 1, align: "right" },
  // **The one nudge away from even**, and it is the column the person
  // asking for even shares also called out as the one whose content
  // 「有长有短」. Half again, not four times: at four the table is the
  // old shape wearing a new formula.
  name: { head: () => cli.top_cmd, w: "var(--cw-cmd)", fr: 1.5, align: "left" },
} as const satisfies Record<SortCol, { head: () => string; w: string; fr: number; align: "left" | "right" }>;
const COL_ORDER = ["pid", "status", "user", "time", "cputime", "cpu", "gpu", "mem", "vram", "io", "name"] as const;


/**
 * **Which optional columns this machine has ever filled**, one latch
 * each.
 *
 * Four of the seven are 「an older khor over there does not send this
 * field at all」 and three are 「this machine has nothing to put in
 * it」, and they behave the same way on screen, so they are one
 * mechanism.
 *
 * **The question is 「can any row fill me」, not 「does this machine
 * have the hardware」** — turing reports two RTX A5000s and not one of
 * its two hundred processes carries a per-process figure, so asking
 * about the cards drew two empty columns and spent 136px on them.
 *
 * **And it latches, only ever upward.** Read off the rows in front of
 * you it is a fact about *this page*: order turing by GPU and the two
 * hundred rows that come back can be all-`null`, so pressing 「GPU」
 * deletes the GPU column and the press has no head left to press again
 * (user, 2026-09-04:「一点整个表就变了」). Cleared only when the machine
 * changes.
 */
type Seen = { status: boolean; user: boolean; time: boolean; cputime: boolean; gpu: boolean; vram: boolean; io: boolean };
/** What counts as a row filling each of them.
 *
 *  **`user` is the odd one**: it is a `String` on the wire and its
 *  absence is spelled `""`, because a machine that will not name an
 *  account has no id to send either. So the emptiness has to be read,
 *  not the null. */
const FILLS: { [K in keyof Seen]: (r: Load["top"][number]) => boolean } = {
  status: (r) => r.status !== null,
  user: (r) => r.user !== "",
  time: (r) => r.run_secs !== null,
  cputime: (r) => r.cpu_ms !== null,
  gpu: (r) => r.gpu_pct !== null,
  vram: (r) => r.gpu_mem !== null,
  io: (r) => r.io_bytes !== null,
};
const SEEN_KEYS = Object.keys(FILLS) as (keyof Seen)[];
const SEEN_NONE: Seen = { status: false, user: false, time: false, cputime: false, gpu: false, vram: false, io: false };

/** The word for a status token, or the token itself.
 *
 *  **A token with no word is printed as it stands** — `unknown-13` on
 *  the glass is something a person can look up, and it is the honest
 *  answer for a status a newer node knows about and this build does
 *  not. What it must never do is fall through to 「运行」: the node
 *  goes to some trouble not to say that word wrongly
 *  (`khor_node::top::status_token`), and a lookup with a default would
 *  undo all of it here. */
// ⚠️ **A function, not a `const`** (批 D3): these are catalog reads, and
// a module-level record would keep whichever language the page loaded in
// for the life of the tab.
const statusWord = (): Record<string, string> => ({
  run: cli.top_st_run,
  sleep: cli.top_st_sleep,
  idle: cli.top_st_idle,
  spawning: cli.top_st_spawning,
  diskwait: cli.top_st_diskwait,
  dead: cli.top_st_dead,
  zombie: cli.top_st_zombie,
  stopped: cli.top_st_stopped,
  tracing: cli.top_st_tracing,
  parked: cli.top_st_parked,
  wakekill: cli.top_st_wakekill,
  waking: cli.top_st_waking,
  suspended: cli.top_st_suspended,
  lockblocked: cli.top_st_lockblocked,
  unclear: cli.top_st_unclear,
});

/**
 * Accumulated CPU as a **stopwatch**, where 时长 next to it is a
 * duration in words.
 *
 * The two columns are both times and they are not the same time — one
 * is the wall clock, the other is core-seconds, and a process on four
 * cores has burned four times the first. Printed in the same shape they
 * read as a contradiction (「起了 2 小时,烧了 8 小时」 looks like a bug);
 * printed as 「2小时」 and 「8:03:41」 they read as two kinds of thing,
 * which is what they are.
 */
const cpuClock = (ms: number) => {
  const s = Math.round(ms / 1000);
  const two = (n: number) => String(n).padStart(2, "0");
  const h = Math.floor(s / 3600);
  const m = two(Math.floor((s % 3600) / 60));
  // **Past a hundred hours the seconds go.** hinton's `go.d.plugin`
  // came back at `1374:35:10` — ten characters, wider than the column,
  // and it spilled into the gap beside it (seen on a real card). The
  // seconds on a figure that size are not a reading anybody uses; the
  // magnitude is. Under a hundred hours they stay, because there they
  // are the difference between a process that just started burning and
  // one that has been at it a while.
  if (h >= 100) return `${h}:${m}`;
  return h > 0 ? `${h}:${m}:${two(s % 60)}` : `${Math.floor(s / 60)}:${two(s % 60)}`;
};

/**
 * What the machine is busy with (2026-09-02:「环下面显示进程 top,最近
 * 24 小时 cpu」): the CPU day as a chart, and the process top as a
 * table, one panel each.
 *
 * **Asked by the card, not carried on the row** — the rings read a
 * reading that rides the device beat, but a process table is asked for
 * only while somebody is looking at it, and the band is a day of
 * buckets that has no business on a two-second heartbeat. The ask
 * itself is `useLoad`, one level up, because the dials' 24-hour marks
 * read the same buckets this chart draws.
 *
 * **Three faces, and the empty one is never drawn.** Nothing until the
 * first answer (the ask is out); the chart and the table once it lands;
 * and the refusal's own sentence in their place when it does not — an
 * older khor over there, or a machine that is away. A blank table would
 * read as an idle machine, which is the one thing this surface must not
 * say by accident (docs/SESSION.md 认不出就不落词).
 */
function Busy({
  load,
  why,
  whyDetail = null,
  sort,
  asking,
  onSort,
  seen,
}: {
  load: Load | null;
  why: string | null;
  /** The node's own sentence behind `why`, for the hover. */
  whyDetail?: string | null;
  /** Which optional columns any row on this machine has ever filled.
      **Latched 「seen」 facts, not properties of the hardware or of the
      far khor's version**, and together they decide which columns the
      table has — see `Seen`, `useLoad` and `Top`. */
  seen: Seen;
  /** **The order the rows on screen are in**, not the one just asked
      for — see `useLoad`. */
  sort: Sort;
  /** The column whose answer is still crossing, if any. */
  asking: SortCol | null;
  onSort: (s: Sort) => void;
}) {
  /** Which row a scatter point last asked for, and how many times it
      has asked. See where it is set. */
  const [hot, setHot] = useState<{ pid: number; n: number } | null>(null);
  // **A refusal keeps the column heads and loses the rows.** It used to
  // replace the whole table, heads included, and that made an ordinary
  // mistake unrecoverable: ordering a remote by PID is refused by a khor
  // that predates the column, and with the heads gone there was no
  // second column to press — the only way back out was to open another
  // machine and come back (measured 2026-09-04; two probes timed out
  // reaching for a head that was no longer there).
  //
  // The rows still go. An empty table under live heads would say this
  // machine is running nothing, which is the reading the refusal exists
  // to prevent — so the sentence sits where the rows would be, and the
  // heads stay because pressing one is the way out.
  if (why) {
    return (
      <TopSection>
        <Top rows={load?.top ?? []} sort={sort} asking={asking} onSort={onSort} why={why} whyDetail={whyDetail} seen={seen} hot={null} />
      </TopSection>
    );
  }
  if (!load) return null;
  //   **The day is not a panel of its own any more** (2026-09-04): it
  //   moved into the slot under the four rings, which is where it is
  //   read against them. See `Readings`.
  return (
    <TopSection settings={<TopSettings rows={load.top} />}>
      {/* **The figure first, the table under it** (2026-09-05:「先把
          散点和表再交换下位置」).

          ⚠️ **This is the second time this pair has swapped, and the
          first swap's reason has not gone away — it was solved
          elsewhere.** The figure sat on top originally; it was moved
          below because its two rows of axis words (「横 CPU 内存 显存
          时长 · 纵 …」) sat directly over the table's column heads and
          were read as them (2026-09-04:「上面的散点图的 X 和 y 设置是
          下面表的表头」). **Those two rows no longer exist**: the axis
          words became one word printed on each axis, and the picker
          opens in place (`components/PlotControls`). What is above
          the table's head now is the figure's foot — a sentence, not a
          row of column-shaped words.

          So the order is the reader's to choose again, and they chose
          this one. **If a third swap is ever proposed, the thing to
          check is not the order but whether anything above the table's
          head has gone back to looking like a row of headings.** */}
      {/* **A point knows which row it is** (2026-09-06:「点击散点要定位
          到表格的行,并高亮一瞬」). What travels is the **pid**, never a
          row index: the table's order changes under the reader's hand,
          and an index is an address that goes stale between the press
          and the scroll.

          **The two cannot disagree about which rows exist**: both are
          `load.top`, the same two hundred. The figure draws a subset of
          them (a row missing either reading is not placed — see
          `Scatter`), so it can only ever ask for a row the table has.
          That is why there is no 「not found」 branch here: it is not a
          case being ignored, it is a case the shared source rules out.

          A counter rides with the pid so pressing the same point twice
          is two answers rather than one. */}
      <Scatter rows={load.top} onPoint={(pid) => setHot({ pid, n: (hot?.n ?? 0) + 1 })} />
      {/* **A rule between the figure and the table.** They are one
          block — the same two hundred processes, drawn and then listed
          — so a hairline and `--gap-group` on both sides rather than
          two cards; splitting them would say they are two subjects.

          **`--gap-group` is the token whose whole definition is this
          case** (「between groups that must not read as one list」);
          `--gap-row` was tried first and measured 25px against an 18px
          row, which is a break narrower than twice the rhythm it is
          breaking. */}
      <div data-top-seam className="mt-group border-t pt-group">
        <Top rows={load.top} sort={sort} asking={asking} onSort={onSort} seen={seen} hot={hot} />
      </div>
    </TopSection>
  );
}

/**
 * The processes section: a hairline over it, a small heading row, and
 * the figure and the table under that (2026-09-28, the owner: the
 * machine card's parts are sections on the board, no boxes). **The
 * heading row carries the figure's •••** at its right end — the one
 * control that is about the whole section rather than one axis; it was
 * in the figure's top-right corner.
 *
 * ⚠️ 墓碑 (2026-09-28): this was `components/Panel` — a hairline border,
 * the card ground and `--gap-row` of inset, a box on the board. The
 * seam between the figure and the table stays: it is 2026-09-04's
 * 「表头要明显」, a rule inside one section, not a box around it. The
 * 额度 card's usage half has the same two sections (`UsageHalf`).
 */
function TopSection({ settings, children }: { settings?: ReactNode; children: ReactNode }) {
  return (
    <section data-machine-section="top" className="flex flex-col border-t pt-row">
      <SectionHead title={gui.top_section}>{settings}</SectionHead>
      {children}
    </section>
  );
}

/**
 * **One button for what the figure is set to** (2026-09-08: 「都做成一个
 * 小的设置按钮,点击后进行选择」), at the right end of the section's
 * heading since 2026-09-28. **Three rows: X, Y, and the dot's area.**
 * The third one was the open question, and what settled it is a
 * measurement: `gpu_mem` is null on 200 of 200 rows on two of the three
 * machines (`zh.toml`'s `top-plot-off` records it), and an area encoding
 * has no 「do not draw」 — a null reading can only land on the floor,
 * which reads as 「the smallest」. So the area row offers only what
 * **every** row on this screen reports (`everyRowHas`), while an axis
 * row offers everything. **Nobody maintains a list** — the day a
 * machine reports video memory for every process, 显存 becomes
 * offerable on its own. The figure reads the same store (`useAxes`).
 */
function TopSettings({ rows }: { rows: Load["top"] }) {
  const axes = useAxes();
  const areaKeys = everyRowHas(AXIS_KEYS, rows, (k, r) => AXES[k].of(r));
  return (
    <PlotSettings
      rows={[
        { key: "x", word: "X", chosen: axes.x, options: AXIS_KEYS.map((k) => ({ key: k, word: AXES[k].word() })) },
        { key: "y", word: "Y", chosen: axes.y, options: AXIS_KEYS.map((k) => ({ key: k, word: AXES[k].word() })) },
        { key: "z", word: gui.usage_pick_area, chosen: axes.z, options: areaKeys.map((k) => ({ key: k, word: AXES[k].word() })) },
      ]}
      onPick={(row, k) => axesStore.set({ ...axes, [row]: k as AxisKey })}
    />
  );
}

/**
 * The process table: what is eating this machine, by any column.
 *
 * # A window with a fixed height, not a fold
 *
 * Every process this machine is running is in here, and the box is a
 * fixed height that scrolls itself (2026-09-03:「就不要展开了,就是一个
 * 固定相对高度的窗口,鼠标在里面可以直接拖」). Both alternatives were
 * worse: a folded table hides how much there is, and an unfolding one
 * drops four hundred rows between the card's head and everything below
 * it — so the control that folds it back ends up a long scroll away
 * from the person who wants it. The height is `--top-h`, which carries
 * the rest of that reasoning.
 *
 * The head stays put while the rows move under it: a column head you
 * have to scroll back up to read is a sort control you cannot reach
 * from where the sorting matters.
 *
 * # A column of numbers with holes in it
 *
 * GPU and 显存 are `None` on every machine that cannot report them
 * (macOS reports no per-process GPU at all) and on every process that
 * is not on a card. **The two columns disappear when nothing on this
 * machine can fill them** — a column of blanks says "khor looked and
 * found nothing", which is the opposite of what happened.
 *
 * # Sorting puts absence last, both ways
 *
 * A row whose GPU is unknown is not a row with 0% GPU, so it does not
 * take a zero's place in the order. It sinks in both directions, which
 * is the only arrangement where reversing the sort does not turn "we
 * did not look" into "this is the smallest".
 */
function Top({
  rows,
  sort,
  asking,
  onSort,
  why,
  whyDetail = null,
  seen,
  hot,
}: {
  rows: Load["top"];
  /** The order these rows are actually in — see `useLoad`. */
  sort: Sort;
  /** The column whose answer has not landed yet. Its head says so, and
      **nothing else on the table moves until the rows do.** */
  asking: SortCol | null;
  onSort: (s: Sort) => void;
  /** Why the machine did not answer this ask. Set means: draw the heads
      and this sentence, and no rows — see `Busy`. */
  why?: string | null;
  /** The node's own sentence behind `why`, for the hover. */
  whyDetail?: string | null;
  /** **Which columns this table has, decided by the machine and not by
      the rows in front of you** — see `Seen`.

      It used to be two props (`cards` and `ages`) and before that
      `rows.some(r => r.gpu_pct !== null)` read inline here, which made
      the column set a function of the current answer.

      The old note, kept because it is what the shape costs:

      It used to be `rows.some(r => r.gpu_pct !== null)`, and that made
      the column set a function of the current answer: order turing by
      GPU and the two hundred rows that come back can be all-`null`, so
      **pressing 「GPU」 deleted the GPU column** — the press had no head
      left to press again (user, 2026-09-04:「一点整个表就变了，比如
      GPU，一点就没有GPU这列了」). A column that appears and disappears
      with the ordering is not a column, it is a reading of the page.

      So it comes from above and it latches — see `useLoad`. */
  seen: Seen;
  /** The row a scatter point asked for, with a counter so the same
      point pressed twice arrives twice. `null` while no point has been
      pressed, and on the surface that has no figure. */
  hot: { pid: number; n: number } | null;
}) {
  // **A column exists when a row on this machine has ever filled it.**
  // A whole column of blanks is worse than no column: it takes width
  // from the ones that speak, and it reads as 「khor looked and found
  // nothing」, which is not the same as 「nothing here runs on a card」.
  // Latched in `useLoad`, so an ordering cannot take a column away.
  //
  // **One list, and the heads, tracks and cells are all read off it.**
  // See `COLS` for why that is not tidiness.
  const shown = COL_ORDER.filter((k) => !(k in FILLS) || seen[k as keyof Seen]);
  const ordered = sortRows(rows, sort);
  // **The top rows, and 显示全部** (the audit fix plan F4.3; W15, N06): the
  // table flows in the page with no scroller of its own (a scroller in a
  // scroller is where a finger gets caught), so it shows the first
  // `TOP_SHOWN` until asked for the rest.
  const [all, setAll] = useState(false);
  const drawn = all ? ordered : ordered.slice(0, TOP_SHOWN);
  const box = useRef<HTMLDivElement>(null);
  const grid = useRef<HTMLDivElement>(null);
  // **The table stacks when its tracks do not fit, and that is asked
  // of the browser** rather than of a constant — the column count runs
  // from seven to eleven, so the width where it stops fitting is not a
  // number anybody can write down once (`useOverflows` carries what
  // writing it down cost). It is asked of the table's own box and not
  // the window's: the card has a rail and a list beside it, and a
  // window breakpoint once left a 106px overflow band invisible to any
  // two-point check.
  const tight = useOverflows(box, grid, shown.join(","));
  // **Go to the row a point asked for, and say which one it was.**
  //
  // ⚠️ **Scrolled by the cell, not the row.** A row here joins the grid
  // with `display: contents` and therefore has no box at all —
  // `getBoundingClientRect()` on one returns zeros, and a scroll
  // computed from that goes to the top of the table every time. The
  // first cell is a real box.
  //
  // ⚠️ **And it stops below the head, not under it.** The heads are
  // sticky, so a row scrolled to the top of the scroll port lands
  // *behind* them — the classic version of this bug, and one that never
  // shows on a table short enough not to scroll.
  useEffect(() => {
    if (!hot) return;
    // A point past the top rows asks for all of them first.
    if (!all && !drawn.some((r) => r.pid === hot.pid)) {
      setAll(true);
      return;
    }
    const b = box.current;
    const row = b?.querySelector(`[data-top-row="${hot.pid}"]`);
    const cell = row?.firstElementChild;
    if (!b || !row || !cell) return;
    // The page scrolls now, not the table (F4.3): the row's first cell
    // into the middle of the view.
    cell.scrollIntoView({ block: "center" });
    row.setAttribute("data-hot", "");
    // **Reduced motion gets a longer, still highlight, not nothing.**
    // The stylesheet drops the fade; if the mark also vanished at the
    // same instant, the one reader who most needs a static cue would
    // get no cue at all.
    const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const t = window.setTimeout(() => row.removeAttribute("data-hot"), still ? 2400 : 1200);
    return () => {
      window.clearTimeout(t);
      row.removeAttribute("data-hot");
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hot, all]);
  const press = (col: SortCol) => {
    // Pressing the column it is already on turns it over. Pressing a
    // new one starts the way that column is read — biggest first for a
    // quantity, A to Z for a name.
    if (sort.col === col) {
      onSort({ col, dir: sort.dir === "down" ? "up" : "down" });
      return;
    }
    // A first press reads the column the way that column is usually
    // read: A to Z for a name, smallest first for a pid — and **longest
    // first for an age**, because 「哪个挂了最久」 is the question, not
    // 「哪个刚起」.
    onSort({ col, dir: col === "name" || col === "user" || col === "pid" ? "up" : "down" });
  };
  const cells = shown.length;
  // **The tracks, in the order the cells are written below** — the same
  // list, so they cannot disagree. Only the order and the share are
  // here; every floor is a token.
  //
  // **`minmax(floor, share fr)`, not `floor` with the leftover dumped
  // on the last column** (2026-09-05:「我们要不按页面宽度均分列宽。或
  // 稍微微调一些,大致是均分」). The old arrangement gave every column
  // exactly its floor and handed the whole remainder to 「命令」, which
  // on a wide card left that one column holding hundreds of pixels of
  // blank while the numbers were crammed at the left.
  //
  // Each track keeps its floor — the head's own word plus the sort
  // chevron, and the widest reading that column has actually shown — and
  // the free space above the floors is shared. **The floors are what
  // stop 「even」 from becoming 「too narrow to read」**: a share smaller
  // than a column's floor simply does not apply, and a table narrower
  // than the floors added up stops being a grid at all (`useOverflows`).
  // **Sized to content** (the audit fix plan F4.3; W15): every column is
  // as wide as its widest reading, over its floor, and 命令 takes what is
  // left — the even shares stood 95 apart whatever was in them, with 读写
  // running into the command. ⚠️ 墓碑: `minmax(floor, share fr)`.
  const tracks = shown.map((k) => (k === "name" ? `minmax(${COLS[k].w}, 1fr)` : `minmax(${COLS[k].w}, max-content)`)).join(" ");
  return (
    <div
      ref={box}
      data-load-top-box
      data-tight={tight ? "" : undefined}
    >
      {/* `data-top-cols` is **which columns this table actually has, in
          order**, and `data-top-frs` the share each was given — so a
          gate can check the division without keeping its own copy of
          the weights. — one attribute rather than a flag per absent column.
          The three it replaces (`data-nogpu` / `data-novram` /
          `data-norun`) were left over from the four hand-written grid
          variants that a generated track list retired; nothing read
          them any more, and at eleven columns the flag-per-column shape
          would have been seven of them. */}
      <div
        ref={grid}
        role="table"
        data-load-top
        data-panel
        data-top-cols={shown.join(" ")}
        data-top-frs={shown.map((k) => `${k}:${COLS[k].fr}`).join(" ")}
        style={{ gridTemplateColumns: tracks }}
        className="text-callout tabular-nums"
      >
        <div data-top-heads role="row" className="bg-card text-callout text-muted-foreground">
          {/* **The order the user named** (2026-09-05:「PID · 状态 ·
              USER · 时长 · CPU时间 · CPU · GPU · 内存 · 显存 · 读写 ·
              命令」): the pid you type next, then what the process *is*,
              then the two times, then the quantities — the two GPU ones
              beside the CPU rather than after the memory that has
              nothing to do with them — and **「命令」 last**.

              **Last is what makes the table fill without a canyon.**
              「命令」 is the only column whose content has no bound, so
              it is the one the leftover width goes to; while it sat
              second, that width opened **between the name and the
              numbers** — measured 700+px on turing at 1500, which is
              the same visual event as the 799px hole this card was
              rebuilt to remove. At the end, the same slack lands inside
              it and runs to the table's own right edge, and no two
              columns are pushed apart at all.

              **One column sorts by a word that is not its own**:
              「命令」 carries a process's short name and its identifying
              word together and orders by the name, which is the text
              the column starts with. Its head is `cmd` in the DOM
              because the column is; the sort is `name` because that is
              what `khor_core::TopBy` knows. */}
          {shown.map((k) => (
            <Column
              key={k}
              col={k}
              head={k === "name" ? "cmd" : undefined}
              sort={sort}
              asking={asking}
              onPress={press}
              align={COLS[k].align}
            >
              {COLS[k].head()}
            </Column>
          ))}
        </div>
        {why ? (
          <div
            data-load-why
            title={whyDetail && whyDetail !== why ? whyDetail : undefined}
            className="py-row text-callout text-muted-foreground"
            style={{ gridColumn: `span ${cells}` }}
          >
            {why}
          </div>
        ) : (
          drawn.map((r) => (
            <div key={r.pid} role="row" data-top-row={r.pid}>
              {shown.map((k) =>
                k === "name" ? (
                  /* Last, and it takes the table's leftover width —
                     see the head. */
                  <Cut key={k} name={r.name} arg={r.arg} />
                ) : (
                  <span
                    key={k}
                    role="cell"
                    data-c={k}
                    {...{ [`data-top-${k}`]: "" }}
                    // **The one cell that can say more than it shows.**
                    // A status this build has no word for prints its
                    // token, and a token is longer than the column
                    // (`unknown-13` against 「休眠」) — the number in it
                    // is the whole reason it was kept, so it must be
                    // readable somewhere even when the cell clips.
                    title={k === "status" && r.status !== null ? r.status : undefined}
                    // The quantity's name, which the narrow row writes
                    // before its value (`app.css` `[data-tight]`): there
                    // is no head row to say which number is which.
                    data-label={k === "cpu" || k === "gpu" || k === "mem" || k === "vram" ? COLS[k].head() : undefined}
                    className={cn(
                      "whitespace-nowrap",
                      COLS[k].align === "right" ? "text-right" : "text-left",
                      // **The pid is muted, and it is the only cell
                      // that is.** It is an identifier rather than a
                      // reading — nobody compares the magnitudes of two
                      // pids — so it stands back from the numbers that
                      // are being compared. (It is also what the head's
                      // 「heavier than every cell」 gate measures
                      // against, which is why that gate had to stop
                      // sampling 「the first cell」 and start sampling
                      // the loudest one.)
                      k === "pid" && "text-muted-foreground",
                      // The one column whose content has no bound and
                      // no room: an account name runs off the end, so
                      // it is cut rather than allowed to push the
                      // numbers over.
                      (k === "user" || k === "status") && "overflow-hidden text-ellipsis",
                    )}
                  >
                    {cellOf(k, r)}
                  </span>
                ),
              )}
            </div>
          ))
        )}
      </div>
      {!why && !all && ordered.length > TOP_SHOWN && (
        <button
          type="button"
          data-top-all
          onClick={() => setAll(true)}
          className="flex min-h-touch items-center rounded-sm text-callout text-muted-foreground hover:text-foreground"
        >
          {gui.top_show_all(ordered.length)}
        </button>
      )}
    </div>
  );
}

/**
 * **The 「命令」 text, in one place**, because two places drift.
 *
 * The table's cell and the scatter's hover both say what a process is,
 * and if each assembled its own string the day would come when one row
 * reads `python3` on the dot and `python3 train.py` in the table — an
 * inconsistency no gate would catch and a reader would spot at once.
 *
 * **The word is not drawn twice**: `arg` repeats `name` for the vLLM
 * workers (`argv[0]` is `VLLM::EngineCore` for both), and printing it
 * twice would spend a column saying one thing.
 */
function cmdText(r: Load["top"][number]): string {
  return r.arg && r.arg !== r.name ? `${r.name} ${r.arg}` : r.name;
}

/**
 * What one cell says, for every column but 「命令」 (which is a run of
 * text with its own truncation — see `Cut`).
 *
 * **An empty string is 「读不到」 and it is deliberate.** Every one of
 * these that can be absent prints nothing rather than a zero: a `0%`
 * where the machine never looked is the neighbouring-value mistake, and
 * on this table it would be sorted, compared and believed.
 */
function cellOf(k: Exclude<SortCol, "name">, r: Load["top"][number]): string {
  switch (k) {
    case "pid":
      return String(r.pid);
    // A token this build has no word for prints as the token — never
    // as a neighbour's word. See `statusWord`.
    case "status":
      return r.status === null ? "" : (statusWord()[r.status] ?? r.status);
    case "user":
      return r.user;
    case "time":
      return r.run_secs === null ? "" : lasting(r.run_secs * 1000);
    case "cputime":
      return r.cpu_ms === null ? "" : cpuClock(r.cpu_ms);
    case "cpu":
      return `${Math.round(r.cpu_pct)}%`;
    case "gpu":
      return r.gpu_pct === null ? "" : `${Math.round(r.gpu_pct)}%`;
    case "mem":
      return bytes(r.mem);
    case "vram":
      return r.gpu_mem === null ? "" : bytes(r.gpu_mem);
    // Absent on most rows of a shared box, because khor may not read
    // another account's — and absent is not zero
    // (`khor_node::top::io_readable`).
    case "io":
      return r.io_bytes === null ? "" : bytes(r.io_bytes);
  }
}

/**
 * The four quantities either axis of the scatter can be, and how each
 * one is read off a row and written on a tick.
 *
 * **The same four the table has columns for** (2026-09-04:「表要可以
 * 设定 x 和 y 轴」), because those are the quantities a person is
 * already looking at when they decide they want to see two of them
 * against each other.
 *
 * `of` returns `null` for 「did not read」, and a row that answers
 * `null` on either axis **is not drawn** — see `Scatter`.
 */
const AXES = {
  //   **A tick's precision is decided beside its neighbours, not here.**
  //   `tick(v, d)` writes the reading with `d` decimals and `tickSet`
  //   (in `Scatter`) picks `d` per tick — the fewest that keep its
  //   label apart from every other label on that axis, and that keep a
  //   reading above zero from printing as zero. Fixed rules were tried
  //   twice and failed the same way twice: `Math.round` gave four ticks
  //   reading 「0%」 on a log axis, and 「two decimals under 1」 gave
  //   `0.00% 0.01% 0.01% 0.02%` once the axis started at the data's own
  //   minimum (2026-09-06, on this Mac). Two labels that read the same
  //   over two different positions break the one promise a tick makes.
  //
  //   时长 ignores `d`: its words are a ladder of units (秒/分钟/小时/天)
  //   with no decimals to add, so two ticks inside one rung print the
  //   same word — and `tickSet` drops the second rather than draw it.
  //   **Only there** (`fold`): on an axis whose formatter takes
  //   decimals, two ticks with one label is a broken formatter, and
  //   folding them would hide it behind a missing tick (measured
  //   2026-09-06 — the gate's duplicate arm could not fire while the
  //   fold applied everywhere).
  cpu: {
    word: () => cli.vitals_name_cpu,
    of: (r: Load["top"][number]) => r.cpu_pct,
    tick: (v: number, d: number) => `${v.toFixed(d)}%`,
  },
  mem: { word: () => cli.vitals_name_mem, of: (r: Load["top"][number]) => Number(r.mem), tick: (v: number, d: number) => bytes(v, d), binary: true },
  vram: { word: () => cli.vitals_name_vram, of: (r: Load["top"][number]) => r.gpu_mem, tick: (v: number, d: number) => bytes(v, d), binary: true },
  time: { word: () => cli.top_time, of: (r: Load["top"][number]) => r.run_secs, tick: (v: number) => lasting(v * 1000), fold: true },
} as const;

/** A log ruler's ticks at its decades — 10ⁿ inside the data's span, with
    the two ends — each label at the decimals the decade needs. */
function decades(
  r: { lo: number; hi: number },
  a: { tick: (v: number, d: number) => string; binary?: boolean },
): { v: number; label: string }[] {
  const vs = new Set<number>([r.lo, r.hi]);
  // **A byte axis ticks in its own units** (W-V8): at powers of 16 —
  // 1M, 16M, 256M, 4G — which print as round binary values; a decade of
  // bytes printed in KiB / MiB came out 977K and 95M.
  const base = a.binary ? 16 : 10;
  if (r.hi > 0) {
    const from = Math.floor(Math.log(Math.max(r.lo, r.hi / 1e6)) / Math.log(base));
    for (let k = from; base ** k <= r.hi; k += 1) {
      const v = base ** k;
      if (v > r.lo && v < r.hi) vs.add(v);
    }
  }
  return [...vs]
    .sort((x, y) => x - y)
    .map((v) => ({ v, label: a.tick(v, v === 0 ? 0 : Math.max(0, Math.min(6, -Math.floor(Math.log10(v))))) }));
}

/** Drops the labels that would overlap the one kept before them (the
    tick mark stays): along x by the label's measured-ish width, along y
    by a line of tick text. */
function thin(ts: { v: number; label: string }[], at: (v: number) => number, axis: "x" | "y") {
  const room = (t: { label: string }) => (axis === "x" ? t.label.length * PLOT.tick * 0.62 : PLOT.tick + 3);
  let edge = -Infinity;
  const sorted = [...ts].sort((p, q) => (axis === "x" ? at(p.v) - at(q.v) : at(q.v) - at(p.v)));
  const keep = new Set<number>();
  for (const t of sorted) {
    const pos = axis === "x" ? at(t.v) : -at(t.v);
    const half = room(t) / 2;
    if (pos - half >= edge + 4) {
      keep.add(t.v);
      edge = pos + half;
    }
  }
  return ts.map((t) => (keep.has(t.v) ? t : { ...t, label: "" }));
}

/* ⚠️ **Tombstone: `tickSet` was defined here until 2026-09-08.** It is
   `lib/ruler` now, unchanged, because the 额度 page's usage half needs
   the same tick precision and a second copy would only be a second
   place for one rule to rot. **The `ruler` beside it did not move** —
   that one has a judgment in it and the two pages do not agree about
   it yet; `lib/ruler`'s head carries the divergence and the number
   that makes it checkable. */
type AxisKey = keyof typeof AXES;
const AXIS_KEYS = Object.keys(AXES) as AxisKey[];

/**
 * **How one axis turns a reading into a position, and which ruler it
 * had to use to do it.**
 *
 * 2026-09-04:「因 X 或 y scale 不一无法绘制出有意义的图的时候,用 log 或
 * 什么方法非线性的绘制呀」.
 *
 * # The measurement that decides, so it is not switched unconditionally
 *
 * `max / median`. Under a square root a ratio `R` puts the median at
 * `1/√R` of the axis, so **`R > 44` is exactly 「half the points are in
 * the first 15%」** — the shape of a picture that answers nothing. It is
 * measured on the readings actually being drawn, not assumed from the
 * quantity: on the live fleet CPU comes out 400–1600 on all three
 * machines, while 内存 is 16 / 34 / 52 and 时长 is 5.5 / 803 / 10.3.
 * **Two of the four are degenerate on one machine and not on another**,
 * which is why the ruler is printed on the axis rather than assumed.
 *
 * A median of 0 is `R = ∞` and takes the log branch: half the readings
 * at zero is the worst case of the same problem, and √ cannot separate
 * the other half either.
 *
 * # Why `log1p` of a ratio, and not `log`
 *
 * **`log 0` is undefined and 0 is the commonest value in this table**
 * (three machines' CPU medians are 0.02 / 1.6 / 0.12 against maxima in
 * the tens and thousands, and plenty of rows read exactly 0). `log1p`
 * is defined there and **`log1p(0) = 0`, so zero is drawn at zero** —
 * no `v + 1` fudge, no ruler whose labels lie about where they sit.
 *
 * The argument is `v / s`, not `v`: `log1p` only compresses above 1, so
 * on a quantity whose values are all below 1 (CPU percent) it is
 * indistinguishable from linear. `s` is the **smallest positive reading
 * on this axis** — the data's own resolution — floored at `max / 1e6`
 * so a single tiny outlier cannot stretch the ruler. Measured with it,
 * the three machines' CPU medians land at 0.219 / 0.221 / 0.176 of the
 * axis instead of 0.025 / 0.027 / 0.05.
 *
 * # Ticks
 *
 * **On the square root the ticks are even fractions of the value** and
 * the transform decides where they sit, so they crowd towards the top —
 * and that crowding is the evidence that the ruler is not linear.
 *
 * **On the log they are even fractions of the axis** and the *labels*
 * come out uneven (0, 0.03%, 0.2%, 1.3%, 8%, 47%). The same evidence,
 * on the other channel, and it has to be that way round here: even
 * value fractions on a log axis put every tick but the first in the top
 * fifth, leaving the stretch where all the points are unlabelled.
 * Either way **a tick's label is a real reading**, never a transformed
 * number.
 */
function ruler(vals: number[]): {
  kind: "sqrt" | "log";
  /** The data's two ends — the first and last tick, and the two
      readings a point can sit exactly on. */
  lo: number;
  hi: number;
  at: (v: number) => number;
  tick: (f: number) => number;
} {
  // **The axis runs from the smallest reading to the largest, not from
  // zero** (2026-09-06:「散点图 x 轴 0% 右侧空了一大截,应该根据数据的最大
  // 最小值自适应调整 x 和 y 的范围」). On a log ruler the stretch between
  // 0 and the first reading is where nothing can ever be — every point
  // is at or above the minimum by definition — and that stretch was
  // the user's 「空了一大截」. So `lo` is the minimum and every position below
  // is measured from it: `at(lo) = 0`. The choice of ruler is untouched
  // — `max / median` on the raw readings, exactly as before — because
  // that is a fact about the spread, not about where the axis starts.
  //
  // **A table where every reading is the same has no spread to draw**,
  // and an axis from `v` to `v` would put every tick on one label; it
  // falls back to `[0, v]` (or `[0, 1]` for all-zero), which is the old
  // picture and an honest one for that table.
  const max = Math.max(...vals, -Infinity);
  const min = Math.min(...vals, Infinity);
  const same = !(max > min);
  const hi = same ? Math.max(1e-9, max) : max;
  const lo = same ? 0 : min;
  const sorted = [...vals].sort((a, b) => a - b);
  const med = sorted.length ? sorted[Math.floor(sorted.length / 2)] : 0;
  const wide = !(med > 0) || hi / med > 44;
  if (!wide) {
    return {
      kind: "sqrt",
      lo,
      hi,
      at: (v) => Math.sqrt(Math.max(0, v - lo) / (hi - lo)),
      tick: (f) => lo + (hi - lo) * f,
    };
  }
  // **A true log** (W-V8), so the decades the axis marks stand evenly
  // apart, as a ruler that says 「(log)」 promises. It starts at the
  // smallest reading above zero; a reading of zero (an idle process's
  // CPU) is drawn at the axis's start, where nothing smaller can be.
  // ⚠️ 墓碑: `log1p((v − lo) / s)`, which bent near the start — decades
  // at 150 / 136 / 135 px under a word that said log.
  const floor = Math.max(Math.min(...vals.filter((v) => v > 0), hi), hi / 1e9);
  const span = Math.log(hi / floor) || 1;
  return {
    kind: "log",
    lo: floor,
    hi,
    at: (v) => (v <= floor ? 0 : Math.log(v / floor) / span),
    // The value whose position is `f` — so the label is a real reading
    // and the tick is where that reading belongs.
    tick: (f) => floor * Math.exp(f * span),
  };
}

/**
 * Which pair of axes the scatter is drawn against — **one pair for
 * every machine**, which is why it is not `useState` inside the card.
 *
 * A card unmounts when somebody opens a different machine, so state
 * kept in it would reset to the default on every hop, and three cards
 * read side by side would be three different pictures. **A silent
 * change of axis is the widest door to a misreading there is** and this
 * component's own history is the proof: the pair before this one fell
 * back to memory on a machine with no card, and nothing said so.
 *
 * Deliberately not persisted to disk. It is a way of looking, not a
 * setting, and it costs one press to get back.
 */
const axesStore = {
  // **Three now, not two** (2026-09-08): the dot's area is a reading
  // too. `mem` is the opening choice because it is one of the two
  // quantities every process on every machine reports — see
  // `everyRowHas`, which is also what decides whether a quantity may be
  // offered here at all.
  pair: { x: "cpu" as AxisKey, y: "mem" as AxisKey, z: "mem" as AxisKey },
  subs: new Set<() => void>(),
  get() {
    return this.pair;
  },
  set(next: { x: AxisKey; y: AxisKey; z: AxisKey }) {
    if (next.x === this.pair.x && next.y === this.pair.y && next.z === this.pair.z) return;
    this.pair = next;
    this.subs.forEach((f) => f());
  },
  subscribe(f: () => void) {
    this.subs.add(f);
    return () => {
      this.subs.delete(f);
    };
  },
};
function useAxes() {
  return useSyncExternalStore(
    (f) => axesStore.subscribe(f),
    () => axesStore.get(),
  );
}

/**
 * Every process as a point, on **two axes the reader picks**, with card
 * memory as the halo around it.
 *
 * # Why the pair defaults to CPU and memory
 *
 * The pair this replaced was CPU against card memory, and it had two
 * faults that only show on a real fleet. On turing the two axes were
 * filled by **two non-overlapping sets of processes** — the compilers
 * are not on a card, so their `gpu_mem` is `None` — and a scatter whose
 * axes describe different populations is not read as a scatter. And a
 * machine with no card had no second axis at all, so it quietly became
 * memory: **a silent change of axis, which is the widest door to a
 * misreading there is.**
 *
 * CPU and memory exist for every process on every machine, so as a
 * default the three cards can be read side by side and every row is on
 * the picture. **The reader may now choose otherwise, and that is not
 * the same thing**: a chosen axis is on screen in the foot, and the
 * count of rows it left out is printed beside it. What was forbidden is
 * the app changing an axis on its own.
 *
 * # A row with nothing to say on an axis is not drawn at zero
 *
 * 显存 and 时长 are not readings every process has. Measured on the
 * live fleet: **200 of 200 rows on Mac-mini and on hinton carry no card
 * memory at all, and 3 of 200 do on turing.** Drawing those at 0 would
 * put 「没看到」 exactly where 「最小的那个」 goes, which is the
 * neighbouring-value mistake this whole card is shaped to avoid. So
 * they are left out — **and the number left out is printed**, because a
 * picture missing two thirds of its points and a machine that is idle
 * look identical.
 *
 * # One ruler for all four
 *
 * Square root, on whichever quantity is chosen. **Not because √ centres
 * these distributions — it does not** — but because the alternative is
 * a ruler that changes with the axis, which is the silent change again.
 * Measured 2026-09-04, where the median lands along a √ axis:
 *
 *     axis     Mac-mini   hinton   turing
 *     CPU         0.025    0.027    0.05
 *     内存        0.171    0.249    0.139
 *     时长        0.426    0.035    0.311
 *
 * 时长 is the one worth checking, since it spans five orders of
 * magnitude — and it comes out **no worse than CPU**, the axis this
 * picture already shipped with. (Linear, for comparison, puts CPU's
 * median at 0.0006.)
 *
 * `√` and not `log` is structural rather than a matter of taste:
 * **`log 0` is undefined and 「0」 is the commonest value here.** A log
 * axis has to be `log(v+1)`, and then the ruler is no longer the one
 * its own labels claim. `√0 = 0`, so zero is drawn at zero.
 *
 * # Card memory is the halo, and area is the encoding
 *
 * Radius by square root, so **area** is proportional to bytes — the
 * codex's rule for encoding a quantity as a size. The foot prints the
 * two largest readings and the pixel radii they produced, so anybody
 * can check on the spot that the two are told apart rather than taking
 * it on faith.
 *
 * **The halo stays 显存 even when 显存 is one of the axes.** It then
 * says one thing twice, which is redundant and harmless. The two
 * alternatives are not: swapping the halo to some other quantity, or
 * dropping it, would both be **the app changing an encoding without
 * being asked** — the exact failure this component's first version was
 * rewritten for.
 *
 * **Three states, three appearances, never merged**: memory on the card
 * draws a halo; exactly zero draws a dashed ring (「looked, it is
 * zero」); nothing to read draws nothing at all (「did not look」). The
 * middle one is why they cannot be merged — a card that is idle and a
 * machine that cannot see cards are different facts.
 */
function Scatter({ rows, onPoint }: { rows: Load["top"]; onPoint: (pid: number) => void }) {
  const axes = useAxes();
  // **The picture is drawn in real pixels, not in a stretched box.**
  // It used to carry `preserveAspectRatio="none"`, which stretches the
  // viewBox to the element — and a circle under a non-uniform stretch
  // is an ellipse. The halo is an *area*, and the foot promises a
  // radius in pixels; both were false by however much the box was
  // wider than it was tall. So the viewBox tracks the element's own
  // size and one unit is one pixel.
  //
  // **That decision is `lib/plot` now, and so are the numbers below** —
  // the 額度 card draws the same kind of figure and was drawing it in a
  // scaled box with its own constants (2026-09-08:「二者统一出一个绘图
  // 方法」). What is still this card's own is the box's *height*.
  const { box, w: W, h: H } = usePlotBox();
  const padL = PLOT.padL;
  // **The two label strips are paid for here.** Tucking the axis labels
  // inside the drawing put them, at first, exactly where the tick
  // numbers live — 「内存」 straight through 「3.8G」 and 「CPU」 across
  // 「38%」 (seen on turing, and the reason this is not four numbers it
  // used to be). A corner inside the *data* area was the other option
  // and it trades one collision for a worse one: points can be
  // anywhere, tick numbers cannot. So each axis gets a thin strip of
  // its own, outside the plotting area and outside the tick gutters —
  // 10px at the top for the vertical label, 12 at the bottom for the
  // horizontal one. That is 22px of the drawing's 176, against the
  // 104px those two words cost as rows in the flow.
  const padB = PLOT.padB;
  const padT = PLOT.padT;
  const padR = PLOT.padR;
  const ax = AXES[axes.x];
  const ay = AXES[axes.y];
  // **Which quantities this screen may offer as an area** — the ones
  // every row has a reading for. See `everyRowHas` for why this is
  // stricter than the rule that decides whether a column is drawn.
  const areaKeys = everyRowHas(AXIS_KEYS, rows, (k, r) => AXES[k].of(r));
  // **The rows this picture can actually place.** Both readings, or the
  // row is not on it — see the head.
  const placed = rows
    .map((r) => ({ r, x: ax.of(r), y: ay.of(r) }))
    .filter((p): p is { r: Load["top"][number]; x: number; y: number } => p.x !== null && p.y !== null);
  const off = rows.length - placed.length;
  const rx = ruler(placed.map((p) => p.x));
  const ry = ruler(placed.map((p) => p.y));
  // **The third quantity is the dot's own area now** (2026-09-08), and
  // which quantities may be offered as one is decided by the data —
  // see `everyRowHas`. `az` is guaranteed a reading on every placed row
  // by that filter, so a floor-sized dot here means 「the smallest」 and
  // never 「nobody said」.
  const az = AXES[areaKeys.includes(axes.z) ? axes.z : (areaKeys[0] ?? "cpu")];
  const zs = placed.map((p) => az.of(p.r)).filter((v): v is number => v !== null && Number.isFinite(v));
  const zmax = zs.length > 0 ? Math.max(...zs) : 0;
  const span = dotSpan(placed.length, W - padL - padR, H - padT - padB);
  const dotR = (r: Load["top"][number]) => dotRadius(az.of(r) ?? 0, zmax, span);
  // **A margin at both ends of each axis**, in pixels rather than as a
  // fraction of the ruler: the two extreme readings sit *exactly* on
  // `at = 0` and `at = 1` now that the axis is the data's own span, and
  // a dot drawn on the frame line is half a dot. Eight pixels clears
  // the dot and its press target (radius 1.8 + a stroke of 11 / 2); the
  // largest halo (`RMAX`, 11) can still cross the frame by three, and
  // that is a halo — an area around a point — not the point. The ticks
  // share the margin (they are readings too), which is what keeps the
  // first tick under the leftmost point instead of eight pixels to its
  // left.
  const inset = PLOT.inset;
  const sx = (v: number) => padL + inset + rx.at(v) * (W - padL - padR - 2 * inset);
  const sy = (v: number) => H - padB - inset - ry.at(v) * (H - padT - padB - 2 * inset);
  // **Ticks at round fractions of the real value**, placed where the
  // root puts them — so they crowd together towards the right, and
  // that crowding is itself the evidence that the ruler is not linear.
  // An axis that looks evenly spaced while a transform hides underneath
  // is the one that misleads.
  const steps = tickFractions();
  // **True log ticks, and never two labels on top of each other** (the
  // audit fix plan F4.3; W15, N06): a log ruler is labelled at its
  // decades (0.01%, 0.1%, 1%…) rather than at even fractions of the axis,
  // whose labels read uneven over even spacing; and a label that would
  // overlap the one kept before it is dropped, its tick mark kept.
  const xTicks = thin(rx.kind === "log" && !("fold" in ax) ? decades(rx, ax) : tickSet(rx, ax, steps), sx, "x");
  const yTicks = thin(ry.kind === "log" && !("fold" in ay) ? decades(ry, ay) : tickSet(ry, ay, steps), sy, "y");
  return (
    <div
      data-top-plot
      data-plot-axes={`${axes.x}x${axes.y}`}
      data-plot-scale-x={rx.kind}
      data-plot-scale-y={ry.kind}
      // The two ends of each axis, as the readings they are — what the
      // gate compares against the data's own minimum and maximum.
      data-plot-x-lo={rx.lo}
      data-plot-x-hi={rx.hi}
      data-plot-y-lo={ry.lo}
      data-plot-y-hi={ry.hi}
      className="flex flex-col gap-in"
    >
      {/* **Both labels are tucked inside the axes** (2026-09-06:「X轴
          label太占空间了。以合适的方式显示x和y label,比如贴在坐标轴内侧。
          不要占用与下方的间隔空间」).

          They used to be two rows in the flow, one above the drawing
          and one below it, each a `min-h-touch` row — **88px of the
          card spent on two words**, and the lower one sat between the
          figure and the table and read as the gap between them. Out of
          the flow they cost nothing, and the space really is given
          back: the block is shorter by exactly those two rows.

          **The corners are chosen, not convenient.** Top-left is min-x
          max-y and bottom-right is max-x min-y — the two places a
          scatter is least likely to put a point, and the two the ruler
          marks already occupied for the same reason. **Written across,
          not turned ninety degrees**: rotated Latin is read by tilting
          the head, and rotated 中文 is a different writing mode with its
          own rules.

          What they cost: a 44px hit area in each of those two corners
          swallows a press meant for a point that lands there. The
          corners are where points are rarest, and the alternative —
          shrinking the target — is the floor this card does not break. */}
      {/* h-44 again (2026-09-28 evening): the head strip went back to 22 when the ••• left it; it was h-50 while the strip was a finger tall
          (`PLOT.padT`, 2026-09-28) and the drawing kept its area. */}
      <div ref={box} className="relative h-44 w-full">
        <span className="pointer-events-none absolute inset-0">
          {/* The hit area is 44 tall and the strip is not, so each
              label is centred on its strip and overflows it invisibly.
              What has to land in the strip is the **ink**. */}
          <span className="pointer-events-auto absolute" style={labelSeat("y")}>
            <AxisLabel
              which="y"
              word={AXES[axes.y].word()}
              rulerWord={ry.kind === "log" ? cli.top_plot_log : cli.top_plot_sqrt}
            />
          </span>
          <span className="pointer-events-auto absolute" style={labelSeat("x")}>
            <AxisLabel
              which="x"
              word={AXES[axes.x].word()}
              rulerWord={rx.kind === "log" ? cli.top_plot_log : cli.top_plot_sqrt}
            />
          </span>
          {/* ⚠️ The ••• sat here, in the figure's top-right corner
              (`labelSeat("settings")`), until 2026-09-28: it moved to
              the right end of the processes section's heading row
              (`TopSettings`), where a section keeps its control. */}
        </span>
        {/* **Named**, because it is no longer the only `<svg>` in this
            block: the axis labels carry chevrons, and a probe reaching
            for 「the figure」 with `querySelector("svg")` picked a 12px
            chevron and reported the drawing as 12 pixels tall
            (2026-09-05, measured — the assertion was about the picker
            opening in place, and it was reading an icon). */}
        <svg data-plot-svg viewBox={`0 0 ${W} ${H}`} width={W} height={H} className="text-foreground">
          <line x1={padL} y1={H - padB} x2={W - padR} y2={H - padB} stroke="currentColor" strokeOpacity={0.15} />
          <line x1={padL} y1={padT} x2={padL} y2={H - padB} stroke="currentColor" strokeOpacity={0.15} />
          {xTicks.map(({ v, label }) => (
            <g key={`x${v}`} data-plot-tick-x={Math.round(v * 10) / 10}>
              <line x1={sx(v)} y1={H - padB} x2={sx(v)} y2={H - padB + 3} stroke="currentColor" strokeOpacity={0.3} />
              <text x={sx(v)} y={xTickY(H)} fontSize={PLOT.tick} textAnchor="middle" fill="currentColor" fillOpacity={0.5}>
                {label}
              </text>
            </g>
          ))}
          {yTicks.map(({ v, label }) => (
            <g key={`y${v}`} data-plot-tick-y={Math.round(v)}>
              <line x1={padL - 3} y1={sy(v)} x2={padL} y2={sy(v)} stroke="currentColor" strokeOpacity={0.3} />
              <text x={padL - 6} y={sy(v) + 3} fontSize={PLOT.tick} textAnchor="end" fill="currentColor" fillOpacity={0.5}>
                {label}
              </text>
            </g>
          ))}
          {placed.map(({ r, x, y }) => {
            const cx = sx(x);
            const cy = sy(y);
            return (
              <g
                key={r.pid}
                data-plot-point={r.pid}
                data-plot-x={Math.round(cx * 10) / 10}
                data-plot-y={Math.round(cy * 10) / 10}
                role="button"
                aria-label={cmdText(r)}
                onClick={() => onPoint(r.pid)}
              >

                {/* ⚠️ **Tombstone: the halo is gone, 2026-09-08.** It was
                    a ring around the dot whose radius was
                    `11 × √(显存 / 最大显存)` — the third quantity, drawn
                    as an area, in a channel of its own.

                    Two reasons, and the second is the user's:
                    **it encoded 显存**, which is null on 200 of 200 rows
                    on two of the three machines — so on those it drew
                    **nothing at all**, and 「nothing」 was exactly the
                    档 a ring can have and a dot cannot; and **it was a
                    second visual channel** for the same idea the 額度
                    card draws as the dot's own size, which is half of
                    「看起来是两个产品」.

                    ⚠️ **When it should come back:** the day 显存 is
                    reported for every process on the machines people
                    actually use, the third quantity's picker will offer
                    it on its own (`everyRowHas` decides, nobody
                    maintains a list). **Whether a separate ring is
                    wanted then is a new question, not this decision
                    carried forward** — the reason it existed was that a
                    ring can be absent, and once the quantity is never
                    absent that reason is gone.

                    turing's three rows lose their halo; 显存 is still a
                    column in the table. */}
                {/* **Nought is not the smallest.** The area's floor is
                    a dot that can still be seen, so a reading of zero
                    and the smallest reading above zero come out nearly
                    the same size — and that is the neighbouring-value
                    error again, one rung down. A row that was looked at
                    and read zero says so in a shape instead: a dashed
                    ring, which no reading draws.
                    ⚠️ It follows whichever quantity carries the area
                    (`az`), not 显存 — it was 显存's until 2026-09-08
                    because 显存 was the only third quantity. */}
                {az.of(r) === 0 && (
                  <circle
                    data-plot-zero
                    cx={cx}
                    cy={cy}
                    r={5}
                    fill="none"
                    stroke="currentColor"
                    strokeOpacity={0.35}
                    strokeDasharray="2 2"
                  />
                )}
                {/* **The target is wider than the dot, and it is the
                    same circle.** The drawn radius is a reading now
                    (`dotSpan` keeps two hundred of them readable); the
                    pointer gets a transparent stroke around it rather
                    than a second element — two hundred extra nodes on a
                    card that re-lays-out on every viewport change is a
                    cost with nothing to show for it.

                    Still far under the finger's floor, and waived by
                    name in the face gate: a scatter point cannot be
                    44px without becoming the plot. **What the waiver
                    does not buy** is a keyboard — a point cannot be
                    tabbed to — and the rows it points at are the table
                    directly below, which is the canonical list and is
                    reachable without any of this. */}
                <circle
                  data-plot-area={Math.round(dotR(r) * 100) / 100}
                  cx={cx}
                  cy={cy}
                  r={dotR(r)}
                  fill="currentColor"
                  fillOpacity={0.75}
                  stroke="transparent"
                  // At least a 24 circle to aim at (the audit fix plan
                  // F1.5), whatever the reading's radius.
                  strokeWidth={Math.max(11, 24 - 2 * dotR(r))}
                >
                  {/* **First line is the command, verbatim what the
                      table's cell says** (`cmdText`) — 2026-09-06:
                      「悬浮散点要显示命令名」. The two readings go under
                      it rather than beside it, so the first line can be
                      compared with the table character for character.

                      A native `<title>` and not a floating panel: it
                      belongs to **this circle**, so on a crowded plot
                      it answers 「指到哪个点就说哪个点」 by construction,
                      where a proximity panel would answer 「附近有个
                      点」. */}
                  <title>{`${cmdText(r)}\n${ax.tick(x, 1)} · ${ay.tick(y, 1)}`}</title>
                </circle>
              </g>
            );
          })}
        </svg>
      </div>
      <div className="flex flex-wrap items-baseline justify-between gap-row text-callout text-muted-foreground">
        <span className="flex flex-wrap items-baseline gap-in">
          {/* **Not optional.** See the head: a picture that quietly
              dropped two thirds of its rows draws the same as an idle
              machine. */}
          {off > 0 && <span data-plot-off={off}>{cli.top_plot_off(off)}</span>}
        </span>
        {/* ⚠️ **Tombstone: 「面积 = X」 left both figures on 2026-09-10**
            (「所有散点,无论机器还是api,都不要有下面的面积注释」). The
            quantity is still named — it is a row in the settings panel,
            beside X and Y — so what went is a sentence repeating on
            screen what a press already says. It was on both figures
            from one key, and it leaves both the same way. */}
      </div>
    </div>
  );
}

/* ⚠️ **Tombstone: `AxisPick` was defined here until 2026-09-08.** It is
   `components/PlotControls` now, with its reasoning, because the 額度 page's
   usage half needed the same control and had grown a different one —
   a row of native `<select>`s, 23px tall, in a strip above the figure.
   **Two controls for one idea is the thing this card's own head keeps
   warning about**, one screen further out.

   What moved is the wording only: the option list and the ruler word
   are arguments now. Every `data-plot-*` attribute and every class is
   unchanged, which is deliberate — this card's gate reads them. */

/**
 * The 「命令」 cell: **a process's short name and the one word that/**
 * The 「命令」 cell: **a process's short name and the one word that
 * identifies it, as one run of text**, with a mark when it did not fit.
 *
 * # One width, not two
 *
 * 2026-09-04:「这俩就是俩字符串,放一列是有原因的,干嘛还要单独显示宽度呢?
 * 他俩加一块共享列宽」.
 *
 * The version this replaces gave each half its own width — a floor on
 * the word, a ceiling on the name, and shrink factors deciding which
 * gave way first. **What that produced is what he saw: a name squeezed
 * out entirely and replaced by a `▪`.** A cell that cannot show what
 * the process *is* has not truncated anything, it has deleted the
 * answer; the identifying word beside it is then a word identifying
 * nothing.
 *
 * So there is one text flow and one truncation, at the end, where a
 * truncation belongs. **Neither half has a `min-width` any more**, and
 * the name comes first, so the first characters on screen are always
 * the ones that say which process this is.
 *
 * The two inks stay (name in the body ink, word in the muted one).
 * **That is semantics, not layout** — it says 「this part is the name
 * and that part qualifies it」 and it costs no width at all.
 *
 * # What may not be squeezed out
 *
 * **The pid is a track of its own and is never dropped**: on turing the
 * two vLLM workers carry the same name *and* the same identifying word
 * — `argv[0]` is `VLLM::EngineCore` for both — so the only things that
 * tell those two rows apart are the pid and the card memory.
 *
 * **When the word repeats the name it is not drawn twice.** That is the
 * vLLM case above, and printing `VLLM::EngineCore VLLM::EngineCore`
 * would spend a column's width saying one thing.
 *
 * # ⚠️ Tombstone: the bordered square is gone, and the argument for it
 * was wrong in a way the argument could not see
 *
 * There used to be a 10px bordered square at the end of a cut cell, and
 * a section here headed 「Why a mark and not an ellipsis」 arguing for
 * it: an ellipsis is punctuation *inside the text* saying 「the value
 * ends about here」, while what has to be said is 「there is more of
 * this, and you can have it」 — a statement about the cell. That
 * reasoning is sound and it is beside the point.
 *
 * 2026-09-05, from the person using this, looking at a real screen:
 * **「关键是它看着就是字体缺字呀。用户看着很难受」**. An empty box
 * immediately after text is what a missing glyph looks like — tofu — and
 * a screen that looks like a font failed to load says 「this software is
 * broken」, not 「there is more here」. **The mark was not overruled by a
 * preference; it was overruled by what it actually looked like.** It
 * meant to say one thing and delivered another, which no amount of
 * correctness about cells and strings can fix.
 *
 * The replacement needed inventing nothing: the line already wears
 * Tailwind's `truncate` (`overflow:hidden; text-overflow:ellipsis`), so
 * the ellipsis was **already being drawn** — measured on a real card,
 * the cell read `…Stable-a44adf7f…` **and then the square**. Deleting
 * the square was the whole change.
 *
 * # Why a press, and a hover as well
 *
 * **The press is the one that must not be lost.** The narrow card has
 * no hover at all, and narrow is exactly where the cell runs out of room
 * first — an answer that lives only in a tooltip is missing where it is
 * needed most. So a press opens the line in place, and the keyboard
 * reaches it the same way (`role`, `tabindex`, Enter and Space).
 *
 * The hover is **added** on top of that (2026-09-05:「命令这种显示不全
 * 的,鼠标悬浮后显示」), for the pointer case where opening the row in
 * place is more disruption than the question deserves. Radix's tooltip
 * because it is the app's existing one: it opens on **focus** as well as
 * hover, and it portals out of the table's scroll box, which an
 * absolutely-positioned panel inside `overflow: auto` cannot do without
 * being clipped.
 *
 * **Only on cells that are actually cut**, so a tooltip never repeats a
 * line the reader can already see in full.
 *
 * # The mark is measured, never guessed
 *
 * `scrollWidth` against `clientWidth` on the one flowing line, watched
 * with a `ResizeObserver`. A mark decided by counting characters would
 * appear on cells that fit (decoration) and miss cells that do not (a
 * silent truncation), and both errors look fine on the machine of
 * whoever wrote the guess.
 */
function Cut({ name, arg }: { name: string; arg: string | null }) {
  // The same string the scatter's hover says — see `cmdText`.
  const full = cmdText({ name, arg } as Load["top"][number]);
  const line = useRef<HTMLSpanElement>(null);
  const [over, setOver] = useState(false);
  const [open, setOpen] = useState(false);
  // **Not drawn twice** — see the head of this component.
  const word = arg && arg !== name ? arg : null;
  useEffect(() => {
    const node = line.current;
    if (!node) return;
    // Only while closed: open, the text wraps and nothing overflows, so
    // measuring then would retract the mark and take the way back with
    // it.
    if (open) return;
    // ⚠️ **A box with no width is not a reading.** When the mark
    // appears the cell gains a tooltip trigger around it, which
    // unmounts this line and mounts a new one — and the observer, still
    // holding the detached node, fires 0×0. Read as a measurement that
    // is 「it fits」, and the mark retracts on a cell that visibly does
    // not fit. Absent is not zero here either.
    const measure = () => {
      if (node.clientWidth === 0) return;
      setOver(node.scrollWidth - node.clientWidth > 1);
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(node);
    return () => ro.disconnect();
    // `over` is a dependency **because the answer changes the tree**:
    // the trigger wraps this line only when it is cut, so the node this
    // effect measured is not the node on screen a render later. Without
    // it the observer is left on the old one for good.
  }, [name, word, open, over]);
  const cut = over;
  const cell = (
    <span
      data-c="cmd"
      data-cut={cut ? "" : undefined}
      data-open={open ? "" : undefined}
      role={cut ? "button" : "cell"}
      tabIndex={cut ? 0 : undefined}
      aria-expanded={cut ? open : undefined}
      onClick={cut ? () => setOpen((on) => !on) : undefined}
      onKeyDown={
        cut
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setOpen((on) => !on);
              }
            }
          : undefined
      }
      // **Opening wraps inside this column; it does not span the
      //     table.** Spanning was the first version and it pushed every
      //     following cell of that row into the next grid line — the
      //     whole table below the opened cell came apart (seen in a
      //     screenshot; the assertion that the cell grew and shrank was
      //     perfectly happy, because it was true). The row grows in
      //     height instead, and the columns stay where they are.
      className="flex min-w-0 items-baseline gap-in"
    >
      {/* **One box, one line, one truncation.** The name and the word
          are two spans only so they can wear two inks; they are not two
          columns, and neither carries a width of its own. */}
      <span ref={line} data-cmd-line className={cn("min-w-0", open ? "[overflow-wrap:anywhere]" : "truncate")}>
        <span data-cmd-name>{name}</span>
        {word && (
          <>
            {" "}
            <span data-cmd-arg className="text-muted-foreground">
              {word}
            </span>
          </>
        )}
      </span>
    </span>
  );
  // **A cell that fits is not a trigger.** Wrapping every one of them
  // would hand two hundred rows Radix's pointer handlers on elements
  // that say they are not targets — which the face gate reads, rightly,
  // as a press with no way in for a keyboard.
  //
  // ⚠️ **And the wrap is downstream of a measurement, which is why the
  // two guards above exist.** The first version had this conditional
  // and nothing else: the moment `over` flipped true the line was
  // unmounted and remounted inside the trigger, the `ResizeObserver`
  // was left holding the detached node, it fired 0×0, `over` flipped
  // back — and the effect never re-ran, because none of its
  // dependencies had changed. Stuck false, on a cell that visibly did
  // not fit. Measured: **0 of 23 marked, 1 silent**, on a build whose
  // marking logic was untouched.
  //
  // **Opening does not unwrap it either**, and that is the same lesson
  // one step along: the first fix dropped the trigger while the line
  // was open, so the press replaced the element under the reader's
  // pointer and the gate measuring 「it opened」 was left holding a
  // removed node (`18 → 0 → 0`). The wrapper survives the press; only
  // the tooltip's *content* goes, because a tooltip repeating a line
  // that is already showing in full is noise.
  if (!cut) return cell;
  return (
    <Tooltip>
      <TooltipTrigger asChild>{cell}</TooltipTrigger>
      {/* **Only where there is something the reader cannot already
          see.** A tooltip that repeats a line showing in full is noise
          on two hundred rows. Wide enough to be worth opening and
          narrow enough to stay on the card; `break-all` because what
          overflows here is a path, and a path has no spaces to break
          at. */}
      {!open && (
        <TooltipContent side="top" className="max-w-[var(--tip-cmd-max)] [overflow-wrap:anywhere] whitespace-normal text-left">
          <span data-cmd-full>{full}</span>
        </TooltipContent>
      )}
    </Tooltip>
  );
}

/** One column head: the word, and whether the table is ordered by it.
    A press is what sorts, so it is a button — `aria-sort` says the same
    thing to a reader who cannot see the chevron. */
function Column({
  col,
  sort,
  asking,
  onPress,
  align,
  head,
  grow,
  children,
}: {
  col: SortCol;
  sort: Sort;
  /** The column whose answer is still crossing — see `Top`. */
  asking: SortCol | null;
  onPress: (col: SortCol) => void;
  align: "left" | "right";
  /** What this head is called **in the DOM**, when that is not the
      wire word it sorts by. One column only: 「命令」 carries a
      process's short name and its identifying word together, and it
      orders by the name — which is the text that column starts with,
      so 「order this column」 and 「order by the name」 are the same
      request. The head is `cmd` because the column is, and the sort is
      `name` because that is the word `khor_core::TopBy` knows. */
  head?: string;
  /** The one column that takes the leftover width, where there is any
      to take — narrow only, and see `Top` for why. */
  grow?: boolean;
  children: React.ReactNode;
}) {
  const on = sort.col === col;
  // **`aria-sort` describes the rows, and the rows have not changed
  // yet.** It used to turn over on the press, which is a promise about
  // an answer still crossing the wire — and to a reader who cannot see
  // the table, a promise is all there is. `data-asking` is the only
  // thing a press moves.
  const waiting = asking === col;
  return (
    // A grid cell, so a `div` with the header's role: a `<th>` outside a
    // table's rows is invalid markup and React says so on every render.
    <div
      role="columnheader"
      data-top-head={head ?? col}
      data-asking={waiting ? "" : undefined}
      aria-sort={on ? (sort.dir === "up" ? "ascending" : "descending") : "none"}
      aria-busy={waiting || undefined}
      className={cn(
        "rounded-xs font-body",
        // **No padding of its own.** It carried `px-row` on the
        // right-aligned heads, which inset the word 8px from the edge
        // its own numbers are flush against — the head and its column
        // did not line up, and the 8px was also what pushed 「CPU」 out
        // of a 48px track (it rendered 「CPL」). Alignment and room, one
        // deletion.
        align === "right" ? "text-right" : "text-left",
        grow && "w-full",
        // Not a new visual language — the head simply goes quiet while
        // its answer is out. What this looks like finally is the design
        // lane's call.
        waiting && "opacity-60",
      )}
    >
      <button
        type="button"
        onClick={() => onPress(col)}
        // The floor on both axes (tokens.css `--touch`): the ink here
        // is two characters, and a head that is 44 tall and 25 wide is
        // a target you miss sideways. `w-full` spreads the press over
        // the whole column head rather than over the word alone, and
        // the floor is what the narrow columns grow to.
        className={cn(
          "inline-flex min-h-touch min-w-touch w-full items-center gap-in hover:text-foreground",
          align === "right" ? "flex-row-reverse justify-start" : "justify-start",
          on && "text-foreground",
        )}
      >
        {children}
        {on && (sort.dir === "up" ? <IconChevronUp className="size-affordance" /> : <IconChevronDown className="size-affordance" />)}
      </button>
    </div>
  );
}

/** The order a column press asks for.
 *
 * **Absence sinks in both directions.** A process khor could not read a
 * GPU figure for is not a process using none of the GPU, so it may not
 * take the place a zero would: reversing the sort would otherwise turn
 * 「没看到」 into 「最小的那个」, which is the same neighbouring-value
 * mistake the readings are built to avoid.
 */
function sortRows(rows: Load["top"], sort: Sort): Load["top"] {
  const dir = sort.dir === "up" ? 1 : -1;
  const num = (r: Load["top"][number]): number | null => {
    switch (sort.col) {
      //   **The pid is a number here too**, and leaving it out was a
      //   bug rather than an omission: every column that falls through
      //   to `null` ends at the tiebreak, and **the tiebreak is not
      //   turned over by the direction** — so pressing PID a second
      //   time drew the arrow the other way and left the rows in
      //   ascending order. It was invisible against a real node, which
      //   re-ranks and sends the rows back already reversed; it is what
      //   the reader sees in the second between the press and that
      //   answer, and it is what a stand-in that ignores the ask shows
      //   the whole time. Found 2026-09-05 by asking every head to read
      //   differently both ways.
      case "pid":
        return r.pid;
      case "cpu":
        return r.cpu_pct;
      case "mem":
        return r.mem;
      case "gpu":
        return r.gpu_pct;
      case "vram":
        return r.gpu_mem;
      case "time":
        return r.run_secs;
      case "cputime":
        return r.cpu_ms;
      case "io":
        return r.io_bytes;
      default:
        return null;
    }
  };
  // **状态 groups; it does not rank** — there is no order in which 僵尸
  // is greater than 休眠 that is not somebody's opinion. What a press
  // is for is 「把不正常的那几行凑到一块」, and it sorts on the **token**
  // rather than the word so a clump cannot split along a translation —
  // the same key `khor_core::TopBy::Status` uses on the machine, so the
  // order here and the order there are one order.
  const text = (r: Load["top"][number]) =>
    sort.col === "user" ? r.user : sort.col === "status" ? (r.status ?? "") : r.name;
  return [...rows].sort((a, b) => {
    if (sort.col === "name" || sort.col === "user" || sort.col === "status") {
      return dir * text(a).localeCompare(text(b)) || a.pid - b.pid;
    }
    const [x, y] = [num(a), num(b)];
    if (x === null && y === null) return a.pid - b.pid;
    if (x === null) return 1;
    if (y === null) return -1;
    return dir * (x - y) || a.pid - b.pid;
  });
}

/**
 * The one ask behind both the dials' 24-hour marks and the chart.
 *
 * Its cadence is the vitals beat the dials already move on; a refusal
 * backs off, because a far machine that is away costs a dial each time
 * and the sentence on the card does not get truer by being re-fetched
 * every five seconds.
 */
function useLoad(row: DeviceRow | null, sort: Sort) {
  // **Over the wire every beat**, so a window nobody is looking at asks
  // at the hidden beat (round NET-2, piece 6): a phone in a pocket was
  // still asking a far machine for its top every five seconds. Coming
  // back restarts the beat, which asks at once.
  const hidden = useHidden();
  // **Kept per machine across the card's unmount** (`lib/kept`,
  // docs/SHELL.md 第十六轮): coming back to the machine page draws the
  // last answer in the first frame and asks again in place. Before, the
  // card started at null on every mount and was empty until the ask came
  // back — every return to the page was a blank card first.
  const keyOf = row?.id ?? "";
  const [load, setLoad] = useKept<Load | null>(`machine.load:${keyOf}`, null);
  const [why, setWhy] = useState<string | null>(null);
  // **Whether this machine's khor puts an age on a process — a fact
  // about the far side, kept here rather than read off the rows on
  // screen.**
  //
  // Read off the rows it is a fact about *this page*, and the page is
  // whatever the current ordering cut: press a column and the answer
  // can come back without a single filled reading in it, and the column
  // vanishes under the finger that just pressed it — the bug the person
  // using this named as 「一点就没有 GPU 这列了」. So it latches: once a
  // machine has shown one, it can report them, and no later ordering
  // unsays it. It only ever goes up, and `id` below clears it when the
  // machine changes.
  //
  // 2026-09-05:「A column the machine cannot fill does not exist」 was
  // once asked of the *machine* — 「does it report video memory」 — and
  // that is the wrong level. turing reports two RTX A5000s and **not
  // one of its two hundred processes carries a per-process figure**, so
  // it drew two empty columns and spent 136px on them. The question a
  // column has to answer is 「can any row fill me」.
  //
  // **What this reports is 「seen」, not a property of the hardware or
  // of the far khor's version.** A machine whose only card process
  // starts after the card is opened gains the columns then and keeps
  // them; one that never runs anything on its cards never grows them.
  // That is a weaker claim than the old one and it is the true one.
  //
  // **One object rather than one state per column**, since 2026-09-05
  // took it from three to seven: seven `useState`s, seven resets and
  // seven merge lines is seven chances for the reset and the merge to
  // disagree about which columns exist.
  const [seen, setSeen] = useKept<Seen>(`machine.seen:${keyOf}`, SEEN_NONE);
  // **Which order the rows on screen are actually in.**
  //
  // Not the same thing as the order somebody just asked for, and the
  // gap between the two is where a real bug lived (measured
  // 2026-09-04): pressing PID re-sorted the rows already on screen
  // while the true answer was still crossing, so the table showed **the
  // busiest 200 re-sorted by pid** — `[10, 11, 25, 31, 43, 386, 410,
  // 422]` where the machine's answer is `[1, 2, 3, 4, 6, 9, …]` — and
  // then silently became the real thing a poll later. That is the
  // 「重排冒充排名」 this file's own comment warns about, and it is
  // worse than a blank: a blank says 「还没好」, while this says
  // 「好了」 and is wrong, for up to five seconds, looking exactly like
  // a true answer.
  //
  // So the rows are drawn in the order of the answer that produced
  // them, and this is that order. `null` until the first one lands.
  const [shown, setShown] = useKept<Sort | null>(`machine.shown:${keyOf}`, null);
  const id = row?.id ?? null;
  const me = row?.me ?? false;
  const name = row?.name ?? "";
  // **Clearing belongs to the machine changing, not to the column.**
  // Pressing 内存 used to blank the chart, the table and everything
  // under them for one round trip — because the answer was dropped
  // while the new one was in the air, and the day chart is drawn from
  // the same answer. The day did not change; only the order did. So
  // the old answer stays on screen until the new one lands, and the
  // only thing that empties the card is picking a different machine.
  //
  // (Since 第十六轮 the three answers are kept per machine, so a new
  // machine switches to *its* last answer rather than to nothing; only
  // the refusal is this mount's own.)
  useEffect(() => {
    setWhy(null);
  }, [id]);
  useEffect(() => {
    if (id === null) return;
    let stopped = false;
    let timer = 0;
    // **Whether this is the ask a press caused.** The effect restarts
    // when the ordering changes, so the first ask of a run is the one
    // somebody asked for and every ask after it is the beat — see where
    // this is read, below.
    let pressed = true;
    const ask = async () => {
      let wait = hidden ? HIDDEN_MS : LOAD_MS;
      const reorder = pressed;
      pressed = false;
      try {
        // A name, not an id: the node resolves machines by the name its
        // table knows them by, and its own by being asked for itself
        // (`Node::load_on`).
        // **The ranking travels; the rows do not.** turing answered
        // with 10 175 processes the first time this asked for all of
        // them — 800 KB every five seconds to show nine. So the column
        // goes over with the ask and the machine ranks before it cuts
        // (`khor_core::TopBy`), which makes「按显存排的前 200」a true
        // answer rather than a re-sort of the busiest 200 by CPU.
        const l = await fetchLoad(me ? "" : name, TOP_LIMIT, sort.col, sort.dir === "down");
        if (stopped) return;
        // **A press re-orders the table and touches nothing else.**
        // 2026-09-04:「表点一些 sorting 方式,趋势和 CPU、GPU、内存都在
        // 变。不应该这样」.
        //
        // One ask carries both the rows and the day, so the answer to
        // 「order these by memory」 arrives with a *newer day* attached
        // — a different `now_ms`, and with it a redrawn chart. Nothing
        // about the last 24 hours changed; the reader asked for the
        // rows to move and the figure under them moved too.
        //
        // Measured against a control, because 「it changed after I
        // clicked」 and 「it changes anyway」 look identical on a live
        // machine: five presses rebuilt the chart 5/5 times, five waits
        // of the same length without a press rebuilt it 1/5 — the beat.
        // (**The dials came out 3/5 either way**, which is how it was
        // settled that their numbers moving is the beat and not the
        // press. They are drawn from `row.vitals`, which this ask never
        // touches.)
        //
        // So a press keeps the day already on screen, and the next beat
        // brings a new one. There is no old day to keep on the first
        // ask for a machine, and none is wanted — `id` cleared it.
        setLoad((was) => (reorder && was ? { ...l, cpu_day: was.cpu_day } : l));
        setShown({ col: sort.col, dir: sort.dir });
        setWhy(null);
        // **Per column, never as a group**: they are separate
        // readings and a machine answers some and not others — a
        // process can hold card memory with no utilisation figure and
        // the other way round, and an older khor sends four of these
        // seven and nothing else.
        //
        // The same object comes back when nothing was learned, so a
        // beat that teaches this nothing does not re-render the table.
        setSeen((was) => {
          const next = { ...was };
          let grew = false;
          for (const k of SEEN_KEYS) {
            if (!next[k] && l.top.some(FILLS[k])) {
              next[k] = true;
              grew = true;
            }
          }
          return grew ? next : was;
        });
      } catch (e: unknown) {
        if (stopped) return;
        setWhy(String((e as { message?: string })?.message ?? e));
        wait = LOAD_RETRY_MS;
      }
      if (!stopped) timer = window.setTimeout(() => void ask(), wait);
    };
    void ask();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, [id, me, name, sort?.col, sort?.dir, hidden]);
  // `asking` is the column somebody pressed whose answer has not landed
  // yet — the head wears it, and nothing else moves until the rows do.
  const asking = shown === null || shown.col !== sort.col || shown.dir !== sort.dir ? sort.col : null;
  return { load, why, shown, asking, seen };
}

/**
 * The machine's last day — **the prototype's panel, mounted, not
 * re-drawn.**
 *
 * Everything inside the box below is built by `day/day-core.js`, which
 * is `card-whole.html`'s `KhorDay` moved across whole. This component
 * owns the box, the heading, and the one call that hands the module a
 * new answer. It owns no geometry, no scale, no colour and no cursor —
 * those are the module's, and that is the point: there is no second
 * implementation to keep in step.
 *
 * **The thing this replaced was a React re-implementation written from
 * the spec's description of the same drawing**, and everything that was
 * wrong with it came out of that gap — a profile drawn with the
 * vertical-hair fill the design had measured and thrown away, the
 * parallel coordinates with no axes, sizes and weights off by whatever
 * the prose did not pin. 「artifacts 已经写好代码画了,为什么还要自己画
 * 一遍」.
 *
 * The heading stays here because it is the card's, not the figure's:
 * the two ends of the window are the same fact the dials above are read
 * against.
 */
function DayChart({ day }: { day: Load["cpu_day"] }) {
  const box = useRef<HTMLDivElement>(null);
  const panel = useRef<DayPanel | null>(null);
  // **Every word the drawing says, from the catalog.** The module holds
  // no Chinese; this is where the two meet.
  const words: DayWords = {
    title: cli.day_title,
    cpu: cli.vitals_name_cpu,
    gpu: cli.vitals_name_gpu,
    mem: cli.vitals_name_mem,
    disk: cli.vitals_name_disk,
    vram: cli.vitals_name_vram,
    heat: cli.day_view_heat,
    para: cli.day_view_para,
    gap: cli.day_gap,
    now: cli.day_now,
    strainLegend: cli.day_strain_legend(90),
    peakAt: (pct, at) => cli.day_peak_at(pct, at),
    brushHead: (what, n, hours) => cli.day_brush_head(what, n, hours),
    brushMedian: (name, pct) => cli.day_brush_median(name, pct),
    brushNone: (name) => cli.day_brush_none(name),
    brushEmpty: (name, band) => cli.day_brush_empty(name, band),
  };
  // **The reading's picker is the settings rows' popup button** (2026-10-02,
  // the owner: 「做成和设置页面一致的，上下箭头，默认是 log，点击后弹出的菜单
  // 可切换到 Stats」), seated by a portal at the right end of the
  // section's head row, which the module builds and keeps
  // (`[data-day-switch-seat]`). The module draws; this chooses.
  const [view, setView] = useState<"heat" | "para">("heat");
  const [seat, setSeat] = useState<HTMLElement | null>(null);
  useEffect(() => {
    const node = box.current;
    if (!node) return;
    panel.current = mountDay(node);
    setSeat(node.querySelector<HTMLElement>("[data-day-switch-seat]"));
    return () => {
      panel.current?.destroy();
      panel.current = null;
      setSeat(null);
    };
  }, []);
  // **A new answer redraws; it does not remount.** The reading somebody
  // picked and the moment they left the cursor on live inside the
  // module, and tearing it down every five seconds would take both.
  //
  // ⚠️ **A new *answer*, not a new render.** This effect had no
  // dependency list, so it ran after every render of this component —
  // and a machine card re-renders for a dozen reasons that have nothing
  // to do with the day. Each run rebuilt the whole figure. Measured
  // 2026-09-04 on the real face: eleven of sixteen one-second samples
  // caught a fresh node set, most of them mid-fade. The signature below
  // is what a redraw is *for*: the window moved, or its shape changed.
  // `day` itself cannot be the dependency — it is parsed fresh from the
  // wire on every poll, so its identity changes even when it says the
  // same thing.
  const answer = `${day.now_ms}/${day.bucket_ms}/${day.buckets.length}`;
  useEffect(() => {
    panel.current?.update(day, words);
    // `day` and `words` are read, not depended on: the signature above
    // is the dependency, and `words` is the same catalog every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [answer]);
  // **No heading, and no window written out.** It used to carry
  // 「24 小时趋势」 and 「昨天 15:34 → 15:34」 above the figure;
  // 2026-09-04 both went, for two different reasons.
  //
  // The window is **drawn on the axis** — hour marks every three hours
  // and 「现在」 at its right end — so printing it again was the same
  // fact said twice, and the second telling is the one that goes stale
  // in somebody's memory. The heading went because the slot this block
  // now lives in has one line at the top for whatever it is showing,
  // and for the figure that line is the three reading words. A title
  // there would push them out or push the figure down.
  //
  // What that buys is the thing the merge is for: **the reading words
  // do not vanish and come back** when a ring is pressed. They are
  // replaced, in place, by the drawer's own heading — one line, one
  // position, one height, so nothing moves.
  return (
    <>
      <div ref={box} data-day-band-axis className="h-full w-full" />
      {seat &&
        createPortal(
          <PopupButton
            label={cli.day_title}
            value={view}
            choices={[
              { value: "heat", label: cli.day_view_heat, mark: { "data-day-view-pick": "heat" } },
              { value: "para", label: cli.day_view_para, mark: { "data-day-view-pick": "para" } },
            ]}
            onPick={(v) => {
              const next = v === "para" ? "para" : "heat";
              setView(next);
              panel.current?.setView(next);
            }}
            mark={{ "data-day-view": view }}
          />,
          seat,
        )}
    </>
  );
}

/**
 * One dial: an arc with its share inside, its name under it, and the
 * quantities behind the share on its back.
 *
 * **Only the arc turns** (2026-09-03:「翻转只翻转环,下面的标题保留」).
 * The name is what a person is looking for while scanning four of
 * these, and a name that vanishes mid-turn makes them wait for the
 * animation to know which reading they are reading.
 *
 * **What the back says is what the front cannot.** The front is a share
 * — 96% — and the back is the quantity a person acts on: 还剩 51.6G, the
 * day's high-water mark, how many cards or volumes the whole is made
 * of. Under those sits the sentence `khor devices` prints, which is
 * both the provenance of the numbers above it and what smoke compares
 * word for word.
 *
 * The unit and the sentence's mark ride together only when this dial
 * *is* a reading — see `Readings` on why an aggregate over parts is
 * not.
 */
function Dial({
  dial,
  open,
  onOpen,
  name,
  word,
  facts,
  sentence,
  fraction,
  strain,
  unknown,
  pips,
  size,
  stroke,
}: {
  /** **Which reading this dial is, for anything that has to find it.**
      Separate from `name`, which is the key on the wire and is absent
      on the dials whose parts are counted elsewhere — leaving those two
      unaddressable, which is how a gate came to measure the wrong
      element (2026-09-04). Every dial has one of these. */
  dial: string;
  /** Whether this dial's drawer is the one showing. */
  open: boolean;
  /** **How big to draw, decided by the row and not by this dial**
      (2026-09-09:「要铺平宽度」). Handed down rather than read from a
      constant so that four dials on one row cannot disagree about it —
      `Vitals` divides the row and every dial gets the same share. */
  size: number;
  stroke: number;
  onOpen: (dial: string) => void;
  /** The reading's key on the wire, or none for a dial that is a
      picture of readings counted elsewhere (the GPU and disk dials
      above their per-part rows). Without it the unit and detail marks
      are not set. */
  name?: string;
  /** The reading's name, under the dial and never turning with it. */
  word: string;
  /** One or two lines of what the share is made of. `null` entries are
      dropped, so a caller may pass a fact that does not exist yet (a
      machine with no day recorded has no 24-hour mark). */
  facts: (React.ReactNode | null)[];
  /** What `khor devices` prints for this reading, word for word. */
  sentence: string;
  /** `null` when there is no denominator to divide by, and then only the
      track is drawn — a full circle of arc would read as "this is at
      100%", and no circle at all as "this is at zero". */
  fraction: number | null;
  /** The word gui-core minted, absent for the units that carry none
      (CPU, GPU) as much as for a reading below the first step. */
  strain?: Strain | null;
  /** This reading could not be taken at all, as opposed to being zero. */
  unknown?: boolean;
  /** The parts this reading is made of — cards, volumes — each marked
      on the rim at its own reading. **Only where parts exist**: the
      processes under CPU and memory are occupants, not parts (`Face`
      carries why). */
  pips?: Pip[];
}) {
  // A percentage, because the arc is one and the two should agree at a
  // glance. The exact quantity — bytes, cores, cards — is the back's job.
  const value = fraction === null ? null : `${Math.round(fraction * 100)}%`;
  const lines = facts.filter((f) => f !== null && f !== undefined);
  return (
    <div className="flex flex-col items-center gap-in">
      <Tooltip>
      <TooltipTrigger asChild>
      <button
        type="button"
        data-metric
        data-dial={dial}
        data-open={open}
        data-vitals-unit={name}
        data-vitals-unknown={unknown ? "" : undefined}
        aria-expanded={open}
        // The sentence, not "CPU" — a name that reads out the short form
        // would make a screen reader turn the card over for the number.
        aria-label={sentence}
        // **A toggle, not an open.** Pressing the dial that is already
        // showing closes it — anything else makes the press do nothing
        // when it is already on, which reads as a stuck control.
        onClick={() => onOpen(open ? "" : dial)}
        // **Answers the pointer like every pressable thing** (W-V12): the
        // shared hover fill and press (`data-glass-hit`), and a tooltip
        // saying what the press does.
        data-glass-hit
        className="rounded-lg"
        /* ⚠️ **Tombstone: this button carried
           `style={{ width: RING_SIZE_CARD * 1.6, height: RING_SIZE_CARD }}`
           until 2026-09-08.** Its reason was 「the back has to fit a
           sentence」 — and that back is `sr-only` now (the press opens
           the drawer), so it occupies nothing at all: measured that
           day, `position: absolute`, 1×1.

           **But removing it changed no pixel, and that is the part
           worth writing down.** Both numbers were duplicates of what
           `Face` already computes: its svg is `width={size * ratio}`
           with `ratio` defaulting to `BOX_RATIO` = 1.6, and
           `height={boxH}` which is `size` when uncropped. **Two mirrors
           of one number, with nothing keeping them equal** — they
           agreed by coincidence, and a third edit to either side would
           have moved the box without moving the drawing.

           The 52.8px of air per dial is real and it is **`BOX_RATIO`'s**,
           not this line's. Whether that ratio still earns its width is
           a separate question, and it is `Face`'s to answer. */
      >
        <span data-metric-faces className="block h-full w-full">
          <span data-metric-face="front" className="grid h-full w-full place-items-center">
            {/* **`crop`, since 2026-09-08.** Uncropped, the drawing's
                box is `size` tall (88) while the arc's ink stops at
                60.6 — it sweeps to 15°, not to the bottom of its own
                circle. Measured that day: **27.4px of empty box under
                the ink**, and the ring's name sits below that plus
                `gap-in`, so the gap a reader sees between an arc and
                its word was 35.4 where 8 was intended (the user:
                「环和下方的标题艰巨太远了」).

                Cropping is what the 額度 card's dial already did — this
                one was the odd one out, so this is a unification as
                much as a fix. `crop` reads the same `reachOf` the
                drawing does, so the box follows the ink at any size. */}
            <Face
              fraction={unknown ? null : fraction}
              strain={strain}
              pips={pips}
              joinPips
              unknown={unknown}
              size={size}
              stroke={stroke}
              crop
              /* **And cut sideways too** (`hugRatio`, 2026-09-08). The
                 default box is `size × BOX_RATIO` — 1.6, which left
                 52.8px of empty box on each dial at size 88. Measured
                 that day at the 1120 posture: the boxes sat the stated
                 18 apart while **the arcs sat about 90 apart**, because
                 each box carried its own slack. Cutting the width the
                 way `crop` cuts the height is what makes the stated gap
                 the gap a reader sees.
                 **The default is not touched**: it decides the box for
                 every caller that does not say, and that is a set we
                 cannot see. This one says. */
              ratio={hugRatio(size, stroke)}
            >
              {value === null || unknown ? "—" : value}
            </Face>
          </span>
          {/* **The attribute marks the sentence itself**, not the box
              around it: it is what `khor devices` prints word for word,
              and it is compared as text — so it has to name exactly the
              sentence and not a box that also holds the quantities. */}
          {/* **Every sentence stays in the document.** This used to be
              the dial's other face, turned to by pressing; the press
              opens the drawer now, and the sentences would have gone
              with the flip. They cannot: `smoke.mjs` counts the card's
              readings against what `khor devices` prints, word for
              word, so dropping them makes the card disagree with the
              terminal. Read aloud, not drawn. */}
          <span data-metric-face="back" className="sr-only">
            {lines.map((line, i) => (
              <span key={i} className="text-callout tabular-nums">
                {line}
              </span>
            ))}
            <span
              data-vitals-detail={name}
              data-strain={strain ?? undefined}
              className="text-callout tabular-nums text-muted-foreground data-[strain]:text-strain data-[strain=critical]:font-label"
            >
              {sentence}
            </span>
          </span>
        </span>
      </button>
      </TooltipTrigger>
      <TooltipContent side="top">{open ? gui.metric_close : gui.metric_open(word)}</TooltipContent>
      </Tooltip>
      {/* **The open ring's name says so, in weight and ink and nothing
          else** (2026-09-04:「开着抽屉时那个环要看得出是选中的」). The
          slot under these four belongs to whichever one is open, and
          without a mark on the ring the block below is a set of rows
          with no stated owner.

          No ground, no border, no capsule: the same two channels the
          24-hour block's reading words use and the rail's own rule —
          a plate says 「a place」, and this is one place looked at from
          four sides. The word and not the arc, because the arc is a
          reading and a reading may not be dimmed to mean something
          about the interface. */}
      <span data-dial-word data-on={open ? "true" : "false"} className={cn("text-callout", open && "font-title text-foreground")}>
        {word}
      </span>
    </div>
  );
}

/**
 * Bytes as a person reads them: 1024-based, one decimal, no space.
 *
 * The CLI has its own copy of this (`crates/cli` `bytes`), deliberately:
 * formatting is painting, not judgment. The wire carries bytes, nothing
 * depends on the two producing identical characters, and a library
 * handing back strings would be deciding what a screen it cannot see has
 * room for.
 */
/** How a volume is **named on the card**: its mount point as the
    machine spells it (2026-09-03:「就写 /Volumes/disk0 就可以」), with
    one substitution — macOS mounts the volume everything a person
    touches lives on at `/System/Volumes/Data`, and nobody calls it
    that. Every other mount keeps every character, because the path is
    what a person picks a volume by and the last step alone (`disk0`)
    threw away where it is. */
function mountWord(mount: string): string {
  return mount === "/System/Volumes/Data" ? "/" : mount;
}

/** A volume's short name **in the sentence `khor devices` prints** —
    `/System/Volumes/Data` and `/` both draw as `/`, the rest keep their
    last step. The CLI keeps a copy of its own (`vol_word`), the `bytes`
    precedent below: painting is not judgment, and the wire carries the
    honest mount. */
function volWord(mount: string): string {
  if (mount === "/System/Volumes/Data" || mount === "/") return "/";
  const steps = mount.split("/").filter(Boolean);
  return steps[steps.length - 1] ?? mount;
}

/** `d` is the decimals in the unit chosen — one by default, more only
    where an axis's ticks need them to stay apart (`tickSet`). */
function bytes(n: number, d = 1): string {
  const units = ["B", "K", "M", "G", "T"];
  let v = n;
  let u = 0;
  while (v >= 1024 && u + 1 < units.length) {
    v /= 1024;
    u += 1;
  }
  return u === 0 ? `${Math.round(n)}${units[0]}` : `${v.toFixed(d)}${units[u]}`;
}
